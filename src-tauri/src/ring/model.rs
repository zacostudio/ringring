// 링의 데이터 모양과 저장 전 검증 — 링, 칸, 칸의 동작, 하위 링의 순환·깊이
//
// 검증은 모두 여기서 한다. 프런트는 보지 않는다.

use serde::{Deserialize, Serialize};

use super::geometry::{MAX_SLOTS, MIN_SLOTS};

/// 저장이나 실행을 거절한 이유. 화면은 [`Refusal::code`] 로 자기 언어의 문장을 고른다.
///
/// 영어 문장([`std::fmt::Display`])은 로그에만 간다. 화면에 그대로 보이지 않는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
	OpenValueMissing,
	UrlNotHttp,
	CommandMissing,
	TimeoutOutOfRange,
	CombosMissing,
	CombosTooMany,
	ComboInvalid,
	RingNotChosen,
	SlotOutsideRing,
	RingNameMissing,
	RingNameTooLong,
	SlotNameMissing,
	SlotNameTooLong,
	IconMissing,
	SlotCountOutOfRange,
	LinkSelf,
	LinkCycle,
	LinkTooDeep,
	RingGone,
	RingEmpty,
	OrderNotPermutation,
	ShortcutInvalid,
	ShortcutNeedsModifier,
	ShortcutNeedsFunctionKey,
	ShortcutTaken,
	ShortcutUnavailable,
	ImportUnreadable,
	ImportVersionUnsupported,
	ImportTooLarge,
	ImportEmpty,
}

impl Refusal {
	/// 화면이 문장을 고를 때 쓰는 값. 프런트의 `entities/Ring/lib/refusal.ts` 가 읽는다.
	pub fn code(self) -> &'static str {
		match self {
			Self::OpenValueMissing => "open_value_missing",
			Self::UrlNotHttp => "url_not_http",
			Self::CommandMissing => "command_missing",
			Self::TimeoutOutOfRange => "timeout_out_of_range",
			Self::CombosMissing => "combos_missing",
			Self::CombosTooMany => "combos_too_many",
			Self::ComboInvalid => "combo_invalid",
			Self::RingNotChosen => "ring_not_chosen",
			Self::SlotOutsideRing => "slot_outside_ring",
			Self::RingNameMissing => "ring_name_missing",
			Self::RingNameTooLong => "ring_name_too_long",
			Self::SlotNameMissing => "slot_name_missing",
			Self::SlotNameTooLong => "slot_name_too_long",
			Self::IconMissing => "icon_missing",
			Self::SlotCountOutOfRange => "slot_count_out_of_range",
			Self::LinkSelf => "link_self",
			Self::LinkCycle => "link_cycle",
			Self::LinkTooDeep => "link_too_deep",
			Self::RingGone => "ring_gone",
			Self::RingEmpty => "ring_empty",
			Self::OrderNotPermutation => "order_not_permutation",
			Self::ShortcutInvalid => "shortcut_invalid",
			Self::ShortcutNeedsModifier => "shortcut_needs_modifier",
			Self::ShortcutNeedsFunctionKey => "shortcut_needs_function_key",
			Self::ShortcutTaken => "shortcut_taken",
			Self::ShortcutUnavailable => "shortcut_unavailable",
			Self::ImportUnreadable => "import_unreadable",
			Self::ImportVersionUnsupported => "import_version_unsupported",
			Self::ImportTooLarge => "import_too_large",
			Self::ImportEmpty => "import_empty",
		}
	}

	/// 값이 틀린 것이 아니라 아직 채우지 않은 것인가.
	///
	/// 설정 화면은 고칠 때마다 저장한다. 그래서 칸을 채우는 도중에도 저장을 시도한다. 그때의 거절은
	/// 오류가 아니다 — 화면은 빨간 글 대신 무엇이 남았는지를 한 줄로 알린다.
	pub fn is_incomplete(self) -> bool {
		matches!(
			self,
			Self::OpenValueMissing
				| Self::CommandMissing
				| Self::CombosMissing
				| Self::RingNotChosen
				| Self::SlotNameMissing
				| Self::IconMissing
		)
	}
}

