// 링의 전역 단축키 — OS 등록, 충돌 검사, 일시 정지, 단축키를 입력받는 동안 잠시 끄기
//
// 다른 앱과 겨루는 것은 조합 그 자체다. bundle identifier 로 갈라지지 않는다. 그래서 개발 빌드는 등록하지
// 않는다 — 등록하면 사용자가 쓰는 설치된 앱들이 그 조합을 잃는다. 켜려면 `RINGRING_DEV_GLOBAL_SHORTCUTS=1`.
//
// **등록과 해제는 한 곳에서, 한 번에 하나씩, main thread 에서만 한다.**
// plugin 의 `unregister_all` 은 자기 mutex 를 쥔 채 main thread 의 답을 기다린다 (2.4.0 `src/lib.rs:234-239`).
// 다른 thread 가 그렇게 기다리는 동안 main thread 가 같은 mutex 를 잡으려 하면 둘 다 영영 멈춘다 — main thread 의
// sync command 가 등록을 고칠 때도, 걸려 있는 단축키가 눌려 plugin 의 handler 가 돌 때도 그렇다.
// 그래서 plugin 을 부르는 곳은 [`pass_now`] 하나고, 그 함수는 main thread 에서만 돈다. main thread 에서는
// plugin 이 일을 그 자리에서 하므로 mutex 를 쥔 채 남을 기다리는 thread 가 없다.
// 여러 단계로 된 일(조합 바꾸기, 가져오기)은 [`Op`] 를 쥐고 한다. 두 일의 단계가 서로 섞이지 않는다.
use std::sync::Mutex;

use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::constants::events;
use crate::ring::controller;
use crate::ring::model::{Refusal, Ring, RingError, ShortcutKind};
use crate::store::{Db, rings as ring_store};

// ── 조합을 입력받는 동안 ───────────────────────────────────────────────

/// 설정 창이 단축키 조합을 입력받는 중인지. 그동안은 등록을 전부 뗀다 — 이미 쓰는 조합을 다시 눌러도
/// 링이 뜨지 않고 입력 칸이 그 조합을 받는다.
///
/// 입력 하나마다 번호가 있다. 화면이 보낸 "시작" 과 "끝" 은 서로 다른 task 로 도착해서 순서가 바뀔 수 있다.
/// 이미 끝난 번호의 "시작" 이 늦게 와도 다시 켜지 않는다 — 켜지면 끌 사람이 없어 단축키가 꺼진 채로 남는다.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Capture {
	active: Option<u64>,
	/// 이 번호까지는 끝났다.
	ended_through: u64,
}

impl Capture {
	/// 돌려주는 것은 "꺼져 있다가 켜졌는가" 다.
	fn begin(&mut self, id: u64) -> bool {
		if id <= self.ended_through {
			return false;
		}
		let was_active = self.active.is_some();
		self.active = Some(self.active.map_or(id, |current| current.max(id)));
		!was_active
	}

	/// 돌려주는 것은 "켜져 있다가 꺼졌는가" 다.
	fn end(&mut self, id: u64) -> bool {
		self.ended_through = self.ended_through.max(id);
		if self.active.is_some_and(|current| current <= id) {
			self.active = None;
			return true;
		}
		false
	}

	/// 받던 입력이 무엇이든 끝낸다.
	fn end_all(&mut self) -> bool {
		match self.active.take() {
			Some(current) => {
				self.ended_through = self.ended_through.max(current);
				true
			}
			None => false,
		}
	}
}

static CAPTURE: Mutex<Capture> = Mutex::new(Capture {
	active: None,
	ended_through: 0,
});

fn capture() -> std::sync::MutexGuard<'static, Capture> {
	CAPTURE.lock().unwrap_or_else(|e| e.into_inner())
}

// ── 무엇을 걸어야 하는가 ───────────────────────────────────────────────

/// 이 빌드가 OS 에 걸 수 있는 조합.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuildGate {
	All,
	/// 개발 빌드만 만든다. 배포하는 빌드는 늘 `All` 이다.
	#[cfg_attr(not(dev), allow(dead_code))]
	Nothing,
	/// dev-agent 의 반복 확인이 켠다. 이 조합 하나만 실제로 건다 (`dev_stress_gate`).
	#[cfg(feature = "dev-agent")]
	Only(Shortcut),
}

impl BuildGate {
	#[cfg_attr(not(feature = "dev-agent"), allow(unused_variables))]
	fn allows(self, combo: &str) -> bool {
		match self {
			BuildGate::All => true,
			BuildGate::Nothing => false,
			#[cfg(feature = "dev-agent")]
			BuildGate::Only(only) => combo
				.parse::<Shortcut>()
				.is_ok_and(|parsed| parsed.mods == only.mods && parsed.key == only.key),
		}
	}
}

#[cfg(feature = "dev-agent")]
static STRESS_ONLY: Mutex<Option<Shortcut>> = Mutex::new(None);

fn build_gate() -> BuildGate {
	#[cfg(dev)]
	{
		if std::env::var("RINGRING_DEV_GLOBAL_SHORTCUTS").as_deref() == Ok("1") {
			return BuildGate::All;
		}
		#[cfg(feature = "dev-agent")]
		if let Some(only) = *STRESS_ONLY.lock().unwrap_or_else(|e| e.into_inner()) {
			return BuildGate::Only(only);
		}
		BuildGate::Nothing
	}
	#[cfg(not(dev))]
	{
		BuildGate::All
	}
}

/// 이 빌드가 전역 단축키를 OS 에 등록하는가.
pub fn build_registers() -> bool {
	build_gate() == BuildGate::All
}

/// 지금 단축키가 걸려 있어야 하는가.
pub fn active() -> bool {
	build_registers() && !crate::settings::current().paused && capture().active.is_none()
}

