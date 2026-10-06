// IPC command — 설정 창의 링 편집·앱 설정·가져오기와, 링 창이 보내는 클릭·키
//
// 판정은 `ring/`, `store/`, `shortcuts.rs` 에 있다. 여기는 DB 연결을 건네고, 바뀐 것을 메모리·트레이·창에 알린다.
pub mod app;
pub mod ring_window;
pub mod rings;
pub mod runs;
pub mod transfer;

use tauri::{AppHandle, Emitter, Manager};

use crate::constants::events;
use crate::error::CommandError;
use crate::ring::controller;
use crate::store::{Db, rings as ring_store};

pub(crate) fn db(app: &AppHandle) -> Db {
	app.state::<Db>().inner().clone()
}

pub(crate) fn emit_all(app: &AppHandle, event: &str) {
	if let Err(e) = app.emit(event, ()) {
		log::warn!("[command] failed to emit {event}: {e}");
	}
}

/// 링을 고친 뒤에 부른다. 메모리의 링 목록을 다시 읽고 [`announce_rings`] 를 부른다.
pub(crate) async fn refresh_rings(app: &AppHandle) -> Result<(), CommandError> {
	let rings = db(app).with(|conn| ring_store::list(conn)).await??;
	controller::set_rings(rings);
	announce_rings(app)
}

/// 메모리의 링 목록이 바뀌었다. 열려 있는 창에 알리고, 링 창과 트레이를 맞춘다.
pub(crate) fn announce_rings(app: &AppHandle) -> Result<(), CommandError> {
	emit_all(app, events::RINGS_CHANGED);
	let handle = app.clone();
	app.run_on_main_thread(move || {
		controller::sync_window(&handle, crate::shortcuts::active());
		crate::ui::tray::refresh(&handle);
	})
	.map_err(|e| CommandError::failed(e.to_string()))
}
