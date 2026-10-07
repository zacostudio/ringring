// 링의 읽기·쓰기 — `rings` · `ring_slots` 표

//! 링과 하위 링은 같은 표에 있다. 단축키가 있으면 전역 단축키로 여는 링이고, 없으면 다른 링의 칸이나
//! 트레이 메뉴가 여는 링이다.
//!
//! 저장 전 검증은 `ring::model` 이 한다. 여기서는 그 검증을 부르고, 다른 링을 읽어야 아는
//! 것(하위 링의 순환과 깊이, 가리키는 칸)을 SQL 로 모아 건넨다.
//!
//! 모든 함수가 `&Connection` 을 받는다 — 호출자는 `Db::with` 안에서 부른다.

use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::ring::model::{
	self, Named, Refusal, Ring, RingAction, RingError, ShortcutKind, Slot, check_name,
	check_slot_count,
};

/// 이 파일의 함수가 돌려주는 값. 거절이면 [`Refusal`] 이고, DB 오류면 그 원문이다.
type Outcome<T> = Result<T, RingError>;

/// 자리에서 만든 하위 링을 여는 칸의 기본 아이콘. 설정 화면의 "하위 링 열기" 종류와 같은 그림이다.
const SUB_RING_ICON: &str = "circle-dot";

/// 새 링의 칸 수.
pub const DEFAULT_SLOT_COUNT: usize = 6;

/// 어떤 링을 여는 칸. 링을 지우려 할 때 그 링을 가리키는 칸을 보이는 데 쓴다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotRef {
	pub ring_id: String,
	pub ring_name: String,
	pub position: usize,
	pub label: String,
}

