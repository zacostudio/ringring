// 링이 OS 에서 읽는 것 — 커서 자리, 눌린 키, 커서 옮기기
//
// 좌표는 전역 화면 좌표다. 논리 픽셀이고 주 모니터의 왼쪽 위가 원점이다 (Quartz 의 좌표계).
// 읽기는 어느 thread 에서 불러도 된다.

use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

/// `Code` 를 macOS 의 가상 키 코드로 바꾼다. 전역 단축키 plugin 이 받는 키와 같은 표다
/// (global-hotkey 0.7.0 `platform_impl/macos/mod.rs` 의 `key_to_scancode` — 그 함수는 밖에서 부를 수 없다).
#[cfg(not(windows))]
pub fn keycode(code: Code) -> Option<u16> {
	Some(match code {
		Code::KeyA => 0x00,
		Code::KeyS => 0x01,
		Code::KeyD => 0x02,
		Code::KeyF => 0x03,
		Code::KeyH => 0x04,
		Code::KeyG => 0x05,
		Code::KeyZ => 0x06,
		Code::KeyX => 0x07,
		Code::KeyC => 0x08,
		Code::KeyV => 0x09,
		Code::KeyB => 0x0b,
		Code::KeyQ => 0x0c,
		Code::KeyW => 0x0d,
		Code::KeyE => 0x0e,
		Code::KeyR => 0x0f,
		Code::KeyY => 0x10,
		Code::KeyT => 0x11,
		Code::Digit1 => 0x12,
		Code::Digit2 => 0x13,
		Code::Digit3 => 0x14,
		Code::Digit4 => 0x15,
		Code::Digit6 => 0x16,
		Code::Digit5 => 0x17,
		Code::Equal => 0x18,
		Code::Digit9 => 0x19,
		Code::Digit7 => 0x1a,
		Code::Minus => 0x1b,
		Code::Digit8 => 0x1c,
		Code::Digit0 => 0x1d,
		Code::BracketRight => 0x1e,
		Code::KeyO => 0x1f,
		Code::KeyU => 0x20,
		Code::BracketLeft => 0x21,
		Code::KeyI => 0x22,
		Code::KeyP => 0x23,
		Code::Enter => 0x24,
		Code::KeyL => 0x25,
		Code::KeyJ => 0x26,
		Code::Quote => 0x27,
		Code::KeyK => 0x28,
		Code::Semicolon => 0x29,
		Code::Backslash => 0x2a,
		Code::Comma => 0x2b,
		Code::Slash => 0x2c,
		Code::KeyN => 0x2d,
		Code::KeyM => 0x2e,
		Code::Period => 0x2f,
		Code::Tab => 0x30,
		Code::Space => 0x31,
		Code::Backquote => 0x32,
		Code::Backspace => 0x33,
		Code::Escape => 0x35,
		Code::CapsLock => 0x39,
		Code::F17 => 0x40,
		Code::NumpadDecimal => 0x41,
		Code::NumpadMultiply => 0x43,
		Code::NumpadAdd => 0x45,
		Code::PrintScreen => 0x46,
		Code::NumLock => 0x47,
		Code::NumpadDivide => 0x4b,
		Code::NumpadEnter => 0x4c,
		Code::NumpadSubtract => 0x4e,
		Code::F18 => 0x4f,
		Code::F19 => 0x50,
		Code::NumpadEqual => 0x51,
		Code::Numpad0 => 0x52,
		Code::Numpad1 => 0x53,
		Code::Numpad2 => 0x54,
		Code::Numpad3 => 0x55,
		Code::Numpad4 => 0x56,
		Code::Numpad5 => 0x57,
		Code::Numpad6 => 0x58,
		Code::Numpad7 => 0x59,
		Code::F20 => 0x5a,
		Code::Numpad8 => 0x5b,
		Code::Numpad9 => 0x5c,
		Code::F5 => 0x60,
		Code::F6 => 0x61,
		Code::F7 => 0x62,
		Code::F3 => 0x63,
		Code::F8 => 0x64,
		Code::F9 => 0x65,
		Code::F11 => 0x67,
		Code::F13 => 0x69,
		Code::F16 => 0x6a,
		Code::F14 => 0x6b,
		Code::F10 => 0x6d,
		Code::F12 => 0x6f,
		Code::F15 => 0x71,
		Code::Insert => 0x72,
		Code::Home => 0x73,
		Code::PageUp => 0x74,
		Code::Delete => 0x75,
		Code::F4 => 0x76,
		Code::End => 0x77,
		Code::F2 => 0x78,
		Code::PageDown => 0x79,
		Code::F1 => 0x7a,
		Code::ArrowLeft => 0x7b,
		Code::ArrowRight => 0x7c,
		Code::ArrowDown => 0x7d,
		Code::ArrowUp => 0x7e,
		_ => return None,
	})
}

