use anyhow::*;
use pegboard::system_info::*;

/// Fetches information about the host system.
///
/// Errors should be logged and place `None` instead, except for required properties that Pegboard
/// cannot run without.
///
/// Required properties:
/// - cpu.physical_core_count
/// - memory.total_memory
pub async fn fetch() -> Result<SystemInfo> {
	let sys = sysinfo::System::new_with_specifics(
		sysinfo::RefreshKind::new()
			.with_cpu(sysinfo::CpuRefreshKind::everything())
			.with_memory(sysinfo::MemoryRefreshKind::everything()),
	);

	// Fetch system information
	let system = System {
		boot_time: sysinfo::System::boot_time(),
	};

	// Fetch CPU information
	let cpu_info = sys.cpus().first();
	let cpu = Cpu {
		vendor_id: cpu_info.map(|x| x.vendor_id().to_string()),
		frequency: cpu_info.map(|x| x.frequency() as u64),
		cpu_arch: sysinfo::System::cpu_arch(),
		physical_core_count: sys
			.physical_core_count()
			.context("failed to get core count")? as u64,
	};

	// Fetch Memory information
	let memory = Memory {
		total_memory: sys.total_memory(),
		total_swap: sys.total_swap(),
	};

	// Fetch Network information
	let sys_networks = sysinfo::Networks::new_with_refreshed_list();
	let mut networks = Vec::new();

	for (name, data) in sys_networks.list() {
		networks.push(NetworkData {
			name: name.to_string(),
			ip_networks: data
				.ip_networks()
				.iter()
				.map(|addr| addr.to_string())
				.collect(),
			mac_address: data.mac_address().to_string(),
		});
	}

	let network = Network {
		hostname: sysinfo::System::host_name(),
		networks,
	};

	// Fetch OS information
	let os = Os {
		name: sysinfo::System::name(),
		distribution_id: sysinfo::System::distribution_id(),
		long_os_version: sysinfo::System::long_os_version(),
		os_version: sysinfo::System::os_version(),
		kernel_version: sysinfo::System::kernel_version(),
	};

	// Fetch Storage information
	let sys_disks = sysinfo::Disks::new_with_refreshed_list();
	let mut disks = Vec::new();
	for disk in sys_disks.list() {
		disks.push(StorageDisk {
			name: disk.name().to_string_lossy().to_string(),
			file_system: disk.file_system().to_string_lossy().to_string(),
			kind: disk.kind().to_string(),
			available_space: disk.available_space(),
			total_space: disk.total_space(),
		});
	}
	let storage = Storage { disks };

	// Construct the System struct
	let system = SystemInfo {
		system,
		cpu,
		memory,
		os,
		network,
		storage,
	};

	Ok(system)
}
