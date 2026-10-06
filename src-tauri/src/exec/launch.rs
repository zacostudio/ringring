// 앱을 인자와 함께 띄운다 — "열기" 칸에 인자가 있을 때만 쓴다

//! Without arguments an app opens through the opener plugin, as before. With
//! arguments the OS has to start the program itself, and each OS does that its
//! own way.

use std::path::Path;

/// Start the app at `path` with `args` (one line of text, as the user typed it).
/// Returns as soon as the OS has taken the request. `Err` is a sentence for the
/// user; it holds no output of the app.
pub fn app_with_args(path: &Path, args: &str) -> Result<(), String> {
	os::launch(path, args)
}

// `.app` 은 실행 파일이 아니라 폴더다. `open` 이 그 안의 실행 파일을 찾아 띄운다.
//
// `--args` 는 앱을 새로 띄울 때만 전해진다. 이미 떠 있는 앱은 앞으로 오기만 하고 인자를 받지 않는다 —
// 창을 하나 더 띄우는 `-n` 은 쓰지 않는다. 같은 앱이 둘 뜨는 쪽이 더 놀랍다.
#[cfg(target_os = "macos")]
mod os {
	use std::path::Path;
	use std::process::{Command, Stdio};

	use crate::ring::model::split_args;

	pub(super) fn launch(path: &Path, args: &str) -> Result<(), String> {
		let status = Command::new("/usr/bin/open")
			.arg("-a")
			.arg(path)
			.arg("--args")
			.args(split_args(args))
			.stdin(Stdio::null())
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map_err(|e| format!("Failed to run open: {e}"))?;
		if status.success() {
			Ok(())
		} else {
			Err(format!("open could not start {}", path.display()))
		}
	}
}

// Windows 의 프로그램은 인자를 한 줄의 글로 받아 스스로 나눈다. 그래서 적은 글을 나누지 않고 그대로 넘긴다.
// `ShellExecuteW` 는 `.exe` 뿐 아니라 바로 가기(`.lnk`)와 `.bat` 도 띄운다. 시작 폴더는 그 파일이 있는 폴더다 —
// 탐색기에서 두 번 눌렀을 때와 같다.
#[cfg(windows)]
mod os {
	use std::os::windows::ffi::OsStrExt;
	use std::path::Path;

	use windows_sys::Win32::UI::Shell::ShellExecuteW;
	use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

	fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
		text.encode_wide().chain(std::iter::once(0)).collect()
	}

	pub(super) fn launch(path: &Path, args: &str) -> Result<(), String> {
		let file = wide(path.as_os_str());
		let params = wide(args.as_ref());
		let folder = path.parent().map(|parent| wide(parent.as_os_str()));
		// SAFETY: 모든 글은 NUL 로 끝나고 이 호출이 끝날 때까지 살아 있다.
		let code = unsafe {
			ShellExecuteW(
				std::ptr::null_mut(),
				wide("open".as_ref()).as_ptr(),
				file.as_ptr(),
				params.as_ptr(),
				folder.as_ref().map_or(std::ptr::null(), |f| f.as_ptr()),
				SW_SHOWNORMAL,
			)
		};
		// 32 보다 크면 성공이다. 그 아래는 오류 코드다.
		if code as usize > 32 {
			Ok(())
		} else {
			Err(format!(
				"Windows could not start {} (code {})",
				path.display(),
				code as usize
			))
		}
	}
}

#[cfg(not(any(target_os = "macos", windows)))]
mod os {
	use std::path::Path;
	use std::process::{Command, Stdio};

	use crate::ring::model::split_args;

	pub(super) fn launch(path: &Path, args: &str) -> Result<(), String> {
		Command::new(path)
			.args(split_args(args))
			.stdin(Stdio::null())
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.spawn()
			.map(|_| ())
			.map_err(|e| format!("Failed to start {}: {e}", path.display()))
	}
}
