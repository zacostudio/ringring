// 링의 상태와 판정 — 언제 뜨고, 어느 칸이 켜지고, 키를 놓으면 무엇을 하는가
//
// 상태는 셋이다.
//
//   숨음 ──단축키 누름──> Holding
//   Holding ──놓음, 칸 위──────────────> 실행 → 숨음
//   Holding ──놓음, 가운데, 220 ms 안──> Open
//   Holding ──놓음, 가운데, 그 뒤──────> 숨음
//   Holding ──바깥 반지름 넘음, 하위 링 칸─> Holding (하위 링)
//   Open ──클릭·키로 칸 선택──> 실행 → 숨음   (하위 링 칸이면 Open 하위 링)
//   Open ──Esc·포커스 잃음·같은 단축키──> 숨음
//   (확인을 묻는 칸) ──> Confirming ──실행 / 취소──> 숨음
//
// **판정은 여기서 한다. webview 는 그리기만 한다.** hold 에서 커서는 링 창 밖으로 나가는데, webview 는 창 밖의
// 마우스 이동을 받지 못한다. 그래서 커서는 이 모듈이 8 ms 마다 읽는다. 칸이 바뀔 때만 webview 에 알린다.
//
// **키를 놓는 순간은 두 길로 받는다.** 전역 단축키 plugin 의 `Released` 와, polling 이 읽는 키 상태다.
// 먼저 온 쪽이 이긴다. 둘째 길이 있는 이유는 조합의 수식키를 먼저 떼는 사람이 있기 때문이다.
//
// 창을 만지는 일은 main thread 에서 한다. 전역 단축키 콜백은 main thread 에서 온다. polling thread 와
// async command 는 `run_on_main_thread` 로 넘긴다.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::Shortcut;

use super::geometry::{self, OUTER_RADIUS, Rect};
use super::model::{MAX_DEPTH, Refusal, Ring, RingAction, Slot};
use super::{actions, platform};
use crate::constants::{events, window_labels};
use crate::ui::ring_window;

/// 이 시간 안에 놓고 커서가 가운데에 있으면 링이 남는다 (tap).
const TAP_WINDOW: Duration = Duration::from_millis(220);
/// 커서와 키 상태를 읽는 간격.
const POLL_INTERVAL: Duration = Duration::from_millis(8);
/// 누른 채로 이 시간이 지나면 스스로 닫는다. 놓음을 영영 못 받는 경우의 안전판이다.
const HOLD_TIMEOUT: Duration = Duration::from_secs(30);
/// 커서를 링의 중심으로 옮기지 못했을 때, 커서가 이만큼 움직인 뒤에야 칸을 켠다.
const ARM_DISTANCE: f64 = 12.0;
/// 커서가 이만큼도 안 움직였으면 움직이지 않은 것으로 본다.
const CURSOR_EPSILON: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
	Holding,
	Open,
	Confirming,
}

impl Mode {
	fn as_str(self) -> &'static str {
		match self {
			Self::Holding => "hold",
			Self::Open => "open",
			Self::Confirming => "confirm",
		}
	}
}

struct Session {
	/// 띄울 때마다 새 값이다. 늦게 도착한 polling 결과가 다음 링을 건드리지 않게 한다.
	generation: u64,
	mode: Mode,
	/// 이 링을 띄운 단축키와 그 링의 id. 명령으로 띄운 링은 없다.
	trigger: Option<(String, Shortcut)>,
	/// 누른 순간에 키 상태를 읽을 수 있었는가. 못 읽으면 polling 은 놓음을 판정하지 않는다.
	key_state_usable: bool,
	/// 맨 위 링부터 지금 보이는 링까지.
	stack: Vec<Ring>,
	/// 지금 링의 중심. 전역 화면 좌표, 논리 픽셀.
	center: (f64, f64),
	hovered: Option<usize>,
	started: Instant,
	last_cursor: (f64, f64),
	/// 커서가 여기서 [`ARM_DISTANCE`] 만큼 움직일 때까지 칸을 켜지 않는다. 다 움직였으면 `None`.
	arm_origin: Option<(f64, f64)>,
	/// 확인을 기다리는 칸.
	confirm: Option<Slot>,
	/// 누르고 있는 동안 OS 가 되풀이해 보낸 누름의 수. 놓을 때 로그에 남긴다.
	repeat_presses: u32,
	/// 키보드를 끝까지 받지 않는다. 개발용 확인에서만 켠다 — 맨 앞 앱의 입력을 가져가지 않는다.
	quiet: bool,
}

impl Session {
	fn ring(&self) -> &Ring {
		self.stack.last().expect("a session always has a ring")
	}

	fn offset(&self, cursor: (f64, f64)) -> (f64, f64) {
		(cursor.0 - self.center.0, cursor.1 - self.center.1)
	}
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);
static RINGS: RwLock<Vec<Ring>> = RwLock::new(Vec::new());
static GENERATION: AtomicU64 = AtomicU64::new(0);

fn session() -> std::sync::MutexGuard<'static, Option<Session>> {
	SESSION.lock().unwrap_or_else(|e| e.into_inner())
}

// ── 링 목록 ────────────────────────────────────────────────────────────