fn now() -> String {
	Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn sql<T>(result: rusqlite::Result<T>) -> Outcome<T> {
	result.map_err(|e| RingError::Other(format!("Ring DB error: {e}")))
}

/// 한 링의 칸들. 읽을 수 없는 `action_json` 은 그 칸만 건너뛰고 로그를 남긴다 — 빈 칸으로 보인다.
fn slots_of(conn: &Connection, ring_id: &str) -> Outcome<Vec<Slot>> {
	let mut stmt = sql(conn.prepare(
		"SELECT position, label, icon, action_json, confirm
		 FROM ring_slots WHERE ring_id = ?1 ORDER BY position",
	))?;
	let rows = sql(stmt.query_map(params![ring_id], |row| {
		Ok((
			row.get::<_, i64>(0)?,
			row.get::<_, String>(1)?,
			row.get::<_, String>(2)?,
			row.get::<_, String>(3)?,
			row.get::<_, i64>(4)?,
		))
	}))?;
	let mut slots = Vec::new();
	for row in rows {
		let (position, label, icon, action_json, confirm) = sql(row)?;
		match serde_json::from_str::<RingAction>(&action_json) {
			Ok(action) => slots.push(Slot {
				position: position as usize,
				label,
				icon,
				action,
				confirm: confirm != 0,
			}),
			Err(e) => {
				log::warn!("[ring] slot {position} of ring {ring_id} has an unreadable action: {e}")
			}
		}
	}
	Ok(slots)
}

fn ring_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Ring> {
	Ok(Ring {
		id: row.get(0)?,
		name: row.get(1)?,
		shortcut: row.get(2)?,
		quick_shortcut: row.get(4)?,
		slot_count: row.get::<_, i64>(3)? as usize,
		slots: Vec::new(),
	})
}

/// 링 전체. 만든 순서다.
pub fn list(conn: &Connection) -> Outcome<Vec<Ring>> {
	let mut stmt = sql(conn.prepare(
		"SELECT id, name, shortcut, slot_count, quick_shortcut FROM rings ORDER BY sort_order, id",
	))?;
	let rings = sql(stmt.query_map([], ring_from_row))?;
	let mut out = Vec::new();
	for ring in rings {
		let mut ring = sql(ring)?;
		ring.slots = slots_of(conn, &ring.id)?;
		out.push(ring);
	}
	Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> Outcome<Option<Ring>> {
	let ring = sql(conn
		.query_row(
			"SELECT id, name, shortcut, slot_count, quick_shortcut FROM rings WHERE id = ?1",
			params![id],
			ring_from_row,
		)
		.optional())?;
	match ring {
		Some(mut ring) => {
			ring.slots = slots_of(conn, &ring.id)?;
			Ok(Some(ring))
		}
		None => Ok(None),
	}
}

fn require(conn: &Connection, id: &str) -> Outcome<Ring> {
	get(conn, id)?.ok_or(RingError::Refused(Refusal::RingGone))
}

/// 새 링을 만든다. 칸은 [`DEFAULT_SLOT_COUNT`] 개고 `slots` 가 미리 들어간다. 단축키는 없다.
pub fn create(conn: &Connection, id: &str, name: &str, slots: &[Slot]) -> Outcome<Ring> {
	check_name(name, Named::Ring)?;
	for slot in slots {
		slot.validate(DEFAULT_SLOT_COUNT)?;
	}
	let tx = sql(conn.unchecked_transaction())?;
	insert_ring(&tx, id, name)?;
	for slot in slots {
		insert_slot(&tx, id, slot)?;
	}
	sql(tx.commit())?;
	require(conn, id)
}

/// 링 행 하나를 목록의 맨 뒤에 넣는다. 칸은 [`DEFAULT_SLOT_COUNT`] 개고 단축키는 없다. 이름 검증은 호출자가 한다.
fn insert_ring(conn: &Connection, id: &str, name: &str) -> Outcome<()> {
	let sort_order: i64 = sql(conn.query_row(
		"SELECT COALESCE(MAX(sort_order), -1) + 1 FROM rings",
		[],
		|row| row.get(0),
	))?;
	sql(conn.execute(
		"INSERT INTO rings (id, name, shortcut, slot_count, sort_order, updated_at)
		 VALUES (?1, ?2, NULL, ?3, ?4, ?5)",
		params![
			id,
			name.trim(),
			DEFAULT_SLOT_COUNT as i64,
			sort_order,
			now()
		],
	))?;
	Ok(())
}

/// 자리에서 하위 링을 만들고 그 칸에 잇는다. 돌려주는 것은 `(칸이 있는 링, 새 하위 링)` 이다.
///
/// 링 만들기와 칸 저장이 한 transaction 이다 — 어느 쪽이 실패해도 아무것도 남지 않는다.
///
/// 새 링은 비어 있다. 하위 링은 다른 동작을 모으려고 만드는 것이다.
///
/// 이름과 아이콘의 기본값은 여기서 정한다. 칸 이름이 있으면 새 링이 그 이름을 쓰고, 없으면
/// `default_name` 이 링과 칸의 이름이 된다. 아이콘이 비어 있으면 [`SUB_RING_ICON`] 이다.
pub fn create_sub_ring(
	conn: &Connection,
	new_id: &str,
	ring_id: &str,
	position: usize,
	label: &str,
	icon: &str,
	default_name: &str,
) -> Outcome<(Ring, Ring)> {
	let ring = require(conn, ring_id)?;
	let name = if label.trim().is_empty() {
		default_name
	} else {
		label
	};
	check_name(name, Named::Ring)?;
	let slot = Slot {
		position,
		label: name.to_string(),
		icon: if icon.trim().is_empty() {
			SUB_RING_ICON.to_string()
		} else {
			icon.to_string()
		},
		action: RingAction::OpenRing {
			ring_id: new_id.to_string(),
		},
		confirm: false,
	};
	slot.validate(ring.slot_count)?;
	// 새 링은 아직 아무 링도 열지 않는다. 깊이는 이 링 위쪽 사슬로만 정해진다.
	let edges = links(conn, Some((ring_id, position)))?;
	model::check_link(&edges, ring_id, new_id)?;
	let tx = sql(conn.unchecked_transaction())?;
	insert_ring(&tx, new_id, name)?;
	insert_slot(&tx, ring_id, &slot)?;
	touch(&tx, ring_id)?;
	sql(tx.commit())?;
	Ok((require(conn, ring_id)?, require(conn, new_id)?))
}

/// 이름과 칸 수를 바꾼다. 칸 수를 줄이면 넘치는 칸은 같은 transaction 에서 지운다.
pub fn update(conn: &Connection, id: &str, name: &str, slot_count: usize) -> Outcome<Ring> {
	check_name(name, Named::Ring)?;
	check_slot_count(slot_count)?;
	let tx = sql(conn.unchecked_transaction())?;
	let changed = sql(tx.execute(
		"UPDATE rings SET name = ?2, slot_count = ?3, updated_at = ?4 WHERE id = ?1",
		params![id, name.trim(), slot_count as i64, now()],
	))?;
	if changed == 0 {
		return Err(Refusal::RingGone.into());
	}
	sql(tx.execute(
		"DELETE FROM ring_slots WHERE ring_id = ?1 AND position >= ?2",
		params![id, slot_count as i64],
	))?;
	sync_sub_ring_labels(&tx)?;
	sql(tx.commit())?;
	require(conn, id)
}

/// `kind` 의 단축키를 바꾼다. `None` 이면 뗀다. 조합의 충돌 검사와 OS 등록은 호출자가 먼저 한다.
pub fn set_shortcut(
	conn: &Connection,
	id: &str,
	kind: ShortcutKind,
	shortcut: Option<&str>,
) -> Outcome<()> {
	let statement = match kind {
		ShortcutKind::Normal => "UPDATE rings SET shortcut = ?2, updated_at = ?3 WHERE id = ?1",
		ShortcutKind::Quick => {
			"UPDATE rings SET quick_shortcut = ?2, updated_at = ?3 WHERE id = ?1"
		}
	};
	let changed = sql(conn.execute(statement, params![id, shortcut, now()]))?;
	if changed == 0 {
		return Err(Refusal::RingGone.into());
	}
	Ok(())
}

/// "링 → 그 링이 여는 하위 링" 전부. `(skip_ring, skip_position)` 자리의 칸은 뺀다 — 고치는 칸의 예전 연결이다.
fn links(conn: &Connection, skip: Option<(&str, usize)>) -> Outcome<Vec<(String, String)>> {
	let mut stmt = sql(conn.prepare(
		"SELECT ring_id, position, json_extract(action_json, '$.ring_id')
		 FROM ring_slots WHERE action_kind = 'open_ring'",
	))?;
	let rows = sql(stmt.query_map([], |row| {
		Ok((
			row.get::<_, String>(0)?,
			row.get::<_, i64>(1)?,
			row.get::<_, Option<String>>(2)?,
		))
	}))?;
	let mut out = Vec::new();
	for row in rows {
		let (ring_id, position, target) = sql(row)?;
		if skip == Some((ring_id.as_str(), position as usize)) {
			continue;
		}
		if let Some(target) = target {
			out.push((ring_id, target));
		}
	}
	Ok(out)
}

/// 칸 하나를 넣는다. "실행 전에 확인" 이 뜻이 없는 동작이면 그 값을 끈 채로 넣는다.
/// 하위 링을 여는 칸의 이름을 그 링의 이름으로 맞춘다. 하위 링 칸은 이름을 따로 갖지 않는다 —
/// 칸의 이름과 링의 이름이 달라 무엇을 고쳤는지 알 수 없던 것을 없앤다. migration v2 도 이 문장을 쓴다.
pub(super) const SYNC_SUB_RING_LABELS: &str = "UPDATE ring_slots
	SET label = (SELECT name FROM rings WHERE rings.id = json_extract(ring_slots.action_json, '$.ring_id'))
	WHERE action_kind = 'open_ring'
		AND EXISTS (SELECT 1 FROM rings WHERE rings.id = json_extract(ring_slots.action_json, '$.ring_id'))";

fn sync_sub_ring_labels(conn: &Connection) -> Outcome<()> {
	sql(conn.execute(SYNC_SUB_RING_LABELS, []))?;
	Ok(())
}

fn insert_slot(conn: &Connection, ring_id: &str, slot: &Slot) -> Outcome<()> {
	let action_json =
		serde_json::to_string(&slot.action).map_err(|e| RingError::Other(e.to_string()))?;
	sql(conn.execute(
		"INSERT INTO ring_slots (ring_id, position, label, icon, action_kind, action_json, confirm)
		 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
		 ON CONFLICT(ring_id, position) DO UPDATE SET
			label = excluded.label, icon = excluded.icon, action_kind = excluded.action_kind,
			action_json = excluded.action_json, confirm = excluded.confirm",
		params![
			ring_id,
			slot.position as i64,
			slot.label.trim(),
			slot.icon.trim(),
			slot.action.kind(),
			action_json,
			(slot.confirm && slot.action.asks_confirm()) as i64
		],
	))?;
	Ok(())
}

/// 칸 하나를 저장한다. 그 자리에 칸이 있으면 바꾼다.
pub fn save_slot(conn: &Connection, ring_id: &str, slot: &Slot) -> Outcome<Ring> {
	let ring = require(conn, ring_id)?;
	slot.validate(ring.slot_count)?;
	if let RingAction::OpenRing { ring_id: target } = &slot.action {
		if get(conn, target)?.is_none() {
			return Err(Refusal::RingGone.into());
		}
		let edges = links(conn, Some((ring_id, slot.position)))?;
		model::check_link(&edges, ring_id, target)?;
	}
	let tx = sql(conn.unchecked_transaction())?;
	insert_slot(&tx, ring_id, slot)?;
	// 하위 링 칸의 이름은 그 링의 이름이다. 칸에서 이름을 고치면 링의 이름이 바뀌고, 그 링을 여는 다른 칸도 따라간다.
	if let RingAction::OpenRing { ring_id: target } = &slot.action {
		sql(tx.execute(
			"UPDATE rings SET name = ?2, updated_at = ?3 WHERE id = ?1",
			params![target, slot.label.trim(), now()],
		))?;
		sync_sub_ring_labels(&tx)?;
	}
	touch(&tx, ring_id)?;
	sql(tx.commit())?;
	require(conn, ring_id)
}

fn touch(conn: &Connection, ring_id: &str) -> Outcome<()> {
	sql(conn.execute(
		"UPDATE rings SET updated_at = ?2 WHERE id = ?1",
		params![ring_id, now()],
	))?;
	Ok(())
}

/// 칸을 비운다. 이미 비어 있어도 오류가 아니다.
pub fn clear_slot(conn: &Connection, ring_id: &str, position: usize) -> Outcome<Ring> {
	sql(conn.execute(
		"DELETE FROM ring_slots WHERE ring_id = ?1 AND position = ?2",
		params![ring_id, position as i64],
	))?;
	touch(conn, ring_id)?;
	require(conn, ring_id)
}

/// 칸의 순서를 바꾼다. `order[새 자리] = 예전 자리` 다. `order` 는 `0..slot_count` 의 순열이어야 한다.
/// 빈 자리도 같이 움직인다.
pub fn reorder_slots(conn: &Connection, ring_id: &str, order: &[usize]) -> Outcome<Ring> {
	let ring = require(conn, ring_id)?;
	let mut sorted = order.to_vec();
	sorted.sort_unstable();
	if sorted != (0..ring.slot_count).collect::<Vec<_>>() {
		return Err(Refusal::OrderNotPermutation.into());
	}
	let tx = sql(conn.unchecked_transaction())?;
	sql(tx.execute(
		"DELETE FROM ring_slots WHERE ring_id = ?1",
		params![ring_id],
	))?;
	for (new_position, old_position) in order.iter().enumerate() {
		if let Some(slot) = ring.slot(*old_position) {
			let moved = Slot {
				position: new_position,
				..slot.clone()
			};
			insert_slot(&tx, ring_id, &moved)?;
		}
	}
	sql(tx.execute(
		"UPDATE rings SET updated_at = ?2 WHERE id = ?1",
		params![ring_id, now()],
	))?;
	sql(tx.commit())?;
	require(conn, ring_id)
}

/// 하위 링으로 고를 수 있는 링 하나.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkCandidate {
	pub id: String,
	pub name: String,
	/// 그 링에 채워진 칸의 수. 0 이면 열어도 고를 것이 없다.
	pub filled_slots: usize,
	/// 고를 수 없으면 그 이유의 코드다 (`link_cycle`, `link_too_deep`). 고를 수 있으면 `None`.
	pub blocked: Option<&'static str>,
}

/// 한 칸이 하위 링으로 고를 수 있는 것 전부.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkChoices {
	pub candidates: Vec<LinkCandidate>,
	/// 이 칸에서 새 하위 링을 만들 수 없으면 그 이유의 코드다 (`link_too_deep`). 만들 수 있으면 `None`.
	pub create_blocked: Option<&'static str>,
}

