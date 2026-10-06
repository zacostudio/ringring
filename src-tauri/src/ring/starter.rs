// 기본 링 — 처음 쓰는 사람이 고를 수 있는 링 하나. 이 기기에 있는 것만 칸으로 넣는다
use std::path::Path;

use super::model::{OpenTarget, RingAction, Slot};
use crate::settings::Locale;
use crate::ui::texts;

struct Candidate {
	/// `texts::starter_slot_name` 의 key.
	key: &'static str,
	icon: &'static str,
	target: OpenTarget,
	path: &'static str,
}

const CANDIDATES: [Candidate; 5] = [
	Candidate {
		key: "finder",
		icon: "folder",
		target: OpenTarget::App,
		path: "/System/Library/CoreServices/Finder.app",
	},
	Candidate {
		key: "safari",
		icon: "compass",
		target: OpenTarget::App,
		path: "/Applications/Safari.app",
	},
	Candidate {
		key: "terminal",
		icon: "square-terminal",
		target: OpenTarget::App,
		path: "/System/Applications/Utilities/Terminal.app",
	},
	Candidate {
		key: "system_settings",
		icon: "settings",
		target: OpenTarget::App,
		path: "/System/Applications/System Settings.app",
	},
	Candidate {
		key: "downloads",
		icon: "download",
		target: OpenTarget::File,
		path: "~/Downloads",
	},
];

/// 기본 링의 칸들. `exists` 가 참인 대상만 넣고, 자리는 0 부터 빈틈없이 채운다.
/// 셸 명령이나 키 입력은 넣지 않는다 — 모두 "열기" 다.
pub fn slots(locale: Locale, exists: impl Fn(&Path) -> bool) -> Vec<Slot> {
	CANDIDATES
		.iter()
		.filter(|candidate| exists(&crate::exec::shell_command::working_folder(candidate.path)))
		.enumerate()
		.map(|(position, candidate)| Slot {
			position,
			label: texts::starter_slot_name(candidate.key, locale).to_string(),
			icon: candidate.icon.to_string(),
			action: RingAction::Open {
				target: candidate.target,
				value: candidate.path.to_string(),
			},
			confirm: false,
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::store::rings::DEFAULT_SLOT_COUNT;

	#[test]
	fn every_candidate_fits_a_new_ring_and_validates() {
		let all = slots(Locale::Ko, |_| true);
		assert_eq!(all.len(), CANDIDATES.len());
		assert!(all.len() <= DEFAULT_SLOT_COUNT);
		for slot in &all {
			assert!(slot.validate(DEFAULT_SLOT_COUNT).is_ok(), "{}", slot.label);
			assert!(matches!(slot.action, RingAction::Open { .. }));
			assert!(!slot.confirm);
		}
	}

	#[test]
	fn missing_targets_are_left_out_and_positions_stay_packed() {
		let some = slots(Locale::En, |path| {
			!path.to_string_lossy().contains("Safari")
		});
		assert_eq!(some.len(), CANDIDATES.len() - 1);
		let positions: Vec<usize> = some.iter().map(|slot| slot.position).collect();
		assert_eq!(positions, (0..some.len()).collect::<Vec<_>>());
		assert!(some.iter().all(|slot| slot.label != "Safari"));
	}

	#[test]
	fn nothing_on_this_machine_gives_an_empty_ring() {
		assert!(slots(Locale::Ja, |_| false).is_empty());
	}
}