/// 저장된 링 전체를 넣는다. 시작할 때와 설정에서 링을 고칠 때마다 부른다.
///
/// 누르는 순간에 DB 를 읽지 않으려고 메모리에 둔다 — 누른 뒤 링이 보일 때까지가 짧아야 한다.
pub fn set_rings(rings: Vec<Ring>) {
	*RINGS.write().unwrap_or_else(|e| e.into_inner()) = rings;
}

pub fn rings() -> Vec<Ring> {
	RINGS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

fn find_ring(id: &str) -> Option<Ring> {
	RINGS
		.read()
		.unwrap_or_else(|e| e.into_inner())
		.iter()
		.find(|ring| ring.id == id)
		.cloned()
}

// ── webview 에 보내는 모양 ─────────────────────────────────────────────

/// 페이지가 그릴 칸. 링을 그릴 때는 동작의 내용(셸 명령 등)을 보내지 않는다. 내용이 가는 것은
/// 확인 물음뿐이다 ([`ConfirmView::detail`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotView {
	pub position: usize,
	pub label: String,
	pub icon: String,
	/// 하위 링을 여는 칸인가. 바깥 가장자리에 chevron 을 그린다.
	pub opens_ring: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RingView {
	pub id: String,
	pub name: String,
	pub slot_count: usize,
	pub slots: Vec<SlotView>,
}

/// 실행 전에 묻는 내용.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmView {
	pub label: String,
	pub icon: String,
	/// 무엇을 실행하는지. 셸 명령이면 그 명령이다.
	pub detail: String,
}

/// `ring-show` 의 payload 이고 `ring_current` 의 답이다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowPayload {
	/// 띄울 때마다 자란다. 페이지는 더 작은 값을 버린다.
	pub seq: u64,
	pub ring: RingView,
	pub hovered: Option<usize>,
	/// `hold` / `open` / `confirm`.
	pub mode: &'static str,
	/// 1 이 맨 위 링이다. 2 부터 가운데에 뒤로 아이콘을 그린다.
	pub depth: usize,
	pub confirm: Option<ConfirmView>,
}

fn ring_view(ring: &Ring) -> RingView {
	RingView {
		id: ring.id.clone(),
		name: ring.name.clone(),
		slot_count: ring.slot_count,
		slots: ring
			.slots
			.iter()
			.filter(|slot| slot.position < ring.slot_count)
			.map(|slot| SlotView {
				position: slot.position,
				label: slot.label.clone(),
				icon: slot.icon.clone(),
				opens_ring: matches!(slot.action, RingAction::OpenRing { .. }),
			})
			.collect(),
	}
}

fn confirm_view(slot: &Slot) -> ConfirmView {
	ConfirmView {
		label: slot.label.clone(),
		icon: slot.icon.clone(),
		detail: actions::describe(&slot.action),
	}
}

fn payload(session: &Session) -> ShowPayload {
	ShowPayload {
		seq: next_seq(),
		ring: ring_view(session.ring()),
		hovered: session.hovered,
		mode: session.mode.as_str(),
		depth: session.stack.len(),
		confirm: session.confirm.as_ref().map(confirm_view),
	}
}

static SEQ: AtomicU64 = AtomicU64::new(0);
/// 마지막으로 링을 띄운 `(seq, 그 시각)`. 페이지가 그렸다고 알리면 걸린 시간을 로그에 남긴다.
static LAST_OPENED: Mutex<Option<(u64, Instant)>> = Mutex::new(None);

fn next_seq() -> u64 {
	SEQ.fetch_add(1, Ordering::Relaxed) + 1
}

/// 지금 보여 줄 것. 링이 숨어 있으면 `None`. 페이지가 뜰 때 한 번 묻는다 — listener 를 걸기 전에
/// `ring-show` 가 나갔을 수 있다.
pub fn current() -> Option<ShowPayload> {
	session().as_ref().map(payload)
}

// ── 순수 판정 ──────────────────────────────────────────────────────────

/// hold 에서 키를 놓았을 때 할 일.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReleaseOutcome {
	/// 그 칸을 고른다.
	Choose(usize),
	/// 링을 남긴다 (tap).
	StayOpen,
	Cancel,
}

/// 키를 놓은 순간의 커서 자리(`dx`, `dy`)와 누르고 있던 시간으로 정한다.
///
/// 칸 위면 그 칸이다 — 220 ms 안의 빠른 flick 도 실행이다. 빈 칸 위면 취소다.
/// 가운데면 220 ms 안일 때만 링이 남는다.
fn decide_release(dx: f64, dy: f64, filled: &[bool], held: Duration) -> ReleaseOutcome {
	match geometry::hit(dx, dy, filled.len()).slot {
		Some(slot) if filled[slot] => ReleaseOutcome::Choose(slot),
		Some(_) => ReleaseOutcome::Cancel,
		None if held < TAP_WINDOW => ReleaseOutcome::StayOpen,
		None => ReleaseOutcome::Cancel,
	}
}

/// 남아 있는 링(tap)을 클릭했을 때 할 일.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickOutcome {
	Choose(usize),
	/// 한 단계 뒤로.
	Back,
	Close,
	Nothing,
}

