// 사용자의 로그인 셸로 명령 하나를 돌리는 실행기 — 셸 명령 칸이 쓴다

//! The command runs as `$SHELL -l -c <command>`, so the user's PATH and shell
//! setup apply. It leads its own process group: dropping the future (a
//! timeout) ends the whole group through `ProcessGroupGuard`.
//!
//! On Windows the shell is `%ComSpec%` and the command runs as
//! `cmd /D /S /C "<command>"`. The run ends when `cmd` exits: what it started
//! in the background keeps running. A timeout ends everything it started.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};

use super::process_group::ProcessGroupGuard;

/// When `$SHELL` is unset or not an executable file.
#[cfg(not(windows))]
const FALLBACK_SHELL: &str = "/bin/zsh";
/// When `%ComSpec%` is unset or not a file.
#[cfg(windows)]
const FALLBACK_SHELL: &str = r"C:\Windows\System32\cmd.exe";

/// `CREATE_NO_WINDOW` — a console program started from this GUI app would
/// otherwise open a console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// stdout and stderr each keep at most this much. The rest is read and thrown
/// away, so a chatty command never blocks on a full pipe.
pub const OUTPUT_CAP_BYTES: usize = 1024 * 1024;

/// How long the pipes are still read after `cmd` itself has exited.
#[cfg(windows)]
const DRAIN_AFTER_EXIT: Duration = Duration::from_millis(200);

/// Appended to a stream that was cut at `OUTPUT_CAP_BYTES`.
pub const TRUNCATED_NOTE: &str = "\n[RingRing: output cut at 1 MiB]";

pub fn home() -> PathBuf {
	let (var, root) = if cfg!(windows) {
		("USERPROFILE", r"C:\")
	} else {
		("HOME", "/")
	};
	std::env::var_os(var)
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from(root))
}

/// Empty is the home folder; `~` and `~/…` are under it.
pub fn working_folder(setting: &str) -> PathBuf {
	let setting = setting.trim();
	match setting.strip_prefix('~') {
		_ if setting.is_empty() => home(),
		Some("") => home(),
		Some(rest) if rest.starts_with('/') || (cfg!(windows) && rest.starts_with('\\')) => {
			home().join(&rest[1..])
		}
		_ => PathBuf::from(setting),
	}
}

/// `$SHELL` (`%ComSpec%` on Windows) when it names an executable file, else
/// `FALLBACK_SHELL`.
pub fn login_shell() -> PathBuf {
	std::env::var_os(if cfg!(windows) { "ComSpec" } else { "SHELL" })
		.map(PathBuf::from)
		.filter(|p| is_executable(p))
		.unwrap_or_else(|| PathBuf::from(FALLBACK_SHELL))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
	use std::os::unix::fs::PermissionsExt;
	path.is_absolute()
		&& std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
	path.is_absolute() && path.is_file()
}

/// The flags before the command. tcsh and csh refuse `-l` next to any other
/// flag, so they get `-c` alone and read `~/.cshrc` only.
#[cfg(unix)]
pub fn shell_flags(shell: &Path) -> &'static [&'static str] {
	match shell.file_name().and_then(|n| n.to_str()) {
		Some("tcsh" | "csh") => &["-c"],
		_ => &["-l", "-c"],
	}
}

/// One command to run.
pub struct ShellJob {
	pub shell: PathBuf,
	pub command: String,
	pub folder: PathBuf,
	pub timeout: Duration,
}

#[derive(Debug)]
pub struct ShellDone {
	pub stdout: String,
	pub stderr: String,
	/// `None` when a signal ended the command or it ran out of time.
	pub code: Option<i32>,
	pub signal: Option<i32>,
	/// The time limit passed. `stdout` and `stderr` hold what had arrived by then.
	pub timed_out: bool,
}

/// What one pipe has produced so far. Shared with the caller so that a
/// timeout, which drops the reading future, still has the output.
#[derive(Default)]
struct Sink {
	kept: Vec<u8>,
	cut: bool,
}

type SharedSink = std::sync::Mutex<Sink>;

fn sink_text(sink: &SharedSink) -> String {
	let sink = sink.lock().unwrap_or_else(|e| e.into_inner());
	let mut text = String::from_utf8_lossy(&sink.kept).into_owned();
	if sink.cut {
		// A cut can split a character; the lossy decode shows it as U+FFFD.
		text.push_str(TRUNCATED_NOTE);
	}
	text
}

