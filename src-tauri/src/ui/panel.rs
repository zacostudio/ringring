// 링 창을 앱을 활성화하지 않는 NSPanel 로 바꾼다 — Tome 의 `ui/panel.rs` 에서 가져왔다 (Tome)
//
// **왜 panel 인가.** 보통 NSWindow 는 자기 앱이 활성일 때만 key 가 된다. 그래서 tao 의 `set_focus` 는
// `makeKeyAndOrderFront` 뒤에 `activateIgnoringOtherApps:YES` 를 부른다 (tao-0.35.3
// `platform_impl/macos/util/async.rs:231-237`). 앱이 활성화되면 열려 있던 설정 창까지 같이 앞으로 온다.
// `NSWindowStyleMaskNonactivatingPanel` 이 켜진 NSPanel 은 앱을 활성화하지 않고 key 가 된다 — Spotlight 와 같은
// 방식이다. 키보드는 panel 로 가고, 맨 앞 앱은 그대로이며, panel 을 숨기면 키보드가 그 앱으로 돌아간다.
//
// **어떻게 바꾸나.** Tauri 는 panel 을 만들 수 없다. 만든 창의 클래스를 NSPanel 하위 클래스로 바꾼다
// (`object_setClass`). tao 의 창 클래스 `TaoWindow` 는 NSWindow 에 `focusable`
// ivar 하나를 더한 것이다 (tao `window.rs:407-426`). 새 클래스도 같은 이름·타입의 ivar 를 가져 크기와 자리가 같다 —
// tao 의 `set_focusable` 이 이름으로 그 ivar 를 찾는다. 크기가 다르면 바꾸지 않는다 (바꾸면 메모리를 넘어 쓴다).
//
// **보이기는 [`place_and_front`] 다음 [`focus`] 다.** tao 의 `set_focus` 는 panel 이어도 앱을 활성화하므로 쓰지 않는다.
//
// **만들기는 [`build`] 다.** wry 는 webview 를 붙일 때마다 앱을 활성화한다. `build` 는 만드는 동안 그 활성화를 건너뛴다.

use tauri::WebviewWindow;

/// `window` 를 non-activating panel 로 바꾼다. 모든 Space 와 전체 화면 앱 위에 뜬다. 실패하면 보통 창으로
/// 남기고 로그만 남긴다 — 창은 그대로 쓸 수 있고, key 를 줄 때 앱이 활성화될 뿐이다.
pub(crate) fn make_nonactivating(window: &WebviewWindow) {
	#[cfg(target_os = "macos")]
	match macos::convert(window) {
		Ok(()) => log::info!("[panel] '{}' is now a non-activating panel", window.label()),
		Err(e) => log::warn!(
			"[panel] '{}' stays a regular window — giving it the keyboard will activate the app: {e}",
			window.label()
		),
	}
	#[cfg(not(target_os = "macos"))]
	let _ = window;
}

/// 창에 키보드 포커스를 준다. panel 이면 앱을 활성화하지 않는다 — 맨 앞 앱은 그대로다. panel 이 아니면
/// `set_focus` 다.
pub(crate) fn focus(window: &WebviewWindow) -> Result<(), String> {
	#[cfg(target_os = "macos")]
	if macos::is_panel(window) {
		return macos::make_key(window);
	}
	window.set_focus().map_err(|e| e.to_string())
}

/// 창을 화면의 `(x, y)` 로 옮기고 다른 앱의 창 위로 올린다. **key 는 주지 않는다** — 키보드는 맨 앞 앱에
/// 그대로 있다 (링의 hold). 좌표는 창의 왼쪽 위 모서리이고, 논리 픽셀이며 주 모니터의 왼쪽 위가
/// 원점이다.
///
/// **왜 Tauri 의 `set_position` 과 `show` 를 쓰지 않나.** tao 의 `set_outer_position` 은 옮기기를 main queue 에
/// 넣고 곧바로 돌아온다 (tao-0.35.3 `platform_impl/macos/util/async.rs:96-101`). 그 뒤에 창을 올리면 예전 자리에서
/// 한 번 보였다가 옮겨진다. 여기서는 같은 자리에서 옮기고 올린다. `show()` 는 앱이 활성일 때 창에 key 를 준다.
pub(crate) fn place_and_front(window: &WebviewWindow, x: f64, y: f64) -> Result<(), String> {
	#[cfg(target_os = "macos")]
	{
		macos::place_and_front(window, x, y)
	}
	#[cfg(not(target_os = "macos"))]
	{
		window
			.set_position(tauri::LogicalPosition::new(x, y))
			.map_err(|e| e.to_string())?;
		window.show().map_err(|e| e.to_string())
	}
}