/// 클릭한 자리로 정한다. 링 밖은 닫기다. 가운데는 하위 링에서만 뒤로 가기다. 빈 칸은 아무 일도 없다.
fn decide_pick(dx: f64, dy: f64, filled: &[bool], depth: usize) -> PickOutcome {
	let hit = geometry::hit(dx, dy, filled.len());
	if hit.beyond_outer {
		return PickOutcome::Close;
	}
	match hit.slot {
		Some(slot) if filled[slot] => PickOutcome::Choose(slot),
		Some(_) => PickOutcome::Nothing,
		None if depth > 1 => PickOutcome::Back,
		None => PickOutcome::Nothing,
	}
}

/// 커서 자리로 켜질 칸. 빈 칸은 켜지지 않는다. 남아 있는 링에서는 링 안에 있을 때만 켠다 —
/// 거기서 클릭해야 그 칸이 골라지기 때문이다.
fn hover_for(dx: f64, dy: f64, filled: &[bool], mode: Mode) -> Option<usize> {
	let hit = geometry::hit(dx, dy, filled.len());
	if mode == Mode::Open && hit.beyond_outer {
		return None;
	}
	hit.slot.filter(|&slot| filled[slot])
}

// ── 화면 ───────────────────────────────────────────────────────────────

/// `point` 가 있는 모니터의 보이는 영역. 논리 픽셀이다.
///
/// tao 는 모니터 좌표를 "논리 좌표 × 그 모니터의 배율" 로 준다 (tao-0.35.3 `platform_impl/macos/monitor.rs:225-231`).
/// 그래서 모니터마다 자기 배율로 나눠 논리 좌표로 되돌린 뒤 견준다.
fn visible_area(app: &AppHandle, point: (f64, f64)) -> Option<Rect> {
	let monitors = app.available_monitors().ok()?;
	let logical = |monitor: &tauri::Monitor| {
		let scale = monitor.scale_factor();
		let (position, size) = (monitor.position(), monitor.size());
		let bounds = Rect {
			x: position.x as f64 / scale,
			y: position.y as f64 / scale,
			width: size.width as f64 / scale,
			height: size.height as f64 / scale,
		};
		let work = monitor.work_area();
		let visible = Rect {
			x: work.position.x as f64 / scale,
			y: work.position.y as f64 / scale,
			width: work.size.width as f64 / scale,
			height: work.size.height as f64 / scale,
		};
		(bounds, visible)
	};
	let areas: Vec<(Rect, Rect)> = monitors.iter().map(logical).collect();
	areas
		.iter()
		.find(|(bounds, _)| {
			point.0 >= bounds.x
				&& point.0 < bounds.x + bounds.width
				&& point.1 >= bounds.y
				&& point.1 < bounds.y + bounds.height
		})
		.or_else(|| areas.first())
		.map(|(_, visible)| *visible)
}

/// 링의 중심을 정한다. 화면 가장자리에 걸리면 안으로 민다. 밀었으면 커서도 그 중심으로 옮긴다 —
/// 안 옮기면 커서가 처음부터 가운데 밖에 있어 뜨자마자 칸이 켜진다.
///
/// 돌려주는 것은 `(중심, 칸을 켜기 전에 커서가 떠나야 할 자리)` 다. 커서를 옮겼거나 옮길 필요가 없었으면
/// 둘째는 `None` 이다. OS 가 옮기기를 거절했을 때만 값이 있다.
fn place_center(app: &AppHandle, cursor: (f64, f64)) -> ((f64, f64), Option<(f64, f64)>) {
	let Some(area) = visible_area(app, cursor) else {
		return (cursor, None);
	};
	let center = geometry::clamp_center(cursor.0, cursor.1, area);
	let moved = (center.0 - cursor.0).hypot(center.1 - cursor.1) > CURSOR_EPSILON;
	if !moved {
		return (center, None);
	}
	if platform::warp_cursor(center.0, center.1) {
		(center, None)
	} else {
		log::warn!("[ring] the cursor could not be moved to the ring's center");
		(center, Some(cursor))
	}
}

// ── 창에 닿는 일 ───────────────────────────────────────────────────────

fn emit<P: Serialize + Clone>(app: &AppHandle, event: &str, payload: P) {
	if let Err(e) = app.emit_to(window_labels::RING, event, payload) {
		log::warn!("[ring] failed to emit {event}: {e}");
	}
}

/// 창을 지금 중심으로 옮겨 올린다. `take_key` 면 키보드도 준다. `quiet` 인 session 은 키보드를 받지 않는다.
fn present(app: &AppHandle, center: (f64, f64), take_key: bool) {
	let take_key = take_key && !session().as_ref().is_some_and(|s| s.quiet);
	let window = match ring_window::ensure(app) {
		Ok(window) => window,
		Err(e) => {
			log::warn!("[ring] no window to show the ring in: {e}");
			return;
		}
	};
	if let Err(e) = ring_window::show_at(&window, center.0, center.1) {
		log::warn!("[ring] failed to show the window: {e}");
	}
	if take_key {
		if let Err(e) = crate::ui::panel::focus(&window) {
			log::warn!("[ring] failed to give the ring keyboard focus: {e}");
		}
	}
}