/// 등록 한 번이 읽는 상태.
#[derive(Debug, Clone, Copy)]
struct Gate {
	build: BuildGate,
	paused: bool,
	capturing: bool,
}

fn gate() -> Gate {
	Gate {
		build: build_gate(),
		paused: crate::settings::current().paused,
		capturing: capture().active.is_some(),
	}
}

/// 걸 조합 하나 — `(링 id, 종류, 조합)`.
type Binding = (String, ShortcutKind, String);

/// 지금 바꾸는 조합 — `(링 id, 종류, 새 조합)`. 새 조합이 `None` 이면 뗀다.
type Change<'a> = (&'a str, ShortcutKind, Option<&'a str>);

/// OS 가 받지 않은 조합들 — `(링 id, 종류)`.
type Failed = Vec<(String, ShortcutKind)>;

const KINDS: [ShortcutKind; 2] = [ShortcutKind::Normal, ShortcutKind::Quick];

/// 등록 한 번이 할 일.
#[derive(Debug, Default, PartialEq, Eq)]
struct Plan {
	/// 이 순서로 건다.
	register: Vec<Binding>,
	/// 걸어 두지는 않지만 OS 가 받는지만 본다 — 걸었다가 곧바로 뗀다. 일시 정지 중에 조합을 바꿀 때다.
	probe: Option<Binding>,
}

/// 지금 무엇을 걸어야 하는지 정한다. 링마다 일반·빠른 단축키를 둘 다 본다.
///
/// `changed` 의 링과 종류만 저장값 대신 그 새 조합으로 본다 (`None` 이면 걸지 않는다).
fn plan(rings: &[Ring], gate: Gate, changed: Option<Change>) -> Plan {
	let wanted = rings.iter().flat_map(|ring| {
		KINDS.into_iter().filter_map(move |kind| {
			let combo = match changed {
				Some((id, changed_kind, combo)) if id == ring.id && changed_kind == kind => {
					combo.map(str::to_string)
				}
				_ => ring.shortcut_of(kind).map(str::to_string),
			}?;
			gate.build
				.allows(&combo)
				.then(|| (ring.id.clone(), kind, combo))
		})
	});
	if !gate.paused && !gate.capturing {
		return Plan {
			register: wanted.collect(),
			probe: None,
		};
	}
	Plan {
		register: Vec::new(),
		probe: changed.and_then(|(id, kind, _)| {
			wanted
				.into_iter()
				.find(|(ring_id, k, _)| ring_id == id && *k == kind)
		}),
	}
}

/// OS 에 걸고 떼는 일. 테스트는 순서를 적는 가짜를 쓴다.
trait Registrar {
	fn unregister_all(&mut self);
	fn register(&mut self, ring_id: &str, kind: ShortcutKind, combo: &str) -> Result<(), String>;
}

/// 전부 뗀 뒤 `plan` 대로 건다. 돌려주는 것은 등록에 실패한 조합의 `(링 id, 종류)` 다.
fn run_pass(registrar: &mut impl Registrar, plan: Plan) -> Failed {
	registrar.unregister_all();
	let mut failed = Vec::new();
	for (ring_id, kind, combo) in plan.register {
		match registrar.register(&ring_id, kind, &combo) {
			Ok(()) => log::info!("[shortcut] registered '{combo}'"),
			Err(e) => {
				log::warn!("[shortcut] failed to register '{combo}': {e}");
				failed.push((ring_id, kind));
			}
		}
	}
	if let Some((ring_id, kind, combo)) = plan.probe {
		let accepted = registrar.register(&ring_id, kind, &combo);
		registrar.unregister_all();
		if let Err(e) = accepted {
			log::warn!("[shortcut] the OS refused '{combo}': {e}");
			failed.push((ring_id, kind));
		}
	}
	failed
}

/// 수식키 없이 눌러도 글 입력을 가로채지 않는 키. F13~F24 는 기본 기능이 없다.
fn is_spare_function_key(code: Code) -> bool {
	matches!(
		code,
		Code::F13
			| Code::F14
			| Code::F15
			| Code::F16
			| Code::F17
			| Code::F18
			| Code::F19
			| Code::F20
			| Code::F21
			| Code::F22
			| Code::F23
			| Code::F24
	)
}

/// 전역 단축키로 쓸 수 있는 조합인지 읽는다.
///
/// ⌘ · ⌃ · ⌥ 가운데 하나는 있어야 한다. Shift 만 있거나 수식키가 없는 조합은 글 입력을 가로챈다.
/// F13~F24 는 예외다.
pub fn parse_for_registration(combo: &str) -> Result<Shortcut, Refusal> {
	let shortcut = combo
		.parse::<Shortcut>()
		.map_err(|_| Refusal::ShortcutInvalid)?;
	let strong = Modifiers::SUPER | Modifiers::META | Modifiers::CONTROL | Modifiers::ALT;
	if !shortcut.mods.intersects(strong) && !is_spare_function_key(shortcut.key) {
		return Err(Refusal::ShortcutNeedsModifier);
	}
	Ok(shortcut)
}

/// 빠른 단축키로 쓸 수 있는 조합인지 읽는다. 키는 F1~F24 여야 한다. 수식키는 무엇이든, 없어도 된다.
///
/// 수식키 없는 F1~F12 는 다른 앱의 그 키를 가져간다. 막지 않는다 — 고른 사람의 몫이다. 화면이 알린다.
pub fn parse_for_quick(combo: &str) -> Result<Shortcut, Refusal> {
	let shortcut = combo
		.parse::<Shortcut>()
		.map_err(|_| Refusal::ShortcutInvalid)?;
	if !is_function_key(shortcut.key) {
		return Err(Refusal::ShortcutNeedsFunctionKey);
	}
	Ok(shortcut)
}

