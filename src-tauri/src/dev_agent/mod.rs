// dev-agent — 개발 빌드에서만 뜨는 로컬 HTTP 제어 서버. OS 입력 없이 앱을 몰고 화면을 찍는다
//
// `--features dev-agent` 로 컴파일하고 `RINGRING_DEV_AGENT=1` 일 때만 뜬다. 릴리스 빌드에는 없다.
// 127.0.0.1 에만 묶는다. 페이지 안의 이벤트와 IPC 로 몬다. 실제 키 입력을 만드는 길은 `gate.rs` 하나뿐이고,
// 정해 둔 조합 하나를 그 조합이 이 앱의 전역 단축키로 걸려 있을 때만 보낸다. 마우스 클릭은 만들지 않는다.
mod activation;
mod eval;
mod gate;
mod snapshot;

use std::net::SocketAddr;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

use crate::constants::window_labels;
use crate::ring::controller;

/// 기본 포트. Tome 의 dev-agent 가 9787 을 쓴다.
const DEFAULT_PORT: u16 = 9797;

type Reply = Result<Json<Value>, (StatusCode, Json<Value>)>;

fn fail(status: StatusCode, code: &str, message: impl Into<String>) -> (StatusCode, Json<Value>) {
	(
		status,
		Json(json!({ "ok": false, "code": code, "error": message.into() })),
	)
}

fn port() -> u16 {
	std::env::var("RINGRING_DEV_AGENT_PORT")
		.ok()
		.and_then(|value| value.parse().ok())
		.unwrap_or(DEFAULT_PORT)
}

pub fn spawn(app: AppHandle) {
	// `setup` 에서 부른다 — main thread 다.
	activation::install();
	tauri::async_runtime::spawn(async move {
		let addr = SocketAddr::from(([127, 0, 0, 1], port()));
		let listener = match tokio::net::TcpListener::bind(addr).await {
			Ok(listener) => listener,
			Err(e) => {
				log::error!("[dev-agent] failed to bind {addr}: {e}");
				return;
			}
		};
		log::info!("[dev-agent] listening on {addr}");
		let router = Router::new()
			.route("/health", get(health))
			.route("/windows", get(windows))
			.route("/eval", post(eval::handler))
			.route("/invoke", post(eval::invoke))
			.route("/screenshot", post(screenshot))
			.route("/ring/show", post(ring_show))
			.route("/ring/hide", post(ring_hide))
			.route("/ring/state", get(ring_state))
			.route("/settings/open", post(settings_open))
			.route("/settings/close", post(settings_close))
			.route("/tray", get(tray_describe))
			.route("/tray/click", post(tray_click))
			.route("/gate/press", post(gate::press))
			.route("/probe/window", post(probe_window))
			.route("/shortcuts/stress", post(shortcuts_stress))
			.route("/shortcuts/registered", get(shortcuts_registered))
			.route("/app/state", get(app_state))
			.route("/app/deactivate", post(app_deactivate))
			.route("/shutdown", post(shutdown))
			.with_state(app);
		if let Err(e) = axum::serve(listener, router).await {
			log::error!("[dev-agent] server stopped: {e}");
		}
	});
}

async fn health(State(app): State<AppHandle>) -> Json<Value> {
	Json(json!({
		"ok": true,
		"version": app.package_info().version.to_string(),
		"identifier": app.config().identifier,
		"port": port(),
	}))
}

async fn windows(State(app): State<AppHandle>) -> Json<Value> {
	let list: Vec<Value> = app
		.webview_windows()
		.into_iter()
		.map(|(label, window)| {
			let scale = window.scale_factor().unwrap_or(1.0);
			let position = window
				.outer_position()
				.map(|p| p.to_logical::<f64>(scale))
				.ok();
			let size = window.inner_size().map(|s| s.to_logical::<f64>(scale)).ok();
			json!({
				"label": label,
				"visible": window.is_visible().unwrap_or(false),
				"focused": window.is_focused().unwrap_or(false),
				"x": position.map(|p| p.x),
				"y": position.map(|p| p.y),
				"width": size.map(|s| s.width),
				"height": size.map(|s| s.height),
				"scale": scale,
			})
		})
		.collect();
	Json(json!({ "ok": true, "windows": list }))
}

