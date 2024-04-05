use types::rivet::backend::{self, pkg::*};
use uuid::Uuid;

pub mod test;

// NOTE: We don't reserve CPU because Nomad is running as a higher priority process than the rest and
// shouldn't be doing much heavy lifting.
const RESERVE_SYSTEM_MEMORY: u64 = 512;
// See module.traefik_job resources
const RESERVE_LB_MEMORY: u64 = 512;
const RESERVE_MEMORY: u64 = RESERVE_SYSTEM_MEMORY + RESERVE_LB_MEMORY;

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
	pub fn from_linode(instance_type: &linode::instance_type_get::response::InstanceType) -> JobNodeConfig {
		// Account for kernel memory overhead
		// https://www.linode.com/community/questions/17791/why-doesnt-free-m-match-the-full-amount-of-ram-of-my-nanode-plan
		let memory = instance_type.memory * 96 / 100;
		// Remove reserved resources
		let memory = memory - RESERVE_MEMORY;

		JobNodeConfig {
			cpu_cores: instance_type.vcpus,
			cpu: instance_type.vcpus * 1999,
			memory,
			disk: instance_type.disk,
			bandwidth: instance_type.transfer * 1000,
		}
	}

	pub fn cpu_per_core(&self) -> u64 {
		1999
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

pub fn server_name(
	provider_datacenter_id: &str,
	pool_type: backend::cluster::PoolType,
	server_id: Uuid,
) -> String {
	let ns = rivet_util::env::namespace();
	let pool_type_str = match pool_type {
		backend::cluster::PoolType::Job => "job",
		backend::cluster::PoolType::Gg => "gg",
		backend::cluster::PoolType::Ats => "ats",
	};

	format!(
		"{ns}-{provider_datacenter_id}-{pool_type_str}-{server_id}",
	)
}

// Use the hash of the server install script in the image variant so that if the install scripts are updated
// we wont be using the old image anymore
const CLUSTER_SERVER_INSTALL_HASH: &str = include_str!("../gen/hash.txt");

// Used for linode labels which have to be between 3 and 64 characters for some reason
pub fn simple_image_variant(
	provider_datacenter_id: &str,
	pool_type: backend::cluster::PoolType,
) -> String {
	let ns = rivet_util::env::namespace();
	let pool_type_str = match pool_type {
		backend::cluster::PoolType::Job => "job",
		backend::cluster::PoolType::Gg => "gg",
		backend::cluster::PoolType::Ats => "ats",
	};

	format!("{ns}-{provider_datacenter_id}-{pool_type_str}")
}

pub fn image_variant(
	provider: backend::cluster::Provider,
	provider_datacenter_id: &str,
	pool_type: backend::cluster::PoolType,
) -> String {
	let ns = rivet_util::env::namespace();
	let provider_str = match provider {
		backend::cluster::Provider::Linode => "linode",
	};
	let pool_type_str = match pool_type {
		backend::cluster::PoolType::Job => "job",
		backend::cluster::PoolType::Gg => "gg",
		backend::cluster::PoolType::Ats => "ats",
	};

	format!("{ns}-{CLUSTER_SERVER_INSTALL_HASH}-{provider_str}-{provider_datacenter_id}-{pool_type_str}")
}