/// 종류에 맞는 규칙으로 읽는다.
pub fn parse_for(kind: ShortcutKind, combo: &str) -> Result<Shortcut, Refusal> {
	match kind {
		ShortcutKind::Normal => parse_for_registration(combo),
		ShortcutKind::Quick => parse_for_quick(combo),
	}
}

fn is_function_key(code: Code) -> bool {
	matches!(
		code,
		Code::F1
			| Code::F2
			| Code::F3
			| Code::F4
			| Code::F5
			| Code::F6
			| Code::F7
			| Code::F8
			| Code::F9
			| Code::F10
			| Code::F11
			| Code::F12
	) || is_spare_function_key(code)
}

/// `wanted` 를 이미 쓰는 링의 이름. 모든 링의 일반·빠른 단축키를 함께 본다.
/// 바꾸려는 자리(`ring_id` 의 `kind`)만 뺀다 — 같은 링의 다른 종류와 겹쳐도 충돌이다.
///
/// 글자가 아니라 읽은 조합으로 견준다 — 같은 키를 여러 글자로 쓸 수 있다 (`Ctrl` 과 `Control`).
/// 읽지 못하는 저장값은 건너뛴다. 그런 값은 등록되지 않으므로 조합을 차지하지 않는다.
pub fn conflict(
	ring_id: &str,
	kind: ShortcutKind,
	wanted: &Shortcut,
	rings: &[Ring],
) -> Option<String> {
	rings.iter().find_map(|ring| {
		KINDS.into_iter().find_map(|k| {
			if ring.id == ring_id && k == kind {
				return None;
			}
			let parsed = ring.shortcut_of(k)?.parse::<Shortcut>().ok()?;
			(parsed.mods == wanted.mods && parsed.key == wanted.key).then(|| ring.name.clone())
		})
	})
}

/// plugin 을 부르는 단 하나의 자리.
struct Os<'a>(&'a AppHandle);

impl Registrar for Os<'_> {
	fn unregister_all(&mut self) {
		if let Err(e) = self.0.global_shortcut().unregister_all() {
			log::warn!("[shortcut] failed to unregister: {e}");
		}
	}

	/// 링 하나의 조합을 건다. 누름과 놓음을 둘 다 controller 로 넘긴다.
	fn register(&mut self, ring_id: &str, kind: ShortcutKind, combo: &str) -> Result<(), String> {
		let shortcut = combo
			.parse::<Shortcut>()
			.map_err(|e| format!("Invalid shortcut '{combo}': {e}"))?;
		let ring_id = ring_id.to_string();
		self.0
			.global_shortcut()
			.on_shortcut(shortcut, move |app, shortcut, event| match event.state {
				ShortcutState::Pressed => controller::pressed(app, &ring_id, kind, *shortcut),
				ShortcutState::Released => controller::released(app, &ring_id, kind),
			})
			.map_err(|e| e.to_string())
	}
}

fn on_main_thread() -> bool {
	#[cfg(target_os = "macos")]
	{
		objc2::MainThreadMarker::new().is_some()
	}
	// 다른 OS 에는 main thread 를 묻는 싼 방법이 없다. 확인을 건너뛴다.
	#[cfg(not(target_os = "macos"))]
	{
		true
	}
}

/// 등록을 지금 상태에 맞춘다. **main thread 에서만 부른다** — 파일 머리의 설명을 본다.
/// 링 창이 있어야 하는지도 같이 맞춘다.
fn pass_now(app: &AppHandle, changed: Option<Change>) -> Failed {
	debug_assert!(
		on_main_thread(),
		"shortcut registration must run on the main thread"
	);
	let gate = gate();
	let plan = plan(&controller::rings(), gate, changed);
	if plan.register.is_empty() {
		log::info!(
			"[shortcut] nothing to register (build gate: {:?}, paused: {}, capturing: {})",
			gate.build,
			gate.paused,
			gate.capturing
		);
	}
	let failed = run_pass(&mut Os(app), plan);
	note_refused(app, stored_refusals(&failed, changed));
	controller::sync_window(app, active());
	failed
}

// ── OS 가 거절한 저장된 조합 ───────────────────────────────────────────

/// 저장된 조합을 OS 가 받지 않은 `(링 id, 종류)`. 편집기가 그 단축키를 "등록되지 않음" 으로 보인다.
static REFUSED: Mutex<Failed> = Mutex::new(Vec::new());

/// 저장된 `kind` 조합이 지금 OS 에 걸려 있지 않은 링들의 id.
pub fn refused(kind: ShortcutKind) -> Vec<String> {
	REFUSED
		.lock()
		.unwrap_or_else(|e| e.into_inner())
		.iter()
		.filter(|(_, k)| *k == kind)
		.map(|(id, _)| id.clone())
		.collect()
}

/// 실패한 것 가운데 **저장된** 조합이 거절된 것만 남긴다. `changed` 의 자리는 아직 저장하지 않은 조합을
/// 시험한 것이다 — 그 거절은 조합을 바꾼 쪽이 바로 알린다.
fn stored_refusals(failed: &[(String, ShortcutKind)], changed: Option<Change>) -> Failed {
	failed
		.iter()
		.filter(|(id, kind)| {
			changed.is_none_or(|(changed_id, changed_kind, _)| {
				changed_id != id.as_str() || changed_kind != *kind
			})
		})
		.cloned()
		.collect()
}

/// 거절된 조합의 목록을 바꾼다. 바뀌었으면 설정 창에 알린다.
fn note_refused(app: &AppHandle, now: Failed) {
	let changed = {
		let mut guard = REFUSED.lock().unwrap_or_else(|e| e.into_inner());
		let changed = *guard != now;
		*guard = now;
		changed
	};
	if changed {
		if let Err(e) = app.emit(events::APP_STATE_CHANGED, ()) {
			log::warn!(
				"[shortcut] failed to emit {}: {e}",
				events::APP_STATE_CHANGED
			);
		}
	}
}

