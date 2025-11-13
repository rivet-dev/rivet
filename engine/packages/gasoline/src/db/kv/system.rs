use std::time::Instant;

use sysinfo::{CpuRefreshKind, MINIMUM_CPU_UPDATE_INTERVAL, RefreshKind, System};

pub struct SystemInfo {
	system: System,
	last_cpu_usage_read: Instant,
}

impl SystemInfo {
	pub fn new() -> Self {
		SystemInfo {
			system: System::new_with_specifics(
				RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing().with_cpu_usage()),
			),
			last_cpu_usage_read: Instant::now(),
		}
	}

	/// Returns a float 0.0-100.0 of the avg cpu usage over the entire system.
	pub fn cpu_usage(&mut self) -> f32 {
		if self.last_cpu_usage_read.elapsed() > MINIMUM_CPU_UPDATE_INTERVAL {
			self.system.refresh_cpu_usage();
			self.last_cpu_usage_read = Instant::now();
		}

		self.system
			.cpus()
			.iter()
			.fold(0.0, |s, cpu| s + cpu.cpu_usage())
			/ self.system.cpus().len() as f32
	}
}
