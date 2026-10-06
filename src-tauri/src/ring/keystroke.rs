// 맨 앞 앱에 키 입력을 보낸다 — "키 입력 보내기" 칸
//
// 손쉬운 사용(Accessibility) 권한이 있어야 한다. 권한이 없으면 보내지 않는다. 권한 창도 띄우지 않는다 —
// 시스템 설정을 여는 것은 사용자가 설정 화면의 단추를 눌렀을 때뿐이다.
//
// 보내는 thread 는 main thread 가 아니다. 수식키가 떨어지기를 기다리고 조합 사이에 쉬기 때문이다.

use std::time::{Duration, Instant};

use tauri_plugin_global_shortcut::Shortcut;

use super::platform;

/// 사용자가 누르고 있던 수식키가 떨어지기를 기다리는 최대 시간.
///
/// 기다리지 않으면 `⌥Space` 로 링을 띄운 사용자의 `⌥` 가 보내는 키에 섞인다.
const MODIFIER_RELEASE_TIMEOUT: Duration = Duration::from_millis(400);
const MODIFIER_POLL_INTERVAL: Duration = Duration::from_millis(10);
/// 조합과 조합 사이.
const COMBO_GAP: Duration = Duration::from_millis(12);

/// 보내지 못한 이유.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendError {
	/// 손쉬운 사용 권한이 없다.
	NotTrusted,
	/// 사용자가 수식키를 계속 누르고 있다.
	ModifiersHeld,
	/// 조합을 읽을 수 없거나 이 키보드에 없는 키다.
	BadCombo(String),
}

/// 저장된 조합 글자를 읽는다. 저장할 때 같은 파서로 검증했으므로 여기서 실패하는 것은 손상된 값이다.
pub fn parse(combos: &[String]) -> Result<Vec<Shortcut>, SendError> {
	combos
		.iter()
		.map(|combo| {
			let shortcut = combo
				.parse::<Shortcut>()
				.map_err(|_| SendError::BadCombo(combo.clone()))?;
			if platform::keycode(shortcut.key).is_none() {
				return Err(SendError::BadCombo(combo.clone()));
			}
			Ok(shortcut)
		})
		.collect()
}

/// 이 앱이 다른 앱에 키 입력을 보낼 수 있는가.
pub fn trusted() -> bool {
	os::trusted()
}

/// 조합을 차례로 맨 앞 앱에 보낸다. 끝날 때까지 이 thread 를 잡는다 — main thread 에서 부르지 않는다.
pub fn send(combos: &[String]) -> Result<(), SendError> {
	let shortcuts = parse(combos)?;
	if !trusted() {
		return Err(SendError::NotTrusted);
	}
	let deadline = Instant::now() + MODIFIER_RELEASE_TIMEOUT;
	while platform::any_modifier_down() {
		if Instant::now() >= deadline {
			return Err(SendError::ModifiersHeld);
		}
		std::thread::sleep(MODIFIER_POLL_INTERVAL);
	}
	for (index, shortcut) in shortcuts.iter().enumerate() {
		if index > 0 {
			std::thread::sleep(COMBO_GAP);
		}
		let Some(code) = platform::keycode(shortcut.key) else {
			continue;
		};
		os::post_key(code, platform::modifier_flags(shortcut.mods));
	}
	Ok(())
}

#[cfg(target_os = "macos")]
mod os {
	use std::ffi::c_void;

	#[link(name = "ApplicationServices", kind = "framework")]
	unsafe extern "C" {
		fn AXIsProcessTrusted() -> bool;
		fn CGEventCreateKeyboardEvent(
			source: *const c_void,
			virtual_key: u16,
			key_down: bool,
		) -> *const c_void;
		fn CGEventSetFlags(event: *const c_void, flags: u64);
		fn CGEventPost(tap: u32, event: *const c_void);
		fn CFRelease(cf: *const c_void);
	}

	/// `kCGHIDEventTap` — 하드웨어 입력이 들어오는 자리에 넣는다. 맨 앞 앱이 받는다.
	const HID_EVENT_TAP: u32 = 0;