/// `CGEventFlags` 의 수식키 비트. Windows 에서도 같은 비트로 수식키를 나타낸다 (`FLAG_COMMAND` 는 Win 키).
pub const FLAG_SHIFT: u64 = 0x0002_0000;
pub const FLAG_CONTROL: u64 = 0x0004_0000;
pub const FLAG_ALTERNATE: u64 = 0x0008_0000;
pub const FLAG_COMMAND: u64 = 0x0010_0000;
const ALL_MODIFIER_FLAGS: u64 = FLAG_SHIFT | FLAG_CONTROL | FLAG_ALTERNATE | FLAG_COMMAND;

/// `Modifiers` 를 `CGEventFlags` 비트로 바꾼다.
pub fn modifier_flags(mods: Modifiers) -> u64 {
	let mut flags = 0;
	if mods.contains(Modifiers::SHIFT) {
		flags |= FLAG_SHIFT;
	}
	if mods.contains(Modifiers::CONTROL) {
		flags |= FLAG_CONTROL;
	}
	if mods.contains(Modifiers::ALT) {
		flags |= FLAG_ALTERNATE;
	}
	if mods.intersects(Modifiers::SUPER | Modifiers::META) {
		flags |= FLAG_COMMAND;
	}
	flags
}

/// 조합의 키가 전부 눌려 있는가. 하나라도 떨어졌으면 `false` 다.
///
/// 키 코드를 모르는 키(매체 키 등)는 수식키만 본다.
pub fn combo_held(shortcut: &Shortcut) -> bool {
	let wanted = modifier_flags(shortcut.mods);
	if os::modifier_flags() & wanted != wanted {
		return false;
	}
	match keycode(shortcut.key) {
		Some(code) => os::key_down(code),
		None => true,
	}
}

/// 수식키가 하나라도 눌려 있는가.
pub fn any_modifier_down() -> bool {
	os::modifier_flags() & ALL_MODIFIER_FLAGS != 0
}

/// 커서의 자리.
pub fn cursor() -> (f64, f64) {
	os::cursor()
}

/// 커서를 `(x, y)` 로 옮긴다. OS 가 거절하면 `false`.
pub fn warp_cursor(x: f64, y: f64) -> bool {
	os::warp_cursor(x, y)
}

#[cfg(target_os = "macos")]
mod os {
	use std::ffi::c_void;

	#[repr(C)]
	#[derive(Clone, Copy)]
	struct CGPoint {
		x: f64,
		y: f64,
	}

	#[link(name = "ApplicationServices", kind = "framework")]
	unsafe extern "C" {
		fn CGEventSourceKeyState(state_id: i32, key: u16) -> bool;
		fn CGEventSourceFlagsState(state_id: i32) -> u64;
		fn CGWarpMouseCursorPosition(point: CGPoint) -> i32;
		fn CGAssociateMouseAndMouseCursorPosition(connected: bool) -> i32;
		fn CGEventCreate(source: *const c_void) -> *const c_void;
		fn CGEventGetLocation(event: *const c_void) -> CGPoint;
		fn CFRelease(cf: *const c_void);
	}

	/// `kCGEventSourceStateCombinedSessionState` — 하드웨어와 다른 프로세스가 보낸 입력을 합친 상태.
	const COMBINED_SESSION_STATE: i32 = 0;

	pub(super) fn modifier_flags() -> u64 {
		unsafe { CGEventSourceFlagsState(COMBINED_SESSION_STATE) }
	}

	pub(super) fn key_down(code: u16) -> bool {
		unsafe { CGEventSourceKeyState(COMBINED_SESSION_STATE, code) }
	}

	pub(super) fn cursor() -> (f64, f64) {
		unsafe {
			let event = CGEventCreate(std::ptr::null());
			if event.is_null() {
				return (0.0, 0.0);
			}
			let point = CGEventGetLocation(event);
			CFRelease(event);
			(point.x, point.y)
		}
	}

	pub(super) fn warp_cursor(x: f64, y: f64) -> bool {
		unsafe {
			if CGWarpMouseCursorPosition(CGPoint { x, y }) != 0 {
				return false;
			}
			// warp 뒤에 OS 는 마우스 이동을 잠깐(기본 0.25초) 커서에 반영하지 않는다. 다시 이으면 바로 움직인다.
			CGAssociateMouseAndMouseCursorPosition(true);
			true
		}
	}
}

