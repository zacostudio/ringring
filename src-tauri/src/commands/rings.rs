// 링 편집 command — 설정 창이 부른다. 고칠 때마다 저장하고, 거절은 이유 코드로 돌려준다
use serde::Serialize;
use tauri::AppHandle;

use super::{announce_rings, db, refresh_rings};
use crate::error::CommandError;
use crate::ring::controller;
use crate::ring::model::{LIMITS, Limits, OpenTarget, Ring, ShortcutKind, Slot};
use crate::ring::starter;
use crate::store::rings::{self as ring_store, LinkChoices, SlotRef};
use crate::ui::path_panel::{self, PanelOptions};
use crate::ui::texts;

#[tauri::command]
pub async fn rings_list(app: AppHandle) -> Result<Vec<Ring>, CommandError> {
	Ok(db(&app).with(|conn| ring_store::list(conn)).await??)
}

/// 빈 링을 만든다. 6칸이고 단축키는 없다. 이름은 사용자 언어의 "새 링" 이다.
#[tauri::command]
pub async fn ring_create(app: AppHandle) -> Result<Ring, CommandError> {
	let id = uuid::Uuid::new_v4().to_string();
	let name = texts::new_ring_name(crate::settings::locale());
	let ring = db(&app)
		.with(move |conn| ring_store::create(conn, &id, name, &[]))
		.await??;
	refresh_rings(&app).await?;
	Ok(ring)
}

/// 기본 링을 만든다. 이 기기에 있는 앱과 폴더만 칸으로 들어간다. 단축키는 없다 — 사용자가 정한다.
#[tauri::command]
pub async fn ring_create_starter(app: AppHandle) -> Result<Ring, CommandError> {
	let id = uuid::Uuid::new_v4().to_string();
	let locale = crate::settings::locale();
	let ring = db(&app)
		.with(move |conn| {
			let slots = starter::slots(locale, |path| path.exists());
			ring_store::create(conn, &id, texts::starter_ring_name(locale), &slots)
		})
		.await??;
	refresh_rings(&app).await?;
	Ok(ring)
}

/// 링의 이름과 칸 수를 바꾼다. 칸 수를 줄이면 넘치는 칸은 지워진다 — 설정 화면이 먼저 묻는다.
#[tauri::command]
pub async fn ring_save(
	app: AppHandle,
	ring_id: String,
	name: String,
	slot_count: usize,
) -> Result<Ring, CommandError> {
	let ring = db(&app)
		.with(move |conn| ring_store::update(conn, &ring_id, &name, slot_count))
		.await??;
	refresh_rings(&app).await?;
	Ok(ring)
}

/// 링의 `kind` 전역 단축키(일반·빠른)를 바꾼다. 빈 값이면 뗀다. OS 에 등록된 뒤에만 저장한다.
///
/// 조합을 입력받던 중이면 여기서 끝낸다. 끝내고, 걸어 보고, 저장하고, 답하는 순서는 Rust 가 지킨다
/// (`shortcuts::change_shortcut`).
#[tauri::command]
pub async fn ring_set_shortcut(
	app: AppHandle,
	ring_id: String,
	kind: ShortcutKind,
	shortcut: Option<String>,
) -> Result<(), CommandError> {
	crate::shortcuts::set_ring_shortcut(&app, &db(&app), &ring_id, kind, shortcut).await?;
	announce_rings(&app)
}

#[tauri::command]
pub async fn slot_save(app: AppHandle, ring_id: String, slot: Slot) -> Result<Ring, CommandError> {
	let ring = db(&app)
		.with(move |conn| ring_store::save_slot(conn, &ring_id, &slot))
		.await??;
	refresh_rings(&app).await?;
	Ok(ring)
}

#[tauri::command]
pub async fn slot_clear(
	app: AppHandle,
	ring_id: String,
	position: usize,
) -> Result<Ring, CommandError> {
	let ring = db(&app)
		.with(move |conn| ring_store::clear_slot(conn, &ring_id, position))
		.await??;
	refresh_rings(&app).await?;
	Ok(ring)
}

/// 칸의 순서를 바꾼다. `order[새 자리] = 예전 자리` 다.
#[tauri::command]
pub async fn slots_reorder(
	app: AppHandle,
	ring_id: String,
	order: Vec<usize>,
) -> Result<Ring, CommandError> {
	let ring = db(&app)
		.with(move |conn| ring_store::reorder_slots(conn, &ring_id, &order))
		.await??;
	refresh_rings(&app).await?;
	Ok(ring)
}

