// 사용자의 로그인 셸로 명령 하나를 돌리는 실행기 — 셸 명령 칸이 쓴다

//! The command runs as `$SHELL -l -c <command>`, so the user's PATH and shell
//! setup apply. It leads its own process group: dropping the future (a
//! timeout) ends the whole group through `ProcessGroupGuard`.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};

use super::process_group::ProcessGroupGuard;

/// When `$SHELL` is unset or not an executable file.
const FALLBACK_SHELL: &str = "/bin/zsh";

/// stdout and stderr each keep at most this much. The rest is read and thrown
/// away, so a chatty command never blocks on a full pipe.
pub const OUTPUT_CAP_BYTES: usize = 1024 * 1024;

/// Appended to a stream that was cut at `OUTPUT_CAP_BYTES`.
pub const TRUNCATED_NOTE: &str = "\n[RingRing: output cut at 1 MiB]";

pub fn home() -> PathBuf {
	std::env::var_os("HOME")
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("/"))
}

/// Empty is the home folder; `~` and `~/…` are under it.
pub fn working_folder(setting: &str) -> PathBuf {
	let setting = setting.trim();
	match setting.strip_prefix('~') {
		_ if setting.is_empty() => home(),
		Some("") => home(),
		Some(rest) if rest.starts_with('/') => home().join(&rest[1..]),
		_ => PathBuf::from(setting),
	}
}

/// `$SHELL` when it names an executable file, else `FALLBACK_SHELL`.
pub fn login_shell() -> PathBuf {
	std::env::var_os("SHELL")
		.map(PathBuf::from)
		.filter(|p| is_executable(p))
		.unwrap_or_else(|| PathBuf::from(FALLBACK_SHELL))
}

fn is_executable(path: &Path) -> bool {
	use std::os::unix::fs::PermissionsExt;
	path.is_absolute()
		&& std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

/// The flags before the command. tcsh and csh refuse `-l` next to any other
/// flag, so they get `-c` alone and read `~/.cshrc` only.
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
		use std::os::unix::process::ExitStatusExt;
		Ok(ShellDone {
			stdout: sink_text(&out),
			stderr: sink_text(&err),
			code: status.and_then(|status| status.code()),
			signal: status.and_then(|status| status.signal()),
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
		cmd.args(shell_flags(&self.shell))
			.arg(&self.command)
			.current_dir(&self.folder)
			.stdin(Stdio::null())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.kill_on_drop(true)
			.process_group(0);
		let mut child = cmd
			.spawn()
			.map_err(|e| format!("Failed to start {}: {e}", self.shell.display()))?;
		let group = ProcessGroupGuard::adopt(child.id(), "shell command");

		let stdout = read_capped(child.stdout.take(), out);
		let stderr = read_capped(child.stderr.take(), err);
		let ((), (), status) = tokio::join!(stdout, stderr, child.wait());
		let status = status.map_err(|e| format!("Failed to wait for the command: {e}"))?;
		group.ended();
		Ok(status)
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
			shell: PathBuf::from("/bin/sh"),
			command: command.to_string(),
			folder: std::env::temp_dir(),
			timeout,
		}
	}

	fn run(job: ShellJob) -> Result<ShellDone, String> {
		tokio::runtime::Builder::new_current_thread()
			.enable_all()
			.build()
			.unwrap()
			.block_on(job.run())
	}

	#[test]
	fn working_folder_expands_home() {
		assert_eq!(working_folder(""), home());
		assert_eq!(working_folder("  ~ "), home());
		assert_eq!(working_folder("~/Downloads"), home().join("Downloads"));
		assert_eq!(working_folder("/tmp"), PathBuf::from("/tmp"));
	}

	#[test]
	fn csh_family_gets_no_login_flag() {
		assert_eq!(shell_flags(Path::new("/bin/tcsh")), ["-c"]);
		assert_eq!(shell_flags(Path::new("/bin/zsh")), ["-l", "-c"]);
	}

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

	#[test]
	fn a_background_job_holding_the_output_keeps_the_run_going_until_the_limit() {
		// 셸은 곧바로 끝나지만 `sleep` 이 stdout 을 쥐고 있다. 제한 시간까지 끝나지 않는다.
		let done = run(job("sleep 30 & echo started", Duration::from_millis(400))).unwrap();
		assert!(done.timed_out);
		assert_eq!(done.stdout, "started\n");
	}

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
		let mut missing = job("true", Duration::from_secs(5));
		missing.folder = PathBuf::from("/definitely/not/here");
		assert!(run(missing).unwrap_err().contains("is not a folder"));
	}
}
