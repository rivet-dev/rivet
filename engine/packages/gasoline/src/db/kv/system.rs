use std::fs;
use std::time::{Duration, Instant};

use sysinfo::{CpuRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

const CPU_UPDATE_INTERVAL: Duration = Duration::from_millis(150);

pub struct SystemInfo {
	system: System,
	pid: Pid,
	last_cpu_usage_read: Instant,
	cgroup_cpu_max: Option<CgroupCpuMax>,
	last_cgroup_usage_usec: Option<u64>,
	last_cgroup_cores: f32,
}

impl SystemInfo {
	pub fn new() -> Self {
		SystemInfo {
			system: System::new_with_specifics(
				RefreshKind::nothing()
					.with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
					.with_processes(ProcessRefreshKind::nothing().with_cpu()),
			),
			pid: Pid::from_u32(std::process::id()),
			last_cpu_usage_read: Instant::now(),
			cgroup_cpu_max: CgroupCpuMax::read(),
			last_cgroup_usage_usec: CgroupCpuUsage::read(),
			last_cgroup_cores: 0.0,
		}
	}

	/// Returns a float 0.0-1.0 of the avg cpu usage in the current container (if cgroups are configured) or
	/// otherwise for the current process.
	pub fn cpu_usage_ratio(&mut self, cpu_max: Option<usize>) -> f32 {
		// 1 = 1 core
		let cpu_max = if let Some(cpu_max) = cpu_max {
			cpu_max as f32 / 1000.0
		} else {
			if let Some(CgroupCpuMax { quota, period }) = self.cgroup_cpu_max {
				if quota > 0 {
					quota as f32 / period as f32
				} else {
					// Negative quota means unlimited, use cpu count
					self.system.cpus().len() as f32
				}
			} else {
				self.system.cpus().len() as f32
			}
		};

		let total = if let Some(last_usage_usec) = self.last_cgroup_usage_usec {
			// Use cgroup cpu.stat for usage (cumulative counter)
			if self.last_cpu_usage_read.elapsed() > CPU_UPDATE_INTERVAL {
				if let Some(current_usage_usec) = CgroupCpuUsage::read() {
					let elapsed_usec = self.last_cpu_usage_read.elapsed().as_micros() as u64;
					let usage_delta_usec = current_usage_usec.saturating_sub(last_usage_usec);

					// Calculate cores used: (usage_delta / elapsed_time)
					let cores_used = if elapsed_usec > 0 {
						usage_delta_usec as f32 / elapsed_usec as f32
					} else {
						0.0
					};

					self.last_cgroup_usage_usec = Some(current_usage_usec);
					self.last_cpu_usage_read = Instant::now();
					self.last_cgroup_cores = cores_used;

					cores_used
				} else {
					// Failed to read cgroup, disable cgroup usage tracking
					self.last_cgroup_usage_usec = None;
					0.0
				}
			} else {
				// Not time to update yet, return last calculated value
				self.last_cgroup_cores
			}
		} else {
			// Use per-process CPU metrics
			if self.last_cpu_usage_read.elapsed() > CPU_UPDATE_INTERVAL {
				self.system.refresh_processes_specifics(
					ProcessesToUpdate::Some(&[self.pid]),
					true,
					ProcessRefreshKind::nothing().with_cpu(),
				);
				self.last_cpu_usage_read = Instant::now();
			}

			// Get CPU usage for current process (returns percentage 0-100 per core)
			self.system
				.process(self.pid)
				.map(|p| p.cpu_usage() / 100.0)
				.unwrap_or(0.0)
		};

		crate::metrics::CPU_USAGE.observe(total as f64);

		total / cpu_max
	}
}

struct CgroupCpuMax {
	quota: i64,
	period: u64,
}

impl CgroupCpuMax {
	fn read() -> Option<Self> {
		// cgroups v2
		if let Ok(content) = fs::read_to_string("/sys/fs/cgroup/cpu.max") {
			let parts = content.trim().split_whitespace().collect::<Vec<&str>>();
			if parts.len() == 2 {
				return Some(CgroupCpuMax {
					quota: parts[0].parse::<i64>().ok()?,
					period: parts[1].parse::<u64>().ok()?,
				});
			}
		}

		// cgroups v1
		let quota = fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_quota_us")
			.ok()?
			.trim()
			.parse()
			.ok()?;
		let period = fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_period_us")
			.ok()?
			.trim()
			.parse()
			.ok()?;

		Some(CgroupCpuMax { quota, period })
	}
}

struct CgroupCpuUsage;

impl CgroupCpuUsage {
	/// Reads CPU usage from cgroup cpu.stat
	/// Returns usage in microseconds
	fn read() -> Option<u64> {
		// cgroups v2
		if let Ok(content) = fs::read_to_string("/sys/fs/cgroup/cpu.stat") {
			for line in content.lines() {
				if let Some(usage) = line.strip_prefix("usage_usec ") {
					return usage.trim().parse().ok();
				}
			}
		}

		// cgroups v1
		if let Ok(content) = fs::read_to_string("/sys/fs/cgroup/cpuacct/cpuacct.usage") {
			// cpuacct.usage is in nanoseconds, convert to microseconds
			let usage_nsec: u64 = content.trim().parse().ok()?;
			return Some(usage_nsec / 1000);
		}

		None
	}
}
