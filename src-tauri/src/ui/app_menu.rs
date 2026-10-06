// 앱 메뉴 — 메뉴 막대에는 보이지 않지만 ⌘C · ⌘V · ⌘W · ⌘Q 같은 키가 이 메뉴를 거친다
//
// Tauri 의 기본 메뉴와 같은 짜임이다. 끝내기 하나만 다르다. 기본 끝내기는 `NSApp terminate:` 로 곧바로
// 끝내서 설정 창이 저장하지 않은 입력을 저장할 틈이 없다. 여기서는 트레이의 끝내기와 같은 길로 보낸다.
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Wry};

use crate::ui::settings_window::{self, Leave};

/// 끝내기 항목의 id. 트레이 메뉴의 `quit` 과 겹치지 않게 한다 — 메뉴 이벤트는 앱 전체에서 한 곳으로 온다.
const ID_QUIT: &str = "app-quit";

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
	let name = app.package_info().name.clone();
	Menu::with_items(
		app,
		&[
			&Submenu::with_items(
				app,
				&name,
				true,
				&[
					&PredefinedMenuItem::hide(app, None)?,
					&PredefinedMenuItem::hide_others(app, None)?,
					&PredefinedMenuItem::separator(app)?,
					&MenuItem::with_id(
						app,
						ID_QUIT,
						format!("Quit {name}"),
						true,
						Some("CmdOrCtrl+Q"),
					)?,
				],
			)?,
			&Submenu::with_items(
				app,
				"File",
				true,
				&[&PredefinedMenuItem::close_window(app, None)?],
			)?,
			&Submenu::with_items(
				app,
				"Edit",
				true,
				&[
					&PredefinedMenuItem::undo(app, None)?,
					&PredefinedMenuItem::redo(app, None)?,
					&PredefinedMenuItem::separator(app)?,
					&PredefinedMenuItem::cut(app, None)?,
					&PredefinedMenuItem::copy(app, None)?,
					&PredefinedMenuItem::paste(app, None)?,
					&PredefinedMenuItem::select_all(app, None)?,
				],
			)?,
			&Submenu::with_items(
				app,
				"Window",
				true,
				&[
					&PredefinedMenuItem::minimize(app, None)?,
					&PredefinedMenuItem::maximize(app, None)?,
				],
			)?,
		],
	)
}

/// 앱 메뉴의 항목을 골랐다. 트레이 메뉴의 항목도 여기로 오지만 id 가 달라 지나간다.
pub fn on_menu_event(app: &AppHandle, id: &str) {
	if id == ID_QUIT {
		settings_window::request_leave(app, Leave::Quit);
	}
}
