use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct SystemInfo {
	pub system: System,
	pub cpu: Cpu,
	pub memory: Memory,
	pub os: Os,
	pub network: Network,
	pub storage: Storage,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct System {
	pub boot_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Cpu {
	pub vendor_id: Option<String>,
	pub frequency: Option<u64>,
	pub cpu_arch: Option<String>,
	pub physical_core_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Memory {
	// Bytes
	pub total_memory: u64,
	// Bytes
	pub total_swap: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Os {
	pub name: Option<String>,
	pub distribution_id: String,
	pub long_os_version: Option<String>,
	pub os_version: Option<String>,
	pub kernel_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Network {
	pub hostname: Option<String>,
	pub networks: Vec<NetworkData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct NetworkData {
	pub name: String,
	pub ip_networks: Vec<String>,
	pub mac_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Storage {
	pub disks: Vec<StorageDisk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct StorageDisk {
	pub name: String,
	pub file_system: String,
	pub kind: String,
	pub available_space: u64,
	pub total_space: u64,
}
