// 링의 내보내기와 가져오기 — JSON 파일 하나. 가져온 것을 그대로 믿지 않고 여기서 다시 검증한다
//
// 가져오기는 링을 더하기만 한다. 있는 링을 바꾸거나 지우지 않는다. 밖에서 온 파일이 명령을 곧바로
// 돌리지 못하게, 셸 명령과 키 입력 칸은 "실행 전에 확인" 을 켠 채로 들인다.
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use tauri_plugin_global_shortcut::Shortcut;

use super::model::{
	Named, OpenTarget, Refusal, Ring, RingAction, ShortcutKind, Slot, check_link, check_name,
	check_slot_count,
};
use crate::shortcuts::parse_for;

/// 파일의 `format` 칸. 다른 값이면 이 앱의 파일이 아니다.
pub const FORMAT: &str = "ringring.rings";
/// 이 빌드가 쓰고 읽는 파일 버전.
pub const FORMAT_VERSION: u32 = 1;
/// 읽는 파일의 최대 크기.
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;
/// 한 파일이 담는 링의 최대 수.
pub const MAX_RINGS: usize = 64;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportFile<'a> {
	format: &'static str,
	version: u32,
	exported_at: &'a str,
	app_version: &'a str,
	rings: &'a [Ring],
}

/// 링 전체를 파일의 글로 만든다.
pub fn export_text(rings: &[Ring], exported_at: &str, app_version: &str) -> Result<String, String> {
	serde_json::to_string_pretty(&ExportFile {
		format: FORMAT,
		version: FORMAT_VERSION,
		exported_at,
		app_version,
		rings,
	})
	.map_err(|e| format!("Failed to write the export: {e}"))
}

#[derive(Deserialize)]
struct ImportFile {
	format: String,
	version: u32,
	rings: Vec<ImportRing>,
}

/// 칸은 하나씩 따로 읽는다. 읽지 못하는 칸 하나 때문에 파일 전체를 버리지 않는다.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportRing {
	id: String,
	name: String,
	#[serde(default)]
	shortcut: Option<String>,
	/// 빠른 단축키. 이 칸이 생기기 전의 파일에는 없다.
	#[serde(default)]
	quick_shortcut: Option<String>,
	slot_count: usize,
	#[serde(default)]
	slots: Vec<serde_json::Value>,
}

/// 가져오기가 무엇을 들이는지. 가져오기 전에 사용자에게 보이고, 가져온 뒤에도 같은 모양으로 돌려준다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
	pub rings: usize,
	pub slots: usize,
	/// 셸 명령 칸. "실행 전에 확인" 이 켜진 채로 들어온다.
	pub shell_slots: usize,
	/// 키 입력 칸. "실행 전에 확인" 이 켜진 채로 들어온다.
	pub keystroke_slots: usize,
	/// 파일·폴더나 앱을 여는 칸. 이것도 "실행 전에 확인" 이 켜진 채로 들어온다. 웹 주소를 여는 칸은 세지 않는다.
	pub open_slots: usize,
	/// 읽지 못했거나 검증을 통과하지 못해 버린 칸.
	pub skipped_slots: usize,
	/// 이미 쓰는 조합이거나 쓸 수 없는 조합이라 뗀 단축키.
	pub dropped_shortcuts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPlan {
	/// 새 id 를 받은 링들. 이대로 저장한다.
	pub rings: Vec<Ring>,
	pub summary: ImportSummary,
}

fn same_combo(a: &Shortcut, b: &Shortcut) -> bool {
	a.mods == b.mods && a.key == b.key
}

