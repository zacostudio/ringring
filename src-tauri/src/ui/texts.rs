// Rust 가 직접 보이는 글 — 트레이 메뉴, 실패 알림, 새 링의 기본 이름 (ko / en / ja)
//
// 설정 창과 링 창의 글은 프런트의 `shared/i18n` 에 있다. 여기는 webview 없이 보이는 글만 둔다.
use crate::settings::Locale;

/// 앱 이름. 한국어에서는 "링링이" 다.
pub fn app_name(locale: Locale) -> &'static str {
	match locale {
		Locale::Ko => "링링이",
		Locale::En | Locale::Ja => "RingRing",
	}
}

pub struct TrayTexts {
	pub settings: &'static str,
	pub pause: &'static str,
	pub launch_at_login: &'static str,
	pub no_rings: &'static str,
	/// 아직 보지 않은 실패가 있을 때 메뉴 맨 위에 오는 줄. `{n}` 은 그 수다.
	pub failed_runs: fn(usize) -> String,
	pub about: String,
	pub quit: String,
	pub tooltip: String,
	pub tooltip_paused: String,
}

pub fn tray(locale: Locale, version: &str) -> TrayTexts {
	let name = app_name(locale);
	match locale {
		Locale::Ko => TrayTexts {
			settings: "설정…",
			pause: "단축키 일시 정지",
			launch_at_login: "로그인할 때 실행",
			no_rings: "링이 없습니다",
			failed_runs: |n| format!("실패한 실행 {n}건 보기…"),
			about: format!("{name} 정보 ({version})"),
			quit: format!("{name} 종료"),
			tooltip: name.to_string(),
			tooltip_paused: format!("{name} — 단축키 일시 정지 중"),
		},
		Locale::En => TrayTexts {
			settings: "Settings…",
			pause: "Pause Shortcuts",
			launch_at_login: "Launch at Login",
			no_rings: "No rings yet",
			failed_runs: |n| format!("Show Failed Runs ({n})…"),
			about: format!("About {name} ({version})"),
			quit: format!("Quit {name}"),
			tooltip: name.to_string(),
			tooltip_paused: format!("{name} — shortcuts paused"),
		},
		Locale::Ja => TrayTexts {
			settings: "設定…",
			pause: "ショートカットを一時停止",
			launch_at_login: "ログイン時に起動",
			no_rings: "リングがありません",
			failed_runs: |n| format!("失敗した実行を表示 ({n})…"),
			about: format!("{name} について ({version})"),
			quit: format!("{name} を終了"),
			tooltip: name.to_string(),
			tooltip_paused: format!("{name} — ショートカット一時停止中"),
		},
	}
}

pub fn new_ring_name(locale: Locale) -> &'static str {
	match locale {
		Locale::Ko => "새 링",
		Locale::En => "New ring",
		Locale::Ja => "新しいリング",
	}
}

pub fn new_sub_ring_name(locale: Locale) -> &'static str {
	match locale {
		Locale::Ko => "새 하위 링",
		Locale::En => "New sub-ring",
		Locale::Ja => "新しいサブリング",
	}
}

pub fn starter_ring_name(locale: Locale) -> &'static str {
	match locale {
		Locale::Ko => "기본 링",
		Locale::En => "Starter",
		Locale::Ja => "スターター",
	}
}

/// 기본 링의 칸 이름. `key` 는 `ring::starter` 의 후보 이름이다.
pub fn starter_slot_name(key: &str, locale: Locale) -> &'static str {
	match (key, locale) {
		("finder", _) => "Finder",
		("safari", _) => "Safari",
		("explorer", Locale::Ko) => "파일 탐색기",
		("explorer", Locale::En) => "File Explorer",
		("explorer", Locale::Ja) => "エクスプローラー",
		("edge", _) => "Edge",
		("notepad", Locale::Ko) => "메모장",
		("notepad", Locale::En) => "Notepad",
		("notepad", Locale::Ja) => "メモ帳",
		("terminal", Locale::Ko) => "터미널",
		("terminal", Locale::En) => "Terminal",
		("terminal", Locale::Ja) => "ターミナル",
		("system_settings", Locale::Ko) => "시스템 설정",
		("system_settings", Locale::En) => "System Settings",
		("system_settings", Locale::Ja) => "システム設定",
		("downloads", Locale::Ko) => "다운로드",
		("downloads", Locale::En) => "Downloads",
		("downloads", Locale::Ja) => "ダウンロード",
		_ => "",
	}
}

/// 실패 알림의 제목. 칸의 이름이 있으면 붙인다.
pub fn notice_title(label: Option<&str>, locale: Locale) -> String {
	let name = app_name(locale);
	match label {
		Some(label) => format!("{name} — {label}"),
		None => name.to_string(),
	}
}

pub fn target_missing(locale: Locale) -> &'static str {
	match locale {
		Locale::Ko => "대상이 없습니다.",
		Locale::En => "The target no longer exists.",
		Locale::Ja => "対象がありません。",
	}
}

pub fn sub_ring_empty(locale: Locale) -> &'static str {
	match locale {
		Locale::Ko => "하위 링이 비어 있습니다. 설정에서 그 링의 칸을 채우세요.",
		Locale::En => "That sub-ring is empty. Fill its slots in Settings.",
		Locale::Ja => "サブリングが空です。設定でそのリングのスロットを設定してください。",
	}
}

/// 글에 적는 OS 의 이름.
#[cfg(windows)]
macro_rules! os_name {
	() => {
		"Windows"
	};
}
#[cfg(not(windows))]
macro_rules! os_name {
	() => {
		"macOS"
	};
}

