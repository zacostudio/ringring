// 앱 상태의 타입 — Rust 의 `commands/app.rs` `AppStateView` 그대로다

export type Language = "system" | "ko" | "en" | "ja";
export type Locale = "ko" | "en" | "ja";
export type Theme = "system" | "light" | "dark";

export interface LoginItemState {
	/** 로그인할 때 실행되게 등록돼 있다. */
	enabled: boolean;
	/** 이 빌드에서 켜고 끌 수 있다. 개발 빌드는 false 다. */
	available: boolean;
	/** 등록은 됐지만 시스템 설정의 로그인 항목에서 허용해야 한다. */
	needsApproval: boolean;
}

export interface AppState {
	version: string;
	isDev: boolean;
	language: Language;
	/** 실제로 쓰는 언어. */
	locale: Locale;
	theme: Theme;
	/** 전역 단축키가 일시 정지돼 있다. */
	paused: boolean;
	/** 이 빌드가 전역 단축키를 OS 에 등록하는가. 개발 빌드는 기본으로 등록하지 않는다. */
	shortcutsRegister: boolean;
	loginItem: LoginItemState;
	/** 다른 앱에 키 입력을 보낼 수 있는가 (손쉬운 사용 권한). */
	accessibilityTrusted: boolean;
	/** 키 입력 칸이 하나라도 있는가. */
	usesKeystrokes: boolean;
	/** 저장된 일반 단축키를 OS 가 받지 않은 링의 id. 그 링의 단축키는 지금 걸려 있지 않다. */
	refusedShortcuts: string[];
	/** 저장된 빠른 단축키를 OS 가 받지 않은 링의 id. */
	refusedQuickShortcuts: string[];
	/** 아직 보지 않은 실패의 수. 실행 기록 페이지를 열면 0 이 된다. */
	unseenFailures: number;
	/** 로그 파일의 경로. */
	logPath: string;
}

/** 실행 기록 하나 — 셸 명령의 결과이거나, 다른 동작의 실패다. Rust 의 `runs.rs` `RunRecord`. */
export interface RunRecord {
	id: number;
	/** 칸의 이름. 칸 없이 난 실패면 비어 있다. */
	label: string;
	/** 셸 명령의 실행인가. 아니면 출력과 걸린 시간이 없다. */
	shell: boolean;
	/** 끝난 시각 (RFC 3339). */
	finishedAt: string;
	durationMs: number;
	ok: boolean;
	exitCode: number | null;
	/** 실패의 한 줄. 기록할 때의 언어다. */
	error: string | null;
	output: string;
}