impl std::fmt::Display for Refusal {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::OpenValueMissing => write!(f, "Enter what to open"),
			Self::UrlNotHttp => write!(f, "The address must start with http:// or https://"),
			Self::CommandMissing => write!(f, "Enter a command"),
			Self::TimeoutOutOfRange => write!(
				f,
				"The time limit must be between 1 and {MAX_SHELL_TIMEOUT_SECS} seconds"
			),
			Self::CombosMissing => write!(f, "Add at least one key combination"),
			Self::CombosTooMany => write!(f, "At most {MAX_COMBOS} key combinations"),
			Self::ComboInvalid => write!(f, "Invalid key combination"),
			Self::RingNotChosen => write!(f, "Choose a ring"),
			Self::SlotOutsideRing => write!(f, "That slot is outside this ring"),
			Self::RingNameMissing => write!(f, "Enter a ring name"),
			Self::RingNameTooLong => {
				write!(f, "The ring name is at most {NAME_MAX_CHARS} characters")
			}
			Self::SlotNameMissing => write!(f, "Enter a slot name"),
			Self::SlotNameTooLong => {
				write!(f, "The slot name is at most {NAME_MAX_CHARS} characters")
			}
			Self::IconMissing => write!(f, "Choose an icon"),
			Self::SlotCountOutOfRange => write!(f, "A ring has {MIN_SLOTS} to {MAX_SLOTS} slots"),
			Self::LinkSelf => write!(f, "A ring cannot open itself"),
			Self::LinkCycle => write!(f, "That ring already leads back to this one"),
			Self::LinkTooDeep => write!(f, "Sub-rings go at most {MAX_DEPTH} levels deep"),
			Self::RingGone => write!(f, "That ring no longer exists"),
			Self::RingEmpty => write!(f, "That ring has no slots yet"),
			Self::OrderNotPermutation => write!(f, "The new order must list every slot once"),
			Self::ShortcutInvalid => write!(f, "That key combination cannot be a global shortcut"),
			Self::ShortcutNeedsModifier => write!(f, "A global shortcut needs a modifier key"),
			Self::ShortcutNeedsFunctionKey => {
				write!(f, "A quick shortcut must be a function key (F1-F24)")
			}
			Self::ShortcutTaken => write!(f, "Another ring already uses that shortcut"),
			Self::ShortcutUnavailable => write!(f, "The system refused that shortcut"),
			Self::ImportUnreadable => write!(f, "The file is not a RingRing export"),
			Self::ImportVersionUnsupported => {
				write!(f, "The file was written by a newer version")
			}
			Self::ImportTooLarge => write!(f, "The file holds too many rings"),
			Self::ImportEmpty => write!(f, "The file holds no rings"),
		}
	}
}

/// 링의 읽기·쓰기가 실패한 이유. 거절이면 화면이 문장을 고르고, 그 밖이면 원문이 로그로 간다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RingError {
	Refused(Refusal),
	/// 거절이고, 문장에 끼워 넣을 이름이 있다 (같은 단축키를 쓰는 링의 이름).
	RefusedWith(Refusal, String),
	/// DB 오류처럼 사용자가 고칠 수 없는 실패.
	Other(String),
}

impl From<Refusal> for RingError {
	fn from(refusal: Refusal) -> Self {
		Self::Refused(refusal)
	}
}

impl From<String> for RingError {
	fn from(message: String) -> Self {
		Self::Other(message)
	}
}

impl std::fmt::Display for RingError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Refused(refusal) => write!(f, "{refusal}"),
			Self::RefusedWith(refusal, subject) => write!(f, "{refusal} ({subject})"),
			Self::Other(message) => write!(f, "{message}"),
		}
	}
}

/// 이름을 검증할 때 그것이 무엇의 이름인지.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Named {
	Ring,
	Slot,
}

/// 링 이름과 칸 이름의 최대 글자 수.
pub const NAME_MAX_CHARS: usize = 24;
/// 키 입력 칸 하나가 보내는 조합의 최대 수.
pub const MAX_COMBOS: usize = 8;
/// 링에서 하위 링으로 내려가는 최대 단계. 링 → 하위 링 → 그 하위 링이 3 이다.
pub const MAX_DEPTH: usize = 3;
/// 셸 명령 칸의 제한 시간. 기본값과 상한이다 (초).
pub const DEFAULT_SHELL_TIMEOUT_SECS: u32 = 120;
pub const MAX_SHELL_TIMEOUT_SECS: u32 = 60 * 60;

