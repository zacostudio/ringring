// "열기" 칸의 대상을 고르는 창 — 파일과 폴더를 한 창에서 고른다
//
// dialog plugin 의 `open` 은 파일만 고르거나(`directory: false`) 폴더만 고른다(`directory: true`). 둘을 같이
// 고르게 하는 값이 없다. "파일 또는 폴더" 칸은 둘 다 골라야 하므로 `NSOpenPanel` 을 직접 연다.
// 설정 창에 붙는 sheet 다. 답은 main thread 의 block 으로 온다.
use tauri::WebviewWindow;

use crate::ring::model::OpenTarget;

/// 창이 무엇을 고르게 하는가.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelOptions {
	pub files: bool,
	pub directories: bool,
	/// 처음 보이는 폴더.
	pub start_dir: Option<&'static str>,
}

impl PanelOptions {
	/// 웹 주소는 고르는 것이 아니라 적는 것이다. 창이 없다.
	pub fn for_target(target: OpenTarget) -> Option<Self> {
		match target {
			// `.app` 은 폴더처럼 생긴 파일(package)이다. 파일로 고른다. 폴더까지 고르게 하면 `/Applications`
			// 자체가 골라진다.
			OpenTarget::App => Some(Self {
				files: true,
				directories: false,
				start_dir: Some("/Applications"),
			}),
			OpenTarget::File => Some(Self {
				files: true,
				directories: true,
				start_dir: None,
			}),
			OpenTarget::Url => None,
		}
	}
}

/// 창을 열고 고른 경로를 기다린다. 취소하면 `None`.
pub async fn choose(
	window: &WebviewWindow,
	options: PanelOptions,
) -> Result<Option<String>, String> {
	#[cfg(target_os = "macos")]
	{
		macos::choose(window, options).await
	}
	#[cfg(not(target_os = "macos"))]
	{
		other::choose(window, options).await
	}
}

// macOS 밖에서는 dialog plugin 을 쓴다. 파일과 폴더를 한 창에서 고를 수 없어 파일을 고른다 — 폴더는
// 칸에 경로를 적는다.
#[cfg(not(target_os = "macos"))]
mod other {
	use tauri::{Manager, WebviewWindow};
	use tauri_plugin_dialog::DialogExt;

	use super::PanelOptions;

	pub(super) async fn choose(
		window: &WebviewWindow,
		options: PanelOptions,
	) -> Result<Option<String>, String> {
		let mut dialog = window.app_handle().dialog().file().set_parent(window);
		if !options.directories {
			// 앱을 고르는 창이다. 시작 메뉴에서 시작한다 — 깔린 앱의 바로 가기가 거기 모여 있다.
			dialog = dialog.add_filter("Apps", &["exe", "lnk"]);
			if let Some(programs) = std::env::var_os("ProgramData") {
				dialog = dialog.set_directory(
					std::path::Path::new(&programs).join(r"Microsoft\Windows\Start Menu\Programs"),
				);
			}
		}
		let (tx, rx) = tokio::sync::oneshot::channel();
		dialog.pick_file(move |path| {
			let _ = tx.send(path);
		});
		let picked = rx.await.map_err(|e| e.to_string())?;
		picked
			.map(|path| {
				path.into_path()
					.map(|path| path.to_string_lossy().into_owned())
					.map_err(|e| e.to_string())
			})
			.transpose()
	}
}

#[cfg(target_os = "macos")]
mod macos {
	use std::ffi::{CStr, CString, c_char};
	use std::sync::Mutex;

	use block2::RcBlock;
	use objc2::rc::Retained;
	use objc2::runtime::{AnyObject, Bool};
	use objc2::{class, msg_send};
	use tauri::WebviewWindow;

	use super::PanelOptions;

	/// `NSModalResponseOK`.
	const RESPONSE_OK: isize = 1;

	pub async fn choose(
		window: &WebviewWindow,
		options: PanelOptions,
	) -> Result<Option<String>, String> {
		let (tx, rx) = tokio::sync::oneshot::channel::<Option<String>>();
		let tx = Mutex::new(Some(tx));
		let parent = window.clone();
		window
			.run_on_main_thread(move || {
				let answer = move |path: Option<String>| {
					if let Some(tx) = tx.lock().unwrap_or_else(|e| e.into_inner()).take() {
						let _ = tx.send(path);
					}
				};
				let Ok(ns_window) = parent.ns_window() else {
					answer(None);
					return;
				};
				// SAFETY: main thread 다. `ns_window` 는 살아 있는 창의 NSWindow 다. panel 은 block 이 쥐고 있어
				// 답이 올 때까지 산다.
				unsafe { begin(ns_window as *mut AnyObject, options, answer) };
			})
			.map_err(|e| e.to_string())?;
		rx.await.map_err(|e| e.to_string())
	}

	unsafe fn begin(
		ns_window: *mut AnyObject,
		options: PanelOptions,
		answer: impl Fn(Option<String>) + 'static,
	) {
		unsafe {
			let panel: Option<Retained<AnyObject>> = msg_send![class!(NSOpenPanel), openPanel];
			let Some(panel) = panel else {
				answer(None);
				return;
			};
			let _: () = msg_send![&*panel, setCanChooseFiles: Bool::new(options.files)];
			let _: () = msg_send![&*panel, setCanChooseDirectories: Bool::new(options.directories)];
			let _: () = msg_send![&*panel, setAllowsMultipleSelection: Bool::NO];
			let _: () = msg_send![&*panel, setCanCreateDirectories: Bool::NO];
			if let Some(dir) = options.start_dir.and_then(|dir| CString::new(dir).ok()) {
				let path: *mut AnyObject =
					msg_send![class!(NSString), stringWithUTF8String: dir.as_ptr()];
				let url: *mut AnyObject = msg_send![class!(NSURL), fileURLWithPath: path];
				let _: () = msg_send![&*panel, setDirectoryURL: url];
			}
			let held = panel.clone();
			let done = RcBlock::new(move |response: isize| {
				answer((response == RESPONSE_OK).then(|| chosen(&held)).flatten());
			});
			let _: () = msg_send![
				&*panel,
				beginSheetModalForWindow: ns_window,
				completionHandler: &*done
			];
		}
	}

	/// 고른 경로. 못 읽으면 `None`.
	unsafe fn chosen(panel: &AnyObject) -> Option<String> {
		unsafe {
			let url: *mut AnyObject = msg_send![panel, URL];
			if url.is_null() {
				return None;
			}
			let path: *mut AnyObject = msg_send![url, path];
			if path.is_null() {
				return None;
			}
			let text: *const c_char = msg_send![path, UTF8String];
			if text.is_null() {
				return None;
			}
			Some(CStr::from_ptr(text).to_string_lossy().into_owned())
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_file_target_can_choose_a_file_or_a_folder() {
		let options = PanelOptions::for_target(OpenTarget::File).unwrap();
		assert!(options.files);
		assert!(options.directories);
	}

	#[test]
	fn the_app_target_chooses_a_bundle_and_starts_in_applications() {
		let options = PanelOptions::for_target(OpenTarget::App).unwrap();
		assert!(options.files);
		assert!(!options.directories, "a folder is not an app");
		assert_eq!(options.start_dir, Some("/Applications"));
	}

	#[test]
	fn a_web_address_has_no_panel() {
		assert_eq!(PanelOptions::for_target(OpenTarget::Url), None);
	}
}
