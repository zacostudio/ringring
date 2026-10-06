// 실행 기록 command — 셸 명령 칸이 남긴 최근 실행을 설정 창에 준다
use tauri::AppHandle;

use crate::runs::{self, RunRecord};

#[tauri::command]
pub fn runs_list() -> Vec<RunRecord> {
	runs::list()
}

/// 실행 기록 페이지가 떠서 기록을 보이고 있다. 보지 않은 실패가 없어진다.
#[tauri::command]
pub fn runs_seen(app: AppHandle) {
	runs::mark_seen(&app);
}

#[tauri::command]
pub fn runs_clear(app: AppHandle) {
	runs::clear(&app);
}