/// main thread 에서 `f` 를 돌린다.
fn on_main(app: &AppHandle, f: impl FnOnce(&AppHandle) + Send + 'static) {
	let handle = app.clone();
	if let Err(e) = app.run_on_main_thread(move || f(&handle)) {
		log::warn!("[ring] failed to reach the main thread: {e}");
	}
}

// ── 띄우기 ─────────────────────────────────────────────────────────────

/// 전역 단축키를 눌렀다. main thread 에서 온다.
///
/// 누르고 있는 동안 OS 가 누름을 되풀이해 보내도 안전하다 — 이미 떠 있으면 새로 띄우지 않는다.
pub fn pressed(app: &AppHandle, ring_id: &str, shortcut: Shortcut) {
	let pressed_at = Instant::now();
	let showing = {
		let mut guard = session();
		guard.as_mut().map(|current| {
			let same = current
				.trigger
				.as_ref()
				.is_some_and(|(id, _)| id == ring_id);
			if same && current.mode == Mode::Holding {
				current.repeat_presses += 1;
			}
			(same, current.mode)
		})
	};
	match showing {
		Some((true, Mode::Holding)) => return,
		Some((true, _)) => {
			// 남아 있는 링에서 같은 단축키는 닫기다.
			hide(app);
			return;
		}
		// 다른 링이 떠 있다. 닫고 이 링을 띄운다.
		Some((false, _)) => hide(app),
		None => {}
	}
	let Some(ring) = find_ring(ring_id) else {
		log::warn!("[ring] the shortcut's ring {ring_id} is gone");
		return;
	};
	if ring.slots.is_empty() {
		log::info!("[ring] ring {ring_id} has no slots — not shown");
		return;
	}
	let key_state_usable = platform::combo_held(&shortcut);
	if !key_state_usable {
		log::info!(
			"[ring] key state is not readable — waiting for the shortcut's release event only"
		);
	}
	open_session(
		app,
		ring,
		Mode::Holding,
		Some((ring_id.to_string(), shortcut)),
		key_state_usable,
		None,
	);
	log::info!(
		"[ring] shown {:.1} ms after the press (ring {ring_id})",
		pressed_at.elapsed().as_secs_f64() * 1000.0
	);
}

/// 전역 단축키를 놓았다. main thread 에서 온다.
pub fn released(app: &AppHandle, ring_id: &str) {
	let generation = match session().as_ref() {
		Some(current)
			if current.mode == Mode::Holding
				&& current
					.trigger
					.as_ref()
					.is_some_and(|(id, _)| id == ring_id) =>
		{
			current.generation
		}
		_ => return,
	};
	finish_hold(app, generation, "the shortcut's release event");
}

/// 링 id 로 남아 있는 상태(tap)로 띄운다. 단축키 없이 띄우는 길이다. main thread 에서 부른다.
///
/// `center` 를 주면 커서 자리 대신 그 자리에 띄운다. `take_key` 가 `false` 면 키보드를 받지 않는다 —
/// 둘 다 개발용 확인에서만 쓴다.
pub fn show_open(
	app: &AppHandle,
	ring_id: &str,
	center: Option<(f64, f64)>,
	take_key: bool,
) -> Result<(), Refusal> {
	let ring = find_ring(ring_id).ok_or(Refusal::RingGone)?;
	if ring.slots.is_empty() {
		return Err(Refusal::RingEmpty);
	}
	hide(app);
	open_session(
		app,
		ring,
		Mode::Open,
		None,
		false,
		center.map(|c| (c, take_key)),
	);
	Ok(())
}

/// 새 session 을 만들고 창을 올린다. `fixed` 는 `(중심, key 를 받는가)` 를 직접 정할 때 준다.
fn open_session(
	app: &AppHandle,
	ring: Ring,
	mode: Mode,
	trigger: Option<(String, Shortcut)>,
	key_state_usable: bool,
	fixed: Option<((f64, f64), bool)>,
) {
	let cursor = platform::cursor();
	let (center, arm_origin, take_key) = match fixed {
		Some((center, take_key)) => (center, None, take_key),
		None => {
			let (center, arm_origin) = place_center(app, cursor);
			(center, arm_origin, mode == Mode::Open)
		}
	};
	let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
	let new = Session {
		generation,
		mode,
		trigger,
		key_state_usable,
		stack: vec![ring],
		center,
		hovered: None,
		started: Instant::now(),
		last_cursor: platform::cursor(),
		arm_origin,
		confirm: None,
		repeat_presses: 0,
		quiet: fixed.is_some_and(|(_, take_key)| !take_key),
	};
	let show = payload(&new);
	*LAST_OPENED.lock().unwrap_or_else(|e| e.into_inner()) = Some((show.seq, new.started));
	*session() = Some(new);
	emit(app, events::RING_SHOW, show);
	present(app, center, take_key);
	start_polling(app.clone(), generation);
}

// ── polling ────────────────────────────────────────────────────────────

/// polling 한 번이 알아낸 것.
enum Tick {
	Nothing,
	Hover(Option<usize>),
	/// 조합의 키가 떨어졌다.
	Released,
	/// hold 에서 하위 링 칸 쪽으로 바깥 반지름을 넘었다.
	EnterSubRing(usize, (f64, f64)),
	TimedOut,
}

