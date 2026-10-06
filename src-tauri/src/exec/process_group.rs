// 셸 명령 칸이 띄운 자식 process group 을 지키는 guard — timeout·앱 종료에 group 전체를 끝낸다

//! A child spawned with `process_group(0)` leads its own group, so its pid is
//! the group id. `kill_on_drop` only reaches that one process; whatever it
//! started (a shell's pipeline, a `sleep`) would outlive
//! a timed-out command. This guard closes that gap.

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex, MutexGuard};
#[cfg(unix)]
use std::time::Duration;

/// Group ids of the children still running. The app exit path ends them —
/// exiting does not drop futures, so `Drop` below never runs then.
static RUNNING: LazyLock<Mutex<HashSet<i32>>> = LazyLock::new(Default::default);

/// How long a group gets between SIGTERM and SIGKILL.
#[cfg(unix)]
const KILL_GRACE: Duration = Duration::from_secs(3);

fn running() -> MutexGuard<'static, HashSet<i32>> {
	// Every critical section is one insert, remove or drain.
	RUNNING.lock().unwrap_or_else(|e| e.into_inner())
}

/// One child's process group. Dropped before `ended` — a cancel or a timeout
/// dropped the future holding it — it sends SIGTERM to the group, then
/// SIGKILL after `KILL_GRACE`.
pub(crate) struct ProcessGroupGuard {
	pgid: Option<i32>,
	what: &'static str,
	ended: bool,
}

impl ProcessGroupGuard {
	/// `pid` is the child spawned with `process_group(0)`. `what` names it in the log.
	pub(crate) fn adopt(pid: Option<u32>, what: &'static str) -> Self {
		let pgid = pid.map(|p| p as i32);
		if let Some(pgid) = pgid {
			running().insert(pgid);
			#[cfg(windows)]
			job::adopt(pgid);
		}
		Self {
			pgid,
			what,
			ended: false,
		}
	}

	/// The child ended on its own. The group gets no signal.
	pub(crate) fn ended(mut self) {
		self.ended = true;
	}
}

impl Drop for ProcessGroupGuard {
	fn drop(&mut self) {
		let Some(pgid) = self.pgid else {
			return;
		};
		running().remove(&pgid);
		if self.ended {
			// 스스로 끝난 자식이 남긴 프로세스는 건드리지 않는다. job 만 놓는다.
			#[cfg(windows)]
			job::close(pgid, false);
			return;
		}
		log::info!(
			"[process-group] {} dropped — stopping process group {pgid}",
			self.what
		);
		#[cfg(unix)]
		{
			// SAFETY: killpg 는 메모리를 건드리지 않는다. 대상이 이미 없으면 ESRCH 로 끝난다.
			unsafe {
				libc::killpg(pgid, libc::SIGTERM);
			}
			std::thread::spawn(move || {
				std::thread::sleep(KILL_GRACE);
				// SAFETY: 위와 같다.
				unsafe {
					libc::killpg(pgid, libc::SIGKILL);
				}
			});
		}
		#[cfg(windows)]
		job::close(pgid, true);
	}
}

/// Windows has no process group to signal. The child goes into a job object
/// instead: whatever it starts joins the job, and ending the job ends them all
/// — also the ones whose parent has already exited, which a walk down the
/// process tree would miss.
#[cfg(windows)]
mod job {
	use std::collections::HashMap;
	use std::sync::{LazyLock, Mutex, MutexGuard};

	use windows_sys::Win32::Foundation::CloseHandle;
	use windows_sys::Win32::System::JobObjects::{
		AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
	};
	use windows_sys::Win32::System::Threading::{
		OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
	};

	/// The job of each child still running, by the child's pid. A handle is kept
	/// as an integer so the map can cross threads.
	static JOBS: LazyLock<Mutex<HashMap<i32, isize>>> = LazyLock::new(Default::default);

	fn jobs() -> MutexGuard<'static, HashMap<i32, isize>> {
		JOBS.lock().unwrap_or_else(|e| e.into_inner())
	}

	/// Put the child `pid` into a new job. A failure leaves the child without
	/// one: a timeout then ends the child alone (`kill_on_drop`).
	pub(super) fn adopt(pid: i32) {
		// SAFETY: 핸들은 여기서 만들고 여기서 닫는다. 실패한 호출은 null 이나 0 을 돌려주고 메모리를 건드리지 않는다.
		unsafe {
			let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
			if job.is_null() {
				log::warn!("[process-group] failed to create a job for {pid}");
				return;
			}
			let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid as u32);
			let assigned = !process.is_null() && AssignProcessToJobObject(job, process) != 0;
			if !process.is_null() {
				CloseHandle(process);
			}
			if !assigned {
				log::warn!("[process-group] failed to put {pid} into its job");
				CloseHandle(job);
				return;
			}
			jobs().insert(pid, job as isize);
		}
	}

	/// Forget the job of `pid`. With `end`, everything in it ends first.
	pub(super) fn close(pid: i32, end: bool) {
		let Some(job) = jobs().remove(&pid) else {
			return;
		};
		// SAFETY: `adopt` 가 넣은 핸들이고, map 에서 꺼냈으므로 한 번만 닫힌다.
		unsafe {
			if end {
				TerminateJobObject(job as _, 1);
			}
			CloseHandle(job as _);
		}
	}
}

/// The app exit path. Ends every group still running.
pub(crate) fn terminate_all_on_exit() {
	let groups: Vec<i32> = running().drain().collect();
	for pgid in groups {
		log::info!("[process-group] app exiting — terminating process group {pgid}");
		#[cfg(unix)]
		// SAFETY: killpg 는 메모리를 건드리지 않는다.
		unsafe {
			libc::killpg(pgid, libc::SIGTERM);
		}
		#[cfg(windows)]
		job::close(pgid, true);
	}
}