/// 저장된 단축키를 OS 가 받지 않았다. 실행 기록에 링의 이름과 함께 남는다.
pub fn shortcut_refused(locale: Locale) -> &'static str {
	match locale {
		Locale::Ko => {
			concat!(
				os_name!(),
				" 가 이 링의 단축키를 등록하지 않았습니다. 다른 앱이 쓰는 조합일 수 있습니다. 설정에서 다른 조합을 고르세요."
			)
		}
		Locale::En => {
			concat!(
				os_name!(),
				" did not register this ring's shortcut. Another app may hold the combination. Choose another one in Settings."
			)
		}
		Locale::Ja => {
			concat!(
				os_name!(),
				" がこのリングのショートカットを登録しませんでした。他のアプリが使っている可能性があります。設定で別の組み合わせを選んでください。"
			)
		}
	}
}

pub fn not_trusted(locale: Locale) -> &'static str {
	match locale {
		Locale::Ko => "키 입력을 보내려면 시스템 설정의 손쉬운 사용에서 링링이를 허용해야 합니다.",
		Locale::En => "Allow RingRing under Accessibility in System Settings to send keystrokes.",
		Locale::Ja => {
			"キー入力を送るには、システム設定のアクセシビリティで RingRing を許可してください。"
		}
	}
}

pub fn modifiers_held(locale: Locale) -> &'static str {
	match locale {
		Locale::Ko => "수식키를 누르고 있어 키 입력을 보내지 않았습니다.",
		Locale::En => "Keystrokes were not sent because a modifier key was still held.",
		Locale::Ja => "修飾キーが押されたままのため、キー入力を送りませんでした。",
	}
}

/// 셸 명령이 실패한 모양. 출력은 싣지 않는다 — 알림은 OS 의 알림 기록에 남는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellFailure {
	/// 0 이 아닌 종료 코드.
	Exit(i32),
	Signal(i32),
	/// 제한 시간(초)을 넘겨 끝냈다.
	TimedOut(u64),
}

/// 셸 명령이 어떻게 실패했는지 한 줄. 실행 기록이 보인다.
pub fn shell_failure(failure: ShellFailure, locale: Locale) -> String {
	match (failure, locale) {
		(ShellFailure::Exit(code), Locale::Ko) => format!("명령이 실패했습니다. 종료 코드 {code}."),
		(ShellFailure::Exit(code), Locale::En) => format!("The command exited with code {code}."),
		(ShellFailure::Exit(code), Locale::Ja) => {
			format!("コマンドがコード {code} で終了しました。")
		}
		(ShellFailure::Signal(signal), Locale::Ko) => {
			format!("명령이 중단됐습니다. signal {signal}.")
		}
		(ShellFailure::Signal(signal), Locale::En) => {
			format!("The command was stopped by signal {signal}.")
		}
		(ShellFailure::Signal(signal), Locale::Ja) => {
			format!("コマンドが signal {signal} で停止しました。")
		}
		(ShellFailure::TimedOut(secs), Locale::Ko) => {
			format!("제한 시간 {secs}초를 넘겨 명령을 끝냈습니다.")
		}
		(ShellFailure::TimedOut(secs), Locale::En) => {
			format!("The command passed its {secs} s time limit and was stopped.")
		}
		(ShellFailure::TimedOut(secs), Locale::Ja) => {
			format!("制限時間 {secs} 秒を超えたため、コマンドを停止しました。")
		}
	}
}

/// 알림 끝에 붙는 한 줄. 출력은 알림에 싣지 않고 어디서 보는지를 말한다.
pub fn see_recent_runs(locale: Locale) -> &'static str {
	match locale {
		Locale::Ko => "출력은 설정의 실행 기록에 있습니다.",
		Locale::En => "Its output is under Recent runs in Settings.",
		Locale::Ja => "出力は設定の「実行履歴」にあります。",
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const LOCALES: [Locale; 3] = [Locale::Ko, Locale::En, Locale::Ja];

	#[test]
	fn every_tray_text_is_filled_in_every_language() {
		for locale in LOCALES {
			let texts = tray(locale, "1.2.3");
			for text in [
				texts.settings,
				texts.pause,
				texts.launch_at_login,
				texts.no_rings,
			] {
				assert!(!text.is_empty(), "{locale:?}");
			}
			assert!((texts.failed_runs)(3).contains('3'), "{locale:?}");
			assert!(texts.about.contains("1.2.3"));
			assert!(texts.quit.contains(app_name(locale)));
			assert_ne!(texts.tooltip, texts.tooltip_paused);
		}
	}

	#[test]
	fn every_starter_slot_has_a_name() {
		for key in [
			"finder",
			"safari",
			"explorer",
			"edge",
			"notepad",
			"terminal",
			"system_settings",
			"downloads",
		] {
			for locale in LOCALES {
				assert!(
					!starter_slot_name(key, locale).is_empty(),
					"{key} {locale:?}"
				);
			}
		}
	}

	#[test]
	fn an_empty_sub_ring_is_not_reported_as_a_missing_target() {
		for locale in LOCALES {
			assert_ne!(sub_ring_empty(locale), target_missing(locale));
		}
	}

	#[test]
	fn a_shell_failure_line_names_the_code_or_the_limit_in_every_language() {
		for locale in LOCALES {
			assert!(shell_failure(ShellFailure::Exit(3), locale).contains('3'));
			assert!(shell_failure(ShellFailure::Signal(9), locale).contains('9'));
			assert!(shell_failure(ShellFailure::TimedOut(120), locale).contains("120"));
			assert!(!see_recent_runs(locale).is_empty());
		}
	}

	#[test]
	fn the_notice_title_carries_the_slot_name() {
		assert_eq!(notice_title(Some("배포"), Locale::Ko), "링링이 — 배포");
		assert_eq!(notice_title(None, Locale::En), "RingRing");
	}
}
