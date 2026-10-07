// 앱 상태와 설정 command — 언어, 테마, 단축키 일시 정지, 로그인 항목, 손쉬운 사용 권한
use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::{db, emit_all};
use crate::constants::events;
use crate::error::CommandError;
use crate::login_item::{self, LoginItemState};
use crate::ring::controller;
use crate::ring::keystroke;
use crate::ring::model::{RingAction, ShortcutKind};
use crate::settings::{self, Language, Locale, Theme};
use crate::store::settings as settings_store;
use crate::ui::{settings_window, tray};

/// 시스템 설정의 손쉬운 사용 화면.
const ACCESSIBILITY_SETTINGS_URL: &str =
	"x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

/// 설정 창이 그리는 앱 상태. 화면은 이 값을 그리기만 한다.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateView {
	pub version: String,
	/// 개발 빌드인가. 개발 빌드는 로그인 항목을 등록하지 않는다.
	pub is_dev: bool,
	pub language: Language,
	/// 실제로 쓰는 언어. `language` 가 `system` 이면 OS 의 언어다.
	pub locale: Locale,
	pub theme: Theme,
	pub paused: bool,
	/// 이 빌드가 전역 단축키를 OS 에 등록하는가. 개발 빌드는 기본으로 등록하지 않는다.
	pub shortcuts_register: bool,
	pub login_item: LoginItemState,
	/// 다른 앱에 키 입력을 보낼 수 있는가 (손쉬운 사용 권한).
	pub accessibility_trusted: bool,
	/// 키 입력 칸이 하나라도 있는가. 있는데 권한이 없으면 화면이 권한 페이지를 가리킨다.
	pub uses_keystrokes: bool,
	/// 저장된 일반 단축키를 OS 가 받지 않은 링의 id. 편집기가 그 링에 "등록되지 않음" 을 보인다.
	pub refused_shortcuts: Vec<String>,
	/// 저장된 빠른 단축키를 OS 가 받지 않은 링의 id.
	pub refused_quick_shortcuts: Vec<String>,
	/// 아직 보지 않은 실패의 수. 왼쪽 줄의 "실행 기록" 이 점으로 알린다.
	pub unseen_failures: usize,
	/// 로그 파일의 경로. 정보 페이지가 보인다.
	pub log_path: String,
}

fn view(app: &AppHandle) -> AppStateView {
	let current = settings::current();
	AppStateView {
		version: app.package_info().version.to_string(),
		is_dev: cfg!(dev),
		language: current.language,
		locale: settings::locale(),
		theme: current.theme,
		paused: current.paused,
		shortcuts_register: crate::shortcuts::build_registers(),
		login_item: login_item::state(),
		accessibility_trusted: keystroke::trusted(),
		uses_keystrokes: controller::rings().iter().any(|ring| {
			ring.slots
				.iter()
				.any(|slot| matches!(slot.action, RingAction::Keystroke { .. }))
		}),
		refused_shortcuts: crate::shortcuts::refused(ShortcutKind::Normal),
		refused_quick_shortcuts: crate::shortcuts::refused(ShortcutKind::Quick),
		unseen_failures: crate::runs::unseen_failures(),
		log_path: app
			.path()
			.app_log_dir()
			.map(|dir| {
				dir.join(format!("{}.log", crate::constants::LOG_FILE_NAME))
					.to_string_lossy()
					.into_owned()
			})
			.unwrap_or_default(),
	}
}

/// 상태가 바뀌었다. 열려 있는 창에 알리고 트레이를 맞춘다.
fn changed(app: &AppHandle) {
	emit_all(app, events::APP_STATE_CHANGED);
	let handle = app.clone();
	if let Err(e) = app.run_on_main_thread(move || tray::refresh(&handle)) {
		log::warn!("[app] failed to reach the main thread: {e}");
	}
}

#[tauri::command]
pub fn app_state(app: AppHandle) -> AppStateView {
	view(&app)
}

#[tauri::command]
pub async fn settings_set_language(
	app: AppHandle,
	language: Language,
) -> Result<AppStateView, CommandError> {
	db(&app)
		.with(move |conn| settings_store::save_language(conn, language))
		.await??;
	settings::update(|current| current.language = language);
	changed(&app);
	Ok(view(&app))
}

#[tauri::command]
pub async fn settings_set_theme(
	app: AppHandle,
	theme: Theme,
) -> Result<AppStateView, CommandError> {
	db(&app)
		.with(move |conn| settings_store::save_theme(conn, theme))
		.await??;
	settings::update(|current| current.theme = theme);
	changed(&app);
	Ok(view(&app))
}

