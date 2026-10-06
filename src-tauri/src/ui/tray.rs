// 트레이 아이콘과 그 메뉴 — 이 앱에 늘 보이는 UI 는 이것뿐이다
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Wry};

use crate::ring::controller;
use crate::ui::{settings_window, texts};

const TRAY_ID: &str = "main";
const ID_SETTINGS: &str = "settings";
const ID_PAUSE: &str = "pause";
const ID_LOGIN: &str = "login";
const ID_ABOUT: &str = "about";
const ID_QUIT: &str = "quit";
const ID_NO_RINGS: &str = "no-rings";
const ID_FAILED_RUNS: &str = "failed-runs";
/// 보지 않은 실패가 있을 때 아이콘 옆에 보이는 글자.
const FAILED_MARK: &str = "!";
/// 링 항목의 id 접두사. 뒤에 링 id 가 붙는다.
const RING_PREFIX: &str = "ring:";
/// 메뉴에 올리는 링의 최대 수. 넘는 링은 설정에서 연다.
const MAX_RING_ITEMS: usize = 12;

const ICON: &[u8] = include_bytes!("../../icons/tray.png");
const ICON_PAUSED: &[u8] = include_bytes!("../../icons/tray-paused.png");

fn icon(paused: bool) -> tauri::Result<Image<'static>> {
	Image::from_bytes(if paused { ICON_PAUSED } else { ICON })
}

/// 지금 상태로 메뉴를 만든다.
fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
	let locale = crate::settings::locale();
	let texts = texts::tray(locale, &app.package_info().version.to_string());
	let menu = Menu::new(app)?;

	// 보지 않은 실패가 있으면 맨 위에서 알린다. OS 알림이 꺼져 있어도 여기서 보인다.
	let failed = crate::runs::unseen_failures();
	if failed > 0 {
		menu.append(&MenuItem::with_id(
			app,
			ID_FAILED_RUNS,
			(texts.failed_runs)(failed),
			true,
			None::<&str>,
		)?)?;
		menu.append(&PredefinedMenuItem::separator(app)?)?;
	}

	// 칸이 하나도 없는 링은 띄울 것이 없다. 메뉴에 올리지 않는다.
	let rings: Vec<_> = controller::rings()
		.into_iter()
		.filter(|ring| !ring.slots.is_empty())
		.take(MAX_RING_ITEMS)
		.collect();
	if rings.is_empty() {
		menu.append(&MenuItem::with_id(
			app,
			ID_NO_RINGS,
			texts.no_rings,
			false,
			None::<&str>,
		)?)?;
	}
	for ring in rings {
		let id = format!("{RING_PREFIX}{}", ring.id);
		// 단축키를 메뉴의 오른쪽에 보인다. 메뉴가 읽지 못하는 조합이면 이름만 보인다.
		let item = MenuItem::with_id(app, &id, &ring.name, true, ring.shortcut.as_deref())
			.or_else(|_| MenuItem::with_id(app, &id, &ring.name, true, None::<&str>))?;
		menu.append(&item)?;
	}

	let login = crate::login_item::state();
	menu.append(&PredefinedMenuItem::separator(app)?)?;
	menu.append(&MenuItem::with_id(
		app,
		ID_SETTINGS,
		texts.settings,
		true,
		None::<&str>,
	)?)?;
	menu.append(&CheckMenuItem::with_id(
		app,
		ID_PAUSE,
		texts.pause,
		true,
		crate::settings::current().paused,
		None::<&str>,
	)?)?;
	menu.append(&CheckMenuItem::with_id(
		app,
		ID_LOGIN,
		texts.launch_at_login,
		login.available,
		login.enabled,
		None::<&str>,
	)?)?;
	menu.append(&PredefinedMenuItem::separator(app)?)?;
	menu.append(&MenuItem::with_id(
		app,
		ID_ABOUT,
		&texts.about,
		true,
		None::<&str>,
	)?)?;
	menu.append(&MenuItem::with_id(
		app,
		ID_QUIT,
		&texts.quit,
		true,
		None::<&str>,
	)?)?;
	Ok(menu)
}

