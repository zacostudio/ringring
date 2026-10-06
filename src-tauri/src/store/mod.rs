// 앱의 SQLite 저장소 — 파일 하나, 연결 하나. 스키마 migration 은 1 부터 차례로 올린다
pub mod rings;
pub mod settings;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;

/// 지금 코드가 아는 스키마 버전.
pub const SCHEMA_VERSION: i64 = 1;

/// DB 연결 하나. Tauri state 로 둔다.
#[derive(Clone)]
pub struct Db(Arc<Mutex<Connection>>);

impl Db {
	/// 파일을 열고 스키마를 맞춘다. 폴더가 없으면 만든다.
	pub fn open(path: &Path) -> Result<Self, String> {
		if let Some(dir) = path.parent() {
			std::fs::create_dir_all(dir)
				.map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
		}
		let conn = Connection::open(path)
			.map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
		ensure_schema(&conn)?;
		Ok(Self(Arc::new(Mutex::new(conn))))
	}

	/// 곧바로 잠근다. 시작할 때와 main thread 의 짧은 읽기에만 쓴다.
	pub fn lock(&self) -> MutexGuard<'_, Connection> {
		self.0.lock().unwrap_or_else(|e| e.into_inner())
	}

	/// blocking thread 에서 `f` 를 돌린다. command 는 이 길로 읽고 쓴다.
	pub async fn with<T, F>(&self, f: F) -> Result<T, String>
	where
		T: Send + 'static,
		F: FnOnce(&Connection) -> T + Send + 'static,
	{
		let db = self.clone();
		tauri::async_runtime::spawn_blocking(move || f(&db.lock()))
			.await
			.map_err(|e| format!("DB task failed: {e}"))
	}
}

/// 스키마를 [`SCHEMA_VERSION`] 까지 올린다. 한 단계가 한 transaction 이다.
pub fn ensure_schema(conn: &Connection) -> Result<(), String> {
	conn.execute_batch(
		"PRAGMA foreign_keys = ON;
		CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);",
	)
	.map_err(|e| format!("DB setup failed: {e}"))?;
	let version: i64 = conn
		.query_row(
			"SELECT COALESCE(MAX(version), 0) FROM schema_version",
			[],
			|row| row.get(0),
		)
		.map_err(|e| format!("Failed to read the schema version: {e}"))?;
	if version > SCHEMA_VERSION {
		return Err(format!(
			"The database is version {version} but this build knows {SCHEMA_VERSION}"
		));
	}

	// v1 — 링, 칸, 설정.
	//
	// `shortcut` 이 NULL 이면 전역 단축키가 없는 링이다. `action_json` 은 `ring::model::RingAction` 의
	// serde 모양이다. 행이 없는 `position` 은 빈 칸이다. 하위 링을 여는 칸을 SQL 로 찾으려고 `action_kind` 를
	// 따로 둔다 (`store::rings` 의 순환 검사와 삭제 전 검사).
	if version < 1 {
		conn.execute_batch(
			"BEGIN;
			CREATE TABLE rings (
				id         TEXT PRIMARY KEY,
				name       TEXT NOT NULL,
				shortcut   TEXT UNIQUE,
				slot_count INTEGER NOT NULL CHECK (slot_count BETWEEN 4 AND 8),
				sort_order INTEGER NOT NULL,
				updated_at TEXT NOT NULL
			);
			CREATE TABLE ring_slots (
				ring_id     TEXT NOT NULL REFERENCES rings(id) ON DELETE CASCADE,
				-- 0 이 12시 방향이고 시계 방향으로 자란다.
				position    INTEGER NOT NULL CHECK (position BETWEEN 0 AND 7),
				label       TEXT NOT NULL,
				-- lucide 아이콘 이름 (kebab-case).
				icon        TEXT NOT NULL,
				action_kind TEXT NOT NULL,
				action_json TEXT NOT NULL,
				confirm     INTEGER NOT NULL DEFAULT 0 CHECK (confirm IN (0, 1)),
				PRIMARY KEY (ring_id, position)
			);
			CREATE TABLE settings (
				key   TEXT PRIMARY KEY,
				value TEXT NOT NULL
			);
			DELETE FROM schema_version;
			INSERT INTO schema_version (version) VALUES (1);
			COMMIT;",
		)
		.map_err(|e| format!("DB migration v1 failed: {e}"))?;
		log::info!("[store] schema v1 created");
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn version(conn: &Connection) -> i64 {
		conn.query_row("SELECT version FROM schema_version", [], |row| row.get(0))
			.unwrap()
	}

	#[test]
	fn a_new_database_lands_on_the_current_version() {
		let conn = Connection::open_in_memory().unwrap();
		ensure_schema(&conn).unwrap();
		assert_eq!(version(&conn), SCHEMA_VERSION);
		for table in ["rings", "ring_slots", "settings"] {
			let found: i64 = conn
				.query_row(
					"SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
					[table],
					|row| row.get(0),
				)
				.unwrap();
			assert_eq!(found, 1, "{table}");
		}
	}

	#[test]
	fn running_the_migration_twice_changes_nothing() {
		let conn = Connection::open_in_memory().unwrap();
		ensure_schema(&conn).unwrap();
		conn.execute(
			"INSERT INTO settings (key, value) VALUES ('theme', 'dark')",
			[],
		)
		.unwrap();
		ensure_schema(&conn).unwrap();
		assert_eq!(version(&conn), SCHEMA_VERSION);
		let rows: i64 = conn
			.query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))
			.unwrap();
		assert_eq!(rows, 1);
		let kept: String = conn
			.query_row(
				"SELECT value FROM settings WHERE key = 'theme'",
				[],
				|row| row.get(0),
			)
			.unwrap();
		assert_eq!(kept, "dark");
	}

	#[test]
	fn a_database_from_a_newer_build_is_refused() {
		let conn = Connection::open_in_memory().unwrap();
		ensure_schema(&conn).unwrap();
		conn.execute(
			"UPDATE schema_version SET version = ?1",
			[SCHEMA_VERSION + 1],
		)
		.unwrap();
		assert!(ensure_schema(&conn).is_err());
	}

	#[test]
	fn deleting_a_ring_cascades_to_its_slots() {
		let conn = Connection::open_in_memory().unwrap();
		ensure_schema(&conn).unwrap();
		conn.execute_batch(
			"INSERT INTO rings (id, name, slot_count, sort_order, updated_at) VALUES ('a', 'a', 6, 0, '');
			INSERT INTO ring_slots (ring_id, position, label, icon, action_kind, action_json)
			VALUES ('a', 0, 'x', 'x', 'open', '{}');
			DELETE FROM rings WHERE id = 'a';",
		)
		.unwrap();
		let left: i64 = conn
			.query_row("SELECT COUNT(*) FROM ring_slots", [], |row| row.get(0))
			.unwrap();
		assert_eq!(left, 0);
	}
}
