// 앱 설정의 모양과 메모리의 지금 값 — 언어, 테마, 단축키 일시 정지, 첫 실행 여부
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

/// 사용자가 고른 언어. `System` 이면 OS 의 언어를 따른다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
	#[default]
	System,
	Ko,
	En,
	Ja,
}

impl Language {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::System => "system",
			Self::Ko => "ko",
			Self::En => "en",
			Self::Ja => "ja",
		}
	}

	pub fn from_stored(value: Option<&str>) -> Self {
		match value {
			Some("ko") => Self::Ko,
			Some("en") => Self::En,
			Some("ja") => Self::Ja,
			_ => Self::System,
		}
	}

	/// 실제로 쓸 언어. `system_tag` 는 OS 의 언어 태그다 (`ko-KR` 등).
	pub fn resolve(self, system_tag: Option<&str>) -> Locale {
		match self {
			Self::Ko => Locale::Ko,
			Self::En => Locale::En,
			Self::Ja => Locale::Ja,
			Self::System => Locale::from_tag(system_tag),
		}
	}
}

/// 화면과 알림에 실제로 쓰는 언어.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Locale {
	Ko,
	En,
	Ja,
}

impl Locale {
	/// OS 의 언어 태그로 정한다. 한국어·일본어가 아니면 영어다.
	pub fn from_tag(tag: Option<&str>) -> Self {
		let lower = tag.unwrap_or_default().to_ascii_lowercase();
		if lower.starts_with("ko") {
			Self::Ko
		} else if lower.starts_with("ja") {
			Self::Ja
		} else {
			Self::En
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
	#[default]
	System,
	Light,
	Dark,
}

impl Theme {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::System => "system",
			Self::Light => "light",
			Self::Dark => "dark",
		}
	}

	pub fn from_stored(value: Option<&str>) -> Self {
		match value {
			Some("light") => Self::Light,
			Some("dark") => Self::Dark,
			_ => Self::System,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AppSettings {
	pub language: Language,
	pub theme: Theme,
	/// 전역 단축키를 잠시 끈다.
	pub paused: bool,
	/// 첫 실행의 안내를 이미 보였다.
	pub first_run_done: bool,
}

static CURRENT: RwLock<AppSettings> = RwLock::new(AppSettings {
	language: Language::System,
	theme: Theme::System,
	paused: false,
	first_run_done: false,
});

/// 지금 설정. 시작할 때 DB 에서 읽어 넣고, 바꿀 때마다 같이 바꾼다.
pub fn current() -> AppSettings {
	*CURRENT.read().unwrap_or_else(|e| e.into_inner())
}

pub fn replace(settings: AppSettings) {
	*CURRENT.write().unwrap_or_else(|e| e.into_inner()) = settings;
}

pub fn update(change: impl FnOnce(&mut AppSettings)) -> AppSettings {
	let mut guard = CURRENT.write().unwrap_or_else(|e| e.into_inner());
	change(&mut guard);
	*guard
}

/// 지금 쓸 언어.
pub fn locale() -> Locale {
	current()
		.language
		.resolve(sys_locale::get_locale().as_deref())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn an_explicit_language_wins_over_the_system() {
		assert_eq!(Language::Ja.resolve(Some("ko-KR")), Locale::Ja);
		assert_eq!(Language::En.resolve(Some("ko-KR")), Locale::En);
	}

	#[test]
	fn the_system_language_falls_back_to_english() {
		assert_eq!(Language::System.resolve(Some("ko-KR")), Locale::Ko);
		assert_eq!(Language::System.resolve(Some("ja_JP")), Locale::Ja);
		assert_eq!(Language::System.resolve(Some("fr-FR")), Locale::En);
		assert_eq!(Language::System.resolve(None), Locale::En);
	}

	#[test]
	fn stored_text_round_trips() {
		for language in [Language::System, Language::Ko, Language::En, Language::Ja] {
			assert_eq!(Language::from_stored(Some(language.as_str())), language);
		}
		for theme in [Theme::System, Theme::Light, Theme::Dark] {
			assert_eq!(Theme::from_stored(Some(theme.as_str())), theme);
		}
	}
}