/// 메뉴의 항목들을 글로 적는다. 개발용 확인이 메뉴를 열지 않고 그 내용을 읽는다.
#[cfg(feature = "dev-agent")]
pub(crate) fn describe(app: &AppHandle) -> tauri::Result<serde_json::Value> {
	use tauri::menu::MenuItemKind;
	let items = build_menu(app)?
		.items()?
		.into_iter()
		.map(|item| match item {
			MenuItemKind::MenuItem(item) => Ok(serde_json::json!({
				"id": item.id().as_ref(), "text": item.text()?, "enabled": item.is_enabled()?,
			})),
			MenuItemKind::Check(item) => Ok(serde_json::json!({
				"id": item.id().as_ref(), "text": item.text()?, "enabled": item.is_enabled()?,
				"checked": item.is_checked()?,
			})),
			MenuItemKind::Predefined(_) => Ok(serde_json::json!({ "separator": true })),
			_ => Ok(serde_json::json!({ "other": true })),
		})
		.collect::<tauri::Result<Vec<_>>>()?;
	let rect = app
		.tray_by_id(TRAY_ID)
		.and_then(|tray| tray.rect().ok().flatten())
		.map(|rect| format!("{:?} {:?}", rect.position, rect.size));
	Ok(serde_json::json!({ "items": items, "iconRect": rect }))
}

/// 트레이 아이콘을 만든다. 시작할 때 한 번 부른다.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
	let paused = crate::settings::current().paused;
	TrayIconBuilder::with_id(TRAY_ID)
		.icon(icon(paused)?)
		.icon_as_template(true)
		.menu(&build_menu(app)?)
		.show_menu_on_left_click(true)
		.on_menu_event(|app, event| on_menu_event(app, event.id().as_ref()))
		.build(app)?;
	refresh(app);
	Ok(())
}

/// 메뉴와 아이콘을 지금 상태에 맞춘다. 링·언어·일시 정지·로그인 항목이 바뀔 때마다 부른다.
pub fn refresh(app: &AppHandle) {
	let Some(tray) = app.tray_by_id(TRAY_ID) else {
		return;
	};
	let paused = crate::settings::current().paused;
	let texts = texts::tray(
		crate::settings::locale(),
		&app.package_info().version.to_string(),
	);
	let result = build_menu(app)
		.and_then(|menu| tray.set_menu(Some(menu)))
		.and_then(|()| tray.set_icon(Some(icon(paused)?)))
		.and_then(|()| tray.set_icon_as_template(true))
		.and_then(|()| tray.set_title((crate::runs::unseen_failures() > 0).then_some(FAILED_MARK)))
		.and_then(|()| {
			tray.set_tooltip(Some(if paused {
				&texts.tooltip_paused
			} else {
				&texts.tooltip
			}))
		});
	if let Err(e) = result {
		log::warn!("[tray] failed to refresh: {e}");
	}
}

pub(crate) fn on_menu_event(app: &AppHandle, id: &str) {
	if let Some(ring_id) = id.strip_prefix(RING_PREFIX) {
		let (handle, ring_id) = (app.clone(), ring_id.to_string());
		// 창을 올리는 일은 main thread 에서 한다.
		if let Err(e) = app.run_on_main_thread(move || {
			if let Err(refusal) = controller::show_open(&handle, &ring_id, None, true) {
				log::warn!("[tray] the ring was not shown: {refusal}");
			}
		}) {
			log::warn!("[tray] failed to reach the main thread: {e}");
		}
		return;
	}
	match id {
		ID_SETTINGS => settings_window::open(app, None),
		ID_ABOUT => settings_window::open(app, Some("about")),
		ID_FAILED_RUNS => settings_window::open(app, Some("runs")),
		ID_PAUSE => {
			let app = app.clone();
			tauri::async_runtime::spawn(async move {
				let paused = !crate::settings::current().paused;
				if let Err(e) = crate::commands::app::apply_paused(&app, paused).await {
					log::warn!("[tray] failed to change pause: {}", e.message);
				}
			});
		}
		ID_LOGIN => {
			let enabled = !crate::login_item::state().enabled;
			if let Err(e) = crate::commands::app::apply_launch_at_login(app, enabled) {
				log::warn!("[tray] failed to change the login item: {}", e.message);
			}
		}
		// 설정 창에 저장하지 않은 입력이 있으면 먼저 저장하게 한다.
		ID_QUIT => settings_window::request_leave(app, settings_window::Leave::Quit),
		_ => {}
	}
}