/// main thread 로 건너가 [`pass_now`] 를 돌리고 답을 기다린다. 기다리는 동안 thread 를 붙잡지 않는다.
/// main thread 에 닿지 못하면 `changed` 의 자리를 실패로 돌려준다 — 걸리지 않은 조합을 저장하지 않게 한다.
async fn pass(app: &AppHandle, changed: Option<Change<'_>>) -> Failed {
	let owned = changed.map(|(id, kind, combo)| (id.to_string(), kind, combo.map(str::to_string)));
	let unreachable: Failed = owned
		.iter()
		.map(|(id, kind, _)| (id.clone(), *kind))
		.collect();
	let (tx, rx) = tokio::sync::oneshot::channel();
	let handle = app.clone();
	let sent = app.run_on_main_thread(move || {
		let changed = owned
			.as_ref()
			.map(|(id, kind, combo)| (id.as_str(), *kind, combo.as_deref()));
		let _ = tx.send(pass_now(&handle, changed));
	});
	if let Err(e) = sent {
		log::warn!("[shortcut] failed to reach the main thread: {e}");
		return unreachable;
	}
	rx.await.unwrap_or(unreachable)
}

// ── 한 번에 하나씩 ─────────────────────────────────────────────────────

static OPS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 등록을 고치는 일 하나. 쥐고 있는 동안 다른 일은 기다린다. main thread 는 이것을 기다리지 않는다 —
/// main thread 에서 시작하는 일은 task 로 넘긴다.
pub struct Op(#[allow(dead_code)] tokio::sync::MutexGuard<'static, ()>);

pub async fn begin() -> Op {
	Op(OPS.lock().await)
}

impl Op {
	/// 등록을 지금 상태(메모리의 링 목록, 일시 정지, 입력받는 중)에 맞춘다. 실패한 `(링 id, 종류)` 를 돌려준다.
	pub async fn resync(&self, app: &AppHandle) -> Vec<(String, ShortcutKind)> {
		pass(app, None).await
	}
}

/// 등록을 지금 상태에 맞춘다. 링 목록이나 일시 정지가 바뀐 뒤에 부른다.
pub async fn resync(app: &AppHandle) -> Vec<(String, ShortcutKind)> {
	begin().await.resync(app).await
}

/// 앱을 켤 때 한 번. `setup` 은 main thread 에서 돌고, 이때는 다른 일이 없다.
pub fn start(app: &AppHandle) {
	let _op = OPS.try_lock();
	let failed = pass_now(app, None);
	// 저장된 조합을 OS 가 받지 않았다. 로그에만 두지 않는다 — 실행 기록에 남겨 트레이와 설정 창이 가리키게 한다.
	let locale = crate::settings::locale();
	for ring in controller::rings()
		.iter()
		.filter(|ring| failed.iter().any(|(id, _)| *id == ring.id))
	{
		crate::runs::record_failure(
			app,
			Some(&ring.name),
			crate::ui::texts::shortcut_refused(locale).to_string(),
		);
	}
}

// ── 링의 조합 바꾸기 ───────────────────────────────────────────────────

/// 조합을 바꾸는 일의 단계들. 순서는 [`change_shortcut`] 이 정하고, 테스트는 그 순서를 적는 가짜를 쓴다.
trait ShortcutSteps {
	/// 받던 입력을 끝낸다. 돌려주는 것은 "받는 중이었는가" 다.
	fn end_capture(&mut self) -> bool;
	/// 그 종류로 쓸 수 있는 조합이고 다른 자리가 쓰지 않는가.
	fn check(&self, ring_id: &str, kind: ShortcutKind, combo: &str) -> Result<(), RingError>;
	async fn pass(&mut self, changed: Option<Change<'_>>) -> Failed;
	async fn save(
		&mut self,
		ring_id: &str,
		kind: ShortcutKind,
		combo: Option<&str>,
	) -> Result<(), RingError>;
	/// 메모리의 링 목록을 저장된 값으로 다시 읽는다.
	async fn reload(&mut self) -> Result<(), RingError>;
}

/// 링의 전역 단축키를 바꾼다. 순서가 곧 규칙이다.
///
/// 1. **입력받기를 끝낸다.** 화면이 "끝" 을 따로 보내기를 기다리지 않는다. 끝나지 않은 채로 등록하면 아무것도
///    걸리지 않아 OS 가 받는지 알 수 없다.
/// 2. **조합을 본다.** 거절이면 저장된 값들로 다시 건다 — 입력받느라 떼어 둔 것을 돌려놓는다.
/// 3. **OS 에 건다.** 거절되면 저장된 값들로 되돌리고 저장하지 않는다. 화면이 보이는 값은 저장된 값이다 —
///    거절된 조합을 저장하면 걸린 것처럼 보이는데 눌러도 아무 일이 없다.
/// 4. **저장한다.** 실패하면 등록도 되돌린다.
/// 5. **메모리의 링 목록을 다시 읽는다.** 다음 등록이 새 조합을 건다.
async fn change_shortcut(
	steps: &mut impl ShortcutSteps,
	ring_id: &str,
	kind: ShortcutKind,
	combo: Option<&str>,
) -> Result<(), RingError> {
	let was_capturing = steps.end_capture();
	if let Some(combo) = combo {
		if let Err(refused) = steps.check(ring_id, kind, combo) {
			if was_capturing {
				steps.pass(None).await;
			}
			return Err(refused);
		}
	}
	if steps
		.pass(Some((ring_id, kind, combo)))
		.await
		.iter()
		.any(|(id, k)| id == ring_id && *k == kind)
	{
		steps.pass(None).await;
		return Err(Refusal::ShortcutUnavailable.into());
	}
	if let Err(e) = steps.save(ring_id, kind, combo).await {
		steps.pass(None).await;
		return Err(e);
	}
	steps.reload().await
}

struct LiveSteps<'a> {
	app: &'a AppHandle,
	db: &'a Db,
}

impl ShortcutSteps for LiveSteps<'_> {
	fn end_capture(&mut self) -> bool {
		capture().end_all()
	}

	fn check(&self, ring_id: &str, kind: ShortcutKind, combo: &str) -> Result<(), RingError> {
		let parsed = parse_for(kind, combo)?;
		match conflict(ring_id, kind, &parsed, &controller::rings()) {
			Some(owner) => Err(RingError::RefusedWith(Refusal::ShortcutTaken, owner)),
			None => Ok(()),
		}
	}

	async fn pass(&mut self, changed: Option<Change<'_>>) -> Failed {
		pass(self.app, changed).await
	}

	async fn save(
		&mut self,
		ring_id: &str,
		kind: ShortcutKind,
		combo: Option<&str>,
	) -> Result<(), RingError> {
		let (id, combo) = (ring_id.to_string(), combo.map(str::to_string));
		self.db
			.with(move |conn| ring_store::set_shortcut(conn, &id, kind, combo.as_deref()))
			.await
			.map_err(RingError::Other)
			.and_then(|result| result)
	}

	async fn reload(&mut self) -> Result<(), RingError> {
		let rings = self
			.db
			.with(|conn| ring_store::list(conn))
			.await
			.map_err(RingError::Other)
			.and_then(|result| result)?;
		controller::set_rings(rings);
		Ok(())
	}
}