#[derive(Deserialize)]
struct ScreenshotReq {
	window: String,
	/// PNG 를 쓸 경로. 폴더가 없으면 만든다.
	path: String,
}

/// 창의 webview 를 PNG 로 찍는다. `WKWebView.takeSnapshot` 이라 화면 기록 권한이 필요 없다.
/// 창의 테두리와 그림자, 그 뒤의 화면은 찍히지 않는다 — 투명한 자리는 투명하게 남는다.
async fn screenshot(State(app): State<AppHandle>, Json(req): Json<ScreenshotReq>) -> Reply {
	let png = snapshot::png(&app, &req.window)
		.await
		.map_err(|e| fail(StatusCode::INTERNAL_SERVER_ERROR, "snapshot_failed", e))?;
	let path = std::path::PathBuf::from(&req.path);
	if let Some(dir) = path.parent() {
		tokio::fs::create_dir_all(dir).await.map_err(|e| {
			fail(
				StatusCode::INTERNAL_SERVER_ERROR,
				"write_failed",
				e.to_string(),
			)
		})?;
	}
	tokio::fs::write(&path, &png).await.map_err(|e| {
		fail(
			StatusCode::INTERNAL_SERVER_ERROR,
			"write_failed",
			e.to_string(),
		)
	})?;
	Ok(Json(
		json!({ "ok": true, "path": req.path, "bytes": png.len() }),
	))
}

/// main thread 에서 `f` 를 돌리고 그 답을 기다린다.
async fn on_main<T: Send + 'static>(
	app: &AppHandle,
	f: impl FnOnce(&AppHandle) -> T + Send + 'static,
) -> Result<T, (StatusCode, Json<Value>)> {
	let (tx, rx) = tokio::sync::oneshot::channel();
	let handle = app.clone();
	app.run_on_main_thread(move || {
		let _ = tx.send(f(&handle));
	})
	.map_err(|e| {
		fail(
			StatusCode::INTERNAL_SERVER_ERROR,
			"main_thread",
			e.to_string(),
		)
	})?;
	rx.await.map_err(|e| {
		fail(
			StatusCode::INTERNAL_SERVER_ERROR,
			"main_thread",
			e.to_string(),
		)
	})
}

#[derive(Deserialize)]
struct RingShowReq {
	ring_id: String,
	/// 링의 중심. 전역 화면 좌표, 논리 픽셀.
	x: f64,
	y: f64,
}

/// 커서 자리 대신 `(x, y)` 에 링을 띄운다. 키보드는 끝까지 받지 않는다 — 맨 앞 앱의 입력을 가져가지 않는다.
async fn ring_show(State(app): State<AppHandle>, Json(req): Json<RingShowReq>) -> Reply {
	let shown = on_main(&app, move |app| {
		controller::show_open(app, &req.ring_id, Some((req.x, req.y)), false)
	})
	.await?;
	match shown {
		Ok(()) => Ok(Json(json!({ "ok": true }))),
		Err(refusal) => Err(fail(
			StatusCode::UNPROCESSABLE_ENTITY,
			refusal.code(),
			refusal.to_string(),
		)),
	}
}

async fn ring_hide(State(app): State<AppHandle>) -> Reply {
	on_main(&app, controller::hide).await?;
	Ok(Json(json!({ "ok": true })))
}

/// 지금 떠 있는 링. 숨어 있으면 `current` 가 null 이다.
async fn ring_state(State(app): State<AppHandle>) -> Json<Value> {
	let visible = app
		.get_webview_window(window_labels::RING)
		.map(|window| window.is_visible().unwrap_or(false));
	Json(json!({
		"ok": true,
		"current": controller::current(),
		"windowExists": visible.is_some(),
		"windowVisible": visible.unwrap_or(false),
		"shortcutsActive": crate::shortcuts::active(),
	}))
}

#[derive(Deserialize, Default)]
struct SettingsOpenReq {
	#[serde(default)]
	page: Option<String>,
}