fn tick(current: &mut Session) -> Tick {
	let cursor = platform::cursor();
	if current.mode == Mode::Holding {
		if current.started.elapsed() > HOLD_TIMEOUT {
			return Tick::TimedOut;
		}
		if current.key_state_usable
			&& current
				.trigger
				.as_ref()
				.is_some_and(|(_, shortcut)| !platform::combo_held(shortcut))
		{
			return Tick::Released;
		}
	}
	if current.mode == Mode::Confirming {
		return Tick::Nothing;
	}
	let moved =
		(cursor.0 - current.last_cursor.0).hypot(cursor.1 - current.last_cursor.1) > CURSOR_EPSILON;
	if !moved {
		// 화살표 키로 옮긴 강조를 가만히 있는 커서가 되돌리지 않게 한다.
		return Tick::Nothing;
	}
	current.last_cursor = cursor;
	if let Some(origin) = current.arm_origin {
		if (cursor.0 - origin.0).hypot(cursor.1 - origin.1) < ARM_DISTANCE {
			return Tick::Nothing;
		}
		current.arm_origin = None;
	}
	let (dx, dy) = current.offset(cursor);
	let filled = current.ring().filled();
	let hovered = hover_for(dx, dy, &filled, current.mode);
	if current.mode == Mode::Holding && dx.hypot(dy) > OUTER_RADIUS {
		if let Some(slot) = hovered {
			let opens_ring = current
				.ring()
				.slot(slot)
				.is_some_and(|s| matches!(s.action, RingAction::OpenRing { .. }));
			if opens_ring {
				return Tick::EnterSubRing(slot, cursor);
			}
		}
	}
	if hovered == current.hovered {
		return Tick::Nothing;
	}
	current.hovered = hovered;
	Tick::Hover(hovered)
}

fn start_polling(app: AppHandle, generation: u64) {
	let spawned = std::thread::Builder::new()
		.name("ring-poll".to_string())
		.spawn(move || {
			loop {
				std::thread::sleep(POLL_INTERVAL);
				let step = {
					let mut guard = session();
					match guard.as_mut() {
						Some(current) if current.generation == generation => tick(current),
						// 링이 닫혔거나 다른 링이 떴다. 이 thread 는 끝난다.
						_ => return,
					}
				};
				match step {
					Tick::Nothing => {}
					Tick::Hover(slot) => emit(&app, events::RING_HOVER, slot),
					Tick::Released => {
						on_main(&app, move |app| {
							finish_hold(app, generation, "key-state polling")
						});
					}
					Tick::EnterSubRing(slot, at) => {
						on_main(&app, move |app| {
							enter_sub_ring_in_hold(app, generation, slot, at)
						});
					}
					Tick::TimedOut => on_main(&app, move |app| {
						if session()
							.as_ref()
							.is_some_and(|s| s.generation == generation)
						{
							log::info!("[ring] held for 30 s without a release — closing");
							hide(app);
						}
					}),
				}
			}
		});
	if let Err(e) = spawned {
		log::warn!("[ring] failed to start polling: {e}");
	}
}

// ── 놓음 ───────────────────────────────────────────────────────────────

/// hold 가 끝났다. main thread 에서 부른다. 두 길(plugin 과 polling)이 다 불러도 한 번만 한다.
/// `via` 는 어느 길이 먼저 왔는지다. 로그에만 쓴다.
fn finish_hold(app: &AppHandle, generation: u64, via: &str) {
	let outcome = {
		let mut guard = session();
		let Some(current) = guard.as_mut() else {
			return;
		};
		if current.generation != generation || current.mode != Mode::Holding {
			return;
		}
		log::info!(
			"[ring] hold ended by {via} after {} ms ({} repeated presses)",
			current.started.elapsed().as_millis(),
			current.repeat_presses
		);
		let cursor = platform::cursor();
		let (dx, dy) = current.offset(cursor);
		let outcome = match current.arm_origin {
			// 커서를 가운데로 옮기지 못했고 아직 그 자리다. 방향을 고른 적이 없다.
			Some(_) if current.started.elapsed() < TAP_WINDOW => ReleaseOutcome::StayOpen,
			Some(_) => ReleaseOutcome::Cancel,
			None => decide_release(dx, dy, &current.ring().filled(), current.started.elapsed()),
		};
		if outcome == ReleaseOutcome::StayOpen {
			current.mode = Mode::Open;
			current.arm_origin = None;
			current.last_cursor = cursor;
		}
		outcome
	};
	match outcome {
		ReleaseOutcome::Choose(slot) => choose(app, slot),
		ReleaseOutcome::Cancel => hide(app),
		ReleaseOutcome::StayOpen => {
			let Some((show, center)) = session().as_ref().map(|s| (payload(s), s.center)) else {
				return;
			};
			emit(app, events::RING_SHOW, show);
			present(app, center, true);
		}
	}
}

// ── 고르기 ─────────────────────────────────────────────────────────────

