// 로그인할 때 실행 — `SMAppService.mainApp` 으로 등록하고, 상태는 OS 에 묻는다 (저장하지 않는다)
//
// 개발 빌드는 등록하지 않는다. 개발 바이너리는 앱 번들이 아니고, 사용자의 로그인 항목에 개발 빌드를
// 넣어서도 안 된다.
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginItemState {
	/// 로그인할 때 실행되게 등록돼 있다.
	pub enabled: bool,
	/// 이 빌드에서 켜고 끌 수 있다. 개발 빌드는 `false` 다.
	pub available: bool,
	/// 등록은 됐지만 사용자가 시스템 설정의 로그인 항목에서 허용해야 한다.
	pub needs_approval: bool,
}

/// `SMAppServiceStatus` 의 값.
const STATUS_ENABLED: isize = 1;
const STATUS_REQUIRES_APPROVAL: isize = 2;

/// 시스템 설정의 로그인 항목 화면.
pub const LOGIN_ITEMS_SETTINGS_URL: &str =
	"x-apple.systempreferences:com.apple.LoginItems-Settings.extension";

fn state_of(status: isize, available: bool) -> LoginItemState {
	LoginItemState {
		enabled: status == STATUS_ENABLED,
		available,
		needs_approval: status == STATUS_REQUIRES_APPROVAL,
	}
}

pub fn state() -> LoginItemState {
	if cfg!(dev) {
		return state_of(0, false);
	}
	match os::status() {
		Some(status) => state_of(status, true),
		None => state_of(0, false),
	}
}

/// 등록하거나 뗀다. 사용자가 토글을 눌렀을 때만 부른다.
pub fn set(enabled: bool) -> Result<(), String> {
	if cfg!(dev) {
		return Err("Development builds do not register a login item".to_string());
	}
	os::set(enabled)
}

#[cfg(target_os = "macos")]
mod os {
	use objc2::msg_send;
	use objc2::runtime::{AnyClass, AnyObject, Bool};

	// `SMAppService` 는 ServiceManagement 에 있다. 링크해야 클래스가 실린다.
	#[link(name = "ServiceManagement", kind = "framework")]
	unsafe extern "C" {}

	fn service() -> Option<*mut AnyObject> {
		let class = AnyClass::get(c"SMAppService")?;
		let service: *mut AnyObject = unsafe { msg_send![class, mainAppService] };
		(!service.is_null()).then_some(service)
	}

	pub(super) fn status() -> Option<isize> {
		let service = service()?;
		Some(unsafe { msg_send![service, status] })
	}

	pub(super) fn set(enabled: bool) -> Result<(), String> {
		let service = service().ok_or("SMAppService is not available on this macOS")?;
		let mut error: *mut AnyObject = std::ptr::null_mut();
		let ok: Bool = unsafe {
			if enabled {
				msg_send![service, registerAndReturnError: &mut error]
			} else {
				msg_send![service, unregisterAndReturnError: &mut error]
			}
		};
		if ok.as_bool() {
			return Ok(());
		}
		Err(describe(error))
	}

	fn describe(error: *mut AnyObject) -> String {
		if error.is_null() {
			return "The system refused the login item".to_string();
		}
		unsafe {
			let text: *mut AnyObject = msg_send![error, localizedDescription];
			if text.is_null() {
				return "The system refused the login item".to_string();
			}
			let utf8: *const std::ffi::c_char = msg_send![text, UTF8String];
			if utf8.is_null() {
				return "The system refused the login item".to_string();
			}
			std::ffi::CStr::from_ptr(utf8)
				.to_string_lossy()
				.into_owned()
		}
	}
}

#[cfg(not(target_os = "macos"))]
mod os {
	pub(super) fn status() -> Option<isize> {
		None
	}

	pub(super) fn set(_enabled: bool) -> Result<(), String> {
		Err("Login items are only supported on macOS".to_string())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn only_the_enabled_status_counts_as_enabled() {
		assert!(!state_of(0, true).enabled);
		assert!(state_of(STATUS_ENABLED, true).enabled);
		assert!(!state_of(STATUS_REQUIRES_APPROVAL, true).enabled);
		assert!(state_of(STATUS_REQUIRES_APPROVAL, true).needs_approval);
		assert!(!state_of(3, true).enabled);
	}
}
