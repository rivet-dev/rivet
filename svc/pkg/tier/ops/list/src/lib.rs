use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

// See pkr/static/nomad-config.hcl.tpl client.reserved
const RESERVE_SYSTEM_CPU: u64 = 500;
const RESERVE_SYSTEM_MEMORY: u64 = 512;

// See module.traefik_job resources
const RESERVE_LB_CPU: u64 = 1500;
const RESERVE_LB_MEMORY: u64 = 512;

const RESERVE_CPU: u64 = RESERVE_SYSTEM_CPU + RESERVE_LB_CPU;
const RESERVE_MEMORY: u64 = RESERVE_SYSTEM_MEMORY + RESERVE_LB_MEMORY;

struct GameNodeConfig {
	cpu_cores: u64,
	cpu: u64,
	memory: u64,
	disk: u64,
	bandwidth: u64,
}

impl GameNodeConfig {
	fn cpu_per_core(&self) -> u64 {
		self.cpu / self.cpu_cores
	}

	fn memory_per_core(&self) -> u64 {
		self.memory / self.cpu_cores
	}

	fn disk_per_core(&self) -> u64 {
		self.disk / self.cpu_cores
	}

	fn bandwidth_per_core(&self) -> u64 {
		self.bandwidth / self.cpu_cores
	}
}

/// Returns the default game node config.
fn get_game_node_config() -> GameNodeConfig {
	// TODO: CPU should be different based on the provider. For now, we use the
	// minimum value from tf/prod/config.tf

	// Multiply config for 2 core, 4 GB to scale up to the 4 core, 8 GB
	// plan
	let mut config = GameNodeConfig {
		cpu_cores: 4,
		// DigitalOcean: 7,984
		// Linode: 7,996
		cpu: 7900,
		// DigitalOcean: 7,957
		// Linode: 7,934
		memory: 7900,
		disk: 64_000,
		bandwidth: 2_000_000,
	};

	// Remove reserved resources
	config.cpu -= RESERVE_CPU;
	config.memory -= RESERVE_MEMORY;

	config
}

#[operation(name = "tier-list")]
async fn handle(ctx: OperationContext<tier::list::Request>) -> GlobalResult<tier::list::Response> {
	let region_ids = ctx
		.region_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let tiers = vec![
		generate_tier("basic-4d1", 4, 1),
		generate_tier("basic-2d1", 2, 1),
		generate_tier("basic-1d1", 1, 1),
		generate_tier("basic-1d2", 1, 2),
		generate_tier("basic-1d4", 1, 4),
		generate_tier("basic-1d8", 1, 8),
		generate_tier("basic-1d16", 1, 16),
	];

	Ok(tier::list::Response {
		regions: region_ids
			.into_iter()
			.map(|region_id| tier::list::response::Region {
				region_id: Some(region_id.into()),
				tiers: tiers.clone(),
			})
			.collect::<Vec<_>>(),
	})
}

fn generate_tier(name: &str, numerator: u64, denominator: u64) -> backend::region::Tier {
	let c = get_game_node_config();

	backend::region::Tier {
		tier_name_id: name.into(),
		rivet_cores_numerator: numerator as u32,
		rivet_cores_denominator: denominator as u32,
		cpu: c.cpu_per_core() * numerator / denominator,
		memory: c.memory_per_core() * numerator / denominator,
		// Allow oversubscribing memory by 50% of the reserved
		// memory
		memory_max: u64::min(
			(c.memory_per_core() * numerator / denominator) * 3 / 2,
			c.memory,
		),
		disk: c.disk_per_core() * numerator / denominator,
		bandwidth: c.bandwidth_per_core() * numerator / denominator,
	}
}
