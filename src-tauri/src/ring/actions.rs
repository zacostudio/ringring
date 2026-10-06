// 칸이 고른 동작을 실행한다 — 열기, 셸 명령, 키 입력
//
// 링은 실행 전에 이미 숨었다. 성공은 조용하다 — 셸 명령만 실행 기록에 남는다.
// 실패는 두 곳에 남는다. 실행 기록에 남고(트레이와 설정 창이 가리킨다), OS 알림 한 줄로도 알린다.
// 알림은 꺼져 있을 수 있어서 실행 기록이 먼저다.
//
// 셸 명령의 글과 출력은 로그에도 알림에도 싣지 않는다. 로그에는 칸의 번호와 종류만, 알림에는 어떻게
// 끝났는지와 출력을 어디서 보는지만 싣는다. 출력은 실행 기록(메모리)에만 있다.

use std::path::Path;
use std::time::{Duration, Instant};

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use super::keystroke::{self, SendError};
use super::model::{OpenTarget, RingAction, Slot, is_http_url};
use crate::exec::shell_command::{ShellJob, login_shell, working_folder};
use crate::runs::{self, RunRecord};
use crate::settings::Locale;
use crate::ui::texts::{self, ShellFailure};

/// 알림에 싣는 실패 글의 최대 글자 수.
const NOTICE_MAX_CHARS: usize = 300;

/// 사용자에게 알릴 실패.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
	/// 칸이 가리키는 파일·앱·하위 링이 없다.
	TargetMissing,
	/// 칸이 여는 하위 링에 채워진 칸이 없다.
	SubRingEmpty,
	/// 키 입력을 보낼 권한(손쉬운 사용)이 없다.
	NotTrusted,
	/// 수식키를 계속 누르고 있어 키 입력을 보내지 않았다.
	ModifiersHeld,
	/// 셸 명령이 실패로 끝났다. 실행 기록에는 이미 남았다.
	Shell(ShellFailure),
	/// 그 밖의 실패. 글은 실행한 쪽이 준 그대로다. 명령의 출력은 들어 있지 않다.
	Other(String),
}

fn message(failure: &Failure, locale: Locale) -> String {
	match failure {
		Failure::TargetMissing => texts::target_missing(locale).to_string(),
		Failure::SubRingEmpty => texts::sub_ring_empty(locale).to_string(),
		Failure::NotTrusted => texts::not_trusted(locale).to_string(),
		Failure::ModifiersHeld => texts::modifiers_held(locale).to_string(),
		Failure::Shell(failure) => format!(
			"{} {}",
			texts::shell_failure(*failure, locale),
			texts::see_recent_runs(locale)
		),
		Failure::Other(text) => text.chars().take(NOTICE_MAX_CHARS).collect(),
	}
}

/// 실패를 실행 기록에 남기고 OS 알림으로 알린다. 어느 thread 에서 불러도 된다. `label` 은 칸의 이름이다.
pub fn notify_failure(app: &AppHandle, label: &str, failure: Failure) {
	notify(app, Some(label), failure);
}

/// OS 알림을 올리는 일을 자기 thread 로 넘기고 곧바로 돌아온다.
///
/// macOS 에서 알림을 올리는 호출은 전달을 기다리며 약 2초 동안 돌아오지 않는다 (`mac-notification-sys`).
/// async worker 에서 그대로 부르면 그 worker 가 멈추고, 같은 때 돌던 셸 명령의 제한 시간이 그만큼 늦게 끝난다.
fn post_detached(post: impl FnOnce() + Send + 'static) {
	let spawned = std::thread::Builder::new()
		.name("ring-notice".to_string())
		.spawn(post);
	if let Err(e) = spawned {
		log::warn!("[ring] failed to start the notification thread: {e}");
	}
}