/// `open` 동작이 여는 것.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenTarget {
	App,
	File,
	Url,
}

/// 칸 하나가 하는 일. `action_json` 칸에 이 모양 그대로 들어간다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RingAction {
	Open {
		target: OpenTarget,
		value: String,
		/// 앱에 넘기는 인자. 한 줄의 글이고 `target` 이 앱일 때만 쓴다. 없으면 JSON 에 싣지 않는다.
		#[serde(default, skip_serializing_if = "String::is_empty")]
		args: String,
	},
	Shell {
		command: String,
		/// 빈 문자열이면 홈 폴더다.
		working_dir: String,
		timeout_secs: u32,
	},
	Keystroke {
		combos: Vec<String>,
	},
	OpenRing {
		ring_id: String,
	},
}

impl RingAction {
	/// `action_kind` 칸의 값. serde tag 와 같은 글자다.
	pub fn kind(&self) -> &'static str {
		match self {
			Self::Open { .. } => "open",
			Self::Shell { .. } => "shell",
			Self::Keystroke { .. } => "keystroke",
			Self::OpenRing { .. } => "open_ring",
		}
	}

	/// 이 동작만 보고 알 수 있는 검증. 하위 링의 순환·깊이는 다른 링을 읽어야 하므로 [`check_link`] 가 본다.
	pub fn validate(&self) -> Result<(), Refusal> {
		match self {
			Self::Open { target, value, .. } => {
				let value = value.trim();
				if value.is_empty() {
					return Err(Refusal::OpenValueMissing);
				}
				if *target == OpenTarget::Url && !is_http_url(value) {
					return Err(Refusal::UrlNotHttp);
				}
			}
			Self::Shell {
				command,
				timeout_secs,
				..
			} => {
				if command.trim().is_empty() {
					return Err(Refusal::CommandMissing);
				}
				if !(1..=MAX_SHELL_TIMEOUT_SECS).contains(timeout_secs) {
					return Err(Refusal::TimeoutOutOfRange);
				}
			}
			Self::Keystroke { combos } => {
				if combos.is_empty() {
					return Err(Refusal::CombosMissing);
				}
				if combos.len() > MAX_COMBOS {
					return Err(Refusal::CombosTooMany);
				}
				// 설정의 단축키 입력과 같은 문법이다. 키 코드를 모르는 키도 여기서 거절한다 — 보낼 수 없다.
				// `parse` 는 조합을 읽지 못할 때만 실패한다.
				super::keystroke::parse(combos).map_err(|_| Refusal::ComboInvalid)?;
			}
			Self::OpenRing { ring_id } => {
				if ring_id.trim().is_empty() {
					return Err(Refusal::RingNotChosen);
				}
			}
		}
		Ok(())
	}

	/// "실행 전에 확인" 을 묻는 동작인가. 하위 링 열기는 묻지 않는다 — 실행이 아니라 다음 링을 보이는 것이다.
	pub fn asks_confirm(&self) -> bool {
		!KINDS_WITHOUT_CONFIRM.contains(&self.kind())
	}
}

/// 인자 글을 인자들로 나눈다. 빈칸에서 나누고, 따옴표(`"` 나 `'`) 안의 빈칸은 나누지 않는다.
///
/// `\` 는 뜻이 없는 보통 글자다 — Windows 의 경로에 그대로 쓴다. 닫지 않은 따옴표는 글 끝까지다.
// Windows 는 인자 글을 나누지 않고 그대로 넘긴다 (`exec/launch.rs`). 거기서는 테스트만 이 함수를 쓴다.
#[cfg_attr(windows, allow(dead_code))]
pub fn split_args(text: &str) -> Vec<String> {
	let mut args = Vec::new();
	let mut current = String::new();
	// 빈 따옴표(`""`)도 인자 하나다.
	let mut started = false;
	let mut quote: Option<char> = None;
	for c in text.chars() {
		match quote {
			Some(q) if c == q => quote = None,
			Some(_) => current.push(c),
			None if c == '"' || c == '\'' => {
				quote = Some(c);
				started = true;
			}
			None if c.is_whitespace() => {
				if started {
					args.push(std::mem::take(&mut current));
					started = false;
				}
			}
			None => {
				current.push(c);
				started = true;
			}
		}
	}
	if started {
		args.push(current);
	}
	args
}

