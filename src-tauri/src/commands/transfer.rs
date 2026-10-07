// 내보내기·가져오기 command — 파일은 Rust 가 읽고 쓴다. 화면은 경로를 고르고 요약을 보일 뿐이다
use std::path::{Path, PathBuf};

use tauri::AppHandle;

use super::{db, refresh_rings};
use crate::error::CommandError;
use crate::ring::model::{Refusal, RingError};
use crate::ring::transfer::{self, ImportPlan, ImportSummary, MAX_FILE_BYTES};
use crate::store::rings as ring_store;

/// 링 전체를 `path` 에 쓴다. 돌려주는 것은 쓴 링의 수다.
#[tauri::command]
pub async fn rings_export(app: AppHandle, path: String) -> Result<usize, CommandError> {
	let rings = db(&app).with(|conn| ring_store::list(conn)).await??;
	let text = transfer::export_text(
		&rings,
		&chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
		&app.package_info().version.to_string(),
	)?;
	let target = PathBuf::from(path);
	tauri::async_runtime::spawn_blocking(move || std::fs::write(&target, text))
		.await
		.map_err(|e| CommandError::failed(e.to_string()))?
		.map_err(|e| CommandError::failed(format!("Failed to write the file: {e}")))?;
	Ok(rings.len())
}

/// 파일을 읽는다. 너무 크거나 글이 아니면 이 앱의 파일이 아니다.
fn read_capped(path: &Path) -> Result<String, RingError> {
	let size = std::fs::metadata(path)
		.map_err(|e| RingError::Other(format!("Failed to read the file: {e}")))?
		.len();
	if size > MAX_FILE_BYTES {
		return Err(Refusal::ImportUnreadable.into());
	}
	std::fs::read_to_string(path).map_err(|_| Refusal::ImportUnreadable.into())
}

/// 파일을 읽고 무엇을 들일지 정한다. 아무것도 저장하지 않는다.
async fn plan_from(app: &AppHandle, path: String) -> Result<ImportPlan, CommandError> {
	Ok(db(app)
		.with(move |conn| -> Result<ImportPlan, RingError> {
			let text = read_capped(Path::new(&path))?;
			let existing = ring_store::list(conn)?;
			Ok(transfer::plan(&text, &existing, || {
				uuid::Uuid::new_v4().to_string()
			})?)
		})
		.await??)
}

/// 가져오기 전에 보일 요약.
#[tauri::command]
pub async fn rings_import_preview(
	app: AppHandle,
	path: String,
) -> Result<ImportSummary, CommandError> {
	Ok(plan_from(&app, path).await?.summary)
}

/// 파일의 링을 더한다. 있는 링은 건드리지 않는다. 가져온 링의 단축키를 OS 가 거절하면 떼고 요약에 센다.
///
/// 원래 있던 링의 등록이 실패해도 그 링의 저장값은 그대로 둔다 — 가져오기가 있던 것을 바꾸면 안 된다.
/// 등록을 고치는 다른 일과 섞이지 않게 처음부터 끝까지 [`crate::shortcuts::Op`] 를 쥐고 한다.
#[tauri::command]
pub async fn rings_import(app: AppHandle, path: String) -> Result<ImportSummary, CommandError> {
	let op = crate::shortcuts::begin().await;
	let ImportPlan { rings, mut summary } = plan_from(&app, path).await?;
	let imported: Vec<String> = rings.iter().map(|ring| ring.id.clone()).collect();
	db(&app)
		.with(move |conn| ring_store::import(conn, &rings))
		.await??;
	refresh_rings(&app).await?;

	let refused = transfer::imported_among(&op.resync(&app).await, &imported);
	if !refused.is_empty() {
		summary.dropped_shortcuts += refused.len();
		db(&app)
			.with(move |conn| -> Result<(), RingError> {
				for (id, kind) in &refused {
					ring_store::set_shortcut(conn, id, *kind, None)?;
				}
				Ok(())
			})
			.await??;
		refresh_rings(&app).await?;
	}
	Ok(summary)
}
