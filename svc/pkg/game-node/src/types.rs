use crate::{
	CPU_PER_CORE, DISK_PER_CORE, NOMAD_RESERVE_MEMORY, PEGBOARD_RESERVE_MEMORY, RESERVE_LB_MEMORY,
};

/// Provider agnostic hardware specs.
#[derive(Debug)]
pub struct GameNodeConfig {
	pub cpu_cores: u32,
	/// Mhz
	pub cpu: u32,
	/// MiB
	pub memory: u32,
	/// MiB
	pub disk: u32,
	/// Kibps
	pub bandwidth: u32,
}

impl GameNodeConfig {
	pub fn from_linode(instance_type: &linode::types::InstanceType) -> GameNodeConfig {
		// Account for kernel memory overhead
		// https://www.linode.com/community/questions/17791/why-doesnt-free-m-match-the-full-amount-of-ram-of-my-nanode-plan
		let memory = instance_type.memory * 95 / 100;
		// Remove reserved resources
		let memory = memory - RESERVE_LB_MEMORY;

		GameNodeConfig {
			cpu_cores: instance_type.vcpus,
			cpu: instance_type.vcpus * CPU_PER_CORE,
			memory,
			// MB to MiB
			disk: instance_type.disk * 1000 / 1024 * 1000 / 1024,
			// Mbps to Kibps
			bandwidth: instance_type.network_out * 1000 / 1024 * 1000,
		}
	}

	pub fn cpu_per_core(&self) -> u32 {
		CPU_PER_CORE
	}

	pub fn memory_per_core_nomad(&self) -> u32 {
		(self.memory - NOMAD_RESERVE_MEMORY) / self.cpu_cores
	}

	pub fn memory_per_core_pb(&self) -> u32 {
		(self.memory - PEGBOARD_RESERVE_MEMORY) / self.cpu_cores
	}

	pub fn disk_per_core(&self) -> u32 {
		DISK_PER_CORE
	}

	pub fn bandwidth_per_core(&self) -> u32 {
		self.bandwidth / self.cpu_cores
	}
}