impl ShellJob {
	/// `Err` means the command never ran. A command that ran out of time is
	/// `Ok` with `timed_out` set, so its output is not lost.
	///
	/// A background job that keeps the command's stdout or stderr open (`… &`)
	/// keeps the run going after the shell itself exits: the run ends when the
	/// pipes close or the time limit passes, and the limit ends the whole group.
	pub async fn run(self) -> Result<ShellDone, String> {
		match tokio::fs::metadata(&self.folder).await {
			Ok(meta) if meta.is_dir() => {}
			_ => {
				return Err(format!(
					"The working folder {} is not a folder",
					self.folder.display()
				));
			}
		}
		let limit = self.timeout;
		let (out, err) = (SharedSink::default(), SharedSink::default());
		let status = match tokio::time::timeout(limit, self.spawn_and_wait(&out, &err)).await {
			Ok(status) => Some(status?),
			Err(_) => None,
		};
		Ok(ShellDone {
			stdout: sink_text(&out),
			stderr: sink_text(&err),
			code: status.and_then(|status| status.code()),
			signal: status.and_then(exit_signal),
			timed_out: status.is_none(),
		})
	}

	/// Dropping this future (timeout) drops the child and the group guard,
	/// which ends the whole group.
	async fn spawn_and_wait(
		self,
		out: &SharedSink,
		err: &SharedSink,
	) -> Result<std::process::ExitStatus, String> {
		let mut cmd = tokio::process::Command::new(&self.shell);
		#[cfg(unix)]
		cmd.args(shell_flags(&self.shell))
			.arg(&self.command)
			.process_group(0);
		// cmd 는 MSVC 식 따옴표 규칙을 따르지 않는다. `/S` 는 바깥 따옴표 한 쌍만 벗기고 안쪽은 그대로 둔다.
		// 출력은 UTF-8 로 읽는다. `chcp 65001` 이 없으면 cmd 와 그 자식이 시스템 codepage(한국어는 949)로 쓴다.
		#[cfg(windows)]
		cmd.raw_arg(format!(
			"/D /S /C \"chcp 65001 >nul & {}\"",
			cmd_line(&self.command)
		))
		.creation_flags(CREATE_NO_WINDOW);
		cmd.current_dir(&self.folder)
			.stdin(Stdio::null())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.kill_on_drop(true);
		let mut child = cmd
			.spawn()
			.map_err(|e| format!("Failed to start {}: {e}", self.shell.display()))?;
		let group = ProcessGroupGuard::adopt(child.id(), "shell command");

		let stdout = read_capped(child.stdout.take(), out);
		let stderr = read_capped(child.stderr.take(), err);
		#[cfg(not(windows))]
		let ((), (), status) = tokio::join!(stdout, stderr, child.wait());
		// Windows 에서는 cmd 가 띄운 프로세스가 모두 출력 pipe 를 물려받는다 — `start notepad` 로 띄운 앱도 그렇다.
		// pipe 가 닫히기를 기다리면 그 앱을 닫을 때까지 끝나지 않는다. 그래서 cmd 가 끝나면 남은 출력만 잠깐 더 읽고 끝낸다.
		#[cfg(windows)]
		let status = {
			let readers = async { tokio::join!(stdout, stderr) };
			let wait = child.wait();
			tokio::pin!(readers, wait);
			tokio::select! {
				_ = &mut readers => wait.await,
				status = &mut wait => {
					let _ = tokio::time::timeout(DRAIN_AFTER_EXIT, &mut readers).await;
					status
				}
			}
		};
		let status = status.map_err(|e| format!("Failed to wait for the command: {e}"))?;
		group.ended();
		Ok(status)
	}
}

/// `cmd /C` reads one line. A command written on several lines runs them in
/// order, joined with `&`.
#[cfg(windows)]
fn cmd_line(command: &str) -> String {
	command
		.lines()
		.map(str::trim)
		.filter(|line| !line.is_empty())
		.collect::<Vec<_>>()
		.join(" & ")
}

