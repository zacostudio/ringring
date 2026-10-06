// 링 창이 부르는 command — 라벨이 `ring` 인 창에서 온 것만 받는다
//
// 다른 webview 가 칸을 고르거나 확인에 답할 수 없다.
use serde::Deserialize;
use tauri::{Manager, WebviewWindow};

use crate::constants::window_labels;
use crate::ring::controller::{self, RingKey, ShowPayload};

/// 부른 창이 링 창인지 본다.
fn require_ring_window(window: &WebviewWindow) -> Result<(), String> {
	if window.label() == window_labels::RING {
		Ok(())
	} else {
		Err(format!("'{}' is not the ring window", window.label()))
	}
}

/// 지금 보여 줄 링. 링 페이지가 뜰 때 한 번 묻는다 — listener 를 걸기 전에 `ring-show` 가 나갔을 수 있다.
#[tauri::command]
pub fn ring_current(window: WebviewWindow) -> Result<Option<ShowPayload>, String> {
	require_ring_window(&window)?;
	Ok(controller::current())
}

/// 링 창을 클릭했다. 인자가 없다 — 어느 칸인지는 Rust 가 커서 자리로 정한다.
#[tauri::command]
pub fn ring_pick(window: WebviewWindow) -> Result<(), String> {
	require_ring_window(&window)?;
	controller::pick(window.app_handle());
	Ok(())
}

/// 링 창이 키를 받았다.
#[tauri::command]
pub fn ring_key(window: WebviewWindow, key: RingKey) -> Result<(), String> {
	require_ring_window(&window)?;
	controller::key(window.app_handle(), key);
	Ok(())
}

/// 확인 물음의 답.
#[tauri::command]
pub fn ring_confirm(window: WebviewWindow, accepted: bool) -> Result<(), String> {
	require_ring_window(&window)?;
	controller::confirm(window.app_handle(), accepted);
	Ok(())
}

/// 링 창이 닫으라고 한다 (확인의 취소 단추).
#[tauri::command]
pub fn ring_hide(window: WebviewWindow) -> Result<(), String> {
	require_ring_window(&window)?;
	controller::hide(window.app_handle());
	Ok(())
}

/// 링 페이지가 `seq` 번 보이기를 그렸다. 띄운 뒤 그릴 때까지를 로그에 남긴다.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaintedReport {
	seq: u64,
}

#[tauri::command]
pub fn ring_painted(window: WebviewWindow, report: PaintedReport) -> Result<(), String> {
	require_ring_window(&window)?;
	controller::painted(report.seq);
	Ok(())
}