/// 창을 다른 앱의 창 위로 올리기만 한다. key 도 주지 않고 앱도 활성화하지 않는다.
/// 개발용 확인에서 설정 창을 사용자의 키보드를 건드리지 않고 보일 때 쓴다.
pub(crate) fn order_front(window: &WebviewWindow) -> Result<(), String> {
	#[cfg(target_os = "macos")]
	{
		macos::order_front(window)
	}
	#[cfg(not(target_os = "macos"))]
	{
		window.show().map_err(|e| e.to_string())
	}
}

/// 이 앱이 지금 맨 앞 앱인가. 개발용 확인이 사용자의 포커스를 가져갔는지 볼 때 쓴다.
#[cfg(feature = "dev-agent")]
pub(crate) fn app_is_active() -> bool {
	#[cfg(target_os = "macos")]
	{
		macos::app_is_active()
	}
	#[cfg(not(target_os = "macos"))]
	{
		false
	}
}

/// 이 앱의 활성 상태를 내려놓는다. 키보드가 그 전에 앞에 있던 앱으로 돌아간다. main thread 에서 부른다.
#[cfg(feature = "dev-agent")]
pub(crate) fn deactivate_app() {
	#[cfg(target_os = "macos")]
	macos::deactivate_app();
}

/// panel 이 될 창을 만든다. 만드는 동안 앱을 활성화하지 않는다.
///
/// **왜 따로 만드나.** wry 는 창에 webview 를 붙일 때마다 `NSApp activate` 를 부른다 — 조건 없이 (wry-0.55.1
/// `wkwebview/mod.rs:690-700`, "make sure the window is always on top"). 다른 앱이 앞일 때 이 창을 처음 만들면 그
/// 한 번의 활성화로 이 앱이 맨 앞 앱이 된다 (Tome 에서 실측). 그래서 만드는 동안만 그 호출을 건너뛴다.
///
/// **만드는 동안이 언제 끝나나.** main thread 가 아닌 곳에서 `build()` 는 만들기를 event loop 에 넣고 곧바로
/// 돌아온다 (tauri-runtime-wry `send_user_message`). 그래서 main thread 에서 한 번 더 돌아 그 만들기가 끝난 뒤에
/// 막기를 푼다.
pub(crate) fn build<M: tauri::Manager<tauri::Wry>>(
	builder: tauri::WebviewWindowBuilder<'_, tauri::Wry, M>,
) -> tauri::Result<WebviewWindow> {
	#[cfg(target_os = "macos")]
	{
		let _quiet = macos::ActivationGate::hold();
		let window = builder.build()?;
		if let Err(e) = macos::wait_for_main(&window) {
			log::warn!(
				"[panel] '{}' main-thread barrier failed: {e}",
				window.label()
			);
		}
		Ok(window)
	}
	#[cfg(not(target_os = "macos"))]
	builder.build()
}

#[cfg(target_os = "macos")]
mod macos {
	use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
	use objc2::{class, msg_send, sel};
	use std::collections::HashMap;
	use std::sync::{Mutex, OnceLock};
	use tauri::WebviewWindow;

	/// `NSWindowStyleMaskNonactivatingPanel`.
	const STYLE_NONACTIVATING_PANEL: usize = 1 << 7;
	/// `NSWindowCollectionBehavior` 의 비트.
	const BEHAVIOR_CAN_JOIN_ALL_SPACES: usize = 1 << 0;
	const BEHAVIOR_FULL_SCREEN_AUXILIARY: usize = 1 << 8;

	const CLASS_NAME: &std::ffi::CStr = c"RingRingNonactivatingPanel";

	extern "C" fn no(_: &AnyObject, _: Sel) -> Bool {
		Bool::NO
	}

	thread_local! {
		/// [`make_key`] 가 key 로 만드는 중이다. main thread 에서만 쓴다.
		static MAKING_KEY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
	}