/// The signal that ended the command. Only Unix has one.
fn exit_signal(status: std::process::ExitStatus) -> Option<i32> {
	#[cfg(unix)]
	{
		use std::os::unix::process::ExitStatusExt;
		status.signal()
	}
	#[cfg(not(unix))]
	{
		let _ = status;
		None
	}
}

/// Read a pipe to its end, keeping the first `OUTPUT_CAP_BYTES` in `sink`.
async fn read_capped<R: AsyncRead + Unpin>(pipe: Option<R>, sink: &SharedSink) {
	let Some(mut pipe) = pipe else {
		return;
	};
	let mut buf = [0u8; 16 * 1024];
	loop {
		match pipe.read(&mut buf).await {
			Ok(0) | Err(_) => break,
			Ok(n) => {
				let mut sink = sink.lock().unwrap_or_else(|e| e.into_inner());
				let room = OUTPUT_CAP_BYTES - sink.kept.len();
				if n > room {
					sink.cut = true;
				}
				sink.kept.extend_from_slice(&buf[..n.min(room)]);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn job(command: &str, timeout: Duration) -> ShellJob {
		ShellJob {
			shell: if cfg!(windows) {
				login_shell()
			} else {
				PathBuf::from("/bin/sh")
			},
			command: command.to_string(),
			folder: std::env::temp_dir(),
			timeout,
		}
	}

	fn run(job: ShellJob) -> Result<ShellDone, String> {
		let runtime = tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.unwrap();
		let done = runtime.block_on(job.run());
		// Windows 의 pipe 읽기는 blocking thread 에서 돈다. 남은 작업이 pipe 를 쥐고 있으면 runtime 을 그냥
		// 놓을 때 그 thread 를 기다린다. 앱의 runtime 은 끝까지 살아 있어 기다릴 일이 없다.
		runtime.shutdown_background();
		done
	}

	#[test]
	fn working_folder_expands_home() {
		assert_eq!(working_folder(""), home());
		assert_eq!(working_folder("  ~ "), home());
		assert_eq!(working_folder("~/Downloads"), home().join("Downloads"));
		assert_eq!(working_folder("/tmp"), PathBuf::from("/tmp"));
	}

	#[cfg(windows)]
	#[test]
	fn working_folder_expands_home_with_a_backslash() {
		assert_eq!(working_folder(r"~\Downloads"), home().join("Downloads"));
		assert_eq!(working_folder(r"C:\Temp"), PathBuf::from(r"C:\Temp"));
	}

	#[cfg(windows)]
	#[test]
	fn a_cmd_command_reports_its_output_and_exit_code() {
		let done = run(job(
			r#"echo "a b" c& echo err 1>&2& exit /b 3"#,
			Duration::from_secs(10),
		))
		.unwrap();
		// 따옴표가 든 명령이 그대로 cmd 에 닿는다.
		assert_eq!(done.stdout.trim_end(), "\"a b\" c");
		assert_eq!(done.stderr.trim_end(), "err");
		assert_eq!(done.code, Some(3));
		assert_eq!(done.signal, None);
	}

	#[cfg(windows)]
	#[test]
	fn a_cmd_command_past_its_time_limit_is_stopped_and_keeps_its_output() {
		let started = std::time::Instant::now();
		let done = run(job(
			"echo before& ping -n 30 127.0.0.1 >nul",
			Duration::from_millis(600),
		))
		.unwrap();
		assert!(done.timed_out);
		assert_eq!(done.code, None);
		assert_eq!(done.stdout.trim_end(), "before");
		assert!(started.elapsed() < Duration::from_secs(8));
	}

	#[cfg(windows)]
	#[test]
	fn a_cmd_command_on_several_lines_runs_every_line() {
		let done = run(job(
			"echo one\r\n\r\n  echo two\nexit /b 4",
			Duration::from_secs(10),
		))
		.unwrap();
		let lines: Vec<&str> = done.stdout.lines().map(str::trim).collect();
		assert_eq!(lines, ["one", "two"]);
		assert_eq!(done.code, Some(4));
	}

	/// 이 명령줄을 가진 프로세스가 몇 개 떠 있는가.
	#[cfg(windows)]
	fn processes_with(marker: &str) -> usize {
		let query = format!(
			"@(Get-CimInstance Win32_Process -Filter \"Name='ping.exe' AND CommandLine LIKE '%{marker}%'\").Count"
		);
		let out = std::process::Command::new("powershell")
			.args(["-NoProfile", "-Command", &query])
			.output()
			.expect("powershell runs");
		String::from_utf8_lossy(&out.stdout)
			.trim()
			.parse()
			.expect("a count")
	}

	#[cfg(windows)]
	#[test]
	fn a_cmd_command_past_its_time_limit_ends_what_it_started() {
		// `ping` 은 cmd 의 자식이다. cmd 만 끝내면 ping 이 남는다.
		let marker = "127.0.0.213";
		let done = run(job(
			&format!("echo started & ping -n 60 {marker} >nul"),
			Duration::from_millis(1500),
		))
		.unwrap();
		assert!(done.timed_out);
		assert!(done.stdout.starts_with("started"), "{}", done.stdout);
		std::thread::sleep(Duration::from_millis(300));
		assert_eq!(processes_with(marker), 0);
	}

	#[cfg(windows)]
	#[test]
	fn a_cmd_background_job_outlives_the_run_and_does_not_hold_it() {
		// `start` 로 띄운 프로세스는 출력 pipe 를 물려받는다. 그래도 cmd 가 끝나면 실행은 끝난다.
		let marker = "127.0.0.214";
		let started = std::time::Instant::now();
		let done = run(job(
			&format!("start /b ping -n 12 {marker} >nul 2>&1 & echo started"),
			Duration::from_secs(30),
		))
		.unwrap();
		assert!(!done.timed_out);
		assert_eq!(done.code, Some(0));
		assert!(done.stdout.starts_with("started"), "{}", done.stdout);
		assert!(started.elapsed() < Duration::from_secs(3));
		assert_eq!(processes_with(marker), 1);
	}

	#[cfg(unix)]
	#[test]
	fn csh_family_gets_no_login_flag() {
		assert_eq!(shell_flags(Path::new("/bin/tcsh")), ["-c"]);
		assert_eq!(shell_flags(Path::new("/bin/zsh")), ["-l", "-c"]);
	}

	#[cfg(unix)]
	#[test]
	fn a_command_reports_its_output_and_exit_code() {
		let done = run(job(
			"printf out; printf err >&2; exit 3",
			Duration::from_secs(10),
		))
		.unwrap();
		assert_eq!(done.stdout, "out");
		assert_eq!(done.stderr, "err");
		assert_eq!(done.code, Some(3));
	}

	#[cfg(unix)]
	#[test]
	fn a_command_past_its_time_limit_is_stopped_and_keeps_its_output() {
		let started = std::time::Instant::now();
		let done = run(job(
			"echo before; echo oops >&2; sleep 30; echo after",
			Duration::from_millis(400),
		))
		.unwrap();
		assert!(done.timed_out);
		assert_eq!(done.code, None);
		assert_eq!(done.stdout, "before\n");
		assert_eq!(done.stderr, "oops\n");
		assert!(started.elapsed() < Duration::from_secs(5));
	}

	#[cfg(unix)]
	#[test]
	fn a_background_job_holding_the_output_keeps_the_run_going_until_the_limit() {
		// 셸은 곧바로 끝나지만 `sleep` 이 stdout 을 쥐고 있다. 제한 시간까지 끝나지 않는다.
		let done = run(job("sleep 30 & echo started", Duration::from_millis(400))).unwrap();
		assert!(done.timed_out);
		assert_eq!(done.stdout, "started\n");
	}

	#[cfg(unix)]
	#[test]
	fn a_background_job_with_its_output_redirected_does_not_hold_the_run() {
		let started = std::time::Instant::now();
		let done = run(job(
			"sleep 3 >/dev/null 2>&1 & echo started",
			Duration::from_secs(20),
		))
		.unwrap();
		assert!(!done.timed_out);
		assert_eq!(done.code, Some(0));
		assert_eq!(done.stdout, "started\n");
		assert!(started.elapsed() < Duration::from_secs(2));
	}

	#[test]
	fn a_missing_working_folder_is_an_error_before_anything_runs() {
		let mut missing = job("exit 0", Duration::from_secs(5));
		missing.folder = std::env::temp_dir().join("definitely").join("not-here");
		assert!(run(missing).unwrap_err().contains("is not a folder"));
	}
}