/// 설정 창을 연다. 새로 만든 창은 처음 화면에 올라온 직후에 이 앱을 맨 앞 앱으로 만든다 (실측 —
/// `NSApplication` 의 `activate` 를 거치지 않는다. 원인은 찾지 못했다). 보통 실행에서는 그게 맞는 동작이다.
/// 조용한 확인(`RINGRING_DEV_QUIET=1`)에서는 그 활성화를 곧바로 내려놓고 창만 다시 올린다.
async fn settings_open(State(app): State<AppHandle>, Json(req): Json<SettingsOpenReq>) -> Reply {
	on_main(&app, move |app| {
		crate::ui::settings_window::open(app, req.page.as_deref())
	})
	.await?;
	let mut yielded = false;
	if std::env::var("RINGRING_DEV_QUIET").as_deref() == Ok("1") {
		for _ in 0..40 {
			tokio::time::sleep(std::time::Duration::from_millis(50)).await;
			let took_focus = on_main(&app, |app| {
				if !crate::ui::panel::app_is_active() {
					return false;
				}
				crate::ui::panel::deactivate_app();
				crate::ui::settings_window::reveal(app);
				true
			})
			.await?;
			if took_focus {
				yielded = true;
				break;
			}
		}
	}
	Ok(Json(json!({ "ok": true, "yieldedFocus": yielded })))
}

/// 설정 창에 닫기를 요청한다. 닫기 단추나 ⌘W 와 같은 길이다 — 창의 `CloseRequested` 가 돈다.
/// 저장하지 않은 입력이 저장되지 않으면 창은 그대로 있다. `/windows` 로 닫혔는지 본다.
async fn settings_close(State(app): State<AppHandle>) -> Reply {
	if let Some(window) = app.get_webview_window(window_labels::SETTINGS) {
		window.close().map_err(|e| {
			fail(
				StatusCode::INTERNAL_SERVER_ERROR,
				"close_failed",
				e.to_string(),
			)
		})?;
	}
	Ok(Json(json!({ "ok": true })))
}

/// 트레이 메뉴의 항목들. 메뉴를 실제로 열지는 않는다 — 여는 것은 OS 클릭이라 여기서 만들지 않는다.
async fn tray_describe(State(app): State<AppHandle>) -> Reply {
	let described = on_main(&app, |app| crate::ui::tray::describe(app)).await?;
	described
		.map(|menu| Json(json!({ "ok": true, "menu": menu })))
		.map_err(|e| {
			fail(
				StatusCode::INTERNAL_SERVER_ERROR,
				"tray_failed",
				e.to_string(),
			)
		})
}

#[derive(Deserialize)]
struct TrayClickReq {
	id: String,
}

