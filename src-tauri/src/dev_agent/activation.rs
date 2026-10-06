// dev-agent: 이 앱이 맨 앞 앱이 되는 순간을 로그에 남긴다 — 누가 불렀는지(call stack)와 그때의 이벤트
//
// 메뉴 막대 앱은 사용자가 창을 열 때만 앞에 나와야 한다. 무엇이 앱을 앞으로 가져오는지는 코드만 읽어서는
// 알 수 없다 — AppKit 안에서 일어난다. 그래서 일어나는 순간의 stack 을 찍는다.
use std::ffi::{CStr, c_char};

use block2::RcBlock;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

/// 로그에 남기는 stack 의 줄 수.
const STACK_LINES: usize = 28;

unsafe fn text(object: *mut AnyObject) -> String {
	unsafe {
		if object.is_null() {
			return String::new();
		}
		let description: *mut AnyObject = msg_send![object, description];
		if description.is_null() {
			return String::new();
		}
		let utf8: *const c_char = msg_send![description, UTF8String];
		if utf8.is_null() {
			return String::new();
		}
		CStr::from_ptr(utf8).to_string_lossy().into_owned()
	}
}

/// 관찰자를 건다. main thread 에서 한 번 부른다.
pub fn install() {
	// SAFETY: main thread 다. block 은 알림 센터가 쥐고 있어 앱이 끝날 때까지 산다.
	unsafe {
		let center: *mut AnyObject = msg_send![class!(NSNotificationCenter), defaultCenter];
		for (name, label) in [
			(
				c"NSApplicationWillBecomeActiveNotification",
				"will become active",
			),
			(
				c"NSApplicationDidResignActiveNotification",
				"did resign active",
			),
		] {
			let name: *mut AnyObject =
				msg_send![class!(NSString), stringWithUTF8String: name.as_ptr()];
			let block = RcBlock::new(move |_note: *mut AnyObject| {
				let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
				let event: *mut AnyObject = msg_send![app, currentEvent];
				let stack: *mut AnyObject = msg_send![class!(NSThread), callStackSymbols];
				let frames: Vec<String> = text(stack)
					.lines()
					.take(STACK_LINES)
					.map(|line| line.trim().to_string())
					.collect();
				log::info!(
					"[activation] {label} — event: {} — stack: {}",
					text(event).replace('\n', " "),
					frames.join(" | ")
				);
			});
			let _: *mut AnyObject = msg_send![
				center,
				addObserverForName: name,
				object: std::ptr::null::<AnyObject>(),
				queue: std::ptr::null::<AnyObject>(),
				usingBlock: &*block
			];
		}
	}
}

/// AppKit 의 `NSRect`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
	x: f64,
	y: f64,
	width: f64,
	height: f64,
}

// SAFETY: `CGRect` 는 `CGPoint`(f64 둘)와 `CGSize`(f64 둘)다. 필드의 순서와 크기가 같다.
unsafe impl objc2::Encode for Rect {
	const ENCODING: objc2::Encoding = objc2::Encoding::Struct(
		"CGRect",
		&[
			objc2::Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]),
			objc2::Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]),
		],
	);
}

/// tao 와 wry 를 거치지 않고 제목 줄이 있는 보통 창을 만들어 앞으로 올린다. 돌려주는 것은 그 창의 주소다.
/// main thread 에서 부른다.
pub fn raw_window() -> usize {
	/// `NSWindowStyleMaskTitled | NSWindowStyleMaskClosable`.
	const STYLE: usize = 1 | 2;
	/// `NSBackingStoreBuffered`.
	const BUFFERED: usize = 2;
	// SAFETY: main thread 다. 창은 `close_raw_window` 가 닫을 때까지 산다 (`releasedWhenClosed` 를 끈다).
	unsafe {
		let window: *mut AnyObject = msg_send![class!(NSWindow), alloc];
		let frame = Rect {
			x: 200.0,
			y: 200.0,
			width: 360.0,
			height: 240.0,
		};
		let window: *mut AnyObject = msg_send![
			window,
			initWithContentRect: frame,
			styleMask: STYLE,
			backing: BUFFERED,
			defer: objc2::runtime::Bool::NO
		];
		let _: () = msg_send![window, setReleasedWhenClosed: objc2::runtime::Bool::NO];
		let _: () = msg_send![window, orderFrontRegardless];
		window as usize
	}
}

pub fn close_raw_window(window: usize) {
	// SAFETY: `raw_window` 가 돌려준 살아 있는 창이다. main thread 다.
	unsafe {
		let window = window as *mut AnyObject;
		let _: () = msg_send![window, close];
		let _: () = msg_send![window, release];
	}
}