/// 아직 없는 링의 id 자리에 쓰는 값. 깊이 검사에만 쓴다. 링 id 는 UUID 라 이 값과 겹치지 않는다.
const NEW_RING_PROBE: &str = "(new)";

/// `ring_id` 의 `position` 칸이 열 수 있는 링 목록. 자기 자신은 뺀다. 순환이 되거나 깊이를 넘는 링은
/// 이유와 함께 준다 — 설정 화면이 고를 수 없게 보인다. 새 하위 링을 만들 수 있는지도 같이 준다.
pub fn link_candidates(conn: &Connection, ring_id: &str, position: usize) -> Outcome<LinkChoices> {
	let edges = links(conn, Some((ring_id, position)))?;
	let blocked = |target: &str| {
		model::check_link(&edges, ring_id, target)
			.err()
			.map(Refusal::code)
	};
	let candidates = list(conn)?
		.into_iter()
		.filter(|ring| ring.id != ring_id)
		.map(|ring| LinkCandidate {
			blocked: blocked(&ring.id),
			filled_slots: ring.slots.len(),
			id: ring.id,
			name: ring.name,
		})
		.collect();
	Ok(LinkChoices {
		candidates,
		create_blocked: blocked(NEW_RING_PROBE),
	})
}

/// `ring_id` 를 여는 칸들.
pub fn references(conn: &Connection, ring_id: &str) -> Outcome<Vec<SlotRef>> {
	let mut stmt = sql(conn.prepare(
		"SELECT s.ring_id, r.name, s.position, s.label
		 FROM ring_slots s JOIN rings r ON r.id = s.ring_id
		 WHERE s.action_kind = 'open_ring' AND json_extract(s.action_json, '$.ring_id') = ?1
		 ORDER BY r.sort_order, s.position",
	))?;
	let rows = sql(stmt.query_map(params![ring_id], |row| {
		Ok(SlotRef {
			ring_id: row.get(0)?,
			ring_name: row.get(1)?,
			position: row.get::<_, i64>(2)? as usize,
			label: row.get(3)?,
		})
	}))?;
	sql(rows.collect())
}

