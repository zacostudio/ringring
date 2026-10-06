// 앱 상태와 설정의 Tauri IPC — Rust 의 `commands/app.rs` · `commands/runs.rs`
import { invoke } from "@/shared/tauri/ipc";
import type { AppState, Language, RunRecord, Theme } from "../domain/types";

export const appRepository = {
	state: () => invoke<AppState>("app_state"),
	setLanguage: (language: Language) => invoke<AppState>("settings_set_language", { language }),
	setTheme: (theme: Theme) => invoke<AppState>("settings_set_theme", { theme }),
	setPaused: (paused: boolean) => invoke<AppState>("settings_set_paused", { paused }),
	setLaunchAtLogin: (enabled: boolean) => invoke<AppState>("settings_set_launch_at_login", { enabled }),
	openLoginItemsSettings: () => invoke<void>("login_items_open_settings"),
	openAccessibilitySettings: () => invoke<void>("accessibility_open_settings"),
	/** `id` 번 단축키 입력을 받기 시작했거나 끝냈다. 받는 동안 Rust 가 링의 단축키를 뗀다. */
	shortcutCapture: (active: boolean, id: number) =>
		invoke<void>(active ? "shortcut_capture_begin" : "shortcut_capture_end", { id }),
	/** 설정 페이지가 첫 화면을 그렸다. Rust 가 창을 보인다. */
	settingsReady: () => invoke<void>("settings_ready"),
	/** 창을 닫거나 앱을 끝내도 되는지 답한다. `mayLeave` 가 false 면 창이 그대로 있다. */
	leaveAnswer: (id: number, mayLeave: boolean) => invoke<void>("settings_leave_answer", { id, mayLeave }),
	runs: () => invoke<RunRecord[]>("runs_list"),
	/** 실행 기록을 보고 있다. 보지 않은 실패가 없어진다. */
	runsSeen: () => invoke<void>("runs_seen"),
	clearRuns: () => invoke<void>("runs_clear")
};
