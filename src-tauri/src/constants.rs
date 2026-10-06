// 창 라벨과 Tauri 이벤트 이름 — 프런트의 `shared/tauri/events.ts` 와 같은 글자다

pub mod window_labels {
	/// 커서 자리에 뜨는 링 창.
	pub const RING: &str = "ring";
	pub const SETTINGS: &str = "settings";
}

pub mod events {
	/// 링 창에만 간다. payload 는 `ShowPayload`.
	pub const RING_SHOW: &str = "ring-show";
	/// 링 창에만 간다. payload 는 켜진 칸 번호 또는 null.
	pub const RING_HOVER: &str = "ring-hover";
	/// 링 창에만 간다. payload 없음.
	pub const RING_HIDE: &str = "ring-hide";
	/// 모든 창에 간다. 저장된 링이 바뀌었다.
	pub const RINGS_CHANGED: &str = "rings-changed";
	/// 모든 창에 간다. 언어·테마·일시 정지·로그인 항목이 바뀌었다.
	pub const APP_STATE_CHANGED: &str = "app-state-changed";
	/// 설정 창에 간다. 셸 명령의 실행 기록이 바뀌었다.
	pub const RUNS_CHANGED: &str = "runs-changed";
	/// 설정 창에 간다. payload 는 보여 줄 페이지 이름.
	pub const SETTINGS_NAVIGATE: &str = "settings-navigate";
	/// 설정 창에 간다. 창을 닫거나 앱을 끝내려 한다 — 저장하지 않은 입력을 먼저 저장하라는 뜻이다.
	/// payload 는 요청 번호. 페이지는 `settings_leave_answer` 로 답한다.
	pub const SETTINGS_LEAVE_REQUEST: &str = "settings-leave-request";
}

/// 로그 파일 이름 (확장자 없이). `~/Library/Logs/<identifier>/RingRing.log` 가 된다.
pub const LOG_FILE_NAME: &str = "RingRing";
/// DB 파일 이름. 앱 데이터 폴더 아래에 둔다.
pub const DB_FILE_NAME: &str = "ringring.db";