	/// `NSEventType` 의 누름 셋 — 왼쪽·오른쪽·그 밖의 단추.
	const MOUSE_DOWN_EVENT_TYPES: [usize; 3] = [1, 3, 25];

	/// key 가 될 수 있나.
	///
	/// **앱이 비활성일 때는 사람이 누르거나 [`make_key`] 가 부를 때만 된다.** 비활성 앱에서 key panel 이 닫히면
	/// AppKit 이 같은 앱의 다른 panel 을 key 로 올린다 (Tome 에서 실측). 그 승격만 막는다 — 앱이 활성이면
	/// 보통 창과 같다.
	extern "C" fn can_become_key(this: &AnyObject, _: Sel) -> Bool {
		if MAKING_KEY.with(std::cell::Cell::get) {
			return Bool::YES;
		}
		unsafe {
			let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
			let active: bool = msg_send![app, isActive];
			if active {
				return Bool::YES;
			}
			let event: *mut AnyObject = msg_send![app, currentEvent];
			if event.is_null() {
				return Bool::NO;
			}
			let kind: usize = msg_send![event, type];
			let target: *mut AnyObject = msg_send![event, window];
			Bool::new(
				MOUSE_DOWN_EVENT_TYPES.contains(&kind)
					&& std::ptr::eq(target, this as *const AnyObject as *mut AnyObject),
			)
		}
	}

	/// 바꾸기 전 클래스. NSWindow 포인터 → 클래스.
	///
	/// **닫기 전에 되돌려야 한다.** AppKit 은 창을 만들 때 그 창에 KVO observer 를 건다(`_windowLayerContext`).
	/// 클래스를 바꾼 채로 해제하면 `-[NSWindow dealloc]` 이 그 observer 를 떼다가 `NSRangeException` 을 던진다 —
	/// Rust 는 이 예외를 잡지 못해 앱이 abort 한다 (Tome 에서 실측. 닫기 전에 원래 클래스로 되돌리면 사라진다).
	fn original_classes() -> &'static Mutex<HashMap<usize, &'static AnyClass>> {
		static ORIGINAL: OnceLock<Mutex<HashMap<usize, &'static AnyClass>>> = OnceLock::new();
		ORIGINAL.get_or_init(Default::default)
	}

	/// `-close` 를 가로챈다. 원래 클래스로 되돌린 뒤 그 클래스의 `close` 를 부른다. 창을 부수는 길은 모두 여기를
	/// 지난다 — 창 닫기 단추와 `destroy()` 는 tao 가 창을 버리며 `close` 를 부른다 (tao `window.rs:1739-1744`).
	/// 숨기기(`orderOut:`)는 여기를 지나지 않는다.
	extern "C" fn close(this: &AnyObject, _: Sel) {
		let original = original_classes()
			.lock()
			.unwrap_or_else(|e| e.into_inner())
			.remove(&(this as *const AnyObject as usize));
		match original {
			Some(original) => unsafe {
				AnyObject::set_class(this, original);
				let _: () = msg_send![this, close];
			},
			// 기록이 없으면 되돌릴 곳을 모른다. NSPanel 의 close 로 닫는다.
			None => unsafe {
				let _: () = msg_send![super(this, class!(NSPanel)), close];
			},
		}
	}