/// 트레이 메뉴의 항목을 고른 것과 같은 일을 한다. 메뉴의 handler 를 그대로 부른다.
async fn tray_click(State(app): State<AppHandle>, Json(req): Json<TrayClickReq>) -> Reply {
	on_main(&app, move |app| {
		crate::ui::tray::on_menu_event(app, &req.id)
	})
	.await?;
	Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct ProbeReq {
	/// `raw` — AppKit 의 창을 직접 만들어 앞으로 올린다 (tao · wry · webview 없음).
	/// `webview-hidden` — webview 창을 만들기만 하고 보이지 않는다.
	/// `webview-front` — webview 창을 만들고 앞으로 올린다 (조용한 설정 창과 같은 길).
	mode: String,
}

/// 새 창이 이 앱을 맨 앞 앱으로 만드는지 재 본다. 무엇이 활성화를 부르는지 가려내는 데 쓴다.
/// 창을 만들고 1.5초 동안 `NSApp.isActive` 를 본 뒤 창을 없앤다. 활성화됐으면 곧바로 내려놓는다.
async fn probe_window(State(app): State<AppHandle>, Json(req): Json<ProbeReq>) -> Reply {
	const LABEL: &str = "probe";
	let mode = req.mode.clone();
	let created = on_main(&app, move |app| -> Result<usize, String> {
		match mode.as_str() {
			"raw" => Ok(activation::raw_window()),
			"webview-hidden" | "webview-front" => {
				let builder = tauri::WebviewWindowBuilder::new(
					app,
					LABEL,
					tauri::WebviewUrl::App("index.html?probe=1".into()),
				)
				.title("probe")
				.inner_size(360.0, 240.0)
				.visible(false);
				let window = crate::ui::panel::build(builder).map_err(|e| e.to_string())?;
				if mode == "webview-front" {
					crate::ui::panel::order_front(&window)?;
				}
				Ok(0)
			}
			other => Err(format!("unknown mode: {other}")),
		}
	})
	.await?
	.map_err(|e| fail(StatusCode::BAD_REQUEST, "bad_request", e))?;

	let mut active_after_ms = None;
	for tick in 0..30u64 {
		tokio::time::sleep(std::time::Duration::from_millis(50)).await;
		if on_main(&app, |_| crate::ui::panel::app_is_active()).await? {
			active_after_ms = Some((tick + 1) * 50);
			on_main(&app, |_| crate::ui::panel::deactivate_app()).await?;
			break;
		}
	}
	on_main(&app, move |app| {
		if created != 0 {
			activation::close_raw_window(created);
		}
		if let Some(window) = app.get_webview_window(LABEL) {
			let _ = window.destroy();
		}
	})
	.await?;
	Ok(Json(
		json!({ "ok": true, "mode": req.mode, "activeAfterMs": active_after_ms }),
	))
}

/// 반복 확인이 실제로 OS 에 거는 단 하나의 조합. 이 Mac 의 키보드에는 F17 이 없다.
const STRESS_COMBO: &str = "Ctrl+Alt+Shift+F17";

#[derive(Deserialize)]
struct StressReq {
	on: bool,
}

/// 조합을 입력받아 거는 길을 진짜 plugin 호출로 되풀이해 볼 수 있게, 개발 빌드가 `STRESS_COMBO` 하나만
/// 실제로 걸게 한다. `on: false` 면 다시 아무것도 걸지 않고, 걸려 있던 것을 뗀다. 키 입력은 만들지 않는다.
async fn shortcuts_stress(State(app): State<AppHandle>, Json(req): Json<StressReq>) -> Reply {
	let combo = STRESS_COMBO
		.parse()
		.map_err(|_| fail(StatusCode::INTERNAL_SERVER_ERROR, "failed", "bad combo"))?;
	crate::shortcuts::dev_stress_gate(&app, req.on.then_some(combo)).await;
	let registered = crate::shortcuts::dev_registered(&app).await;
	Ok(Json(
		json!({ "ok": true, "combo": STRESS_COMBO, "on": req.on, "registered": registered }),
	))
}

/// plugin 이 지금 쥐고 있는 링의 조합들.
async fn shortcuts_registered(State(app): State<AppHandle>) -> Json<Value> {
	Json(json!({ "ok": true, "registered": crate::shortcuts::dev_registered(&app).await }))
}

/// 이 앱이 맨 앞 앱인가. 확인하는 동안 사용자의 포커스를 가져가지 않았는지 본다.
async fn app_state(State(app): State<AppHandle>) -> Reply {
	let active = on_main(&app, |_| crate::ui::panel::app_is_active()).await?;
	// 앱 메뉴가 있어야 글 입력 칸에서 ⌘C · ⌘V · ⌘W · ⌘Q 가 듣는다. 메뉴 막대에 보이지 않아도 그렇다.
	let menu: Vec<String> = app
		.menu()
		.and_then(|menu| menu.items().ok())
		.unwrap_or_default()
		.iter()
		.filter_map(|item| item.as_submenu().and_then(|submenu| submenu.text().ok()))
		.collect();
	Ok(Json(json!({
		"ok": true,
		"active": active,
		"appMenu": menu,
		"release": !cfg!(debug_assertions),
		"devConfig": cfg!(dev),
	})))
}

/// 활성 상태를 내려놓는다. 키보드가 그 전에 앞에 있던 앱으로 돌아간다.
async fn app_deactivate(State(app): State<AppHandle>) -> Reply {
	on_main(&app, |_| crate::ui::panel::deactivate_app()).await?;
	Ok(Json(json!({ "ok": true })))
}

/// 앱을 끝낸다. 트레이의 종료와 같은 길이다.
async fn shutdown(State(app): State<AppHandle>) -> Json<Value> {
	log::info!("[dev-agent] shutdown requested");
	let handle = app.clone();
	tauri::async_runtime::spawn(async move {
		// 답을 먼저 보낸다.
		tokio::time::sleep(std::time::Duration::from_millis(100)).await;
		handle.exit(0);
	});
	Json(json!({ "ok": true }))
}
