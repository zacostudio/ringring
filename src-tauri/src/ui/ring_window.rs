// 링 창 — 커서 자리에 뜨는 작은 투명 panel 하나
//
// 창은 하나다. 숨겼다가 다시 쓴다. 무엇을 보일지와 어느 칸이 켜졌는지는 `ring/controller.rs` 가
// 정하고, 이 파일은 창을 만들고 옮기고 숨기기만 한다.
//
// non-activating panel 이다 (`ui/panel.rs`). 다른 앱 위에 떠도 이 앱이 활성화되지 않는다.
// 누르고 있는 동안(hold)에는 key 도 받지 않는다 — 키보드는 맨 앞 앱에 그대로 있다.
// 짧게 눌러 남은 링(tap)만 key 를 받아 숫자 키와 Esc 를 듣는다.
//
// vibrancy 는 쓰지 않는다. 도넛 모양으로 자를 수 없다. 창 그림자도 끈다 — 그림자는 페이지가 링 모양을 따라 그린다.

use tauri::window::Color;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::constants::window_labels;
use crate::ring::controller;
use crate::ring::geometry::WINDOW_SIZE;

/// 창을 만든다. 이미 있으면 그 창을 돌려준다. 보이지 않는 채로 만든다.
pub(crate) fn ensure(app: &AppHandle) -> Result<WebviewWindow, String> {
	if let Some(window) = app.get_webview_window(window_labels::RING) {
		return Ok(window);
	}

	// `WebviewUrl::App` 이어야 IPC 가 붙는다.
	let url = WebviewUrl::App("index.html?window=ring".into());
	let builder = WebviewWindowBuilder::new(app, window_labels::RING, url)
		.title("RingRing")
		.inner_size(WINDOW_SIZE, WINDOW_SIZE)
		.resizable(false)
		.decorations(false)
		.transparent(true)
		.background_color(Color(0, 0, 0, 0))
		.shadow(false)
		.visible(false)
		.always_on_top(true)
		.skip_taskbar(true)
		.focused(false)
		// 다른 앱이 앞에 있을 때 첫 클릭을 창 활성화에만 쓰지 않고 페이지로도 넘긴다.
		.accept_first_mouse(true);

	let window = crate::ui::panel::build(builder).map_err(|e| e.to_string())?;
	crate::ui::panel::make_nonactivating(&window);

	let app_handle = app.clone();
	window.on_window_event(move |event| match event {
		tauri::WindowEvent::CloseRequested { api, .. } => {
			// ⌘W 같은 닫기 요청은 링을 닫는 것이다. 창은 남긴다.
			api.prevent_close();
			controller::hide(&app_handle);
		}
		tauri::WindowEvent::Focused(false) => controller::focus_lost(&app_handle),
		_ => {}
	});

	Ok(window)
}

/// 창을 부순다. 단축키가 걸린 링이 하나도 없으면 창을 들고 있을 이유가 없다 — 숨긴 창도 WebContent
/// 프로세스 하나다.
pub(crate) fn destroy(app: &AppHandle) {
	if let Some(window) = app.get_webview_window(window_labels::RING) {
		if let Err(e) = window.destroy() {
			log::warn!("[ring] failed to destroy the window: {e}");
		}
	}
}

/// 링의 중심이 `(center_x, center_y)` 에 오게 창을 옮기고 다른 앱 위로 올린다. key 는 주지 않는다.
pub(crate) fn show_at(window: &WebviewWindow, center_x: f64, center_y: f64) -> Result<(), String> {
	crate::ui::panel::place_and_front(
		window,
		center_x - WINDOW_SIZE / 2.0,
		center_y - WINDOW_SIZE / 2.0,
	)
}

pub(crate) fn hide(app: &AppHandle) {
	if let Some(window) = app.get_webview_window(window_labels::RING) {
		if let Err(e) = crate::ui::panel::hide(&window) {
			log::warn!("[ring] failed to hide the window: {e}");
		}
	}
}