/// 링을 지운다. 그 링을 여는 칸이 있으면 지우지 않고 그 칸들을 돌려준다. 빈 목록이면 지운 것이다.
pub fn delete(conn: &Connection, id: &str) -> Outcome<Vec<SlotRef>> {
	let refs = references(conn, id)?;
	if !refs.is_empty() {
		return Ok(refs);
	}
	// 칸은 `ON DELETE CASCADE` 로 같이 지워진다. 연결마다 foreign key 가 켜져 있지 않을 수 있어 직접도 지운다.
	let tx = sql(conn.unchecked_transaction())?;
	sql(tx.execute("DELETE FROM ring_slots WHERE ring_id = ?1", params![id]))?;
	sql(tx.execute("DELETE FROM rings WHERE id = ?1", params![id]))?;
	sql(tx.commit())?;
	Ok(Vec::new())
}

/// 가져온 링들을 한 transaction 으로 넣는다. 하나라도 실패하면 아무것도 남지 않는다.
///
/// 검증과 id 새로 매기기는 `ring::transfer::plan` 이 끝냈다. 여기서는 넣기만 한다.
pub fn import(conn: &Connection, rings: &[Ring]) -> Outcome<()> {
	let tx = sql(conn.unchecked_transaction())?;
	for ring in rings {
		insert_ring(&tx, &ring.id, &ring.name)?;
		sql(tx.execute(
			"UPDATE rings SET shortcut = ?2, quick_shortcut = ?3, slot_count = ?4 WHERE id = ?1",
			params![
				ring.id,
				ring.shortcut,
				ring.quick_shortcut,
				ring.slot_count as i64
			],
		))?;
		for slot in &ring.slots {
			insert_slot(&tx, &ring.id, slot)?;
		}
	}
	sync_sub_ring_labels(&tx)?;
	sql(tx.commit())?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ring::model::OpenTarget;
	use crate::store::ensure_schema;

	fn conn() -> Connection {
		let c = Connection::open_in_memory().expect("in-memory db");
		ensure_schema(&c).expect("schema");
		c
	}

	fn url_slot(position: usize, label: &str) -> Slot {
		Slot {
			position,
			label: label.to_string(),
			icon: "globe".to_string(),
			action: RingAction::Open {
				target: OpenTarget::Url,
				value: "https://example.com".to_string(),
				args: String::new(),
			},
			confirm: false,
		}
	}

	fn link_slot(position: usize, target: &str) -> Slot {
		Slot {
			position,
			label: "하위".to_string(),
			icon: "circle-dot".to_string(),
			action: RingAction::OpenRing {
				ring_id: target.to_string(),
			},
			confirm: false,
		}
	}

	fn ring(c: &Connection, id: &str) -> Ring {
		create(c, id, id, &[]).expect("create")
	}

	fn sub_slot(position: usize, label: &str, target: &str) -> Slot {
		Slot {
			label: label.to_string(),
			..link_slot(position, target)
		}
	}

	fn label_at(c: &Connection, ring_id: &str, position: usize) -> String {
		get(c, ring_id)
			.unwrap()
			.unwrap()
			.slot(position)
			.unwrap()
			.label
			.clone()
	}

	#[test]
	fn naming_a_sub_ring_slot_renames_the_ring_and_every_slot_that_opens_it() {
		let c = conn();
		ring(&c, "a");
		ring(&c, "b");
		ring(&c, "sub");
		save_slot(&c, "a", &sub_slot(0, "sub", "sub")).unwrap();
		save_slot(&c, "b", &sub_slot(3, "sub", "sub")).unwrap();
		save_slot(&c, "a", &sub_slot(0, "  개발 도구 ", "sub")).unwrap();
		assert_eq!(get(&c, "sub").unwrap().unwrap().name, "개발 도구");
		assert_eq!(label_at(&c, "a", 0), "개발 도구");
		assert_eq!(label_at(&c, "b", 3), "개발 도구");
	}

	#[test]
	fn renaming_a_ring_renames_the_slots_that_open_it() {
		let c = conn();
		ring(&c, "a");
		ring(&c, "sub");
		save_slot(&c, "a", &url_slot(1, "sub")).unwrap();
		save_slot(&c, "a", &sub_slot(0, "sub", "sub")).unwrap();
		update(&c, "sub", "도구", 6).unwrap();
		assert_eq!(label_at(&c, "a", 0), "도구");
		// 다른 동작의 칸은 이름이 같아도 그대로다.
		assert_eq!(label_at(&c, "a", 1), "sub");
	}

	#[test]
	fn imported_sub_ring_slots_take_the_name_of_the_ring_they_open() {
		let c = conn();
		let sub = Ring {
			id: "sub".to_string(),
			name: "도구".to_string(),
			shortcut: None,
			quick_shortcut: None,
			slot_count: 6,
			slots: Vec::new(),
		};
		let top = Ring {
			id: "top".to_string(),
			name: "작업".to_string(),
			shortcut: None,
			quick_shortcut: None,
			slot_count: 6,
			slots: vec![sub_slot(0, "옛 이름", "sub")],
		};
		import(&c, &[sub, top]).unwrap();
		assert_eq!(label_at(&c, "top", 0), "도구");
	}

	#[test]
	fn a_new_database_has_no_rings() {
		assert!(list(&conn()).unwrap().is_empty());
	}

	#[test]
	fn create_makes_a_six_slot_ring_without_a_shortcut() {
		let c = conn();
		let made = create(&c, "a", "  작업  ", &[url_slot(0, "사이트")]).unwrap();
		assert_eq!(made.name, "작업");
		assert_eq!(made.slot_count, DEFAULT_SLOT_COUNT);
		assert_eq!(made.shortcut, None);
		assert_eq!(made.slots.len(), 1);
		assert_eq!(list(&c).unwrap(), vec![made]);
	}

	#[test]
	fn rings_list_in_creation_order() {
		let c = conn();
		ring(&c, "z");
		ring(&c, "a");
		let ids: Vec<String> = list(&c).unwrap().into_iter().map(|r| r.id).collect();
		assert_eq!(ids, ["z", "a"]);
	}

	#[test]
	fn save_slot_replaces_the_slot_at_that_position() {
		let c = conn();
		ring(&c, "a");
		save_slot(&c, "a", &url_slot(2, "처음")).unwrap();
		let after = save_slot(&c, "a", &url_slot(2, "다음")).unwrap();
		assert_eq!(after.slots.len(), 1);
		assert_eq!(after.slot(2).unwrap().label, "다음");
		assert_eq!(after.filled(), [false, false, true, false, false, false]);
	}

	#[test]
	fn save_slot_refuses_a_position_outside_the_ring() {
		let c = conn();
		ring(&c, "a");
		assert!(save_slot(&c, "a", &url_slot(6, "밖")).is_err());
	}

	#[test]
	fn save_slot_refuses_an_invalid_action() {
		let c = conn();
		ring(&c, "a");
		let mut bad = url_slot(0, "나쁜 주소");
		bad.action = RingAction::Open {
			target: OpenTarget::Url,
			value: "file:///etc/passwd".to_string(),
			args: String::new(),
		};
		assert!(save_slot(&c, "a", &bad).is_err());
		assert!(get(&c, "a").unwrap().unwrap().slots.is_empty());
	}

	#[test]
	fn shrinking_a_ring_drops_the_overflowing_slots() {
		let c = conn();
		ring(&c, "a");
		save_slot(&c, "a", &url_slot(1, "남는다")).unwrap();
		save_slot(&c, "a", &url_slot(5, "사라진다")).unwrap();
		let after = update(&c, "a", "작게", 4).unwrap();
		assert_eq!(after.slot_count, 4);
		assert_eq!(after.slots.len(), 1);
		assert_eq!(after.slots[0].label, "남는다");
	}

	#[test]
	fn update_refuses_a_slot_count_outside_four_to_eight() {
		let c = conn();
		ring(&c, "a");
		assert!(update(&c, "a", "a", 3).is_err());
		assert!(update(&c, "a", "a", 9).is_err());
		assert!(update(&c, "a", "a", 8).is_ok());
	}

	#[test]
	fn the_table_itself_refuses_a_bad_slot_count_and_position() {
		let c = conn();
		ring(&c, "a");
		assert!(
			c.execute("UPDATE rings SET slot_count = 9 WHERE id = 'a'", [])
				.is_err()
		);
		assert!(
			c.execute(
				"INSERT INTO ring_slots (ring_id, position, label, icon, action_kind, action_json)
				 VALUES ('a', 8, 'x', 'x', 'open', '{}')",
				[]
			)
			.is_err()
		);
	}

	#[test]
	fn clear_slot_empties_one_position() {
		let c = conn();
		ring(&c, "a");
		save_slot(&c, "a", &url_slot(0, "하나")).unwrap();
		save_slot(&c, "a", &url_slot(1, "둘")).unwrap();
		let after = clear_slot(&c, "a", 0).unwrap();
		assert_eq!(after.slots.len(), 1);
		assert_eq!(after.slots[0].position, 1);
		// 이미 빈 칸을 비워도 오류가 아니다.
		assert!(clear_slot(&c, "a", 0).is_ok());
	}

	#[test]
	fn reorder_moves_slots_and_empty_positions_together() {
		let c = conn();
		ring(&c, "a");
		save_slot(&c, "a", &url_slot(0, "가")).unwrap();
		save_slot(&c, "a", &url_slot(1, "나")).unwrap();
		// 0번을 맨 뒤로 보낸다.
		let after = reorder_slots(&c, "a", &[1, 2, 3, 4, 5, 0]).unwrap();
		assert_eq!(after.slot(0).unwrap().label, "나");
		assert_eq!(after.slot(5).unwrap().label, "가");
		assert_eq!(after.slots.len(), 2);
	}

	#[test]
	fn reorder_refuses_anything_but_a_permutation() {
		let c = conn();
		ring(&c, "a");
		assert!(reorder_slots(&c, "a", &[0, 1, 2]).is_err());
		assert!(reorder_slots(&c, "a", &[0, 0, 1, 2, 3, 4]).is_err());
		assert!(reorder_slots(&c, "a", &[0, 1, 2, 3, 4, 6]).is_err());
	}

	#[test]
	fn a_sub_ring_link_is_checked_for_cycles() {
		let c = conn();
		ring(&c, "a");
		ring(&c, "b");
		save_slot(&c, "a", &link_slot(0, "b")).unwrap();
		assert!(save_slot(&c, "b", &link_slot(0, "a")).is_err());
		assert!(save_slot(&c, "a", &link_slot(1, "a")).is_err());
	}

	#[test]
	fn a_sub_ring_link_is_checked_for_depth() {
		let c = conn();
		for id in ["a", "b", "c", "d"] {
			ring(&c, id);
		}
		save_slot(&c, "a", &link_slot(0, "b")).unwrap();
		save_slot(&c, "b", &link_slot(0, "c")).unwrap();
		assert!(save_slot(&c, "c", &link_slot(0, "d")).is_err());
	}

	#[test]
	fn re_saving_the_same_link_slot_is_allowed() {
		// 고치는 칸의 예전 연결을 빼지 않으면 a → b → c 의 b → c 칸을 다시 저장할 때 자기 연결 때문에 깊이를 넘는다.
		let c = conn();
		for id in ["a", "b", "c"] {
			ring(&c, id);
		}
		save_slot(&c, "a", &link_slot(0, "b")).unwrap();
		save_slot(&c, "b", &link_slot(0, "c")).unwrap();
		assert!(save_slot(&c, "b", &link_slot(0, "c")).is_ok());
	}

	#[test]
	fn link_candidates_mark_rings_that_would_loop_or_go_too_deep() {
		let c = conn();
		for id in ["a", "b", "c", "d"] {
			ring(&c, id);
		}
		save_slot(&c, "a", &link_slot(0, "b")).unwrap();
		save_slot(&c, "b", &link_slot(0, "c")).unwrap();
		// b 의 새 칸: a 는 순환, c 는 이미 여는 링이라 가능, d 는 가능 (a → b → d 는 3단).
		let from_b = link_candidates(&c, "b", 1).unwrap().candidates;
		let blocked = |id: &str| {
			from_b
				.iter()
				.find(|candidate| candidate.id == id)
				.map(|candidate| candidate.blocked.is_some())
		};
		assert_eq!(blocked("a"), Some(true));
		assert_eq!(blocked("c"), Some(false));
		assert_eq!(blocked("d"), Some(false));
		assert_eq!(blocked("b"), None, "the ring itself is not listed");
		// c 의 새 칸: d 는 4단이 된다.
		let from_c = link_candidates(&c, "c", 0).unwrap();
		assert!(
			from_c
				.candidates
				.iter()
				.any(|x| x.id == "d" && x.blocked == Some("link_too_deep"))
		);
		assert_eq!(from_c.create_blocked, Some("link_too_deep"));
		// b 의 조상 a 는 순환이라는 코드로 온다. b 에서는 새 하위 링을 만들 수 있다.
		let a = from_b.iter().find(|x| x.id == "a").unwrap();
		assert_eq!(a.blocked, Some("link_cycle"));
		assert_eq!(link_candidates(&c, "b", 1).unwrap().create_blocked, None);
	}

	#[test]
	fn link_candidates_count_the_filled_slots() {
		let c = conn();
		ring(&c, "a");
		ring(&c, "b");
		save_slot(&c, "b", &url_slot(0, "하나")).unwrap();
		save_slot(&c, "b", &url_slot(1, "둘")).unwrap();
		let choices = link_candidates(&c, "a", 0).unwrap();
		assert_eq!(choices.candidates.len(), 1);
		assert_eq!(choices.candidates[0].filled_slots, 2);
		assert_eq!(choices.candidates[0].blocked, None);
	}

	#[test]
	fn create_sub_ring_makes_an_empty_ring_and_links_the_slot() {
		let c = conn();
		ring(&c, "a");
		let (parent, sub) = create_sub_ring(&c, "new", "a", 2, "", "", "새 하위 링").unwrap();
		assert_eq!(sub.id, "new");
		assert_eq!(sub.name, "새 하위 링");
		assert_eq!(sub.shortcut, None);
		assert!(sub.slots.is_empty());
		let slot = parent.slot(2).unwrap();
		assert_eq!(slot.label, "새 하위 링");
		assert_eq!(slot.icon, SUB_RING_ICON);
		assert_eq!(
			slot.action,
			RingAction::OpenRing {
				ring_id: "new".to_string()
			}
		);
		assert!(!slot.confirm);
	}

	#[test]
	fn create_sub_ring_names_the_ring_after_the_slot_and_keeps_its_icon() {
		let c = conn();
		ring(&c, "a");
		let (parent, sub) =
			create_sub_ring(&c, "new", "a", 0, " 브라우저 ", "globe", "새 하위 링").unwrap();
		assert_eq!(sub.name, "브라우저");
		assert_eq!(parent.slot(0).unwrap().label, "브라우저");
		assert_eq!(parent.slot(0).unwrap().icon, "globe");
	}

	#[test]
	fn create_sub_ring_leaves_nothing_behind_when_it_is_refused() {
		let c = conn();
		for id in ["a", "b", "c"] {
			ring(&c, id);
		}
		save_slot(&c, "a", &link_slot(0, "b")).unwrap();
		save_slot(&c, "b", &link_slot(0, "c")).unwrap();
		// c 는 이미 3단째다. 그 아래에 링을 만들 수 없다.
		assert_eq!(
			create_sub_ring(&c, "new", "c", 0, "", "", "새 하위 링"),
			Err(RingError::Refused(Refusal::LinkTooDeep))
		);
		// 자리가 링 밖이어도, 링이 없어도 거절한다.
		assert_eq!(
			create_sub_ring(&c, "new", "a", 6, "", "", "새 하위 링"),
			Err(RingError::Refused(Refusal::SlotOutsideRing))
		);
		assert_eq!(
			create_sub_ring(&c, "new", "ghost", 0, "", "", "새 하위 링"),
			Err(RingError::Refused(Refusal::RingGone))
		);
		assert!(get(&c, "new").unwrap().is_none());
		assert_eq!(list(&c).unwrap().len(), 3);
		assert!(get(&c, "c").unwrap().unwrap().slots.is_empty());
	}

	#[test]
	fn a_sub_ring_slot_never_stores_the_confirm_flag() {
		let c = conn();
		ring(&c, "a");
		ring(&c, "b");
		let mut link = link_slot(0, "b");
		link.confirm = true;
		let after = save_slot(&c, "a", &link).unwrap();
		assert!(!after.slot(0).unwrap().confirm);
		// 다른 동작은 그 값을 그대로 둔다.
		let mut url = url_slot(1, "사이트");
		url.confirm = true;
		assert!(save_slot(&c, "a", &url).unwrap().slot(1).unwrap().confirm);
	}

	#[test]
	fn refusals_carry_their_code() {
		let c = conn();
		ring(&c, "a");
		ring(&c, "b");
		assert_eq!(
			save_slot(&c, "a", &link_slot(0, "")),
			Err(RingError::Refused(Refusal::RingNotChosen))
		);
		assert_eq!(
			save_slot(&c, "a", &link_slot(0, "ghost")),
			Err(RingError::Refused(Refusal::RingGone))
		);
		assert_eq!(
			save_slot(&c, "a", &link_slot(0, "a")),
			Err(RingError::Refused(Refusal::LinkSelf))
		);
		assert_eq!(
			update(&c, "a", "  ", 6),
			Err(RingError::Refused(Refusal::RingNameMissing))
		);
		assert_eq!(
			reorder_slots(&c, "a", &[0, 1]),
			Err(RingError::Refused(Refusal::OrderNotPermutation))
		);
	}

	#[test]
	fn a_link_to_a_missing_ring_is_refused() {
		let c = conn();
		ring(&c, "a");
		assert!(save_slot(&c, "a", &link_slot(0, "ghost")).is_err());
	}

	#[test]
	fn deleting_a_referenced_ring_is_refused_with_the_referring_slots() {
		let c = conn();
		ring(&c, "a");
		ring(&c, "b");
		save_slot(&c, "a", &link_slot(3, "b")).unwrap();
		let refs = delete(&c, "b").unwrap();
		assert_eq!(
			refs,
			vec![SlotRef {
				ring_id: "a".to_string(),
				ring_name: "a".to_string(),
				position: 3,
				label: "하위".to_string(),
			}]
		);
		assert!(get(&c, "b").unwrap().is_some());
	}

	#[test]
	fn deleting_a_ring_removes_its_slots() {
		let c = conn();
		ring(&c, "a");
		save_slot(&c, "a", &url_slot(0, "하나")).unwrap();
		assert!(delete(&c, "a").unwrap().is_empty());
		assert!(get(&c, "a").unwrap().is_none());
		let left: i64 = c
			.query_row("SELECT COUNT(*) FROM ring_slots", [], |row| row.get(0))
			.unwrap();
		assert_eq!(left, 0);
	}

	#[test]
	fn set_shortcut_stores_and_clears_the_combination() {
		let c = conn();
		ring(&c, "a");
		set_shortcut(&c, "a", ShortcutKind::Normal, Some("Alt+Space")).unwrap();
		assert_eq!(
			get(&c, "a").unwrap().unwrap().shortcut.as_deref(),
			Some("Alt+Space")
		);
		set_shortcut(&c, "a", ShortcutKind::Normal, None).unwrap();
		assert_eq!(get(&c, "a").unwrap().unwrap().shortcut, None);
		assert!(set_shortcut(&c, "ghost", ShortcutKind::Normal, Some("Alt+Space")).is_err());
	}

	#[test]
	fn the_quick_shortcut_is_stored_apart_from_the_normal_one() {
		let c = conn();
		ring(&c, "a");
		set_shortcut(&c, "a", ShortcutKind::Normal, Some("Alt+Space")).unwrap();
		set_shortcut(&c, "a", ShortcutKind::Quick, Some("F5")).unwrap();
		let saved = get(&c, "a").unwrap().unwrap();
		assert_eq!(saved.shortcut.as_deref(), Some("Alt+Space"));
		assert_eq!(saved.quick_shortcut.as_deref(), Some("F5"));
		set_shortcut(&c, "a", ShortcutKind::Quick, None).unwrap();
		let saved = get(&c, "a").unwrap().unwrap();
		assert_eq!(saved.shortcut.as_deref(), Some("Alt+Space"));
		assert_eq!(saved.quick_shortcut, None);
		assert_eq!(list(&c).unwrap()[0].quick_shortcut, None);
	}

	#[test]
	fn two_rings_cannot_store_the_same_shortcut_text() {
		let c = conn();
		ring(&c, "a");
		ring(&c, "b");
		set_shortcut(&c, "a", ShortcutKind::Normal, Some("Alt+Space")).unwrap();
		assert!(set_shortcut(&c, "b", ShortcutKind::Normal, Some("Alt+Space")).is_err());
	}

	#[test]
	fn import_adds_rings_with_their_shortcut_slot_count_and_slots() {
		let c = conn();
		ring(&c, "a");
		let incoming = Ring {
			id: "new".to_string(),
			name: "가져온 링".to_string(),
			shortcut: Some("Alt+KeyR".to_string()),
			quick_shortcut: Some("F5".to_string()),
			slot_count: 8,
			slots: vec![url_slot(7, "끝")],
		};
		import(&c, std::slice::from_ref(&incoming)).unwrap();
		assert_eq!(get(&c, "new").unwrap().unwrap(), incoming);
		assert_eq!(list(&c).unwrap().len(), 2);
	}

	#[test]
	fn import_leaves_nothing_when_one_ring_fails() {
		let c = conn();
		set_shortcut(
			&c,
			&ring(&c, "a").id,
			ShortcutKind::Normal,
			Some("Alt+Space"),
		)
		.unwrap();
		let ok = Ring {
			id: "x".to_string(),
			name: "x".to_string(),
			shortcut: None,
			quick_shortcut: None,
			slot_count: 6,
			slots: Vec::new(),
		};
		let clash = Ring {
			id: "y".to_string(),
			name: "y".to_string(),
			shortcut: Some("Alt+Space".to_string()),
			quick_shortcut: None,
			slot_count: 6,
			slots: Vec::new(),
		};
		assert!(import(&c, &[ok, clash]).is_err());
		assert_eq!(list(&c).unwrap().len(), 1);
	}

	#[test]
	fn an_unreadable_action_reads_as_an_empty_slot() {
		let c = conn();
		ring(&c, "a");
		c.execute(
			"INSERT INTO ring_slots (ring_id, position, label, icon, action_kind, action_json)
			 VALUES ('a', 0, 'x', 'x', 'future_kind', '{\"kind\":\"future_kind\"}')",
			[],
		)
		.unwrap();
		assert!(get(&c, "a").unwrap().unwrap().slots.is_empty());
	}
}