fn notify(app: &AppHandle, label: Option<&str>, failure: Failure) {
	let locale = crate::settings::locale();
	let (title, body) = (
		texts::notice_title(label, locale),
		message(&failure, locale),
	);
	log::warn!(
		"[ring] action failed: {}",
		match &failure {
			// 실행한 쪽의 글에는 경로가 들어 있을 수 있다. 로그에는 종류만 남긴다.
			Failure::Other(_) => "see Recent runs",
			Failure::Shell(_) => "shell command failed",
			Failure::TargetMissing => "target missing",
			Failure::SubRingEmpty => "sub-ring is empty",
			Failure::NotTrusted => "accessibility not granted",
			Failure::ModifiersHeld => "modifiers still held",
		}
	);
	// 셸 명령의 실패는 출력과 함께 이미 실행 기록에 있다. 나머지 실패는 여기서 남긴다.
	if !matches!(failure, Failure::Shell(_)) {
		runs::record_failure(app, label, body.clone());
	}
	// 기록이 먼저고, 알림은 기다리지 않는다.
	let app = app.clone();
	post_detached(move || {
		if let Err(e) = app.notification().builder().title(title).body(body).show() {
			log::warn!("[ring] failed to post the failure notification: {e}");
		}
	});
}

/// 확인 물음에 보일 한 줄. 무엇을 실행하는지다.
pub fn describe(action: &RingAction) -> String {
	match action {
		RingAction::Open { value, args, .. } if !args.trim().is_empty() => {
			format!("{value} {}", args.trim())
		}
		RingAction::Open { value, .. } => value.clone(),
		RingAction::Shell { command, .. } => command.clone(),
		RingAction::Keystroke { combos } => combos.join(", "),
		RingAction::OpenRing { ring_id } => ring_id.clone(),
	}
}

/// 칸의 동작을 실행한다. main thread 에서 부른다. 오래 걸리는 일은 task 로 넘기고 곧바로 돌아온다.
pub fn run(app: &AppHandle, slot: Slot) {
	log::info!(
		"[ring] run slot {} ({})",
		slot.position + 1,
		slot.action.kind()
	);
	let label = slot.label;
	match slot.action {
		RingAction::Open {
			target,
			value,
			args,
		} => {
			spawn(app, label, move |app| open(app, target, value, args));
		}
		RingAction::Shell {
			command,
			working_dir,
			timeout_secs,
		} => {
			let entry = label.clone();
			spawn(app, label, move |app| {
				run_shell(app, entry, command, working_dir, timeout_secs)
			});
		}
		RingAction::Keystroke { combos } => {
			let app = app.clone();
			// 조합 사이에 쉬고 수식키를 기다리므로 자기 thread 에서 돈다.
			let spawned = std::thread::Builder::new()
				.name("ring-keystroke".to_string())
				.spawn(move || {
					if let Err(failure) = send_keystrokes(&app, &combos) {
						notify(&app, Some(&label), failure);
					}
				});
			if let Err(e) = spawned {
				log::warn!("[ring] failed to start the keystroke thread: {e}");
			}
		}
		// 하위 링은 실행이 아니다. controller 가 먼저 가로챈다.
		RingAction::OpenRing { .. } => {}
	}
}

/// 동작 하나를 async task 로 돌리고, 실패하면 알린다.
fn spawn<F, Fut>(app: &AppHandle, label: String, job: F)
where
	F: FnOnce(AppHandle) -> Fut + Send + 'static,
	Fut: std::future::Future<Output = Result<(), Failure>> + Send,
{
	let app = app.clone();
	tauri::async_runtime::spawn(async move {
		if let Err(failure) = job(app.clone()).await {
			notify(&app, Some(&label), failure);
		}
	});
}

