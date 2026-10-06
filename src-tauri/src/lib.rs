// 링링이 — 커서 주위에 뜨는 링 메뉴. 메뉴 막대에만 사는 앱이다
mod commands;
mod constants;
#[cfg(feature = "dev-agent")]
mod dev_agent;
mod error;
mod exec;
mod login_item;
mod ring;
mod runs;
mod settings;
mod shortcuts;
mod store;
mod ui;

use tauri::Manager;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

use crate::store::Db;

fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
	let mut builder = tauri_plugin_log::Builder::new()
		.clear_targets()
		.target(Target::new(TargetKind::LogDir {
			file_name: Some(constants::LOG_FILE_NAME.to_string()),
		}))
		.level(log::LevelFilter::Info)
		.max_file_size(2_000_000)
		.rotation_strategy(RotationStrategy::KeepOne)
		.timezone_strategy(TimezoneStrategy::UseLocal);
	if cfg!(dev) {
		builder = builder.target(Target::new(TargetKind::Stdout));
	}
	builder.build()
}

/// 시작할 때 한 번. 저장소를 열고, 링과 설정을 메모리에 올리고, 트레이와 단축키를 건다.
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
	// Dock 아이콘이 없는 앱이다. 번들의 `LSUIElement` 와 같은 뜻이고, 개발 바이너리에서도 듣게 여기서도 정한다.
	#[cfg(target_os = "macos")]
	app.set_activation_policy(tauri::ActivationPolicy::Accessory);

	let handle = app.handle().clone();
	log::info!(
		"[app] {} {} starting ({})",
		handle.package_info().name,
		handle.package_info().version,
		handle.config().identifier
	);

	let db_path = handle.path().app_data_dir()?.join(constants::DB_FILE_NAME);
	let db = Db::open(&db_path)?;
	let (loaded_settings, rings) = {
		let conn = db.lock();
		(
			store::settings::load(&conn)?,
			store::rings::list(&conn).map_err(|e| e.to_string())?,
		)
	};
	settings::replace(loaded_settings);
	ring::controller::set_rings(rings);
	app.manage(db.clone());

	ui::tray::create(&handle)?;
	// 등록은 메모리의 링 목록을 읽는다. 그 뒤에 부른다.
	shortcuts::start(&handle);

	if !loaded_settings.first_run_done {
		// 처음 켰다. 메뉴 막대의 아이콘만으로는 무엇을 할지 알 수 없다 — 설정을 한 번 연다.
		ui::settings_window::open(&handle, None);
		store::settings::save_first_run_done(&db.lock())?;
		settings::update(|current| current.first_run_done = true);
	}

	#[cfg(feature = "dev-agent")]
	if std::env::var("RINGRING_DEV_AGENT").as_deref() == Ok("1") {
		dev_agent::spawn(handle.clone());
	} else {
		log::info!("[dev-agent] compiled in but RINGRING_DEV_AGENT is not 1 — not started");
	}

	Ok(())
}

pub fn run() {
	let builder = tauri::Builder::default();
	// tao 는 켜질 때 `activateIgnoringOtherApps:YES` 를 부른다. 창이 없는 메뉴 막대 앱에서는 그 요청이 남아 있다가
	// 이 process 가 처음 보통 창을 올리는 순간 앱을 맨 앞으로 가져온다 (실측 — `context-notes.md` 2026-10-06).
	// 켜는 것만으로 앞에 나올 이유가 없다. 설정 창은 열 때 스스로 앞으로 온다 (`settings_window::reveal`).
	#[cfg(target_os = "macos")]
	let builder = builder.activate_ignoring_other_apps(false);
	// 앱 메뉴는 macOS 의 메뉴 막대에 산다. 다른 OS 에서는 창마다 메뉴 줄이 붙으므로 달지 않는다.
	#[cfg(target_os = "macos")]
	let builder = builder
		.menu(ui::app_menu::build)
		.on_menu_event(|app, event| ui::app_menu::on_menu_event(app, event.id().as_ref()));
	let app = builder
		// 첫 plugin 이어야 한다. 둘째 실행은 여기서 끝나고, 먼저 떠 있던 쪽이 설정 창을 연다.
		.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
			ui::settings_window::open(app, None);
		}))
		.plugin(log_plugin())
		.plugin(tauri_plugin_global_shortcut::Builder::new().build())
		.plugin(tauri_plugin_dialog::init())
		.plugin(tauri_plugin_opener::init())
		.plugin(tauri_plugin_notification::init())
		.invoke_handler(tauri::generate_handler![
			commands::rings::rings_list,
			commands::rings::ring_create,
			commands::rings::ring_create_starter,
			commands::rings::ring_save,
			commands::rings::ring_set_shortcut,
			commands::rings::slot_save,
			commands::rings::slot_clear,
			commands::rings::slots_reorder,
			commands::rings::ring_link_candidates,
			commands::rings::ring_create_sub,
			commands::rings::ring_delete,
			commands::rings::ring_limits,
			commands::rings::open_target_choose,
			commands::rings::ring_show,
			commands::ring_window::ring_current,
			commands::ring_window::ring_pick,
			commands::ring_window::ring_key,
			commands::ring_window::ring_confirm,
			commands::ring_window::ring_hide,
			commands::ring_window::ring_painted,
			commands::app::app_state,
			commands::app::settings_set_language,
			commands::app::settings_set_theme,
			commands::app::settings_set_paused,
			commands::app::settings_set_launch_at_login,
			commands::app::login_items_open_settings,
			commands::app::accessibility_open_settings,
			commands::app::shortcut_capture_begin,
			commands::app::shortcut_capture_end,
			commands::app::settings_ready,
			commands::app::settings_leave_answer,
			commands::runs::runs_list,
			commands::runs::runs_seen,
			commands::runs::runs_clear,
			commands::transfer::rings_export,
			commands::transfer::rings_import_preview,
			commands::transfer::rings_import,
		])
		.setup(setup)
		.build(tauri::generate_context!())
		.expect("error while building the RingRing application");

	app.run(|_app, event| match event {
		// 마지막 창이 닫혔다. 이 앱은 창이 없어도 메뉴 막대에 남는다.
		tauri::RunEvent::ExitRequested {
			code: None, api, ..
		} => api.prevent_exit(),
		// 트레이의 종료(`app.exit`)와 ⌘Q 가 모두 여기로 온다. 돌고 있는 셸 명령의 process group 을 끝낸다.
		tauri::RunEvent::Exit => exec::process_group::terminate_all_on_exit(),
		_ => {}
	});
}