	/// NSPanel 하위 클래스. 한 번만 등록한다.
	fn panel_class() -> &'static AnyClass {
		static CLASS: OnceLock<&'static AnyClass> = OnceLock::new();
		CLASS.get_or_init(|| {
			if let Some(existing) = AnyClass::get(CLASS_NAME) {
				return existing;
			}
			let mut builder = ClassBuilder::new(CLASS_NAME, class!(NSPanel))
				.expect("RingRingNonactivatingPanel is registered only here");
			// tao 의 `TaoWindow` 와 같은 ivar — 이름으로 찾으므로 이름·타입이 같아야 한다.
			builder.add_ivar::<Bool>(c"focusable");
			// 제목 줄 없는 창도 key 가 되어야 글을 받는다 (NSWindow 기본값은 borderless 면 NO) — 언제 되는지는
			// `can_become_key` 가 정한다. main 은 되지 않는다 — 설정 창의 main 자리를 빼앗지 않는다.
			unsafe {
				builder.add_method(
					sel!(canBecomeKeyWindow),
					can_become_key as extern "C" fn(_, _) -> _,
				);
				builder.add_method(sel!(canBecomeMainWindow), no as extern "C" fn(_, _) -> _);
				builder.add_method(sel!(close), close as extern "C" fn(_, _));
			}
			builder.register()
		})
	}

	fn is_main_thread() -> bool {
		unsafe { msg_send![class!(NSThread), isMainThread] }
	}

	/// AppKit 호출은 main thread 에서 한다. 이미 main 이면 곧바로, 아니면 main 에 보내고 끝날 때까지 기다린다.
	fn on_main<R: Send + 'static>(
		window: &WebviewWindow,
		f: impl FnOnce(&AnyObject) -> R + Send + 'static,
	) -> Result<R, String> {
		let ns_window = window.ns_window().map_err(|e| e.to_string())? as usize;
		// SAFETY: 창이 사는 동안의 NSWindow 포인터다. main thread 에서만 만진다.
		let run = move || f(unsafe { &*(ns_window as *const AnyObject) });
		if is_main_thread() {
			return Ok(run());
		}
		let (tx, rx) = std::sync::mpsc::channel();
		window
			.run_on_main_thread(move || {
				let _ = tx.send(run());
			})
			.map_err(|e| e.to_string())?;
		rx.recv().map_err(|e| e.to_string())
	}

	pub(super) fn convert(window: &WebviewWindow) -> Result<(), String> {
		on_main(window, move |ns_window| unsafe {
			let panel = panel_class();
			if is_panel_object(ns_window) {
				return Ok(());
			}
			let current = ns_window.class();
			if current.instance_size() != panel.instance_size() {
				return Err(format!(
					"{} is {} bytes but the panel class is {} — not swapping",
					current.name().to_string_lossy(),
					current.instance_size(),
					panel.instance_size()
				));
			}
			original_classes()
				.lock()
				.unwrap_or_else(|e| e.into_inner())
				.insert(ns_window as *const AnyObject as usize, current);
			AnyObject::set_class(ns_window, panel);

			let mask: usize = msg_send![ns_window, styleMask];
			let _: () = msg_send![ns_window, setStyleMask: mask | STYLE_NONACTIVATING_PANEL];
			prevent_activation(ns_window);
			// NSPanel 은 기본으로 앱이 비활성이 되면 숨는다. 이 창들은 다른 앱이 앞에 있을 때 쓰는 창이다.
			let _: () = msg_send![ns_window, setHidesOnDeactivate: Bool::NO];
			let _: () = msg_send![ns_window, setBecomesKeyOnlyIfNeeded: Bool::NO];

			let behavior: usize = msg_send![ns_window, collectionBehavior];
			let behavior = behavior | BEHAVIOR_CAN_JOIN_ALL_SPACES | BEHAVIOR_FULL_SCREEN_AUXILIARY;
			let _: () = msg_send![ns_window, setCollectionBehavior: behavior];
			Ok(())
		})?
	}

	/// 창을 누르는 것만으로 앱이 활성화되지 않게 한다.
	///
	/// 만든 뒤에 style mask 만 바꾸면 창 서버 쪽 표시는 그대로라 클릭 한 번에 앱이 활성화된다 (Tome 에서 실측).
	/// AppKit 이 panel 을 만들 때 스스로 부르는 private 메서드로 그 표시를 맞춘다. 그 메서드가 없는 macOS 에서는
	/// 띄우기와 숨기기는 여전히 활성화하지 않지만, panel 을 누르면 활성화된다.
	unsafe fn prevent_activation(ns_window: &AnyObject) {
		let sel = sel!(_setPreventsActivation:);
		let responds: bool = unsafe { msg_send![ns_window, respondsToSelector: sel] };
		if responds {
			let _: () = unsafe { msg_send![ns_window, _setPreventsActivation: Bool::YES] };
		} else {
			log::warn!(
				"[panel] _setPreventsActivation: is gone — clicking the ring will activate the app"
			);
		}
	}

	pub(super) fn is_panel(window: &WebviewWindow) -> bool {
		window
			.ns_window()
			// SAFETY: 살아 있는 NSWindow 포인터. 클래스를 읽기만 한다.
			.map(|ptr| is_panel_object(unsafe { &*(ptr as *const AnyObject) }))
			.unwrap_or(false)
	}

	/// 이 창이 panel 클래스이거나 그 하위 클래스인가.
	///
	/// **클래스를 그대로 견주면 안 된다.** 누가 창에 KVO observer 를 걸면 AppKit 이 창의 클래스를
	/// `NSKVONotifying_RingRingNonactivatingPanel` 로 바꾼다. 그때 같은 클래스인지만 물으면 panel 이 아니라고 답하고,
	/// [`super::focus`] 가 tao 의 `set_focus` 로 빠져 앱을 활성화한다 (Tome 에서 실측).
	fn is_panel_object(ns_window: &AnyObject) -> bool {
		unsafe { msg_send![ns_window, isKindOfClass: panel_class()] }
	}

	/// 앞으로 가져오고 key 로 만든다. 앱은 활성화하지 않는다.
	pub(super) fn make_key(window: &WebviewWindow) -> Result<(), String> {
		on_main(window, |ns_window| unsafe {
			let minimized: bool = msg_send![ns_window, isMiniaturized];
			if minimized {
				let _: () = msg_send![ns_window, deminiaturize: std::ptr::null::<AnyObject>()];
			}
			// 앱이 비활성이면 `orderFront:` 는 맨 앞 앱의 창 위로 올리지 않는다 (Tome 에서 실측) — `Regardless` 를 쓴다.
			let _: () = msg_send![ns_window, orderFrontRegardless];
			MAKING_KEY.with(|flag| flag.set(true));
			let _: () = msg_send![ns_window, makeKeyWindow];
			MAKING_KEY.with(|flag| flag.set(false));
		})
	}

	/// `-setFrameTopLeftPoint:` 에 넘길 점. AppKit 의 `NSPoint`(= `CGPoint`) 와 같은 모양이다.
	#[repr(C)]
	#[derive(Clone, Copy)]
	struct Point {
		x: f64,
		y: f64,
	}

	// SAFETY: `CGPoint` 는 `CGFloat`(64-bit 에서 f64) 둘이다. 이름과 필드가 그 encoding 과 같다.
	unsafe impl objc2::Encode for Point {
		const ENCODING: objc2::Encoding =
			objc2::Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
	}

	#[repr(C)]
	struct DisplayRect {
		origin: Point,
		width: f64,
		height: f64,
	}

	#[link(name = "CoreGraphics", kind = "framework")]
	unsafe extern "C" {
		fn CGMainDisplayID() -> u32;
		fn CGDisplayBounds(display: u32) -> DisplayRect;
	}

	/// 창의 왼쪽 위를 `(x, y)` 에 두고 앞으로 올린다. key 는 주지 않는다.
	pub(super) fn place_and_front(window: &WebviewWindow, x: f64, y: f64) -> Result<(), String> {
		on_main(window, move |ns_window| unsafe {
			// AppKit 의 화면 좌표는 주 모니터의 왼쪽 **아래** 가 원점이고 y 가 위로 자란다.
			let main_height = CGDisplayBounds(CGMainDisplayID()).height;
			let point = Point {
				x,
				y: main_height - y,
			};
			let _: () = msg_send![ns_window, setFrameTopLeftPoint: point];
			let _: () = msg_send![ns_window, orderFrontRegardless];
		})
	}

	pub(super) fn order_front(window: &WebviewWindow) -> Result<(), String> {
		on_main(window, |ns_window| unsafe {
			let _: () = msg_send![ns_window, orderFrontRegardless];
		})
	}

	#[cfg(feature = "dev-agent")]
	pub(super) fn app_is_active() -> bool {
		unsafe {
			let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
			msg_send![app, isActive]
		}
	}

	#[cfg(feature = "dev-agent")]
	pub(super) fn deactivate_app() {
		unsafe {
			let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
			// `deactivate` 만으로는 맨 앞 자리를 내놓지 않는다 (실측). 숨기면 그 전에 앞에 있던 앱이 올라온다.
			let _: () = msg_send![app, hide: std::ptr::null::<AnyObject>()];
		}
	}

	/// main thread 에서 한 번 돈다 — 그 앞에 쌓인 일(창 만들기)이 끝날 때까지 기다린다.
	pub(super) fn wait_for_main(window: &WebviewWindow) -> Result<(), String> {
		on_main(window, |_| ())
	}

	/// 0 보다 크면 `NSApp activate` 를 건너뛴다. [`ActivationGate`] 만 바꾼다.
	static ACTIVATION_HOLDS: std::sync::atomic::AtomicUsize =
		std::sync::atomic::AtomicUsize::new(0);
	/// 앱 클래스의 부모 — 건너뛰지 않을 때 원래 `activate` 를 부를 곳.
	static APP_SUPERCLASS: OnceLock<&'static AnyClass> = OnceLock::new();

	fn activation_held() -> bool {
		ACTIVATION_HOLDS.load(std::sync::atomic::Ordering::SeqCst) > 0
	}

	/// 개발용 확인에서, 막지 않은 활성화를 누가 불렀는지 로그에 남긴다.
	#[cfg(feature = "dev-agent")]
	fn trace_activation(selector: &str) {
		log::info!(
			"[panel] NSApp {selector} called outside the gate:\n{}",
			std::backtrace::Backtrace::force_capture()
		);
	}

	#[cfg(not(feature = "dev-agent"))]
	fn trace_activation(_selector: &str) {}

	extern "C" fn activate(this: &AnyObject, _: Sel) {
		if activation_held() {
			log::debug!("[panel] skipped NSApp activate while building a panel window");
			return;
		}
		trace_activation("activate");
		if let Some(superclass) = APP_SUPERCLASS.get() {
			unsafe {
				let _: () = msg_send![super(this, superclass), activate];
			}
		}
	}

	extern "C" fn activate_ignoring_other_apps(this: &AnyObject, _: Sel, flag: Bool) {
		if activation_held() {
			log::debug!(
				"[panel] skipped NSApp activateIgnoringOtherApps: while building a panel window"
			);
			return;
		}
		trace_activation("activateIgnoringOtherApps:");
		if let Some(superclass) = APP_SUPERCLASS.get() {
			unsafe {
				let _: () = msg_send![super(this, superclass), activateIgnoringOtherApps: flag];
			}
		}
	}

	/// 잡고 있는 동안 `NSApp activate` 를 건너뛴다. 놓으면(drop) 풀린다.
	///
	/// 앱 클래스(tao 의 `TaoApp`)에 두 메서드를 더해 가로챈다 — NSApplication 의 것은 건드리지 않는다. 앱 클래스가
	/// 이미 그 메서드를 가지고 있으면 더하지 못하고, 그때는 막지 않은 채 로그만 남긴다. 잡고 있는 사이에는 다른 길의
	/// 활성화(`set_focus` 등)도 건너뛴다 — 창 하나를 만드는 동안뿐이다.
	pub(super) struct ActivationGate;

	impl ActivationGate {
		pub(super) fn hold() -> Self {
			install_activation_overrides();
			ACTIVATION_HOLDS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
			ActivationGate
		}
	}

	impl Drop for ActivationGate {
		fn drop(&mut self) {
			ACTIVATION_HOLDS.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
		}
	}

	fn install_activation_overrides() {
		static ONCE: std::sync::Once = std::sync::Once::new();
		ONCE.call_once(|| unsafe {
			let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
			// `-class` 는 KVO 가 끼운 클래스를 감추고 앱이 만든 클래스를 돌려준다.
			let app_class: *const AnyClass = msg_send![app, class];
			let Some(app_class) = app_class.as_ref() else {
				return;
			};
			let Some(superclass) = app_class.superclass() else {
				return;
			};
			let _ = APP_SUPERCLASS.set(superclass);
			let add = |sel: Sel, imp: objc2::runtime::Imp| {
				let Some(method) = superclass.instance_method(sel) else {
					return; // 이 macOS 에 없는 메서드 (`activate` 는 14+)
				};
				let types = objc2::ffi::method_getTypeEncoding(method);
				let added = objc2::ffi::class_addMethod(
					app_class as *const AnyClass as *mut AnyClass,
					sel,
					imp,
					types,
				);
				if !added.as_bool() {
					log::warn!(
						"[panel] {} already overrides {sel:?} — creating the ring window may activate the app",
						app_class.name().to_string_lossy()
					);
				}
			};
			add(
				sel!(activate),
				std::mem::transmute::<extern "C" fn(&AnyObject, Sel), objc2::runtime::Imp>(
					activate,
				),
			);
			add(
				sel!(activateIgnoringOtherApps:),
				std::mem::transmute::<extern "C" fn(&AnyObject, Sel, Bool), objc2::runtime::Imp>(
					activate_ignoring_other_apps,
				),
			);
		});
	}
}
