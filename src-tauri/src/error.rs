// command 가 프런트에 돌려주는 오류 — 거절은 이유 코드를 싣고, 화면이 그 코드로 문장을 고른다
use serde::Serialize;

use crate::ring::model::{Refusal, RingError};

/// 값이 틀려서 거절했다.
pub const KIND_REFUSED: &str = "refused";
/// 틀린 것이 아니라 아직 채우지 않았다. 화면은 오류 색으로 보이지 않는다.
pub const KIND_INCOMPLETE: &str = "incomplete";
/// DB 오류처럼 사용자가 고칠 수 없는 실패.
pub const KIND_FAILED: &str = "failed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
	pub kind: &'static str,
	/// `Refusal::code`. 거절이 아니면 없다.
	pub code: Option<&'static str>,
	/// 영어 원문. 로그용이다. 화면에 그대로 보이지 않는다.
	pub message: String,
	/// 문장에 끼워 넣을 이름 (같은 단축키를 쓰는 링 등).
	pub subject: Option<String>,
}

impl CommandError {
	pub fn failed(message: impl Into<String>) -> Self {
		Self {
			kind: KIND_FAILED,
			code: None,
			message: message.into(),
			subject: None,
		}
	}
}

impl From<Refusal> for CommandError {
	fn from(refusal: Refusal) -> Self {
		Self {
			kind: if refusal.is_incomplete() {
				KIND_INCOMPLETE
			} else {
				KIND_REFUSED
			},
			code: Some(refusal.code()),
			message: refusal.to_string(),
			subject: None,
		}
	}
}

impl From<RingError> for CommandError {
	fn from(error: RingError) -> Self {
		match error {
			RingError::Refused(refusal) => refusal.into(),
			RingError::RefusedWith(refusal, subject) => Self {
				subject: Some(subject),
				..refusal.into()
			},
			RingError::Other(message) => {
				log::warn!("[command] failed: {message}");
				Self::failed(message)
			}
		}
	}
}

impl From<String> for CommandError {
	fn from(message: String) -> Self {
		RingError::Other(message).into()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_refusal_carries_its_code() {
		let wrong: CommandError = RingError::Refused(Refusal::LinkCycle).into();
		assert_eq!(wrong.kind, KIND_REFUSED);
		assert_eq!(wrong.code, Some("link_cycle"));
		assert_eq!(wrong.message, "That ring already leads back to this one");
	}

	#[test]
	fn an_unfilled_value_is_incomplete_and_not_refused() {
		let unfilled: CommandError = Refusal::RingNotChosen.into();
		assert_eq!(unfilled.kind, KIND_INCOMPLETE);
		assert_eq!(unfilled.code, Some("ring_not_chosen"));
	}

	#[test]
	fn a_refusal_can_name_what_it_collided_with() {
		let taken: CommandError =
			RingError::RefusedWith(Refusal::ShortcutTaken, "작업".to_string()).into();
		assert_eq!(taken.code, Some("shortcut_taken"));
		assert_eq!(taken.subject.as_deref(), Some("작업"));
	}

	#[test]
	fn a_database_failure_has_no_code() {
		let broken: CommandError = RingError::Other("Ring DB error: locked".to_string()).into();
		assert_eq!(broken.kind, KIND_FAILED);
		assert!(broken.code.is_none());
	}
}
