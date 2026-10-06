// 셸 명령 칸이 띄운 자식 process group 을 지키는 guard — timeout·앱 종료에 group 전체를 끝낸다

//! A child spawned with `process_group(0)` leads its own group, so its pid is
//! the group id. `kill_on_drop` only reaches that one process; whatever it
//! started (a shell's pipeline, a `sleep`) would outlive
//! a timed-out command. This guard closes that gap.

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::Duration;

/// Group ids of the children still running. The app exit path ends them —
/// exiting does not drop futures, so `Drop` below never runs then.
static RUNNING: LazyLock<Mutex<HashSet<i32>>> = LazyLock::new(Default::default);

/// How long a group gets between SIGTERM and SIGKILL.
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
	}
}
