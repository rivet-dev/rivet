use rand::{distributions::Alphanumeric, Rng};

pub mod api;
pub mod client;
pub mod consts;

// NOTE: We don't reserve CPU because Nomad is running as a higher priority process than the rest and
// shouldn't be doing much heavy lifting.
const RESERVE_SYSTEM_MEMORY: u64 = 512;
// See module.traefik_job resources
const RESERVE_LB_MEMORY: u64 = 512;
const RESERVE_MEMORY: u64 = RESERVE_SYSTEM_MEMORY + RESERVE_LB_MEMORY;

const CPU_PER_CORE: u64 = 1999;

/// Provider agnostic hardware specs.
#[derive(Debug)]
pub struct JobNodeConfig {
	pub cpu_cores: u64,
	pub cpu: u64,
	pub memory: u64,
	pub disk: u64,
	pub bandwidth: u64,
}

impl JobNodeConfig {
	pub fn from_linode(instance_type: &crate::types::InstanceType) -> JobNodeConfig {
		// Account for kernel memory overhead
		// https://www.linode.com/community/questions/17791/why-doesnt-free-m-match-the-full-amount-of-ram-of-my-nanode-plan
		let memory = instance_type.memory * 96 / 100;
		// Remove reserved resources
		let memory = memory - RESERVE_MEMORY;

		JobNodeConfig {
			cpu_cores: instance_type.vcpus,
			cpu: instance_type.vcpus * CPU_PER_CORE,
			memory,
			disk: instance_type.disk,
			bandwidth: instance_type.transfer * 1000,
		}
	}

	pub fn cpu_per_core(&self) -> u64 {
		CPU_PER_CORE
	}

	pub fn memory_per_core(&self) -> u64 {
		self.memory / self.cpu_cores
	}

	pub fn disk_per_core(&self) -> u64 {
		self.disk / self.cpu_cores
	}

	pub fn bandwidth_per_core(&self) -> u64 {
		self.bandwidth / self.cpu_cores
	}
}

/// Generates a random string for a secret.
pub(crate) fn generate_password(length: usize) -> String {
	rand::thread_rng()
		.sample_iter(&Alphanumeric)
		.take(length)
		.map(char::from)
		.collect()
}
