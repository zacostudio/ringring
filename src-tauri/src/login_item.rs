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

/// 시스템 설정의 로그인 항목 화면. Windows 는 설정의 시작 앱 화면이다.
pub const LOGIN_ITEMS_SETTINGS_URL: &str = if cfg!(windows) {
	"ms-settings:startupapps"
} else {
	"x-apple.systempreferences:com.apple.LoginItems-Settings.extension"
};

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

// Windows 는 `HKCU\…\Run` 의 값 하나다. 값이 이 실행 파일을 가리키면 켜진 것이다.
#[cfg(windows)]
mod os {
	use std::os::windows::ffi::OsStrExt;

	use windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND;
	use windows_sys::Win32::System::Registry::{
		HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
	};

	const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
	const VALUE_NAME: &str = "RingRing";

	fn wide(text: &str) -> Vec<u16> {
		text.encode_utf16().chain(std::iter::once(0)).collect()
	}

	/// 따옴표로 싼 이 실행 파일의 경로. 끝에 NUL 이 붙는다.
	fn command() -> Option<Vec<u16>> {
		let exe = std::env::current_exe().ok()?;
		Some(
			std::iter::once(u16::from(b'"'))
				.chain(exe.as_os_str().encode_wide())
				.chain([u16::from(b'"'), 0])
				.collect(),
		)
	}

	fn registered() -> Option<Vec<u16>> {
		let mut buf = vec![0u16; 1024];
		let mut bytes = (buf.len() * 2) as u32;
		let status = unsafe {
			RegGetValueW(
				HKEY_CURRENT_USER,
				wide(RUN_KEY).as_ptr(),
				wide(VALUE_NAME).as_ptr(),
				RRF_RT_REG_SZ,
				std::ptr::null_mut(),
				buf.as_mut_ptr().cast(),
				&mut bytes,
			)
		};
		(status == 0).then(|| {
			buf.truncate(bytes as usize / 2);
			buf
		})
	}

	pub(super) fn status() -> Option<isize> {
		let command = command()?;
		Some(if registered().is_some_and(|value| value == command) {
			super::STATUS_ENABLED
		} else {
			0
		})
	}

	pub(super) fn set(enabled: bool) -> Result<(), String> {
		let status = if enabled {
			let command = command().ok_or("The path of this app is not readable")?;
			unsafe {
				RegSetKeyValueW(
					HKEY_CURRENT_USER,
					wide(RUN_KEY).as_ptr(),
					wide(VALUE_NAME).as_ptr(),
					REG_SZ,
					command.as_ptr().cast(),
					(command.len() * 2) as u32,
				)
			}
		} else {
			unsafe {
				RegDeleteKeyValueW(
					HKEY_CURRENT_USER,
					wide(RUN_KEY).as_ptr(),
					wide(VALUE_NAME).as_ptr(),
				)
			}
		};
		match status {
			0 => Ok(()),
			// 이미 없는 값을 지우는 것은 실패가 아니다.
			ERROR_FILE_NOT_FOUND if !enabled => Ok(()),
			code => Err(std::io::Error::from_raw_os_error(code as i32).to_string()),
		}
	}
}

#[cfg(not(any(target_os = "macos", windows)))]
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