/// `Code` 를 Windows 의 가상 키 코드(`VK_*`)로 바꾼다.
#[cfg(windows)]
pub fn keycode(code: Code) -> Option<u16> {
	Some(match code {
		Code::KeyA => 0x41,
		Code::KeyB => 0x42,
		Code::KeyC => 0x43,
		Code::KeyD => 0x44,
		Code::KeyE => 0x45,
		Code::KeyF => 0x46,
		Code::KeyG => 0x47,
		Code::KeyH => 0x48,
		Code::KeyI => 0x49,
		Code::KeyJ => 0x4a,
		Code::KeyK => 0x4b,
		Code::KeyL => 0x4c,
		Code::KeyM => 0x4d,
		Code::KeyN => 0x4e,
		Code::KeyO => 0x4f,
		Code::KeyP => 0x50,
		Code::KeyQ => 0x51,
		Code::KeyR => 0x52,
		Code::KeyS => 0x53,
		Code::KeyT => 0x54,
		Code::KeyU => 0x55,
		Code::KeyV => 0x56,
		Code::KeyW => 0x57,
		Code::KeyX => 0x58,
		Code::KeyY => 0x59,
		Code::KeyZ => 0x5a,
		Code::Digit0 => 0x30,
		Code::Digit1 => 0x31,
		Code::Digit2 => 0x32,
		Code::Digit3 => 0x33,
		Code::Digit4 => 0x34,
		Code::Digit5 => 0x35,
		Code::Digit6 => 0x36,
		Code::Digit7 => 0x37,
		Code::Digit8 => 0x38,
		Code::Digit9 => 0x39,
		Code::Backspace => 0x08,
		Code::Tab => 0x09,
		Code::Enter => 0x0d,
		Code::CapsLock => 0x14,
		Code::Escape => 0x1b,
		Code::Space => 0x20,
		Code::PageUp => 0x21,
		Code::PageDown => 0x22,
		Code::End => 0x23,
		Code::Home => 0x24,
		Code::ArrowLeft => 0x25,
		Code::ArrowUp => 0x26,
		Code::ArrowRight => 0x27,
		Code::ArrowDown => 0x28,
		Code::PrintScreen => 0x2c,
		Code::Insert => 0x2d,
		Code::Delete => 0x2e,
		Code::Numpad0 => 0x60,
		Code::Numpad1 => 0x61,
		Code::Numpad2 => 0x62,
		Code::Numpad3 => 0x63,
		Code::Numpad4 => 0x64,
		Code::Numpad5 => 0x65,
		Code::Numpad6 => 0x66,
		Code::Numpad7 => 0x67,
		Code::Numpad8 => 0x68,
		Code::Numpad9 => 0x69,
		Code::NumpadMultiply => 0x6a,
		Code::NumpadAdd => 0x6b,
		Code::NumpadSubtract => 0x6d,
		Code::NumpadDecimal => 0x6e,
		Code::NumpadDivide => 0x6f,
		Code::F1 => 0x70,
		Code::F2 => 0x71,
		Code::F3 => 0x72,
		Code::F4 => 0x73,
		Code::F5 => 0x74,
		Code::F6 => 0x75,
		Code::F7 => 0x76,
		Code::F8 => 0x77,
		Code::F9 => 0x78,
		Code::F10 => 0x79,
		Code::F11 => 0x7a,
		Code::F12 => 0x7b,
		Code::F13 => 0x7c,
		Code::F14 => 0x7d,
		Code::F15 => 0x7e,
		Code::F16 => 0x7f,
		Code::F17 => 0x80,
		Code::F18 => 0x81,
		Code::F19 => 0x82,
		Code::F20 => 0x83,
		Code::NumLock => 0x90,
		Code::Semicolon => 0xba,
		Code::Equal => 0xbb,
		Code::Comma => 0xbc,
		Code::Minus => 0xbd,
		Code::Period => 0xbe,
		Code::Slash => 0xbf,
		Code::Backquote => 0xc0,
		Code::BracketLeft => 0xdb,
		Code::Backslash => 0xdc,
		Code::BracketRight => 0xdd,
		Code::Quote => 0xde,
		_ => return None,
	})
}

/// 논리 좌표 `(x, y)` 를 Windows 의 화면 좌표(물리 픽셀)로 바꾼다. 커서가 있는 모니터의 배율을 쓴다 —
/// 링은 늘 그 모니터에 뜬다.
#[cfg(windows)]
pub fn to_physical(x: f64, y: f64) -> (i32, i32) {
	os::to_physical(x, y)
}

