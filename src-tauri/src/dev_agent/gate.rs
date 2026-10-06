// dev-agent: 실제 키 입력으로 하는 확인 — 정해 둔 조합 하나(⌃⌥⇧F18)만 보낸다
//
// 다른 키는 보낼 수 없다. 조합을 고르는 인자가 없다. 그 조합이 이 앱의 전역 단축키로 걸려 있을 때만 보낸다 —
// 걸려 있으면 OS 가 그 키를 삼키고 다른 앱에 닿지 않는다. 클릭은 만들지 않는다.
use std::ffi::c_void;
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::json;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::Shortcut;

use super::{Reply, fail};
use crate::ring::{controller, platform};

/// 이 파일이 보낼 수 있는 단 하나의 조합.
const GATE_COMBO: &str = "Ctrl+Alt+Shift+F18";
const KEY_F18: u16 = 0x4f;
const KEY_SHIFT: u16 = 0x38;
const KEY_OPTION: u16 = 0x3a;
const KEY_CONTROL: u16 = 0x3b;
/// `kCGEventFlagMaskSecondaryFn`. 만든 기능 키가 전역 단축키에 닿으려면 이 flag 가 있어야 한다 (Tome 에서 실측).
const FLAG_FN: u64 = 0x0080_0000;
/// 한 번 누르는 최대 시간.
const MAX_HOLD_MS: u64 = 3000;
/// 커서를 옮기는 최대 거리.
const MAX_WARP: f64 = 120.0;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
	fn CGEventCreateKeyboardEvent(source: *const c_void, key: u16, down: bool) -> *const c_void;
	fn CGEventSetFlags(event: *const c_void, flags: u64);
	fn CGEventPost(tap: u32, event: *const c_void);
	fn CFRelease(cf: *const c_void);
}

fn post(key: u16, down: bool, flags: u64) {
	unsafe {
		let event = CGEventCreateKeyboardEvent(std::ptr::null(), key, down);
		if event.is_null() {
			return;
		}
		CGEventSetFlags(event, flags);
		CGEventPost(0, event);
		CFRelease(event);
	}
}

/// 그 조합이 지금 이 앱의 전역 단축키로 걸려 있는가.
fn combo_registered() -> bool {
	let Ok(wanted) = GATE_COMBO.parse::<Shortcut>() else {
		return false;
	};
	crate::shortcuts::active()
		&& controller::rings().iter().any(|ring| {
			ring.shortcut
				.as_deref()
				.and_then(|combo| combo.parse::<Shortcut>().ok())
				.is_some_and(|have| have.mods == wanted.mods && have.key == wanted.key)
		})
}

#[derive(Deserialize)]
pub struct PressReq {
	hold_ms: u64,
	/// Shift 를 본 키보다 먼저 뗀다. 키 상태 polling 이 놓음을 잡는 길이다.
	#[serde(default)]
	modifier_first: bool,
	/// 누르고 있는 동안 커서를 이만큼 옮겼다가, 놓은 뒤 제자리로 돌린다.
	#[serde(default)]
	warp: Option<(f64, f64)>,
}

pub async fn press(State(_app): State<AppHandle>, Json(req): Json<PressReq>) -> Reply {
	if !combo_registered() {
		return Err(fail(
			StatusCode::CONFLICT,
			"not_registered",
			format!("{GATE_COMBO} is not registered by this app — nothing was sent"),
		));
	}
	if req.hold_ms == 0 || req.hold_ms > MAX_HOLD_MS {
		return Err(fail(
			StatusCode::BAD_REQUEST,
			"bad_request",
			"hold_ms out of range",
		));
	}
	if req.warp.is_some_and(|(dx, dy)| dx.hypot(dy) > MAX_WARP) {
		return Err(fail(StatusCode::BAD_REQUEST, "bad_request", "warp too far"));
	}
	let done = tauri::async_runtime::spawn_blocking(move || {
		let all = platform::FLAG_CONTROL | platform::FLAG_ALTERNATE | platform::FLAG_SHIFT;
		let nap = |ms: u64| std::thread::sleep(Duration::from_millis(ms));
		post(KEY_CONTROL, true, platform::FLAG_CONTROL);
		post(
			KEY_OPTION,
			true,
			platform::FLAG_CONTROL | platform::FLAG_ALTERNATE,
		);
		post(KEY_SHIFT, true, all);
		nap(15);
		post(KEY_F18, true, all | FLAG_FN);
		let origin = platform::cursor();
		let mut warped = None;
		if let Some((dx, dy)) = req.warp {
			nap(req.hold_ms / 3);
			warped = Some(platform::warp_cursor(origin.0 + dx, origin.1 + dy));
			nap(req.hold_ms - req.hold_ms / 3);
		} else {
			nap(req.hold_ms);
		}
		if req.modifier_first {
			post(
				KEY_SHIFT,
				false,
				platform::FLAG_CONTROL | platform::FLAG_ALTERNATE,
			);
			nap(120);
			post(
				KEY_F18,
				false,
				platform::FLAG_CONTROL | platform::FLAG_ALTERNATE | FLAG_FN,
			);
		} else {
			post(KEY_F18, false, all | FLAG_FN);
			nap(15);
			post(
				KEY_SHIFT,
				false,
				platform::FLAG_CONTROL | platform::FLAG_ALTERNATE,
			);
		}
		post(KEY_OPTION, false, platform::FLAG_CONTROL);
		post(KEY_CONTROL, false, 0);
		let restored = warped.map(|_| {
			nap(150);
			platform::warp_cursor(origin.0, origin.1)
		});
		nap(40);
		json!({
			"ok": true, "origin": [origin.0, origin.1], "warped": warped, "restored": restored,
			"cursorAfter": [platform::cursor().0, platform::cursor().1],
		})
	})
	.await
	.map_err(|e| fail(StatusCode::INTERNAL_SERVER_ERROR, "failed", e.to_string()))?;
	Ok(Json(done))
}