/// 단축키 일시 정지를 저장하고 OS 등록을 맞춘다. 트레이 메뉴와 설정 화면이 같이 쓴다.
///
/// 저장, 메모리, OS 등록을 한 묶음으로 한다 ([`crate::shortcuts::Op`]). 두 번의 바꾸기가 겹쳐도 셋이 서로
/// 다른 값을 갖지 않는다.
pub(crate) async fn apply_paused(app: &AppHandle, paused: bool) -> Result<(), CommandError> {
	let op = crate::shortcuts::begin().await;
	db(app)
		.with(move |conn| settings_store::save_paused(conn, paused))
		.await??;
	settings::update(|current| current.paused = paused);
	op.resync(app).await;
	drop(op);
	if paused {
		// 떠 있는 링은 닫는다. 단축키를 놓는 이벤트가 더는 오지 않는다.
		let handle = app.clone();
		if let Err(e) = app.run_on_main_thread(move || controller::hide(&handle)) {
			log::warn!("[app] failed to reach the main thread: {e}");
		}
	}
	changed(app);
	Ok(())
}

#[tauri::command]
pub async fn settings_set_paused(
	app: AppHandle,
	paused: bool,
) -> Result<AppStateView, CommandError> {
	apply_paused(&app, paused).await?;
	Ok(view(&app))
}

/// 로그인 항목을 등록하거나 뗀다. 상태는 저장하지 않는다 — OS 에 다시 묻는다.
pub(crate) fn apply_launch_at_login(app: &AppHandle, enabled: bool) -> Result<(), CommandError> {
	let result = login_item::set(enabled).map_err(CommandError::failed);
	// 실패해도 알린다. 트레이의 체크 표시가 OS 의 실제 상태로 돌아간다.
	changed(app);
	result
}

#[tauri::command]
pub fn settings_set_launch_at_login(
	app: AppHandle,
	enabled: bool,
) -> Result<AppStateView, CommandError> {
	apply_launch_at_login(&app, enabled)?;
	Ok(view(&app))
}

/// 시스템 설정의 로그인 항목 화면을 연다. 등록은 됐는데 사용자의 허용이 필요할 때 쓴다.
#[tauri::command]
pub fn login_items_open_settings() -> Result<(), CommandError> {
	tauri_plugin_opener::open_url(login_item::LOGIN_ITEMS_SETTINGS_URL, None::<&str>)
		.map_err(|e| CommandError::failed(e.to_string()))
}

/// 시스템 설정의 손쉬운 사용 화면을 연다. 사용자가 설정 화면의 단추를 눌렀을 때만 부른다.
/// 권한을 묻는 시스템 창은 이 앱이 스스로 띄우지 않는다.
#[tauri::command]
pub fn accessibility_open_settings() -> Result<(), CommandError> {
	tauri_plugin_opener::open_url(ACCESSIBILITY_SETTINGS_URL, None::<&str>)
		.map_err(|e| CommandError::failed(e.to_string()))
}

/// 설정 창이 단축키 조합을 입력받기 시작했다. 받는 동안은 링의 단축키를 전부 뗀다.
///
/// `id` 는 그 입력의 번호다. 화면이 입력마다 더 큰 수를 준다. "시작" 과 "끝" 이 뒤바뀌어 도착해도 Rust 가
/// 번호로 바로잡는다 (`shortcuts::Capture`). async 다 — 등록을 고치는 일은 main thread 가 기다리지 않는다.
#[tauri::command]
pub async fn shortcut_capture_begin(app: AppHandle, id: u64) {
	crate::shortcuts::capture_begin(&app, id).await;
}

/// 설정 창이 `id` 번 입력을 끝냈다. 링의 조합을 바꾸는 command 는 스스로 끝내므로 이것을 기다리지 않는다.
#[tauri::command]
pub async fn shortcut_capture_end(app: AppHandle, id: u64) {
	crate::shortcuts::capture_end(&app, id).await;
}

/// 설정 페이지가 첫 화면을 그렸다. 이제 창을 보인다.
#[tauri::command]
pub fn settings_ready(app: AppHandle) {
	settings_window::page_ready(&app);
}

/// 창을 닫거나 앱을 끝내도 되는지에 대한 페이지의 답 (`settings_window::request_leave`).
/// `may_leave` 가 거짓이면 저장할 수 없는 입력이 남았다. 창은 그대로 있고 페이지가 까닭을 보인다.
#[tauri::command]
pub fn settings_leave_answer(app: AppHandle, id: u64, may_leave: bool) {
	settings_window::leave_answer(&app, id, may_leave);
}
