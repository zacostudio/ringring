// dev-agent: `WKWebView.takeSnapshot` 으로 창의 webview 를 PNG 로 찍는다 — 화면 기록 권한이 필요 없다
use block2::RcBlock;
use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};
use tauri::{AppHandle, Manager};

/// `NSBitmapImageFileTypePNG`.
const FILE_TYPE_PNG: usize = 4;

pub async fn png(app: &AppHandle, label: &str) -> Result<Vec<u8>, String> {
	let window = app
		.get_webview_window(label)
		.ok_or_else(|| format!("window not found: {label}"))?;
	let (tx, rx) = tokio::sync::oneshot::channel::<Result<Vec<u8>, String>>();
	let tx = std::sync::Mutex::new(Some(tx));
	// `with_webview` 의 closure 는 main thread 에서 돈다. 끝났다는 block 도 main thread 로 온다.
	window
		.with_webview(move |webview| unsafe {
			let wk = webview.inner() as *mut AnyObject;
			let config: *mut AnyObject = msg_send![class!(WKSnapshotConfiguration), new];
			let _: () = msg_send![config, setAfterScreenUpdates: Bool::YES];
			let done = RcBlock::new(move |image: *mut AnyObject, error: *mut AnyObject| {
				let result = png_from(image, error);
				if let Some(tx) = tx.lock().unwrap_or_else(|e| e.into_inner()).take() {
					let _ = tx.send(result);
				}
			});
			let _: () =
				msg_send![wk, takeSnapshotWithConfiguration: config, completionHandler: &*done];
			let _: () = msg_send![config, release];
		})
		.map_err(|e| e.to_string())?;
	rx.await.map_err(|e| e.to_string())?
}

unsafe fn png_from(image: *mut AnyObject, error: *mut AnyObject) -> Result<Vec<u8>, String> {
	unsafe {
		if !error.is_null() || image.is_null() {
			return Err("WKWebView returned no snapshot".to_string());
		}
		let tiff: *mut AnyObject = msg_send![image, TIFFRepresentation];
		if tiff.is_null() {
			return Err("the snapshot has no TIFF representation".to_string());
		}
		let rep: *mut AnyObject = msg_send![class!(NSBitmapImageRep), imageRepWithData: tiff];
		if rep.is_null() {
			return Err("the snapshot could not be decoded".to_string());
		}
		let properties: *mut AnyObject = msg_send![class!(NSDictionary), dictionary];
		let data: *mut AnyObject =
			msg_send![rep, representationUsingType: FILE_TYPE_PNG, properties: properties];
		if data.is_null() {
			return Err("the snapshot could not be written as PNG".to_string());
		}
		let length: usize = msg_send![data, length];
		let bytes: *const u8 = msg_send![data, bytes];
		if bytes.is_null() || length == 0 {
			return Err("the PNG is empty".to_string());
		}
		Ok(std::slice::from_raw_parts(bytes, length).to_vec())
	}
}
