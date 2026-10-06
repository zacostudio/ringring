// 설정의 읽기·쓰기 — `settings` 표의 key/value. 모르는 값은 기본값으로 읽는다
use rusqlite::{Connection, OptionalExtension, params};

use crate::settings::{AppSettings, Language, Theme};

const KEY_LANGUAGE: &str = "language";
const KEY_THEME: &str = "theme";
const KEY_PAUSED: &str = "paused";
const KEY_FIRST_RUN_DONE: &str = "first_run_done";

fn get(conn: &Connection, key: &str) -> Result<Option<String>, String> {
	conn.query_row(
		"SELECT value FROM settings WHERE key = ?1",
		params![key],
		|row| row.get(0),
	)
	.optional()
	.map_err(|e| format!("Failed to read the setting {key}: {e}"))
}

fn set(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
	conn.execute(
		"INSERT INTO settings (key, value) VALUES (?1, ?2)
		 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
		params![key, value],
	)
	.map(|_| ())
	.map_err(|e| format!("Failed to save the setting {key}: {e}"))
}

fn flag(value: Option<String>) -> bool {
	value.as_deref() == Some("1")
}

pub fn load(conn: &Connection) -> Result<AppSettings, String> {
	Ok(AppSettings {
		language: Language::from_stored(get(conn, KEY_LANGUAGE)?.as_deref()),
		theme: Theme::from_stored(get(conn, KEY_THEME)?.as_deref()),
		paused: flag(get(conn, KEY_PAUSED)?),
		first_run_done: flag(get(conn, KEY_FIRST_RUN_DONE)?),
	})
}

pub fn save_language(conn: &Connection, language: Language) -> Result<(), String> {
	set(conn, KEY_LANGUAGE, language.as_str())
}

pub fn save_theme(conn: &Connection, theme: Theme) -> Result<(), String> {
	set(conn, KEY_THEME, theme.as_str())
}

pub fn save_paused(conn: &Connection, paused: bool) -> Result<(), String> {
	set(conn, KEY_PAUSED, if paused { "1" } else { "0" })
}

pub fn save_first_run_done(conn: &Connection) -> Result<(), String> {
	set(conn, KEY_FIRST_RUN_DONE, "1")
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::store::ensure_schema;

	fn conn() -> Connection {
		let c = Connection::open_in_memory().unwrap();
		ensure_schema(&c).unwrap();
		c
	}

	#[test]
	fn a_new_database_reads_the_defaults() {
		assert_eq!(load(&conn()).unwrap(), AppSettings::default());
	}

	#[test]
	fn saved_values_read_back() {
		let c = conn();
		save_language(&c, Language::Ja).unwrap();
		save_theme(&c, Theme::Dark).unwrap();
		save_paused(&c, true).unwrap();
		save_first_run_done(&c).unwrap();
		assert_eq!(
			load(&c).unwrap(),
			AppSettings {
				language: Language::Ja,
				theme: Theme::Dark,
				paused: true,
				first_run_done: true,
			}
		);
		save_paused(&c, false).unwrap();
		assert!(!load(&c).unwrap().paused);
	}

	#[test]
	fn an_unknown_stored_value_reads_as_the_default() {
		let c = conn();
		set(&c, KEY_LANGUAGE, "tlh").unwrap();
		set(&c, KEY_THEME, "sepia").unwrap();
		let loaded = load(&c).unwrap();
		assert_eq!(loaded.language, Language::System);
		assert_eq!(loaded.theme, Theme::System);
	}
}