/// "실행 전에 확인" 이 기본으로 켜지는 동작 종류. 셸 명령만이다.
pub const CONFIRM_BY_DEFAULT_KINDS: [&str; 1] = ["shell"];
/// "실행 전에 확인" 이 뜻이 없는 동작 종류. 설정 화면은 이 종류에서 그 토글을 감춘다.
pub const KINDS_WITHOUT_CONFIRM: [&str; 1] = ["open_ring"];

/// 설정 화면이 쓰는 한도와 기본값. 화면은 이 값을 받아 쓰고 스스로 정하지 않는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
	pub min_slots: usize,
	pub max_slots: usize,
	pub name_max_chars: usize,
	pub max_combos: usize,
	pub default_shell_timeout_secs: u32,
	pub max_shell_timeout_secs: u32,
	pub confirm_by_default_kinds: &'static [&'static str],
	pub kinds_without_confirm: &'static [&'static str],
	/// 링에서 하위 링으로 내려가는 최대 단계.
	pub max_depth: usize,
}

pub const LIMITS: Limits = Limits {
	min_slots: MIN_SLOTS,
	max_slots: MAX_SLOTS,
	name_max_chars: NAME_MAX_CHARS,
	max_combos: MAX_COMBOS,
	default_shell_timeout_secs: DEFAULT_SHELL_TIMEOUT_SECS,
	max_shell_timeout_secs: MAX_SHELL_TIMEOUT_SECS,
	confirm_by_default_kinds: &CONFIRM_BY_DEFAULT_KINDS,
	kinds_without_confirm: &KINDS_WITHOUT_CONFIRM,
	max_depth: MAX_DEPTH,
};

/// `http://` 나 `https://` 로 시작하는가. 다른 scheme 은 열지 않는다.
pub fn is_http_url(value: &str) -> bool {
	let lower = value.to_ascii_lowercase();
	lower.starts_with("http://") || lower.starts_with("https://")
}

/// 칸 하나.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
	/// 0 이 12시 방향이고 시계 방향으로 자란다.
	pub position: usize,
	pub label: String,
	/// lucide 아이콘 이름 (kebab-case).
	pub icon: String,
	pub action: RingAction,
	/// 실행 전에 확인을 묻는가.
	pub confirm: bool,
}

impl Slot {
	/// 동작을 먼저 본다. 이름이나 아이콘보다 동작이 비어 있다는 것이 사용자에게 먼저 필요한 말이다.
	pub fn validate(&self, slot_count: usize) -> Result<(), Refusal> {
		if self.position >= slot_count {
			return Err(Refusal::SlotOutsideRing);
		}
		self.action.validate()?;
		check_name(&self.label, Named::Slot)?;
		if self.icon.trim().is_empty() {
			return Err(Refusal::IconMissing);
		}
		Ok(())
	}
}

/// 링의 전역 단축키 두 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutKind {
	/// 링을 띄우기만 한다. 키를 떼도 실행하지 않는다 — 링이 남아 클릭이나 키로 고른다.
	Normal,
	/// F1~F24. 누른 채 방향을 잡고 떼면 그 칸을 실행한다.
	Quick,
}

/// 링 하나와 그 칸들. 칸은 `position` 순서다. 행이 없는 자리는 빈 칸이다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ring {
	pub id: String,
	pub name: String,
	/// 일반 단축키. 두 단축키가 다 없으면 하위 링이나 트레이 메뉴로만 연다.
	pub shortcut: Option<String>,
	/// 빠른 단축키.
	#[serde(default)]
	pub quick_shortcut: Option<String>,
	pub slot_count: usize,
	pub slots: Vec<Slot>,
}

impl Ring {
	/// 그 종류의 단축키.
	pub fn shortcut_of(&self, kind: ShortcutKind) -> Option<&str> {
		match kind {
			ShortcutKind::Normal => self.shortcut.as_deref(),
			ShortcutKind::Quick => self.quick_shortcut.as_deref(),
		}
	}

	/// `position` 자리의 칸. 빈 칸이면 `None`.
	pub fn slot(&self, position: usize) -> Option<&Slot> {
		self.slots.iter().find(|slot| slot.position == position)
	}

	/// 자리마다 칸이 차 있는가. 길이는 `slot_count` 다.
	pub fn filled(&self) -> Vec<bool> {
		(0..self.slot_count)
			.map(|position| self.slot(position).is_some())
			.collect()
	}
}