/// 파일의 글을 읽어 무엇을 들일지 정한다. 아무것도 저장하지 않는다.
///
/// `existing` 은 지금 저장된 링이다 — 단축키가 겹치는지 본다. `new_id` 는 링마다 새 id 를 준다.
/// 파일 안의 id 는 칸의 하위 링 연결을 잇는 데만 쓴다.
pub fn plan(
	text: &str,
	existing: &[Ring],
	mut new_id: impl FnMut() -> String,
) -> Result<ImportPlan, Refusal> {
	let file: ImportFile = serde_json::from_str(text).map_err(|_| Refusal::ImportUnreadable)?;
	if file.format != FORMAT {
		return Err(Refusal::ImportUnreadable);
	}
	if file.version > FORMAT_VERSION {
		return Err(Refusal::ImportVersionUnsupported);
	}
	if file.rings.is_empty() {
		return Err(Refusal::ImportEmpty);
	}
	if file.rings.len() > MAX_RINGS {
		return Err(Refusal::ImportTooLarge);
	}

	let mut ids: HashMap<&str, String> = HashMap::new();
	for ring in &file.rings {
		check_name(&ring.name, Named::Ring)?;
		check_slot_count(ring.slot_count)?;
		if ids.insert(ring.id.as_str(), new_id()).is_some() {
			// 같은 id 가 두 번 나오면 칸의 연결이 어느 링을 가리키는지 알 수 없다.
			return Err(Refusal::ImportUnreadable);
		}
	}

	// 일반·빠른 단축키는 같은 조합을 다툰다. 하나의 목록으로 본다.
	let mut taken: Vec<Shortcut> = existing
		.iter()
		.flat_map(|ring| [ring.shortcut.as_deref(), ring.quick_shortcut.as_deref()])
		.filter_map(|combo| combo?.parse().ok())
		.collect();
	let mut summary = ImportSummary::default();
	let mut edges: Vec<(String, String)> = Vec::new();
	let mut rings = Vec::with_capacity(file.rings.len());

	for incoming in &file.rings {
		let id = ids[incoming.id.as_str()].clone();
		let mut accept = |kind: ShortcutKind, combo: Option<&str>| -> Option<String> {
			let combo = combo?;
			let kept = parse_for(kind, combo)
				.ok()
				.filter(|parsed| !taken.iter().any(|used| same_combo(used, parsed)))
				.map(|parsed| {
					taken.push(parsed);
					combo.to_string()
				});
			if kept.is_none() {
				summary.dropped_shortcuts += 1;
			}
			kept
		};
		let shortcut = accept(ShortcutKind::Normal, incoming.shortcut.as_deref());
		let quick_shortcut = accept(ShortcutKind::Quick, incoming.quick_shortcut.as_deref());

		let mut used_positions = HashSet::new();
		let mut slots = Vec::new();
		for raw in &incoming.slots {
			let Some(slot) = readable_slot(raw, incoming.slot_count, &ids) else {
				summary.skipped_slots += 1;
				continue;
			};
			if !used_positions.insert(slot.position) {
				summary.skipped_slots += 1;
				continue;
			}
			if let RingAction::OpenRing { ring_id: target } = &slot.action {
				if check_link(&edges, &id, target).is_err() {
					summary.skipped_slots += 1;
					continue;
				}
				edges.push((id.clone(), target.clone()));
			}
			match slot.action {
				RingAction::Shell { .. } => summary.shell_slots += 1,
				RingAction::Keystroke { .. } => summary.keystroke_slots += 1,
				RingAction::Open { target, .. } if target != OpenTarget::Url => {
					summary.open_slots += 1;
				}
				_ => {}
			}
			slots.push(slot);
		}
		slots.sort_by_key(|slot| slot.position);
		summary.slots += slots.len();
		rings.push(Ring {
			id,
			name: incoming.name.trim().to_string(),
			shortcut,
			quick_shortcut,
			slot_count: incoming.slot_count,
			slots,
		});
	}
	summary.rings = rings.len();
	Ok(ImportPlan { rings, summary })
}

/// 등록에 실패한 조합 가운데 이번에 가져온 링의 것만 고른다. 원래 있던 링은 가져오기가 건드리지 않는다.
pub fn imported_among(
	failed: &[(String, ShortcutKind)],
	imported: &[String],
) -> Vec<(String, ShortcutKind)> {
	failed
		.iter()
		.filter(|(id, _)| imported.contains(id))
		.cloned()
		.collect()
}

