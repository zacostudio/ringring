// dev-agent: POST /eval 과 /invoke — webview 에서 JS 를 돌리고 Tauri 이벤트로 결과를 받는다
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Listener, Manager};

use super::{Reply, fail};

#[derive(Deserialize)]
pub struct EvalReq {
	pub window: String,
	/// async 함수의 몸통이다. `return` 한 값이 결과로 온다.
	pub js: String,
	#[serde(default = "default_timeout")]
	pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
	5000
}

pub async fn handler(State(app): State<AppHandle>, Json(req): Json<EvalReq>) -> Reply {
	let window = app.get_webview_window(&req.window).ok_or_else(|| {
		fail(
			StatusCode::NOT_FOUND,
			"window_not_found",
			format!("window not found: {}", req.window),
		)
	})?;

	let event = format!("dev-agent-result:{}", uuid::Uuid::new_v4());
	let (tx, rx) = tokio::sync::oneshot::channel::<Value>();
	let tx = std::sync::Mutex::new(Some(tx));
	let listener = app.once(event.clone(), move |received| {
		// 페이지는 결과를 JSON 글로 보낸다. payload 는 그 글을 한 번 더 감싼 JSON 문자열이다.
		let raw = received.payload();
		let value = serde_json::from_str::<String>(raw)
			.ok()
			.and_then(|inner| serde_json::from_str::<Value>(&inner).ok())
			.or_else(|| serde_json::from_str::<Value>(raw).ok())
			.unwrap_or_else(|| json!({ "ok": false, "error": "invalid payload" }));
		if let Some(tx) = tx.lock().unwrap_or_else(|e| e.into_inner()).take() {
			let _ = tx.send(value);
		}
	});

	// `withGlobalTauri` 가 꺼져 있어도 `__TAURI_INTERNALS__.invoke` 는 늘 있다.
	let wrapped = format!(
		"(async () => {{ const emit = (p) => window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {{ event: '{event}', payload: JSON.stringify(p) }}); try {{ const r = await (async () => {{ {body} }})(); await emit({{ ok: true, result: r === undefined ? null : r }}); }} catch (e) {{ await emit({{ ok: false, error: (e && e.message) ? e.message : (typeof e === 'string' ? e : JSON.stringify(e)) }}); }} }})();",
		body = req.js,
	);
	if let Err(e) = window.eval(&wrapped) {
		app.unlisten(listener);
		return Err(fail(
			StatusCode::INTERNAL_SERVER_ERROR,
			"eval_failed",
			e.to_string(),
		));
	}

	match tokio::time::timeout(Duration::from_millis(req.timeout_ms), rx).await {
		Ok(Ok(payload)) => {
			if payload.get("ok").and_then(Value::as_bool) == Some(true) {
				Ok(Json(json!({ "ok": true, "result": payload.get("result") })))
			} else {
				Err(fail(
					StatusCode::UNPROCESSABLE_ENTITY,
					"eval_threw",
					payload
						.get("error")
						.and_then(Value::as_str)
						.unwrap_or("unknown")
						.to_string(),
				))
			}
		}
		Ok(Err(_)) => {
			app.unlisten(listener);
			Err(fail(
				StatusCode::INTERNAL_SERVER_ERROR,
				"eval_failed",
				"the result channel closed",
			))
		}
		Err(_) => {
			app.unlisten(listener);
			Err(fail(
				StatusCode::GATEWAY_TIMEOUT,
				"eval_timeout",
				"eval timed out",
			))
		}
	}
}

#[derive(Deserialize)]
pub struct InvokeReq {
	#[serde(default = "default_window")]
	pub window: String,
	pub cmd: String,
	#[serde(default)]
	pub args: Value,
	#[serde(default = "default_timeout")]
	pub timeout_ms: u64,
}

fn default_window() -> String {
	crate::constants::window_labels::SETTINGS.to_string()
}

/// 그 창의 webview 를 거쳐 Tauri command 를 부른다. 창이 떠 있어야 한다.
pub async fn invoke(state: State<AppHandle>, Json(req): Json<InvokeReq>) -> Reply {
	if !req.args.is_object() && !req.args.is_null() {
		return Err(fail(
			StatusCode::BAD_REQUEST,
			"bad_request",
			"args must be a JSON object (or omitted)",
		));
	}
	let args = if req.args.is_null() {
		"{}".to_string()
	} else {
		req.args.to_string()
	};
	let cmd = serde_json::to_string(&req.cmd).unwrap_or_else(|_| "\"\"".to_string());
	let js = format!(
		"try {{ return await window.__TAURI_INTERNALS__.invoke({cmd}, {args}); }} catch (e) {{ throw (typeof e === 'string' ? e : JSON.stringify(e)); }}"
	);
	handler(
		state,
		Json(EvalReq {
			window: req.window,
			js,
			timeout_ms: req.timeout_ms,
		}),
	)
	.await
}