/// 이름 검증. 앞뒤 공백을 뗀 글자 수가 1 ~ [`NAME_MAX_CHARS`] 다.
pub fn check_name(name: &str, named: Named) -> Result<(), Refusal> {
	let length = name.trim().chars().count();
	let (missing, too_long) = match named {
		Named::Ring => (Refusal::RingNameMissing, Refusal::RingNameTooLong),
		Named::Slot => (Refusal::SlotNameMissing, Refusal::SlotNameTooLong),
	};
	if length == 0 {
		return Err(missing);
	}
	if length > NAME_MAX_CHARS {
		return Err(too_long);
	}
	Ok(())
}

pub fn check_slot_count(slot_count: usize) -> Result<(), Refusal> {
	if (MIN_SLOTS..=MAX_SLOTS).contains(&slot_count) {
		Ok(())
	} else {
		Err(Refusal::SlotCountOutOfRange)
	}
}

/// `from` 링의 칸이 `to` 링을 열어도 되는가.
///
/// `edges` 는 지금 저장된 "링 → 그 링이 여는 하위 링" 전부다. 고치는 칸의 예전 연결은 빼고 준다.
/// 자기 자신이나 조상을 가리키면 순환이라 거절한다. 새 연결을 지나는 가장 긴 사슬이 [`MAX_DEPTH`] 를 넘어도
/// 거절한다.
pub fn check_link(edges: &[(String, String)], from: &str, to: &str) -> Result<(), Refusal> {
	if from == to {
		return Err(Refusal::LinkSelf);
	}
	if reaches(edges, to, from, edges.len() + 1) {
		return Err(Refusal::LinkCycle);
	}
	// 순환이 없으므로 어느 사슬도 링 수보다 길지 않다. `limit` 은 손상된 데이터에서 재귀를 끝내는 상한이다.
	let limit = edges.len() + 1;
	let above = chain_length(edges, from, Direction::Up, limit);
	let below = chain_length(edges, to, Direction::Down, limit);
	if above + below > MAX_DEPTH {
		return Err(Refusal::LinkTooDeep);
	}
	Ok(())
}

/// `start` 에서 연결을 따라 `target` 에 닿는가.
fn reaches(edges: &[(String, String)], start: &str, target: &str, limit: usize) -> bool {
	if start == target {
		return true;
	}
	if limit == 0 {
		return false;
	}
	edges
		.iter()
		.filter(|(parent, _)| parent == start)
		.any(|(_, child)| reaches(edges, child, target, limit - 1))
}

#[derive(Clone, Copy)]
enum Direction {
	Up,
	Down,
}

/// `ring` 에서 한 방향으로 가장 긴 사슬의 링 수. `ring` 혼자면 1 이다.
fn chain_length(
	edges: &[(String, String)],
	ring: &str,
	direction: Direction,
	limit: usize,
) -> usize {
	if limit == 0 {
		return 1;
	}
	let longest = edges
		.iter()
		.filter_map(|(parent, child)| match direction {
			Direction::Up if child == ring => Some(parent),
			Direction::Down if parent == ring => Some(child),
			_ => None,
		})
		.map(|next| chain_length(edges, next, direction, limit - 1))
		.max()
		.unwrap_or(0);
	1 + longest
}

#[cfg(test)]
mod tests {
	use super::*;

	fn edges(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
		pairs
			.iter()
			.map(|(a, b)| (a.to_string(), b.to_string()))
			.collect()
	}

	fn slot(action: RingAction) -> Slot {
		Slot {
			position: 0,
			label: "칸".to_string(),
			icon: "terminal".to_string(),
			action,
			confirm: false,
		}
	}

	#[test]
	fn action_json_uses_a_kind_tag() {
		let action = RingAction::Open {
			target: OpenTarget::Url,
			value: "https://example.com".to_string(),
			args: String::new(),
		};
		let json = serde_json::to_value(&action).unwrap();
		assert_eq!(
			json,
			serde_json::json!({"kind": "open", "target": "url", "value": "https://example.com"})
		);
		assert_eq!(json["kind"], action.kind());
		assert_eq!(serde_json::from_value::<RingAction>(json).unwrap(), action);
	}