/// 파일의 칸 하나를 읽어 이 앱의 칸으로 바꾼다. 읽지 못하거나 검증에 걸리면 `None`.
fn readable_slot(
	raw: &serde_json::Value,
	slot_count: usize,
	ids: &HashMap<&str, String>,
) -> Option<Slot> {
	let mut slot: Slot = serde_json::from_value(raw.clone()).ok()?;
	match &mut slot.action {
		// 파일 안의 다른 링만 가리킬 수 있다. 이 기기의 링 id 는 파일이 알 수 없다.
		RingAction::OpenRing { ring_id } => {
			*ring_id = ids.get(ring_id.as_str())?.clone();
			slot.confirm = false;
		}
		RingAction::Shell { .. } | RingAction::Keystroke { .. } => slot.confirm = true,
		// 파일이나 앱을 여는 것도 실행이다 — 스크립트나 앱이 돈다. 웹 주소는 http/https 만 통과하므로 그대로 둔다.
		RingAction::Open { target, .. } => {
			if *target != OpenTarget::Url {
				slot.confirm = true;
			}
		}
	}
	slot.validate(slot_count).ok()?;
	Some(slot)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ring::model::OpenTarget;

	fn ids() -> impl FnMut() -> String {
		let mut next = 0;
		move || {
			next += 1;
			format!("new-{next}")
		}
	}

	fn file(rings: serde_json::Value) -> String {
		serde_json::json!({ "format": FORMAT, "version": FORMAT_VERSION, "rings": rings })
			.to_string()
	}

	fn url_slot(position: usize) -> serde_json::Value {
		serde_json::json!({
			"position": position, "label": "사이트", "icon": "globe", "confirm": false,
			"action": { "kind": "open", "target": "url", "value": "https://example.com" }
		})
	}

	fn existing(combo: &str) -> Ring {
		Ring {
			id: "here".to_string(),
			name: "있던 링".to_string(),
			shortcut: Some(combo.to_string()),
			quick_shortcut: None,
			slot_count: 6,
			slots: Vec::new(),
		}
	}

	#[test]
	fn an_export_reads_back_as_the_same_rings_with_new_ids() {
		let ring = Ring {
			id: "old".to_string(),
			name: "작업".to_string(),
			shortcut: Some("Alt+Space".to_string()),
			quick_shortcut: Some("Ctrl+F5".to_string()),
			slot_count: 4,
			slots: vec![Slot {
				position: 2,
				label: "사이트".to_string(),
				icon: "globe".to_string(),
				action: RingAction::Open {
					target: OpenTarget::Url,
					value: "https://example.com".to_string(),
					args: String::new(),
				},
				confirm: false,
			}],
		};
		let text =
			export_text(std::slice::from_ref(&ring), "2026-10-06T00:00:00Z", "0.1.0").unwrap();
		let planned = plan(&text, &[], ids()).unwrap();
		assert_eq!(
			planned.rings,
			vec![Ring {
				id: "new-1".to_string(),
				..ring
			}]
		);
		assert_eq!(planned.summary.rings, 1);
		assert_eq!(planned.summary.slots, 1);
		assert_eq!(planned.summary.skipped_slots, 0);
	}

	#[test]
	fn a_file_that_is_not_ours_is_unreadable() {
		assert_eq!(plan("not json", &[], ids()), Err(Refusal::ImportUnreadable));
		assert_eq!(plan("{}", &[], ids()), Err(Refusal::ImportUnreadable));
		let other = serde_json::json!({ "format": "something.else", "version": 1, "rings": [] });
		assert_eq!(
			plan(&other.to_string(), &[], ids()),
			Err(Refusal::ImportUnreadable)
		);
	}

	#[test]
	fn a_newer_file_an_empty_file_and_a_huge_file_are_refused() {
		let newer =
			serde_json::json!({ "format": FORMAT, "version": FORMAT_VERSION + 1, "rings": [] });
		assert_eq!(
			plan(&newer.to_string(), &[], ids()),
			Err(Refusal::ImportVersionUnsupported)
		);
		assert_eq!(
			plan(&file(serde_json::json!([])), &[], ids()),
			Err(Refusal::ImportEmpty)
		);
		let many: Vec<_> = (0..=MAX_RINGS)
			.map(|i| serde_json::json!({ "id": i.to_string(), "name": "r", "slotCount": 6 }))
			.collect();
		assert_eq!(
			plan(&file(serde_json::json!(many)), &[], ids()),
			Err(Refusal::ImportTooLarge)
		);
	}

	#[test]
	fn a_bad_ring_name_or_slot_count_refuses_the_file() {
		let unnamed = file(serde_json::json!([{ "id": "a", "name": " ", "slotCount": 6 }]));
		assert_eq!(plan(&unnamed, &[], ids()), Err(Refusal::RingNameMissing));
		let wide = file(serde_json::json!([{ "id": "a", "name": "a", "slotCount": 9 }]));
		assert_eq!(plan(&wide, &[], ids()), Err(Refusal::SlotCountOutOfRange));
		let twice = file(serde_json::json!([
			{ "id": "a", "name": "a", "slotCount": 6 },
			{ "id": "a", "name": "b", "slotCount": 6 }
		]));
		assert_eq!(plan(&twice, &[], ids()), Err(Refusal::ImportUnreadable));
	}

	#[test]
	fn shell_and_keystroke_slots_always_arrive_asking_first() {
		let text = file(
			serde_json::json!([{ "id": "a", "name": "a", "slotCount": 6, "slots": [
				{ "position": 0, "label": "배포", "icon": "rocket", "confirm": false,
				  "action": { "kind": "shell", "command": "make deploy", "working_dir": "", "timeout_secs": 60 } },
				{ "position": 1, "label": "복사", "icon": "copy", "confirm": false,
				  "action": { "kind": "keystroke", "combos": ["Cmd+KeyC"] } },
				url_slot(2)
			]}]),
		);
		let planned = plan(&text, &[], ids()).unwrap();
		let slots = &planned.rings[0].slots;
		assert!(slots[0].confirm, "shell");
		assert!(slots[1].confirm, "keystroke");
		assert!(!slots[2].confirm, "a web address keeps what the file says");
		assert_eq!(planned.summary.shell_slots, 1);
		assert_eq!(planned.summary.keystroke_slots, 1);
	}

	#[test]
	fn open_file_and_open_app_slots_arrive_asking_first() {
		let text = file(
			serde_json::json!([{ "id": "a", "name": "a", "slotCount": 6, "slots": [
				{ "position": 0, "label": "스크립트", "icon": "file", "confirm": false,
				  "action": { "kind": "open", "target": "file", "value": "/tmp/run.command" } },
				{ "position": 1, "label": "앱", "icon": "app-window", "confirm": false,
				  "action": { "kind": "open", "target": "app", "value": "/Applications/Some.app" } },
				url_slot(2)
			]}]),
		);
		let slots = plan(&text, &[], ids()).unwrap().rings.remove(0).slots;
		assert!(slots[0].confirm, "file");
		assert!(slots[1].confirm, "app");
		assert!(!slots[2].confirm, "url");
		assert_eq!(plan(&text, &[], ids()).unwrap().summary.open_slots, 2);
	}

	#[test]
	fn only_imported_rings_lose_a_refused_shortcut() {
		let failed = [
			("old".to_string(), ShortcutKind::Normal),
			("new-2".to_string(), ShortcutKind::Quick),
		];
		let imported = ["new-1".to_string(), "new-2".to_string()];
		assert_eq!(
			imported_among(&failed, &imported),
			[("new-2".to_string(), ShortcutKind::Quick)]
		);
		assert!(imported_among(&failed[..1], &imported).is_empty());
	}

	#[test]
	fn unreadable_invalid_and_duplicate_slots_are_skipped_not_fatal() {
		let text = file(
			serde_json::json!([{ "id": "a", "name": "a", "slotCount": 4, "slots": [
				url_slot(0),
				url_slot(0),
				url_slot(4),
				{ "position": 1, "label": "기능", "icon": "layers", "confirm": false,
				  "action": { "kind": "tome_feature", "feature_id": "quick_memo" } },
				{ "position": 2, "label": "나쁜 주소", "icon": "globe", "confirm": false,
				  "action": { "kind": "open", "target": "url", "value": "file:///etc/passwd" } },
				"not a slot"
			]}]),
		);
		let planned = plan(&text, &[], ids()).unwrap();
		assert_eq!(planned.rings[0].slots.len(), 1);
		assert_eq!(planned.summary.slots, 1);
		assert_eq!(planned.summary.skipped_slots, 5);
	}

	#[test]
	fn sub_ring_links_are_rewired_to_the_new_ids() {
		let text = file(serde_json::json!([
			{ "id": "top", "name": "위", "slotCount": 6, "slots": [
				{ "position": 0, "label": "아래", "icon": "circle-dot", "confirm": true,
				  "action": { "kind": "open_ring", "ring_id": "sub" } }
			]},
			{ "id": "sub", "name": "아래", "slotCount": 6, "slots": [url_slot(0)] }
		]));
		let planned = plan(&text, &[], ids()).unwrap();
		let link = &planned.rings[0].slots[0];
		assert_eq!(
			link.action,
			RingAction::OpenRing {
				ring_id: planned.rings[1].id.clone()
			}
		);
		assert!(!link.confirm, "a sub-ring slot never asks");
	}

	#[test]
	fn links_out_of_the_file_cycles_and_deep_chains_are_skipped() {
		let link = |position: usize, target: &str| {
			serde_json::json!({
				"position": position, "label": "하위", "icon": "circle-dot", "confirm": false,
				"action": { "kind": "open_ring", "ring_id": target }
			})
		};
		let text = file(serde_json::json!([
			{ "id": "a", "name": "a", "slotCount": 6, "slots": [link(0, "b"), link(1, "here"), link(2, "a")] },
			{ "id": "b", "name": "b", "slotCount": 6, "slots": [link(0, "c"), link(1, "a")] },
			{ "id": "c", "name": "c", "slotCount": 6, "slots": [link(0, "d")] },
			{ "id": "d", "name": "d", "slotCount": 6 }
		]));
		let planned = plan(&text, &[existing("Alt+Space")], ids()).unwrap();
		// a: b 만 남는다 (파일 밖의 링과 자기 자신은 버린다). b: c 만 남는다 (a 는 순환). c: d 는 4단이라 버린다.
		assert_eq!(planned.rings[0].slots.len(), 1);
		assert_eq!(planned.rings[1].slots.len(), 1);
		assert!(planned.rings[2].slots.is_empty());
		assert_eq!(planned.summary.skipped_slots, 4);
	}

	#[test]
	fn a_shortcut_already_in_use_is_dropped_and_counted() {
		let text = file(serde_json::json!([
			{ "id": "a", "name": "a", "slotCount": 6, "shortcut": "Option+Space" },
			{ "id": "b", "name": "b", "slotCount": 6, "shortcut": "Alt+KeyR" },
			{ "id": "c", "name": "c", "slotCount": 6, "shortcut": "Alt+KeyR" },
			{ "id": "d", "name": "d", "slotCount": 6, "shortcut": "KeyA" },
			{ "id": "e", "name": "e", "slotCount": 6 }
		]));
		let planned = plan(&text, &[existing("Alt+Space")], ids()).unwrap();
		let shortcuts: Vec<Option<&str>> = planned
			.rings
			.iter()
			.map(|ring| ring.shortcut.as_deref())
			.collect();
		assert_eq!(shortcuts, [None, Some("Alt+KeyR"), None, None, None]);
		assert_eq!(planned.summary.dropped_shortcuts, 3);
	}

	#[test]
	fn a_quick_shortcut_is_imported_and_shares_the_taken_combos_with_normal_ones() {
		let text = file(serde_json::json!([
			{ "id": "a", "name": "a", "slotCount": 6, "quickShortcut": "F5" },
			// 있던 링의 일반 단축키와 같다.
			{ "id": "b", "name": "b", "slotCount": 6, "quickShortcut": "Ctrl+F6" },
			// 파일 안의 다른 링이 먼저 가져갔다.
			{ "id": "c", "name": "c", "slotCount": 6, "shortcut": "Alt+KeyR", "quickShortcut": "F5" },
			// F 키가 아니다.
			{ "id": "d", "name": "d", "slotCount": 6, "quickShortcut": "Alt+KeyQ" },
			// 같은 링의 일반 단축키와 같다.
			{ "id": "e", "name": "e", "slotCount": 6, "shortcut": "Alt+F7", "quickShortcut": "Alt+F7" }
		]));
		let planned = plan(&text, &[existing("Ctrl+F6")], ids()).unwrap();
		let quick: Vec<Option<&str>> = planned
			.rings
			.iter()
			.map(|ring| ring.quick_shortcut.as_deref())
			.collect();
		assert_eq!(quick, [Some("F5"), None, None, None, None]);
		assert_eq!(planned.rings[2].shortcut.as_deref(), Some("Alt+KeyR"));
		assert_eq!(planned.rings[4].shortcut.as_deref(), Some("Alt+F7"));
		assert_eq!(planned.summary.dropped_shortcuts, 4);
	}

	#[test]
	fn a_quick_shortcut_of_an_existing_ring_is_taken() {
		let mut here = existing("Alt+Space");
		here.quick_shortcut = Some("F9".to_string());
		let text = file(serde_json::json!([
			{ "id": "a", "name": "a", "slotCount": 6, "shortcut": "Alt+F9", "quickShortcut": "F9" }
		]));
		let planned = plan(&text, &[here], ids()).unwrap();
		assert_eq!(planned.rings[0].quick_shortcut, None);
		assert_eq!(planned.rings[0].shortcut.as_deref(), Some("Alt+F9"));
		assert_eq!(planned.summary.dropped_shortcuts, 1);
	}
}
