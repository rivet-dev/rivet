use std::{fs, io};

use nix::libc::{c_int, sched_param, sched_setscheduler, setpriority, PRIO_PROCESS};

#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedPolicy {
	Other = nix::libc::SCHED_OTHER,
	Fifo = nix::libc::SCHED_FIFO,
	Rr = nix::libc::SCHED_RR,
	Batch = nix::libc::SCHED_BATCH,
	Idle = nix::libc::SCHED_IDLE,
}

/// `nice_value` must be between -20 (highest priority) to 19 (lowest priority)
pub fn set_nice_level(pid: i32, nice_value: i32) -> io::Result<()> {
	let result = unsafe { setpriority(PRIO_PROCESS, pid as u32, nice_value as c_int) };

	if result == -1 {
		Err(io::Error::last_os_error())
	} else {
		Ok(())
	}
}

/// `oom_score_adj` must be between -1000 (don't kill) to 1000 (most likely to be killed)
pub fn set_oom_score_adj(pid: i32, oom_score_adj: i32) -> io::Result<()> {
	fs::write(
		format!("/proc/{pid}/oom_score_adj"),
		oom_score_adj.to_string(),
	)
}

/// with SchedPolicy::Other, SchedPolicy::Batch, and SchedPolicy::Idle `priority` must be 0. With
/// SchedPolicy::Fifo and SchedPolicy::Rr `priority` must be between 1 (lowest) to 99 (highest).
pub fn set_scheduling_policy(pid: i32, policy: SchedPolicy, priority: i32) -> io::Result<()> {
	let param = sched_param {
		sched_priority: priority,
	};

	let result = unsafe { sched_setscheduler(pid, policy as c_int, &param) };

	if result == -1 {
		Err(io::Error::last_os_error())
	} else {
		Ok(())
	}
}
