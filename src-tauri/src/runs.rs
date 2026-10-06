// 실행 기록 — 셸 명령 칸의 결과와, 어떤 칸이든 실패한 실행. 메모리에만 둔다
//
// 셸 명령의 출력에는 비밀값이 섞일 수 있다. 이 앱은 그 출력을 디스크, 로그, OS 알림 어디에도 쓰지 않는다.
// 실패는 OS 알림으로도 알리지만 알림은 꺼져 있을 수 있다. 그래서 실패는 모두 여기에 남고, 보지 않은 실패가
// 있으면 트레이와 설정 창이 그것을 가리킨다.
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::constants::events;

/// 기록을 이만큼만 남긴다. 넘으면 오래된 것부터 버린다.
pub const MAX_RUNS: usize = 20;
/// 기록 하나가 남기는 출력의 최대 크기.
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
	pub id: u64,
	/// 칸의 이름. 명령 글은 남기지 않는다. 칸 없이 난 실패면 비어 있다.
	pub label: String,
	/// 셸 명령의 실행인가. 아니면 열기·키 입력·하위 링의 실패다 — 출력과 걸린 시간이 없다.
	pub shell: bool,
	/// 끝난 시각 (RFC 3339).
	pub finished_at: String,
	pub duration_ms: u64,
	pub ok: bool,
	/// 종료 코드. signal 이나 timeout 으로 끝났으면 없다.
	pub exit_code: Option<i32>,
	/// 실패의 한 줄. 기록할 때의 언어다. 성공이면 없다.
	pub error: Option<String>,
	pub output: String,
}

static RUNS: Mutex<VecDeque<RunRecord>> = Mutex::new(VecDeque::new());
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
/// 사용자가 아직 보지 않은 실패의 수. 실행 기록 페이지를 열면 0 이 된다.
static UNSEEN_FAILURES: AtomicUsize = AtomicUsize::new(0);

fn runs() -> std::sync::MutexGuard<'static, VecDeque<RunRecord>> {
	RUNS.lock().unwrap_or_else(|e| e.into_inner())
}

/// 출력을 [`MAX_OUTPUT_BYTES`] 로 자른다. 끝쪽을 남긴다 — 실패의 까닭은 대개 끝에 있다.
fn cap_output(output: &str) -> String {
	if output.len() <= MAX_OUTPUT_BYTES {
		return output.to_string();
	}
	let mut start = output.len() - MAX_OUTPUT_BYTES;
	while !output.is_char_boundary(start) {
		start += 1;
	}
	format!("…\n{}", &output[start..])
}

/// 기록 하나를 맨 앞에 넣는다.
fn push(mut record: RunRecord) {
	record.id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
	record.output = cap_output(&record.output);
	if !record.ok {
		UNSEEN_FAILURES.fetch_add(1, Ordering::SeqCst);
	}
	let mut guard = runs();
	guard.push_front(record);
	guard.truncate(MAX_RUNS);
}

/// 기록이나 "보지 않은 실패" 가 바뀌었다. 설정 창에 알리고 트레이를 맞춘다. 어느 thread 에서 불러도 된다.
fn announce(app: &AppHandle) {
	for event in [events::RUNS_CHANGED, events::APP_STATE_CHANGED] {
		if let Err(e) = app.emit(event, ()) {
			log::warn!("[runs] failed to emit {event}: {e}");
		}
	}
	let handle = app.clone();
	if let Err(e) = app.run_on_main_thread(move || crate::ui::tray::refresh(&handle)) {
		log::warn!("[runs] failed to reach the main thread: {e}");
	}
}

/// 기록을 넣고 설정 창과 트레이에 알린다.
pub fn record(app: &AppHandle, record: RunRecord) {
	push(record);
	announce(app);
}

/// 셸 명령이 아닌 실패 하나를 남긴다 — 열 대상이 없다, 권한이 없다 같은 것.
pub fn record_failure(app: &AppHandle, label: Option<&str>, message: String) {
	record(
		app,
		RunRecord {
			id: 0,
			label: label.unwrap_or_default().to_string(),
			shell: false,
			finished_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
			duration_ms: 0,
			ok: false,
			exit_code: None,
			error: Some(message),
			output: String::new(),
		},
	);
}

/// 아직 보지 않은 실패의 수.
pub fn unseen_failures() -> usize {
	UNSEEN_FAILURES.load(Ordering::SeqCst)
}

/// 사용자가 실행 기록 페이지를 보고 있다.
pub fn mark_seen(app: &AppHandle) {
	if UNSEEN_FAILURES.swap(0, Ordering::SeqCst) != 0 {
		announce(app);
	}
}

/// 최근 것부터.
pub fn list() -> Vec<RunRecord> {
	runs().iter().cloned().collect()
}

pub fn clear(app: &AppHandle) {
	runs().clear();
	UNSEEN_FAILURES.store(0, Ordering::SeqCst);
	announce(app);
}

#[cfg(test)]
mod tests {
	use super::*;

	fn sample(label: &str, output: &str) -> RunRecord {
		RunRecord {
			id: 0,
			label: label.to_string(),
			shell: true,
			finished_at: String::new(),
			duration_ms: 1,
			ok: true,
			exit_code: Some(0),
			error: None,
			output: output.to_string(),
		}
	}

	#[test]
	fn long_output_keeps_its_end() {
		let long = format!("{}끝", "x".repeat(MAX_OUTPUT_BYTES * 2));
		let capped = cap_output(&long);
		assert!(capped.starts_with("…\n"));
		assert!(capped.ends_with('끝'));
		assert!(capped.len() <= MAX_OUTPUT_BYTES + 8);
		assert_eq!(cap_output("short"), "short");
	}

	#[test]
	fn the_list_is_newest_first_and_bounded() {
		// 다른 테스트와 같은 static 을 쓴다. 이 테스트만 기록을 넣는다.
		runs().clear();
		for index in 0..MAX_RUNS + 5 {
			push(sample(&format!("run {index}"), ""));
		}
		let kept = list();
		assert_eq!(kept.len(), MAX_RUNS);
		assert_eq!(kept[0].label, format!("run {}", MAX_RUNS + 4));
		assert!(kept[0].id > kept[1].id);
		runs().clear();

		// 실패만 "보지 않은 실패" 로 센다.
		UNSEEN_FAILURES.store(0, Ordering::SeqCst);
		push(sample("ok", ""));
		assert_eq!(unseen_failures(), 0);
		push(RunRecord {
			ok: false,
			..sample("failed", "")
		});
		assert_eq!(unseen_failures(), 1);
		runs().clear();
		UNSEEN_FAILURES.store(0, Ordering::SeqCst);
	}
}
