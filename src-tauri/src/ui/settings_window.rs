// 설정 창 — 필요할 때 만들고 닫으면 부순다. 페이지가 다 그려진 뒤에 보인다
//
// 닫기 전에 페이지에 먼저 묻는다. 글 입력 칸은 포커스를 잃을 때 저장하는데, 창이 사라질 때는 그 저장이
// 돌 틈이 없다. 그래서 창 닫기(빨간 단추, ⌘W)와 앱 끝내기(트레이, ⌘Q)는 [`request_leave`] 를 거친다.
// 페이지가 저장하고 "됐다" 고 답하면 그때 닫는다. 저장할 수 없는 입력이 남았으면 창을 그대로 두고
// 페이지가 그 까닭을 보인다. 페이지가 답하지 않으면 [`LEAVE_DEADLINE`] 뒤에 닫는다 — 닫히지 않는 창은 없다.
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::constants::{events, window_labels};

/// 페이지가 준비됐다고 알리지 않아도 이 시간이 지나면 창을 보인다. 보이지 않는 창은 닫을 수도 없다.
const REVEAL_DEADLINE: Duration = Duration::from_millis(2500);

/// 페이지가 저장을 끝내고 답하기를 기다리는 시간.
const LEAVE_DEADLINE: Duration = Duration::from_millis(2000);

/// 설정 창을 떠나려는 까닭.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leave {
	/// 창만 닫는다.
	Close,
	/// 앱을 끝낸다.
	Quit,
}

/// 페이지의 답을 기다리는 요청. `(요청 번호, 까닭)`.
static PENDING: Mutex<Option<(u64, Leave)>> = Mutex::new(None);
static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);
/// 페이지가 첫 화면을 그렸는가. 그리기 전의 페이지는 물어도 답하지 못하고, 저장할 입력도 없다.
static PAGE_READY: AtomicBool = AtomicBool::new(false);

fn pending() -> std::sync::MutexGuard<'static, Option<(u64, Leave)>> {
	PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// `id` 번 요청의 답이 왔다. 기다리던 요청이면 꺼내고, 떠나도 되면 그 까닭을 돌려준다.
/// 이미 시간이 지나 처리했거나 더 새 요청으로 바뀐 번호면 아무 일도 없다.
fn settle(pending: &mut Option<(u64, Leave)>, id: u64, may_leave: bool) -> Option<Leave> {
	match *pending {
		Some((waiting, leave)) if waiting == id => {
			*pending = None;
			may_leave.then_some(leave)
		}
		_ => None,
	}
}

fn perform(app: &AppHandle, leave: Leave) {
	match leave {
		Leave::Close => {
			if let Some(window) = app.get_webview_window(window_labels::SETTINGS) {
				// `destroy` 는 닫기 요청을 다시 내지 않는다.
				if let Err(e) = window.destroy() {
					log::warn!("[settings] failed to close the window: {e}");
				}
			}
		}
		Leave::Quit => app.exit(0),
	}
}

/// 설정 창을 닫거나 앱을 끝낸다. 창이 떠 있으면 저장하지 않은 입력을 먼저 저장하게 한다.
pub fn request_leave(app: &AppHandle, leave: Leave) {
	let page_can_answer = app.get_webview_window(window_labels::SETTINGS).is_some()
		&& PAGE_READY.load(Ordering::SeqCst);
	if !page_can_answer {
		perform(app, leave);
		return;
	}
	let id = NEXT_REQUEST.fetch_add(1, Ordering::SeqCst);
	*pending() = Some((id, leave));
	if let Err(e) = app.emit_to(window_labels::SETTINGS, events::SETTINGS_LEAVE_REQUEST, id) {
		log::warn!(
			"[settings] failed to emit {}: {e}",
			events::SETTINGS_LEAVE_REQUEST
		);
		pending().take();
		perform(app, leave);
		return;
	}
	let handle = app.clone();
	std::thread::spawn(move || {
		std::thread::sleep(LEAVE_DEADLINE);
		// 답이 왔으면 `pending` 은 비었거나 다른 번호다.
		if settle(&mut pending(), id, true).is_none() {
			return;
		}
		log::warn!("[settings] the page did not answer the leave request in time — leaving anyway");
		let app = handle.clone();
		if let Err(e) = handle.run_on_main_thread(move || perform(&app, leave)) {
			log::warn!("[settings] failed to reach the main thread: {e}");
		}
	});
}

/// 페이지의 답. `may_leave` 가 거짓이면 저장할 수 없는 입력이 남았다 — 창을 그대로 두고 앞으로 가져온다.
/// main thread 에서 부른다.
pub fn leave_answer(app: &AppHandle, id: u64, may_leave: bool) {
	let settled = settle(&mut pending(), id, may_leave);
	match settled {
		Some(leave) => perform(app, leave),
		None if !may_leave => reveal(app),
		None => {}
	}
}

/// 페이지가 첫 화면을 그렸다.
pub fn page_ready(app: &AppHandle) {
	PAGE_READY.store(true, Ordering::SeqCst);
	reveal(app);
}