async fn open(
	app: AppHandle,
	target: OpenTarget,
	value: String,
	args: String,
) -> Result<(), Failure> {
	let value = value.trim().to_string();
	match target {
		OpenTarget::Url => {
			// 저장할 때 본 규칙을 한 번 더 본다. DB 의 값이 그 뒤에 바뀌었을 수 있다.
			if !is_http_url(&value) {
				return Err(Failure::Other(
					"The address must start with http:// or https://".to_string(),
				));
			}
			app.opener()
				.open_url(&value, None::<&str>)
				.map_err(|e| Failure::Other(e.to_string()))
		}
		OpenTarget::App | OpenTarget::File => {
			let path = working_folder(&value);
			let probe = path.clone();
			let exists = tauri::async_runtime::spawn_blocking(move || Path::new(&probe).exists())
				.await
				.map_err(|e| Failure::Other(e.to_string()))?;
			if !exists {
				return Err(Failure::TargetMissing);
			}
			// 인자는 앱에만 넘긴다. 인자가 없으면 전과 같은 길로 연다.
			let args = args.trim().to_string();
			if target == OpenTarget::App && !args.is_empty() {
				return tauri::async_runtime::spawn_blocking(move || {
					crate::exec::launch::app_with_args(&path, &args)
				})
				.await
				.map_err(|e| Failure::Other(e.to_string()))?
				.map_err(Failure::Other);
			}
			app.opener()
				.open_path(path.to_string_lossy(), None::<&str>)
				.map_err(|e| Failure::Other(e.to_string()))
		}
	}
}

/// 셸 명령을 돌리고 결과를 실행 기록에 남긴다. 0 이 아닌 종료 코드와 timeout 은 실패로 알린다.
async fn run_shell(
	app: AppHandle,
	label: String,
	command: String,
	working_dir: String,
	timeout_secs: u32,
) -> Result<(), Failure> {
	let job = ShellJob {
		shell: login_shell(),
		command,
		folder: working_folder(&working_dir),
		timeout: Duration::from_secs(u64::from(timeout_secs)),
	};
	let started = Instant::now();
	// 돌지도 못한 명령(작업 폴더가 없다, 셸을 띄우지 못했다)은 출력이 없다. 그 글을 그대로 알린다.
	let done = match job.run().await {
		Ok(done) => done,
		Err(e) => {
			return Err(Failure::Other(e));
		}
	};
	let failure = match done.code {
		_ if done.timed_out => Some(ShellFailure::TimedOut(u64::from(timeout_secs))),
		Some(0) => None,
		Some(code) => Some(ShellFailure::Exit(code)),
		None => Some(ShellFailure::Signal(done.signal.unwrap_or_default())),
	};
	// 기록의 머리줄에는 칸의 이름을 쓴다. 명령 글에는 비밀값이 들어 있을 수 있다.
	// 제한 시간을 넘긴 실행도 그때까지 나온 출력을 남긴다.
	runs::record(
		&app,
		RunRecord {
			id: 0,
			label,
			shell: true,
			finished_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
			duration_ms: started.elapsed().as_millis() as u64,
			ok: failure.is_none(),
			exit_code: done.code,
			error: failure.map(|failure| texts::shell_failure(failure, crate::settings::locale())),
			output: join_output(&done.stdout, &done.stderr),
		},
	);
	match failure {
		None => Ok(()),
		Some(failure) => Err(Failure::Shell(failure)),
	}
}

fn join_output(stdout: &str, stderr: &str) -> String {
	match (stdout.trim_end().is_empty(), stderr.trim_end().is_empty()) {
		(_, true) => stdout.to_string(),
		(true, false) => stderr.to_string(),
		(false, false) => format!("{}\n{}", stdout.trim_end(), stderr),
	}
}