// Windows 의 화면 좌표는 물리 픽셀이다. 여기서는 커서가 있는 모니터의 배율로 나눠 논리 픽셀로 돌려준다 —
// `controller::visible_area` 가 모니터 영역을 같은 식으로 나눈다.
#[cfg(windows)]
mod os {
	use windows_sys::Win32::Foundation::POINT;
	use windows_sys::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromPoint};
	use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
	use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
		GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
	};
	use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

	/// 배율 100% 의 DPI.
	const BASE_DPI: f64 = 96.0;

	fn down(vk: u16) -> bool {
		// 맨 위 비트가 "지금 눌려 있다" 다.
		unsafe { GetAsyncKeyState(i32::from(vk)) < 0 }
	}

	fn cursor_point() -> POINT {
		let mut point = POINT { x: 0, y: 0 };
		unsafe {
			GetCursorPos(&mut point);
		}
		point
	}

	/// `point` 가 있는 모니터의 배율.
	fn scale_at(point: POINT) -> f64 {
		let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
		let ok = unsafe {
			let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
			GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) == 0
		};
		if ok && dpi_x > 0 {
			f64::from(dpi_x) / BASE_DPI
		} else {
			1.0
		}
	}

	pub(super) fn modifier_flags() -> u64 {
		let mut flags = 0;
		if down(VK_SHIFT) {
			flags |= super::FLAG_SHIFT;
		}
		if down(VK_CONTROL) {
			flags |= super::FLAG_CONTROL;
		}
		if down(VK_MENU) {
			flags |= super::FLAG_ALTERNATE;
		}
		if down(VK_LWIN) || down(VK_RWIN) {
			flags |= super::FLAG_COMMAND;
		}
		flags
	}

	pub(super) fn key_down(code: u16) -> bool {
		down(code)
	}

	pub(super) fn cursor() -> (f64, f64) {
		let point = cursor_point();
		let scale = scale_at(point);
		(f64::from(point.x) / scale, f64::from(point.y) / scale)
	}

	pub(super) fn to_physical(x: f64, y: f64) -> (i32, i32) {
		let scale = scale_at(cursor_point());
		((x * scale).round() as i32, (y * scale).round() as i32)
	}

	pub(super) fn warp_cursor(x: f64, y: f64) -> bool {
		let (x, y) = to_physical(x, y);
		unsafe { SetCursorPos(x, y) != 0 }
	}
}

#[cfg(not(any(target_os = "macos", windows)))]
mod os {
	pub(super) fn modifier_flags() -> u64 {
		0
	}

	pub(super) fn key_down(_code: u16) -> bool {
		false
	}

	pub(super) fn cursor() -> (f64, f64) {
		(0.0, 0.0)
	}

	pub(super) fn warp_cursor(_x: f64, _y: f64) -> bool {
		false
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn modifier_flags_map_each_modifier_to_its_own_bit() {
		assert_eq!(modifier_flags(Modifiers::SHIFT), FLAG_SHIFT);
		assert_eq!(modifier_flags(Modifiers::CONTROL), FLAG_CONTROL);
		assert_eq!(modifier_flags(Modifiers::ALT), FLAG_ALTERNATE);
		assert_eq!(modifier_flags(Modifiers::SUPER), FLAG_COMMAND);
		assert_eq!(
			modifier_flags(Modifiers::ALT | Modifiers::SHIFT),
			FLAG_ALTERNATE | FLAG_SHIFT
		);
		assert_eq!(modifier_flags(Modifiers::empty()), 0);
	}

	#[test]
	fn every_key_the_shortcut_input_offers_has_a_keycode() {
		// 설정의 `ShortcutField` 가 받는 키 — 글자, 숫자, F1~F12, 화살표, 그리고 이름 있는 키들.
		let combos = [
			"Alt+Space",
			"Cmd+Shift+KeyT",
			"Cmd+Shift+Digit2",
			"Ctrl+F12",
			"Alt+ArrowUp",
			"Cmd+Enter",
			"Cmd+Backquote",
			"Cmd+BracketLeft",
		];
		for combo in combos {
			let shortcut: Shortcut = combo.parse().expect("test combo parses");
			assert!(keycode(shortcut.key).is_some(), "{combo}");
		}
	}

	#[test]
	fn parsed_cmd_or_ctrl_maps_to_command_on_macos() {
		let shortcut: Shortcut = "CmdOrCtrl+KeyG".parse().unwrap();
		if cfg!(target_os = "macos") {
			assert_eq!(modifier_flags(shortcut.mods), FLAG_COMMAND);
		}
	}
}