/// 개발용 확인에서 창이 키보드를 가져가지 않게 한다 (`RINGRING_DEV_QUIET=1`). 개발 빌드와 dev-agent 를
/// 넣은 빌드에서만 듣는다 — 배포하는 빌드는 이 값을 읽지 않는다.
/// 켜면 창을 만들 때 앱을 활성화하지 않고, 보일 때 포커스를 주지 않는다 — 사용자가 치던 글이 끊기지 않는다.
fn dev_quiet() -> bool {
	(cfg!(dev) || cfg!(feature = "dev-agent"))
		&& std::env::var("RINGRING_DEV_QUIET").as_deref() == Ok("1")
}

/// 설정 창을 연다. 이미 있으면 앞으로 가져온다. `page` 를 주면 그 페이지를 보인다.
pub fn open(app: &AppHandle, page: Option<&str>) {
	if app.get_webview_window(window_labels::SETTINGS).is_some() {
		if let Some(page) = page {
			if let Err(e) = app.emit_to(window_labels::SETTINGS, events::SETTINGS_NAVIGATE, page) {
				log::warn!(
					"[settings] failed to emit {}: {e}",
					events::SETTINGS_NAVIGATE
				);
			}
		}
		reveal(app);
		return;
	}

	let url = match page {
		Some(page) => format!("index.html?page={page}"),
		None => "index.html".to_string(),
	};
	let builder =
		WebviewWindowBuilder::new(app, window_labels::SETTINGS, WebviewUrl::App(url.into()))
			.title("RingRing")
			.inner_size(1000.0, 660.0)
			.min_inner_size(900.0, 580.0)
			.title_bar_style(tauri::TitleBarStyle::Overlay)
			.hidden_title(true)
			.traffic_light_position(tauri::LogicalPosition::new(18.0, 20.0))
			.visible(false)
			.center();
	// wry 는 webview 를 붙일 때 앱을 활성화한다. 조용한 개발 확인에서는 그 활성화를 건너뛴다.
	let built = if dev_quiet() {
		crate::ui::panel::build(builder)
	} else {
		builder.build()
	};
	let window = match built {
		Ok(window) => window,
		Err(e) => {
			log::error!("[settings] failed to create the window: {e}");
			return;
		}
	};

	let handle = app.clone();
	PAGE_READY.store(false, Ordering::SeqCst);
	window.on_window_event(move |event| match event {
		// 빨간 단추와 ⌘W. 곧바로 닫지 않고 페이지에 먼저 묻는다.
		tauri::WindowEvent::CloseRequested { api, .. } => {
			api.prevent_close();
			request_leave(&handle, Leave::Close);
		}
		tauri::WindowEvent::Destroyed => {
			PAGE_READY.store(false, Ordering::SeqCst);
			pending().take();
			// 조합을 입력받다가 창이 닫혔으면 떼어 둔 단축키를 다시 건다.
			crate::shortcuts::capture_reset(&handle);
		}
		_ => {}
	});

	let handle = app.clone();
	std::thread::spawn(move || {
		std::thread::sleep(REVEAL_DEADLINE);
		let hidden = handle
			.get_webview_window(window_labels::SETTINGS)
			.is_some_and(|window| !window.is_visible().unwrap_or(true));
		if hidden {
			log::warn!("[settings] the page did not report ready in time — showing the window");
			reveal(&handle);
		}
	});
}

/// 창을 보이고 키보드를 준다. 페이지가 준비됐다고 알릴 때 부른다.
pub fn reveal(app: &AppHandle) {
	let Some(window) = app.get_webview_window(window_labels::SETTINGS) else {
		return;
	};
	if dev_quiet() {
		// `show()` 는 창을 key 로 만든다. 여기서는 올리기만 한다.
		if let Err(e) = crate::ui::panel::order_front(&window) {
			log::warn!("[settings] failed to show the window: {e}");
		}
		return;
	}
	if let Err(e) = window.show() {
		log::warn!("[settings] failed to show the window: {e}");
	}
	if let Err(e) = window.unminimize() {
		log::warn!("[settings] failed to restore the window: {e}");
	}
	if let Err(e) = window.set_focus() {
		log::warn!("[settings] failed to focus the window: {e}");
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_awaited_answer_decides_and_clears_the_request() {
		let mut pending = Some((4, Leave::Quit));
		assert_eq!(settle(&mut pending, 4, true), Some(Leave::Quit));
		assert_eq!(pending, None);

		let mut pending = Some((5, Leave::Close));
		assert_eq!(settle(&mut pending, 5, false), None, "unsaved input stays");
		assert_eq!(pending, None);
	}

	#[test]
	fn a_stale_answer_changes_nothing() {
		// 시간이 지나 이미 닫았거나, 그사이 새 요청이 왔다.
		let mut pending = None;
		assert_eq!(settle(&mut pending, 4, true), None);

		let mut pending = Some((6, Leave::Close));
		assert_eq!(settle(&mut pending, 4, true), None);
		assert_eq!(pending, Some((6, Leave::Close)));
	}
}