/// 링의 `kind` 단축키를 바꾼다. `shortcut` 이 `None` 이거나 비어 있으면 뗀다. 순서는 [`change_shortcut`] 에 있다.
/// 돌아왔을 때 메모리의 링 목록은 저장된 값이다. 창과 트레이에 알리는 일은 호출자가 한다.
pub async fn set_ring_shortcut(
	app: &AppHandle,
	db: &Db,
	ring_id: &str,
	kind: ShortcutKind,
	shortcut: Option<String>,
) -> Result<(), RingError> {
	let shortcut = shortcut.filter(|combo| !combo.trim().is_empty());
	let _op = begin().await;
	change_shortcut(
		&mut LiveSteps { app, db },
		ring_id,
		kind,
		shortcut.as_deref(),
	)
	.await
}

/// 설정 창이 `id` 번 입력을 받기 시작했다.
pub async fn capture_begin(app: &AppHandle, id: u64) {
	let op = begin().await;
	if capture().begin(id) {
		op.resync(app).await;
	}
}

/// 설정 창이 `id` 번 입력을 끝냈다.
pub async fn capture_end(app: &AppHandle, id: u64) {
	let op = begin().await;
	if capture().end(id) {
		op.resync(app).await;
	}
}

/// 설정 창이 사라졌다. 받던 입력이 있었으면 끝내고 떼어 둔 단축키를 다시 건다.
/// 새 창의 번호는 처음부터 다시 센다. main thread 에서 불러도 된다 — 등록은 task 가 한다.
pub fn capture_reset(app: &AppHandle) {
	let was_capturing = std::mem::take(&mut *capture()).active.is_some();
	if was_capturing {
		let app = app.clone();
		tauri::async_runtime::spawn(async move {
			resync(&app).await;
		});
	}
}

// ── dev-agent 의 반복 확인 ─────────────────────────────────────────────

/// 개발 빌드가 `combo` 하나만 실제로 OS 에 걸게 하거나(`Some`), 다시 아무것도 걸지 않게 한다(`None`).
/// 조합을 입력받아 거는 길을 진짜 plugin 호출로 되풀이해 보려고 있다. 키 입력은 만들지 않는다.
#[cfg(feature = "dev-agent")]
pub async fn dev_stress_gate(app: &AppHandle, combo: Option<Shortcut>) {
	let op = begin().await;
	*STRESS_ONLY.lock().unwrap_or_else(|e| e.into_inner()) = combo;
	op.resync(app).await;
}