	pub(super) fn trusted() -> bool {
		unsafe { AXIsProcessTrusted() }
	}

	/// 키 하나를 눌렀다 뗀다. 수식키는 이벤트의 flags 로 싣는다 — 수식키를 따로 누르는 이벤트는 보내지 않는다.
	pub(super) fn post_key(code: u16, flags: u64) {
		for key_down in [true, false] {
			unsafe {
				let event = CGEventCreateKeyboardEvent(std::ptr::null(), code, key_down);
				if event.is_null() {
					return;
				}
				CGEventSetFlags(event, flags);
				CGEventPost(HID_EVENT_TAP, event);
				CFRelease(event);
			}
		}
	}
}

#[cfg(windows)]
mod os {
	use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
		INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_CONTROL,
		VK_LWIN, VK_MENU, VK_SHIFT,
	};

	use super::platform;

	/// Windows 에는 손쉬운 사용 같은 권한이 없다. 관리자 권한으로 도는 앱만 입력을 받지 않는다 (UIPI).
	pub(super) fn trusted() -> bool {
		true
	}

	fn key(vk: u16, up: bool) -> INPUT {
		INPUT {
			r#type: INPUT_KEYBOARD,
			Anonymous: INPUT_0 {
				ki: KEYBDINPUT {
					wVk: vk,
					wScan: 0,
					dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
					time: 0,
					dwExtraInfo: 0,
				},
			},
		}
	}

	/// 수식키를 누르고, 키를 눌렀다 떼고, 수식키를 거꾸로 뗀다. 한 번의 `SendInput` 이라 다른 입력이 끼지 않는다.
	pub(super) fn post_key(code: u16, flags: u64) {
		let modifiers: Vec<u16> = [
			(platform::FLAG_CONTROL, VK_CONTROL),
			(platform::FLAG_ALTERNATE, VK_MENU),
			(platform::FLAG_SHIFT, VK_SHIFT),
			(platform::FLAG_COMMAND, VK_LWIN),
		]
		.into_iter()
		.filter(|(flag, _)| flags & flag != 0)
		.map(|(_, vk)| vk)
		.collect();
		let mut inputs: Vec<INPUT> = modifiers.iter().map(|&vk| key(vk, false)).collect();
		inputs.push(key(code, false));
		inputs.push(key(code, true));
		inputs.extend(modifiers.iter().rev().map(|&vk| key(vk, true)));
		let sent = unsafe {
			SendInput(
				inputs.len() as u32,
				inputs.as_ptr(),
				std::mem::size_of::<INPUT>() as i32,
			)
		};
		if sent as usize != inputs.len() {
			log::warn!(
				"[keystroke] the system took {sent} of {} key events",
				inputs.len()
			);
		}
	}
}

#[cfg(not(any(target_os = "macos", windows)))]
mod os {
	pub(super) fn trusted() -> bool {
		false
	}

	pub(super) fn post_key(_code: u16, _flags: u64) {}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn stored_combos_parse_in_order() {
		let parsed = parse(&["Cmd+KeyC".to_string(), "Cmd+Shift+Digit4".to_string()]).unwrap();
		assert_eq!(parsed.len(), 2);
		let (key_c, digit_4) = if cfg!(windows) {
			(0x43, 0x34)
		} else {
			(0x08, 0x15)
		};
		assert_eq!(platform::keycode(parsed[0].key), Some(key_c));
		assert_eq!(platform::keycode(parsed[1].key), Some(digit_4));
	}

	#[test]
	fn an_unreadable_combo_is_named_in_the_error() {
		assert_eq!(
			parse(&["Cmd+NotAKey".to_string()]),
			Err(SendError::BadCombo("Cmd+NotAKey".to_string()))
		);
	}

	#[test]
	fn a_key_without_a_keycode_is_refused_before_anything_is_sent() {
		// 매체 키는 plugin 이 조합으로 받지만 키 코드 표에 없다.
		let media = "Cmd+MediaPlayPause".to_string();
		if media.parse::<Shortcut>().is_ok() {
			assert_eq!(parse(&[media.clone()]), Err(SendError::BadCombo(media)));
		}
	}
}