/// 지금 링의 `position` 칸을 골랐다. 하위 링이면 들어가고, 확인을 묻는 칸이면 묻고, 아니면 실행한다.
fn choose(app: &AppHandle, position: usize) {
	let slot = {
		let guard = session();
		let Some(current) = guard.as_ref() else {
			return;
		};
		match current.ring().slot(position) {
			Some(slot) => slot.clone(),
			None => return,
		}
	};
	if let RingAction::OpenRing { ring_id } = &slot.action {
		enter_sub_ring(app, ring_id, &slot.label, None);
		return;
	}
	if slot.confirm {
		let shown = {
			let mut guard = session();
			let Some(current) = guard.as_mut() else {
				return;
			};
			current.mode = Mode::Confirming;
			current.hovered = Some(position);
			current.confirm = Some(slot);
			(payload(current), current.center)
		};
		emit(app, events::RING_SHOW, shown.0);
		present(app, shown.1, true);
		return;
	}
	hide(app);
	actions::run(app, slot);
}

/// 하위 링으로 들어간다. `at` 을 주면 그 자리가 새 중심이다 (hold 에서 바깥 반지름을 넘은 자리).
/// 안 주면 같은 자리에서 내용만 바뀌고, 링이 남는 상태(Open)가 된다.
fn enter_sub_ring(app: &AppHandle, ring_id: &str, label: &str, at: Option<(f64, f64)>) {
	let sub = match find_ring(ring_id) {
		Some(ring) if !ring.slots.is_empty() => ring,
		found => {
			hide(app);
			// 빈 하위 링은 "대상이 없다" 가 아니다. 무엇을 하면 되는지가 다르므로 따로 알린다.
			// 실행 기록의 머리줄은 고른 칸의 이름이다.
			actions::notify_failure(
				app,
				label,
				match found {
					Some(_) => actions::Failure::SubRingEmpty,
					None => actions::Failure::TargetMissing,
				},
			);
			return;
		}
	};
	let placed = at.map(|cursor| place_center(app, cursor));
	let shown = {
		let mut guard = session();
		let Some(current) = guard.as_mut() else {
			return;
		};
		if current.stack.len() >= MAX_DEPTH {
			// 저장할 때 막는 깊이다. 여기는 손상된 데이터에서만 온다.
			log::warn!("[ring] sub-ring depth limit reached — not entering {ring_id}");
			return;
		}
		current.stack.push(sub);
		current.hovered = None;
		match placed {
			Some((center, arm_origin)) => {
				current.center = center;
				current.arm_origin = arm_origin;
			}
			None => current.mode = Mode::Open,
		}
		current.last_cursor = platform::cursor();
		(payload(current), current.center, current.mode == Mode::Open)
	};
	emit(app, events::RING_SHOW, shown.0);
	present(app, shown.1, shown.2);
}

fn enter_sub_ring_in_hold(app: &AppHandle, generation: u64, position: usize, at: (f64, f64)) {
	let (ring_id, label) = {
		let guard = session();
		let Some(current) = guard.as_ref() else {
			return;
		};
		if current.generation != generation || current.mode != Mode::Holding {
			return;
		}
		match current.ring().slot(position) {
			Some(slot) => match &slot.action {
				RingAction::OpenRing { ring_id } => (ring_id.clone(), slot.label.clone()),
				_ => return,
			},
			None => return,
		}
	};
	enter_sub_ring(app, &ring_id, &label, Some(at));
}

/// 한 단계 뒤로 간다. 맨 위 링이면 아무 일도 없다.
fn back(app: &AppHandle) {
	let shown = {
		let mut guard = session();
		let Some(current) = guard.as_mut() else {
			return;
		};
		if current.mode != Mode::Open || current.stack.len() < 2 {
			return;
		}
		current.stack.pop();
		current.hovered = None;
		payload(current)
	};
	emit(app, events::RING_SHOW, shown);
}

// ── 링 창이 보내는 것 ──────────────────────────────────────────────────