/// plugin 이 지금 쥐고 있는 링의 조합들. main thread 에서 읽는다.
#[cfg(feature = "dev-agent")]
pub async fn dev_registered(app: &AppHandle) -> Vec<String> {
	let _op = begin().await;
	let (tx, rx) = tokio::sync::oneshot::channel();
	let handle = app.clone();
	let sent = app.run_on_main_thread(move || {
		let held = controller::rings()
			.into_iter()
			.flat_map(|ring| [ring.shortcut, ring.quick_shortcut])
			.flatten()
			.filter(|combo| {
				combo
					.parse::<Shortcut>()
					.is_ok_and(|parsed| handle.global_shortcut().is_registered(parsed))
			})
			.collect();
		let _ = tx.send(held);
	});
	if sent.is_err() {
		return Vec::new();
	}
	rx.await.unwrap_or_default()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn shortcut(text: &str) -> Shortcut {
		text.parse().expect("test shortcut must parse")
	}

	fn ring(id: &str, name: &str, combo: Option<&str>) -> Ring {
		ring_with_quick(id, name, combo, None)
	}

	fn ring_with_quick(id: &str, name: &str, combo: Option<&str>, quick: Option<&str>) -> Ring {
		Ring {
			id: id.to_string(),
			name: name.to_string(),
			shortcut: combo.map(str::to_string),
			quick_shortcut: quick.map(str::to_string),
			slot_count: 6,
			slots: Vec::new(),
		}
	}

	#[test]
	fn a_combo_with_a_strong_modifier_is_accepted() {
		for combo in [
			"Alt+Space",
			"CmdOrCtrl+Shift+KeyG",
			"Ctrl+Alt+Shift+F18",
			"Cmd+Digit1",
		] {
			assert!(parse_for_registration(combo).is_ok(), "{combo}");
		}
	}

	#[test]
	fn a_combo_that_would_swallow_typing_is_refused() {
		assert_eq!(
			parse_for_registration("KeyA"),
			Err(Refusal::ShortcutNeedsModifier)
		);
		assert_eq!(
			parse_for_registration("Shift+KeyA"),
			Err(Refusal::ShortcutNeedsModifier)
		);
		assert_eq!(
			parse_for_registration("F5"),
			Err(Refusal::ShortcutNeedsModifier)
		);
	}

	#[test]
	fn a_spare_function_key_needs_no_modifier() {
		assert!(parse_for_registration("F18").is_ok());
		assert!(parse_for_registration("Shift+F13").is_ok());
	}

	#[test]
	fn a_quick_shortcut_is_any_function_key_with_or_without_modifiers() {
		for combo in [
			"F1",
			"F12",
			"Ctrl+F5",
			"Alt+Shift+F9",
			"Cmd+F2",
			"Shift+F13",
			"F24",
		] {
			assert!(parse_for_quick(combo).is_ok(), "{combo}");
		}
	}

	#[test]
	fn a_quick_shortcut_without_a_function_key_is_refused() {
		for combo in ["Ctrl+KeyA", "Space", "Alt+Space", "Cmd+Digit1"] {
			assert_eq!(
				parse_for_quick(combo),
				Err(Refusal::ShortcutNeedsFunctionKey),
				"{combo}"
			);
		}
		assert_eq!(parse_for_quick("F99"), Err(Refusal::ShortcutInvalid));
	}

	#[test]
	fn each_kind_uses_its_own_rules() {
		assert!(parse_for(ShortcutKind::Quick, "F5").is_ok());
		assert_eq!(
			parse_for(ShortcutKind::Normal, "F5"),
			Err(Refusal::ShortcutNeedsModifier)
		);
		assert_eq!(
			parse_for(ShortcutKind::Quick, "Alt+Space"),
			Err(Refusal::ShortcutNeedsFunctionKey)
		);
	}

	#[test]
	fn an_unreadable_combo_is_invalid() {
		assert_eq!(
			parse_for_registration("Cmd+NotAKey"),
			Err(Refusal::ShortcutInvalid)
		);
		assert_eq!(parse_for_registration(""), Err(Refusal::ShortcutInvalid));
	}

	#[test]
	fn the_same_keys_spelled_differently_still_conflict() {
		let rings = [ring("a", "작업", Some("Ctrl+Shift+KeyT"))];
		assert_eq!(
			conflict(
				"b",
				ShortcutKind::Normal,
				&shortcut("Control+Shift+KeyT"),
				&rings
			),
			Some("작업".to_string())
		);
		assert_eq!(
			conflict(
				"b",
				ShortcutKind::Normal,
				&shortcut("Shift+Ctrl+KeyT"),
				&rings
			),
			Some("작업".to_string())
		);
	}

	#[test]
	fn a_ring_does_not_conflict_with_itself() {
		let rings = [ring("a", "작업", Some("Alt+Space"))];
		assert_eq!(
			conflict("a", ShortcutKind::Normal, &shortcut("Alt+Space"), &rings),
			None
		);
	}

	#[test]
	fn a_free_combo_and_an_unreadable_stored_value_do_not_conflict() {
		let rings = [
			ring("a", "작업", Some("Alt+Space")),
			ring("b", "깨진 값", Some("Alt+Shift+˝")),
			ring("c", "단축키 없음", None),
		];
		assert_eq!(
			conflict("z", ShortcutKind::Normal, &shortcut("Alt+KeyR"), &rings),
			None
		);
	}

	#[test]
	fn a_normal_and_a_quick_shortcut_of_different_rings_conflict() {
		let rings = [
			ring_with_quick("a", "작업", Some("Ctrl+F5"), None),
			ring_with_quick("b", "도구", None, Some("Alt+F6")),
		];
		assert_eq!(
			conflict("c", ShortcutKind::Quick, &shortcut("Ctrl+F5"), &rings),
			Some("작업".to_string())
		);
		assert_eq!(
			conflict("c", ShortcutKind::Normal, &shortcut("Alt+F6"), &rings),
			Some("도구".to_string())
		);
	}

	#[test]
	fn the_two_shortcuts_of_one_ring_cannot_be_the_same() {
		let rings = [ring_with_quick("a", "작업", Some("Ctrl+F5"), Some("F6"))];
		assert_eq!(
			conflict("a", ShortcutKind::Quick, &shortcut("Ctrl+F5"), &rings),
			Some("작업".to_string())
		);
		// 같은 자리에 같은 조합을 다시 저장하는 것은 충돌이 아니다.
		assert_eq!(
			conflict("a", ShortcutKind::Quick, &shortcut("F6"), &rings),
			None
		);
	}

	// ── 입력받는 중 ──

	#[test]
	fn a_capture_turns_on_once_and_off_once() {
		let mut capture = Capture::default();
		assert!(capture.begin(10));
		assert!(!capture.begin(10), "already on");
		assert!(capture.end(10));
		assert!(!capture.end(10), "already off");
		assert_eq!(capture.active, None);
	}

	#[test]
	fn a_begin_that_arrives_after_its_end_does_not_turn_capture_back_on() {
		// 화면은 시작 → 조합 저장 → 끝 순서로 보내지만, 서로 다른 task 라 뒤바뀌어 도착할 수 있다.
		let mut capture = Capture::default();
		assert!(!capture.end(7));
		assert!(!capture.begin(7));
		assert_eq!(capture.active, None);

		let mut capture = Capture::default();
		assert!(!capture.end_all());
		capture.begin(3);
		assert!(capture.end_all(), "saving a combo ends the capture");
		assert!(!capture.begin(3), "the late begin of the same capture");
		assert!(!capture.end(3));
		assert_eq!(capture.active, None);
	}

	#[test]
	fn the_end_of_an_older_capture_does_not_end_a_newer_one() {
		let mut capture = Capture::default();
		capture.begin(1);
		capture.begin(2);
		assert!(!capture.end(1));
		assert_eq!(capture.active, Some(2));
		assert!(capture.end(2));
	}

	// ── 무엇을 거는가 ──

	const OPEN: Gate = Gate {
		build: BuildGate::All,
		paused: false,
		capturing: false,
	};

	/// 일반 단축키만의 등록 목록.
	fn pairs(items: &[(&str, &str)]) -> Vec<Binding> {
		items
			.iter()
			.map(|(id, combo)| (id.to_string(), ShortcutKind::Normal, combo.to_string()))
			.collect()
	}

	#[test]
	fn every_ring_with_a_shortcut_is_registered_and_the_changed_one_uses_the_new_combo() {
		let rings = [
			ring("a", "A", Some("Alt+Space")),
			ring("b", "B", None),
			ring("c", "C", Some("Alt+KeyC")),
		];
		assert_eq!(
			plan(&rings, OPEN, None).register,
			pairs(&[("a", "Alt+Space"), ("c", "Alt+KeyC")])
		);
		assert_eq!(
			plan(
				&rings,
				OPEN,
				Some(("b", ShortcutKind::Normal, Some("Alt+KeyB")))
			)
			.register,
			pairs(&[("a", "Alt+Space"), ("b", "Alt+KeyB"), ("c", "Alt+KeyC")])
		);
		assert_eq!(
			plan(&rings, OPEN, Some(("a", ShortcutKind::Normal, None))).register,
			pairs(&[("c", "Alt+KeyC")])
		);
	}

	#[test]
	fn a_ring_registers_both_of_its_shortcuts_and_the_change_hits_only_its_kind() {
		let rings = [ring_with_quick("a", "A", Some("Alt+Space"), Some("F5"))];
		assert_eq!(
			plan(&rings, OPEN, None).register,
			[
				(
					"a".to_string(),
					ShortcutKind::Normal,
					"Alt+Space".to_string()
				),
				("a".to_string(), ShortcutKind::Quick, "F5".to_string()),
			]
		);
		assert_eq!(
			plan(&rings, OPEN, Some(("a", ShortcutKind::Quick, None))).register,
			[(
				"a".to_string(),
				ShortcutKind::Normal,
				"Alt+Space".to_string()
			)]
		);
	}

	#[test]
	fn nothing_stays_registered_while_paused_or_capturing_or_in_a_build_that_does_not_register() {
		let rings = [ring("a", "A", Some("Alt+Space"))];
		for gate in [
			Gate {
				paused: true,
				..OPEN
			},
			Gate {
				capturing: true,
				..OPEN
			},
			Gate {
				build: BuildGate::Nothing,
				..OPEN
			},
		] {
			assert_eq!(plan(&rings, gate, None), Plan::default(), "{gate:?}");
		}
	}

	#[test]
	fn a_combo_changed_while_paused_is_only_probed() {
		let rings = [ring("a", "A", Some("Alt+Space")), ring("b", "B", None)];
		let paused = Gate {
			paused: true,
			..OPEN
		};
		let planned = plan(
			&rings,
			paused,
			Some(("b", ShortcutKind::Normal, Some("Alt+KeyB"))),
		);
		assert!(planned.register.is_empty());
		assert_eq!(
			planned.probe,
			Some((
				"b".to_string(),
				ShortcutKind::Normal,
				"Alt+KeyB".to_string()
			))
		);
		// 떼는 것은 물어볼 것이 없다.
		assert_eq!(
			plan(&rings, paused, Some(("a", ShortcutKind::Normal, None))).probe,
			None
		);
		// 등록하지 않는 빌드는 잠깐도 걸지 않는다.
		let silent = Gate {
			build: BuildGate::Nothing,
			paused: true,
			capturing: false,
		};
		assert_eq!(
			plan(
				&rings,
				silent,
				Some(("b", ShortcutKind::Normal, Some("Alt+KeyB")))
			)
			.probe,
			None
		);
	}

	#[cfg(feature = "dev-agent")]
	#[test]
	fn the_stress_gate_registers_its_one_combo_and_nothing_else() {
		let only = BuildGate::Only(shortcut("Ctrl+Alt+Shift+F17"));
		let rings = [
			ring("a", "A", Some("Alt+Space")),
			ring("b", "B", Some("Control+Shift+Alt+F17")),
		];
		let gate = Gate {
			build: only,
			..OPEN
		};
		assert_eq!(
			plan(&rings, gate, None).register,
			pairs(&[("b", "Control+Shift+Alt+F17")])
		);
	}

	/// 부른 순서를 적는 가짜 OS. `refuse` 에 든 조합은 거절한다.
	#[derive(Default)]
	struct FakeOs {
		calls: Vec<String>,
		held: Vec<String>,
		refuse: Vec<&'static str>,
	}

	impl Registrar for FakeOs {
		fn unregister_all(&mut self) {
			self.calls.push("unregister_all".to_string());
			self.held.clear();
		}

		fn register(
			&mut self,
			_ring_id: &str,
			_kind: ShortcutKind,
			combo: &str,
		) -> Result<(), String> {
			self.calls.push(format!("register {combo}"));
			if self.refuse.contains(&combo) {
				return Err("refused".to_string());
			}
			self.held.push(combo.to_string());
			Ok(())
		}
	}

	#[test]
	fn a_pass_unregisters_everything_first_and_reports_the_rings_the_os_refused() {
		let mut os = FakeOs {
			refuse: vec!["Alt+KeyC"],
			..FakeOs::default()
		};
		let failed = run_pass(
			&mut os,
			Plan {
				register: pairs(&[("a", "Alt+Space"), ("c", "Alt+KeyC")]),
				probe: None,
			},
		);
		assert_eq!(
			os.calls,
			["unregister_all", "register Alt+Space", "register Alt+KeyC"]
		);
		assert_eq!(failed, [("c".to_string(), ShortcutKind::Normal)]);
		assert_eq!(os.held, ["Alt+Space"]);
	}

	#[test]
	fn a_probe_leaves_nothing_registered() {
		let mut os = FakeOs::default();
		let failed = run_pass(
			&mut os,
			Plan {
				register: Vec::new(),
				probe: Some((
					"b".to_string(),
					ShortcutKind::Normal,
					"Alt+KeyB".to_string(),
				)),
			},
		);
		assert!(failed.is_empty());
		assert!(os.held.is_empty());
		assert_eq!(
			os.calls,
			["unregister_all", "register Alt+KeyB", "unregister_all"]
		);
	}

	// ── 조합을 바꾸는 순서 ──

	/// 단계가 불린 순서를 적는 가짜.
	#[derive(Default)]
	struct FakeSteps {
		log: Vec<String>,
		capturing: bool,
		check_refuses: bool,
		os_refuses: bool,
		save_fails: bool,
	}

	impl ShortcutSteps for FakeSteps {
		fn end_capture(&mut self) -> bool {
			self.log.push("end capture".to_string());
			std::mem::take(&mut self.capturing)
		}

		fn check(
			&self,
			_ring_id: &str,
			_kind: ShortcutKind,
			_combo: &str,
		) -> Result<(), RingError> {
			if self.check_refuses {
				return Err(Refusal::ShortcutNeedsModifier.into());
			}
			Ok(())
		}

		async fn pass(&mut self, changed: Option<Change<'_>>) -> Failed {
			assert!(!self.capturing, "a pass must not run while capturing");
			match changed {
				Some((id, kind, combo)) => {
					self.log.push(format!("register with {combo:?}"));
					if self.os_refuses {
						return vec![(id.to_string(), kind)];
					}
				}
				None => self.log.push("register stored".to_string()),
			}
			Vec::new()
		}

		async fn save(
			&mut self,
			_ring_id: &str,
			_kind: ShortcutKind,
			combo: Option<&str>,
		) -> Result<(), RingError> {
			self.log.push(format!("save {combo:?}"));
			if self.save_fails {
				return Err(RingError::Other("disk".to_string()));
			}
			Ok(())
		}

		async fn reload(&mut self) -> Result<(), RingError> {
			self.log.push("reload".to_string());
			Ok(())
		}
	}

	#[tokio::test]
	async fn a_new_combo_ends_capture_then_registers_then_saves_then_reloads() {
		let mut steps = FakeSteps {
			capturing: true,
			..FakeSteps::default()
		};
		assert_eq!(
			change_shortcut(&mut steps, "a", ShortcutKind::Normal, Some("Alt+Space")).await,
			Ok(())
		);
		assert_eq!(
			steps.log,
			[
				"end capture",
				"register with Some(\"Alt+Space\")",
				"save Some(\"Alt+Space\")",
				"reload"
			]
		);
	}

	#[tokio::test]
	async fn a_combo_the_os_refuses_is_never_saved_and_the_stored_ones_come_back() {
		let mut steps = FakeSteps {
			capturing: true,
			os_refuses: true,
			..FakeSteps::default()
		};
		assert_eq!(
			change_shortcut(&mut steps, "a", ShortcutKind::Normal, Some("Alt+Space")).await,
			Err(Refusal::ShortcutUnavailable.into())
		);
		assert_eq!(
			steps.log,
			[
				"end capture",
				"register with Some(\"Alt+Space\")",
				"register stored"
			]
		);
	}

	#[tokio::test]
	async fn a_combo_refused_by_the_rules_restores_what_capture_took_down() {
		let mut steps = FakeSteps {
			capturing: true,
			check_refuses: true,
			..FakeSteps::default()
		};
		assert_eq!(
			change_shortcut(&mut steps, "a", ShortcutKind::Normal, Some("KeyA")).await,
			Err(Refusal::ShortcutNeedsModifier.into())
		);
		assert_eq!(steps.log, ["end capture", "register stored"]);

		// 입력받던 중이 아니면 등록은 그대로다. 건드리지 않는다.
		let mut steps = FakeSteps {
			check_refuses: true,
			..FakeSteps::default()
		};
		let _ = change_shortcut(&mut steps, "a", ShortcutKind::Normal, Some("KeyA")).await;
		assert_eq!(steps.log, ["end capture"]);
	}

	#[test]
	fn only_a_refused_stored_combo_is_kept_as_refused() {
		let failed = vec![
			("a".to_string(), ShortcutKind::Normal),
			("b".to_string(), ShortcutKind::Normal),
			("b".to_string(), ShortcutKind::Quick),
		];
		// 켤 때: 모두 저장된 조합이다.
		assert_eq!(stored_refusals(&failed, None), failed);
		// "b" 의 일반 단축키를 바꾸는 중이다. 그 거절은 저장된 조합의 거절이 아니다. 빠른 단축키의 거절은 남는다.
		assert_eq!(
			stored_refusals(&failed, Some(("b", ShortcutKind::Normal, Some("Cmd+KeyK")))),
			vec![
				("a".to_string(), ShortcutKind::Normal),
				("b".to_string(), ShortcutKind::Quick)
			]
		);
		assert!(stored_refusals(&[], None).is_empty());
	}

	#[tokio::test]
	async fn a_failed_save_puts_the_stored_registrations_back() {
		let mut steps = FakeSteps {
			save_fails: true,
			..FakeSteps::default()
		};
		assert!(
			change_shortcut(&mut steps, "a", ShortcutKind::Normal, None)
				.await
				.is_err()
		);
		assert_eq!(
			steps.log,
			[
				"end capture",
				"register with None",
				"save None",
				"register stored"
			]
		);
	}
}