	#[test]
	fn the_kinds_tome_had_are_not_actions_here() {
		for json in [
			r#"{"kind":"tome_feature","feature_id":"quick_memo"}"#,
			r#"{"kind":"workflow","scope":"templates","workflow_id":"w"}"#,
		] {
			assert!(serde_json::from_str::<RingAction>(json).is_err(), "{json}");
		}
	}

	#[test]
	fn app_arguments_ride_in_the_json_only_when_there_are_some() {
		let action = RingAction::Open {
			target: OpenTarget::App,
			value: "/Applications/Safari.app".to_string(),
			args: "--new-window https://example.com".to_string(),
		};
		let json = serde_json::to_value(&action).unwrap();
		assert_eq!(json["args"], "--new-window https://example.com");
		assert_eq!(serde_json::from_value::<RingAction>(json).unwrap(), action);
		// 인자가 생기기 전에 저장된 칸은 `args` 가 없다.
		let old = serde_json::json!({"kind": "open", "target": "app", "value": "/a.app"});
		assert!(matches!(
			serde_json::from_value::<RingAction>(old).unwrap(),
			RingAction::Open { args, .. } if args.is_empty()
		));
	}

	#[test]
	fn arguments_split_on_blanks_outside_quotes() {
		assert_eq!(split_args(""), Vec::<String>::new());
		assert_eq!(split_args("  -n   --flag=1 "), ["-n", "--flag=1"]);
		assert_eq!(
			split_args(r#"--profile "Work Stuff" 'a b' c"d e"f"#),
			["--profile", "Work Stuff", "a b", "cd ef"]
		);
		// 역슬래시는 그대로다.
		assert_eq!(split_args(r"C:\Temp\a.txt"), [r"C:\Temp\a.txt"]);
		assert_eq!(split_args(r#"--name """#), ["--name", ""]);
		// 닫지 않은 따옴표는 끝까지다.
		assert_eq!(split_args(r#"say "hello world"#), ["say", "hello world"]);
	}

	#[test]
	fn every_kind_matches_its_serde_tag() {
		let actions = [
			RingAction::Open {
				target: OpenTarget::File,
				value: "/tmp/a".to_string(),
				args: String::new(),
			},
			RingAction::Shell {
				command: "true".to_string(),
				working_dir: String::new(),
				timeout_secs: 120,
			},
			RingAction::Keystroke {
				combos: vec!["Cmd+KeyC".to_string()],
			},
			RingAction::OpenRing {
				ring_id: "r".to_string(),
			},
		];
		for action in actions {
			let json = serde_json::to_value(&action).unwrap();
			assert_eq!(json["kind"], action.kind());
		}
	}

	#[test]
	fn url_must_be_http_or_https() {
		let url = |value: &str| RingAction::Open {
			target: OpenTarget::Url,
			value: value.to_string(),
			args: String::new(),
		};
		assert!(url("https://example.com").validate().is_ok());
		assert!(url("HTTP://example.com").validate().is_ok());
		assert!(url("obsidian://open").validate().is_err());
		assert!(url("file:///etc/passwd").validate().is_err());
		assert!(url("javascript:alert(1)").validate().is_err());
		assert!(url("  ").validate().is_err());
	}

	#[test]
	fn a_file_target_is_not_held_to_the_url_rule() {
		let action = RingAction::Open {
			target: OpenTarget::File,
			value: "/Users/me/notes.txt".to_string(),
			args: String::new(),
		};
		assert!(action.validate().is_ok());
	}

	#[test]
	fn shell_needs_a_command_and_a_sane_time_limit() {
		let shell = |command: &str, timeout_secs: u32| RingAction::Shell {
			command: command.to_string(),
			working_dir: String::new(),
			timeout_secs,
		};
		assert!(shell("echo hi", 120).validate().is_ok());
		assert!(shell("   ", 120).validate().is_err());
		assert!(shell("echo hi", 0).validate().is_err());
		assert!(
			shell("echo hi", MAX_SHELL_TIMEOUT_SECS + 1)
				.validate()
				.is_err()
		);
	}

	#[test]
	fn keystroke_combos_must_parse_and_stay_within_the_cap() {
		let keys = |combos: &[&str]| RingAction::Keystroke {
			combos: combos.iter().map(|c| c.to_string()).collect(),
		};
		assert!(keys(&["Cmd+Shift+Digit4"]).validate().is_ok());
		assert!(keys(&[]).validate().is_err());
		assert!(keys(&["Cmd+NotAKey"]).validate().is_err());
		assert!(keys(&["Cmd+KeyA"; MAX_COMBOS]).validate().is_ok());
		assert!(keys(&["Cmd+KeyA"; MAX_COMBOS + 1]).validate().is_err());
	}

	#[test]
	fn only_shell_confirms_by_default() {
		assert_eq!(LIMITS.confirm_by_default_kinds, ["shell"]);
		let shell = RingAction::Shell {
			command: "true".to_string(),
			working_dir: String::new(),
			timeout_secs: LIMITS.default_shell_timeout_secs,
		};
		assert!(LIMITS.confirm_by_default_kinds.contains(&shell.kind()));
		assert!(shell.validate().is_ok());
	}

	#[test]
	fn slot_checks_position_name_and_icon() {
		let ok = slot(RingAction::OpenRing {
			ring_id: "r".to_string(),
		});
		assert!(ok.validate(4).is_ok());

		let mut outside = ok.clone();
		outside.position = 4;
		assert!(outside.validate(4).is_err());

		let mut unnamed = ok.clone();
		unnamed.label = "  ".to_string();
		assert!(unnamed.validate(4).is_err());

		let mut long = ok.clone();
		long.label = "가".repeat(NAME_MAX_CHARS + 1);
		assert!(long.validate(4).is_err());
		long.label = "가".repeat(NAME_MAX_CHARS);
		assert!(long.validate(4).is_ok());

		let mut no_icon = ok;
		no_icon.icon = String::new();
		assert!(no_icon.validate(4).is_err());
	}

	#[test]
	fn an_unfilled_action_is_incomplete_and_a_wrong_value_is_not() {
		let unfilled = [
			RingAction::Open {
				target: OpenTarget::App,
				value: String::new(),
				args: String::new(),
			},
			RingAction::Shell {
				command: String::new(),
				working_dir: String::new(),
				timeout_secs: DEFAULT_SHELL_TIMEOUT_SECS,
			},
			RingAction::Keystroke { combos: Vec::new() },
			RingAction::OpenRing {
				ring_id: String::new(),
			},
		];
		for action in unfilled {
			let refusal = action.validate().unwrap_err();
			assert!(refusal.is_incomplete(), "{} → {refusal:?}", action.kind());
		}
		let wrong = RingAction::Open {
			target: OpenTarget::Url,
			value: "ftp://example.com".to_string(),
			args: String::new(),
		};
		assert_eq!(wrong.validate(), Err(Refusal::UrlNotHttp));
		assert!(!Refusal::UrlNotHttp.is_incomplete());
	}

	#[test]
	fn a_slot_reports_the_unfilled_action_before_the_missing_name() {
		let blank = Slot {
			position: 0,
			label: String::new(),
			icon: String::new(),
			action: RingAction::OpenRing {
				ring_id: String::new(),
			},
			confirm: false,
		};
		assert_eq!(blank.validate(6), Err(Refusal::RingNotChosen));
	}

	#[test]
	fn names_are_refused_with_the_code_of_what_they_name() {
		assert_eq!(check_name(" ", Named::Ring), Err(Refusal::RingNameMissing));
		assert_eq!(check_name(" ", Named::Slot), Err(Refusal::SlotNameMissing));
		let long = "가".repeat(NAME_MAX_CHARS + 1);
		assert_eq!(
			check_name(&long, Named::Ring),
			Err(Refusal::RingNameTooLong)
		);
		assert_eq!(
			check_name(&long, Named::Slot),
			Err(Refusal::SlotNameTooLong)
		);
	}

	#[test]
	fn refusal_codes_are_unique() {
		let all = [
			Refusal::OpenValueMissing,
			Refusal::UrlNotHttp,
			Refusal::CommandMissing,
			Refusal::TimeoutOutOfRange,
			Refusal::CombosMissing,
			Refusal::CombosTooMany,
			Refusal::ComboInvalid,
			Refusal::RingNotChosen,
			Refusal::SlotOutsideRing,
			Refusal::RingNameMissing,
			Refusal::RingNameTooLong,
			Refusal::SlotNameMissing,
			Refusal::SlotNameTooLong,
			Refusal::IconMissing,
			Refusal::SlotCountOutOfRange,
			Refusal::LinkSelf,
			Refusal::LinkCycle,
			Refusal::LinkTooDeep,
			Refusal::RingGone,
			Refusal::RingEmpty,
			Refusal::OrderNotPermutation,
			Refusal::ShortcutInvalid,
			Refusal::ShortcutNeedsModifier,
			Refusal::ShortcutTaken,
			Refusal::ShortcutUnavailable,
			Refusal::ImportUnreadable,
			Refusal::ImportVersionUnsupported,
			Refusal::ImportTooLarge,
			Refusal::ImportEmpty,
		];
		let mut codes: Vec<&str> = all.iter().map(|refusal| refusal.code()).collect();
		codes.sort_unstable();
		codes.dedup();
		assert_eq!(codes.len(), all.len());
	}

	#[test]
	fn only_opening_a_sub_ring_skips_the_confirm_question() {
		assert_eq!(LIMITS.kinds_without_confirm, ["open_ring"]);
		let link = RingAction::OpenRing {
			ring_id: "r".to_string(),
		};
		assert!(!link.asks_confirm());
		let shell = RingAction::Shell {
			command: "true".to_string(),
			working_dir: String::new(),
			timeout_secs: 120,
		};
		assert!(shell.asks_confirm());
	}

	#[test]
	fn link_refusals_say_which_rule_was_broken() {
		assert_eq!(check_link(&[], "a", "a"), Err(Refusal::LinkSelf));
		assert_eq!(
			check_link(&edges(&[("a", "b")]), "b", "a"),
			Err(Refusal::LinkCycle)
		);
		assert_eq!(
			check_link(&edges(&[("a", "b"), ("b", "c")]), "c", "d"),
			Err(Refusal::LinkTooDeep)
		);
	}

	#[test]
	fn slot_count_is_four_to_eight() {
		assert!(check_slot_count(3).is_err());
		assert!(check_slot_count(4).is_ok());
		assert!(check_slot_count(8).is_ok());
		assert!(check_slot_count(9).is_err());
	}

	#[test]
	fn a_ring_cannot_open_itself() {
		assert!(check_link(&[], "a", "a").is_err());
	}

	#[test]
	fn a_ring_cannot_open_its_ancestor() {
		// a → b 가 있을 때 b → a 는 순환이다.
		assert!(check_link(&edges(&[("a", "b")]), "b", "a").is_err());
		// a → b → c 가 있을 때 c → a 도 순환이다.
		assert!(check_link(&edges(&[("a", "b"), ("b", "c")]), "c", "a").is_err());
	}

	#[test]
	fn three_levels_are_allowed_and_four_are_not() {
		assert!(check_link(&[], "a", "b").is_ok());
		// a → b 가 있을 때 b → c 는 3단이다.
		assert!(check_link(&edges(&[("a", "b")]), "b", "c").is_ok());
		// a → b → c 가 있을 때 c → d 는 4단이다.
		assert!(check_link(&edges(&[("a", "b"), ("b", "c")]), "c", "d").is_err());
	}

	#[test]
	fn depth_counts_the_chain_below_the_target_too() {
		// b → c → d 가 있을 때 a → b 는 a, b, c, d 로 4단이다.
		assert!(check_link(&edges(&[("b", "c"), ("c", "d")]), "a", "b").is_err());
		// b → c 만 있으면 a → b 는 3단이다.
		assert!(check_link(&edges(&[("b", "c")]), "a", "b").is_ok());
	}

	#[test]
	fn depth_counts_the_chain_above_the_source_too() {
		// x → a 와 b → c 가 있을 때 a → b 는 x, a, b, c 로 4단이다.
		assert!(check_link(&edges(&[("x", "a"), ("b", "c")]), "a", "b").is_err());
	}

	#[test]
	fn a_shared_sub_ring_is_not_a_cycle() {
		// a 와 b 가 둘 다 c 를 연다. b → c 를 더하는 것은 순환이 아니다.
		assert!(check_link(&edges(&[("a", "c")]), "b", "c").is_ok());
	}

	#[test]
	fn damaged_data_with_a_cycle_does_not_recurse_forever() {
		let looped = edges(&[("a", "b"), ("b", "a")]);
		assert!(check_link(&looped, "c", "a").is_err());
	}
}
