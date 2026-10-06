// Tauri 이벤트 이름 — Rust 의 `constants.rs` `events` 와 같은 글자다
export const EVENTS = {
	/** 링 창에만 온다. payload 는 `RingShow`. */
	ringShow: "ring-show",
	/** 링 창에만 온다. payload 는 켜진 칸 번호 또는 null. */
	ringHover: "ring-hover",
	/** 링 창에만 온다. payload 없음. */
	ringHide: "ring-hide",
	/** 모든 창에 온다. 저장된 링이 바뀌었다. */
	ringsChanged: "rings-changed",
	/** 모든 창에 온다. 언어·테마·일시 정지·로그인 항목이 바뀌었다. */
	appStateChanged: "app-state-changed",
	/** 설정 창에 온다. 셸 명령의 실행 기록이 바뀌었다. */
	runsChanged: "runs-changed",
	/** 설정 창에 온다. payload 는 보여 줄 페이지 이름. */
	settingsNavigate: "settings-navigate",
	/** 설정 창에 온다. 창을 닫거나 앱을 끝내려 한다. payload 는 요청 번호. `settings_leave_answer` 로 답한다. */
	settingsLeaveRequest: "settings-leave-request"
} as const;