/// 키 입력을 보낸다. 자기 thread 에서 부른다.
fn send_keystrokes(app: &AppHandle, combos: &[String]) -> Result<(), Failure> {
	// 링이 키보드를 가지고 있었으면(tap) 숨긴 뒤에 키보드가 원래 앱으로 돌아가야 한다. main loop 가 한 번
	// 돈 뒤에 보낸다.
	let (tx, rx) = std::sync::mpsc::channel();
	if app
		.run_on_main_thread(move || {
			let _ = tx.send(());
		})
		.is_ok()
	{
		let _ = rx.recv_timeout(Duration::from_millis(500));
	}
	keystroke::send(combos).map_err(|e| match e {
		SendError::NotTrusted => Failure::NotTrusted,
		SendError::ModifiersHeld => Failure::ModifiersHeld,
		SendError::BadCombo(combo) => Failure::Other(format!("Invalid key combination '{combo}'")),
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	/// 알림을 올리는 호출이 2초 동안 돌아오지 않아도 1초 제한 시간은 1초 언저리에서 끝난다.
	/// worker 가 하나뿐인 runtime 에서 잰다 — 알림이 worker 를 붙잡으면 여기서 드러난다.
	#[tokio::test(flavor = "current_thread")]
	async fn a_slow_notification_does_not_delay_a_shell_time_limit() {
		let started = Instant::now();
		let (shell, command) = if cfg!(windows) {
			(login_shell(), "ping -n 6 127.0.0.1 >nul")
		} else {
			(std::path::PathBuf::from("/bin/sh"), "sleep 5")
		};
		let job = ShellJob {
			shell,
			command: command.to_string(),
			folder: std::env::temp_dir(),
			timeout: Duration::from_secs(1),
		};
		let run = tokio::spawn(job.run());
		tokio::task::yield_now().await;
		let (tx, rx) = std::sync::mpsc::channel();
		post_detached(move || {
			std::thread::sleep(Duration::from_secs(2));
			let _ = tx.send(());
		});
		let returned_after = started.elapsed();
		let done = run.await.expect("the run task").expect("the command ran");
		let limit_after = started.elapsed();
		println!(
			"post_detached returned after {returned_after:?}, the 1 s limit ended after {limit_after:?}"
		);
		assert!(done.timed_out);
		assert!(
			returned_after < Duration::from_millis(200),
			"{returned_after:?}"
		);
		assert!(limit_after < Duration::from_millis(1500), "{limit_after:?}");
		// 알림 thread 는 그 뒤에도 끝까지 돈다.
		rx.recv_timeout(Duration::from_secs(3))
			.expect("the notification thread finished");
	}

	#[test]
	fn the_confirm_line_shows_what_will_run() {
		assert_eq!(
			describe(&RingAction::Shell {
				command: "make deploy".to_string(),
				working_dir: String::new(),
				timeout_secs: 120,
			}),
			"make deploy"
		);
		assert_eq!(
			describe(&RingAction::Keystroke {
				combos: vec!["Cmd+KeyC".to_string(), "Cmd+KeyV".to_string()],
			}),
			"Cmd+KeyC, Cmd+KeyV"
		);
	}

	#[test]
	fn failure_messages_follow_the_locale() {
		assert_eq!(
			message(&Failure::TargetMissing, Locale::Ko),
			"대상이 없습니다."
		);
		assert_eq!(
			message(&Failure::TargetMissing, Locale::En),
			"The target no longer exists."
		);
		assert!(message(&Failure::SubRingEmpty, Locale::Ko).contains("비어 있습니다"));
	}

	#[test]
	fn a_long_failure_text_is_cut_for_the_notification() {
		let long = "x".repeat(NOTICE_MAX_CHARS * 2);
		assert_eq!(
			message(&Failure::Other(long), Locale::En).chars().count(),
			NOTICE_MAX_CHARS
		);
	}

	#[test]
	fn a_shell_failure_notice_says_where_the_output_is_and_carries_none_of_it() {
		let notice = message(&Failure::Shell(ShellFailure::Exit(3)), Locale::En);
		assert_eq!(
			notice,
			"The command exited with code 3. Its output is under Recent runs in Settings."
		);
	}

	#[test]
	fn output_joins_stdout_and_stderr_without_blank_padding() {
		assert_eq!(join_output("out\n", ""), "out\n");
		assert_eq!(join_output("", "err\n"), "err\n");
		assert_eq!(join_output("out\n", "err\n"), "out\nerr\n");
	}
}