/// 링 창을 클릭했다. 어느 칸인지는 커서 자리로 정한다. main thread 에서 부른다.
pub fn pick(app: &AppHandle) {
	let (mode, generation, outcome) = {
		let guard = session();
		let Some(current) = guard.as_ref() else {
			return;
		};
		let (dx, dy) = current.offset(platform::cursor());
		(
			current.mode,
			current.generation,
			decide_pick(dx, dy, &current.ring().filled(), current.stack.len()),
		)
	};
	match mode {
		// 누른 채로 링을 클릭했다. 놓은 것과 같이 다룬다.
		Mode::Holding => finish_hold(app, generation, "a click on the ring"),
		// 확인은 단추와 키가 답한다.
		Mode::Confirming => {}
		Mode::Open => match outcome {
			PickOutcome::Choose(slot) => choose(app, slot),
			PickOutcome::Back => back(app),
			PickOutcome::Close => hide(app),
			PickOutcome::Nothing => {}
		},
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArrowDirection {
	Up,
	Down,
	Left,
	Right,
}

/// 링 창이 받은 키.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RingKey {
	/// 1~8.
	Digit {
		value: usize,
	},
	Arrow {
		direction: ArrowDirection,
	},
	/// Enter 와 Space.
	Enter,
	/// Backspace.
	Back,
	Escape,
}

/// 링 창이 키를 받았다. main thread 에서 부른다.
pub fn key(app: &AppHandle, key: RingKey) {
	let Some(mode) = session().as_ref().map(|s| s.mode) else {
		return;
	};
	match (mode, key) {
		(_, RingKey::Escape) => hide(app),
		(Mode::Open, RingKey::Digit { value }) => {
			let filled = session()
				.as_ref()
				.map(|s| s.ring().filled())
				.unwrap_or_default();
			if value >= 1 && filled.get(value - 1).copied().unwrap_or(false) {
				choose(app, value - 1);
			}
		}
		(Mode::Open, RingKey::Arrow { direction }) => {
			let forward = matches!(direction, ArrowDirection::Right | ArrowDirection::Down);
			let hovered = {
				let mut guard = session();
				let Some(current) = guard.as_mut() else {
					return;
				};
				let next = geometry::step(current.hovered, forward, &current.ring().filled());
				if next == current.hovered {
					return;
				}
				current.hovered = next;
				next
			};
			emit(app, events::RING_HOVER, hovered);
		}
		(Mode::Open, RingKey::Enter) => {
			let hovered = session().as_ref().and_then(|s| s.hovered);
			if let Some(slot) = hovered {
				choose(app, slot);
			}
		}
		(Mode::Open, RingKey::Back) => back(app),
		// hold 에서는 링이 키보드를 받지 않는다. 확인 중에는 Esc 만 듣는다 — Enter 와 Space 는
		// 페이지에서 focus 가 있는 버튼이 받고, 그 버튼이 `ring_confirm` 이나 `ring_hide` 를 부른다.
		// 여기서 Enter 를 실행으로 읽으면 "취소" 에 focus 를 두고 누른 Enter 도 실행된다.
		_ => {}
	}
}

/// 확인 물음의 답. main thread 에서 부른다.
pub fn confirm(app: &AppHandle, accepted: bool) {
	let slot = {
		let mut guard = session();
		let Some(current) = guard.as_mut() else {
			return;
		};
		if current.mode != Mode::Confirming {
			return;
		}
		current.confirm.take()
	};
	hide(app);
	if let (true, Some(slot)) = (accepted, slot) {
		actions::run(app, slot);
	}
}

// ── 닫기 ───────────────────────────────────────────────────────────────

/// 링을 닫는다. 떠 있지 않으면 아무 일도 없다.
pub fn hide(app: &AppHandle) {
	if session().take().is_none() {
		return;
	}
	emit(app, events::RING_HIDE, ());
	ring_window::hide(app);
}

/// 링 창이 키보드 포커스를 잃었다. 남아 있는 링과 확인 물음은 닫는다. hold 는 포커스를 가진 적이 없다.
pub fn focus_lost(app: &AppHandle) {
	let closes = session()
		.as_ref()
		.is_some_and(|s| matches!(s.mode, Mode::Open | Mode::Confirming));
	if closes {
		hide(app);
	}
}

/// 링 페이지가 `seq` 번 보이기를 그렸다. 링을 띄운 뒤 그릴 때까지 걸린 시간을 로그에 남긴다.
pub fn painted(seq: u64) {
	let opened = *LAST_OPENED.lock().unwrap_or_else(|e| e.into_inner());
	if let Some((opened_seq, at)) = opened {
		if opened_seq == seq {
			log::info!(
				"[ring] painted {:.1} ms after the ring was opened",
				at.elapsed().as_secs_f64() * 1000.0
			);
		}
	}
}

/// 링이 떠 있는가.
pub fn is_showing() -> bool {
	session().is_some()
}

/// 링 창이 있어야 하는 창인지 맞춘다. 단축키가 걸린 링이 하나라도 있고 이 빌드가 전역 단축키를 등록하면
/// 미리 만든다 — 누른 순간 떠야 해서다. 아니면 부순다. main thread 에서 부른다.
pub fn sync_window(app: &AppHandle, shortcuts_registered: bool) {
	let wanted = shortcuts_registered && rings().iter().any(|ring| ring.shortcut.is_some());
	if wanted {
		if let Err(e) = ring_window::ensure(app) {
			log::warn!("[ring] failed to pre-create the window: {e}");
		}
	} else if !is_showing() && app.get_webview_window(window_labels::RING).is_some() {
		ring_window::destroy(app);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const SHORT: Duration = Duration::from_millis(80);
	const LONG: Duration = Duration::from_millis(900);
	/// 6칸: 0, 1, 2 만 차 있다.
	const FILLED: [bool; 6] = [true, true, true, false, false, false];

	/// 12시가 0 이고 시계 방향인 각도와 거리로 `(dx, dy)` 를 만든다.
	fn at(angle_deg: f64, distance: f64) -> (f64, f64) {
		let rad = angle_deg.to_radians();
		(distance * rad.sin(), -distance * rad.cos())
	}

	#[test]
	fn releasing_over_a_slot_chooses_it_however_long_the_hold() {
		let (dx, dy) = at(60.0, 90.0);
		assert_eq!(
			decide_release(dx, dy, &FILLED, LONG),
			ReleaseOutcome::Choose(1)
		);
		// 220 ms 안의 flick 도 실행이다.
		assert_eq!(
			decide_release(dx, dy, &FILLED, SHORT),
			ReleaseOutcome::Choose(1)
		);
	}

	#[test]
	fn releasing_far_outside_the_ring_still_chooses_by_direction() {
		let (dx, dy) = at(120.0, 900.0);
		assert_eq!(
			decide_release(dx, dy, &FILLED, LONG),
			ReleaseOutcome::Choose(2)
		);
	}

	#[test]
	fn releasing_over_an_empty_slot_cancels() {
		let (dx, dy) = at(180.0, 90.0);
		assert_eq!(
			decide_release(dx, dy, &FILLED, LONG),
			ReleaseOutcome::Cancel
		);
		assert_eq!(
			decide_release(dx, dy, &FILLED, SHORT),
			ReleaseOutcome::Cancel
		);
	}

	#[test]
	fn a_short_press_in_the_center_keeps_the_ring_open() {
		assert_eq!(
			decide_release(0.0, 0.0, &FILLED, SHORT),
			ReleaseOutcome::StayOpen
		);
		assert_eq!(
			decide_release(0.0, 0.0, &FILLED, TAP_WINDOW - Duration::from_millis(1)),
			ReleaseOutcome::StayOpen
		);
	}

	#[test]
	fn a_long_hold_released_in_the_center_cancels() {
		assert_eq!(
			decide_release(0.0, 0.0, &FILLED, TAP_WINDOW),
			ReleaseOutcome::Cancel
		);
		assert_eq!(
			decide_release(5.0, -5.0, &FILLED, LONG),
			ReleaseOutcome::Cancel
		);
	}

	#[test]
	fn clicking_a_filled_slot_chooses_it() {
		let (dx, dy) = at(0.0, 90.0);
		assert_eq!(decide_pick(dx, dy, &FILLED, 1), PickOutcome::Choose(0));
	}

	#[test]
	fn clicking_an_empty_slot_does_nothing() {
		let (dx, dy) = at(240.0, 90.0);
		assert_eq!(decide_pick(dx, dy, &FILLED, 1), PickOutcome::Nothing);
	}

	#[test]
	fn clicking_outside_the_ring_closes_it() {
		// 창의 모서리 — 창 안이지만 링 밖이다.
		assert_eq!(decide_pick(150.0, -150.0, &FILLED, 1), PickOutcome::Close);
	}

	#[test]
	fn clicking_the_center_goes_back_only_inside_a_sub_ring() {
		assert_eq!(decide_pick(0.0, 0.0, &FILLED, 1), PickOutcome::Nothing);
		assert_eq!(decide_pick(0.0, 0.0, &FILLED, 2), PickOutcome::Back);
		assert_eq!(decide_pick(0.0, 0.0, &FILLED, 3), PickOutcome::Back);
	}

	#[test]
	fn an_empty_slot_is_never_highlighted() {
		let (dx, dy) = at(180.0, 90.0);
		assert_eq!(hover_for(dx, dy, &FILLED, Mode::Holding), None);
		assert_eq!(hover_for(dx, dy, &FILLED, Mode::Open), None);
	}

	#[test]
	fn hold_highlights_by_direction_beyond_the_ring_and_tap_does_not() {
		let (dx, dy) = at(60.0, 400.0);
		assert_eq!(hover_for(dx, dy, &FILLED, Mode::Holding), Some(1));
		assert_eq!(hover_for(dx, dy, &FILLED, Mode::Open), None);
		let (dx, dy) = at(60.0, 90.0);
		assert_eq!(hover_for(dx, dy, &FILLED, Mode::Open), Some(1));
	}

	#[test]
	fn the_view_marks_sub_ring_slots_and_hides_action_details() {
		let ring = Ring {
			id: "r".to_string(),
			name: "작업".to_string(),
			shortcut: None,
			slot_count: 4,
			slots: vec![
				Slot {
					position: 0,
					label: "배포".to_string(),
					icon: "rocket".to_string(),
					action: RingAction::Shell {
						command: "make deploy TOKEN=abc".to_string(),
						working_dir: String::new(),
						timeout_secs: 120,
					},
					confirm: true,
				},
				Slot {
					position: 2,
					label: "더 보기".to_string(),
					icon: "circle-dot".to_string(),
					action: RingAction::OpenRing {
						ring_id: "sub".to_string(),
					},
					confirm: false,
				},
			],
		};
		let view = ring_view(&ring);
		assert_eq!(view.slot_count, 4);
		assert!(!view.slots[0].opens_ring);
		assert!(view.slots[1].opens_ring);
		let json = serde_json::to_string(&view).unwrap();
		assert!(
			!json.contains("TOKEN"),
			"the page never receives the command"
		);
	}

	#[test]
	fn ring_keys_deserialize_from_the_page_shape() {
		let digit: RingKey = serde_json::from_str(r#"{"kind":"digit","value":3}"#).unwrap();
		assert_eq!(digit, RingKey::Digit { value: 3 });
		let arrow: RingKey =
			serde_json::from_str(r#"{"kind":"arrow","direction":"left"}"#).unwrap();
		assert_eq!(
			arrow,
			RingKey::Arrow {
				direction: ArrowDirection::Left
			}
		);
		let escape: RingKey = serde_json::from_str(r#"{"kind":"escape"}"#).unwrap();
		assert_eq!(escape, RingKey::Escape);
	}
}