/// 이 칸이 하위 링으로 열 수 있는 링 목록. 순환이 되거나 깊이를 넘는 링은 이유 코드와 함께 온다.
#[tauri::command]
pub async fn ring_link_candidates(
	app: AppHandle,
	ring_id: String,
	position: usize,
) -> Result<LinkChoices, CommandError> {
	Ok(db(&app)
		.with(move |conn| ring_store::link_candidates(conn, &ring_id, position))
		.await??)
}

/// 자리에서 만든 하위 링과, 그 링을 열게 된 칸의 링.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubRingCreated {
	pub ring: Ring,
	pub sub_ring: Ring,
}

/// 빈 하위 링을 만들고 `position` 칸에 잇는다. 한 번에 한다 — 링만 남거나 칸만 남지 않는다.
///
/// `label` 과 `icon` 은 화면의 초안이다. 비어 있으면 Rust 가 채운다.
#[tauri::command]
pub async fn ring_create_sub(
	app: AppHandle,
	ring_id: String,
	position: usize,
	label: String,
	icon: String,
) -> Result<SubRingCreated, CommandError> {
	let new_id = uuid::Uuid::new_v4().to_string();
	let default_name = texts::new_sub_ring_name(crate::settings::locale());
	let (ring, sub_ring) = db(&app)
		.with(move |conn| {
			ring_store::create_sub_ring(
				conn,
				&new_id,
				&ring_id,
				position,
				&label,
				&icon,
				default_name,
			)
		})
		.await??;
	refresh_rings(&app).await?;
	Ok(SubRingCreated { ring, sub_ring })
}

/// 링을 지운다. 그 링을 여는 칸이 있으면 지우지 않고 그 칸들을 돌려준다. 빈 목록이면 지운 것이다.
#[tauri::command]
pub async fn ring_delete(app: AppHandle, ring_id: String) -> Result<Vec<SlotRef>, CommandError> {
	let refs = db(&app)
		.with(move |conn| ring_store::delete(conn, &ring_id))
		.await??;
	if refs.is_empty() {
		refresh_rings(&app).await?;
		// 지운 링에 단축키가 걸려 있었으면 OS 에서도 뗀다.
		crate::shortcuts::resync(&app).await;
	}
	Ok(refs)
}

/// "열기" 칸의 대상을 고르는 창을 연다. 파일 칸은 파일과 폴더를, 앱 칸은 앱을 고른다.
/// 고른 경로를 돌려준다. 취소하면 `None`. 저장은 하지 않는다 — 화면이 `slot_save` 로 저장한다.
#[tauri::command]
pub async fn open_target_choose(
	window: tauri::WebviewWindow,
	target: OpenTarget,
) -> Result<Option<String>, CommandError> {
	let options = PanelOptions::for_target(target)
		.ok_or_else(|| CommandError::failed("A web address is typed, not chosen"))?;
	path_panel::choose(&window, options)
		.await
		.map_err(CommandError::failed)
}

/// 설정 화면이 쓰는 한도와 기본값. 화면은 이 값을 받아 쓰고 스스로 정하지 않는다.
#[tauri::command]
pub fn ring_limits() -> Limits {
	LIMITS
}

/// 링 id 로 링을 띄운다. 짧게 누른 것과 같은 상태다 — 클릭이나 키로 고른다.
///
/// 창을 올리는 일은 main thread 에서 한다. sync command 로 곧바로 하지 않고 main thread 의 queue 에 넣는다 —
/// Windows 에서 sync command 는 WebView2 의 callback 안에서 돌고, 거기서 링 창(새 webview)을 만들면
/// 서로 기다리다 멈춘다.
#[tauri::command]
pub async fn ring_show(app: AppHandle, ring_id: String) -> Result<(), CommandError> {
	let (tx, rx) = tokio::sync::oneshot::channel();
	let handle = app.clone();
	app.run_on_main_thread(move || {
		let _ = tx.send(controller::show_open(&handle, &ring_id, None, true));
	})
	.map_err(|e| CommandError::failed(e.to_string()))?;
	Ok(rx
		.await
		.map_err(|e| CommandError::failed(e.to_string()))??)
}
