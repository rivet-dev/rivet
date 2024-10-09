use chirp_workflow::prelude::*;
use cluster::util::GameNodeConfig;

use crate::types::Tier;

#[derive(Debug, Default)]
pub struct Input {
	pub datacenter_ids: Vec<Uuid>,
	pub pegboard: bool,
}

#[derive(Debug)]
pub struct Output {
	pub datacenters: Vec<Datacenter>,
}

#[derive(Debug)]
pub struct Datacenter {
	pub datacenter_id: Uuid,
	pub tiers: Vec<Tier>,
}

#[operation]
pub async fn tier_list(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let datacenters_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: input.datacenter_ids.clone(),
		})
		.await?;

	let hardware = datacenters_res
		.datacenters
		.iter()
		.map(|dc| {
			let pool_type = if input.pegboard {
				cluster::types::PoolType::Pegboard
			} else {
				cluster::types::PoolType::Job
			};
			let game_pool = unwrap!(
				dc.pools.iter().find(|pool| pool.pool_type == pool_type),
				"no game pool"
			);

			// Choose the first hardware in the list
			let hardware = unwrap!(game_pool.hardware.first(), "no hardware")
				.provider_hardware
				.clone();

			Ok((dc.datacenter_id, hardware))
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	// TODO: Hardcoded to linode for now
	let instance_types_res = ctx
		.op(linode::ops::instance_type_get::Input {
			hardware_ids: hardware
				.iter()
				.map(|(_, hardware)| hardware.clone())
				.collect::<Vec<_>>(),
		})
		.await?;

	let datacenters = hardware
		.into_iter()
		.map(|(datacenter_id, hardware)| {
			let instance_type = unwrap!(
				instance_types_res
					.instance_types
					.iter()
					.find(|it| it.hardware_id == hardware),
				"datacenter hardware stats not found"
			);
			let config = GameNodeConfig::from_linode(instance_type);

			Ok(Datacenter {
				datacenter_id,
				tiers: vec![
					generate_tier(input.pegboard, &config, "basic-4d1", 4, 1),
					generate_tier(input.pegboard, &config, "basic-2d1", 2, 1),
					generate_tier(input.pegboard, &config, "basic-1d1", 1, 1),
					generate_tier(input.pegboard, &config, "basic-1d2", 1, 2),
					generate_tier(input.pegboard, &config, "basic-1d4", 1, 4),
					generate_tier(input.pegboard, &config, "basic-1d8", 1, 8),
					generate_tier(input.pegboard, &config, "basic-1d16", 1, 16),
				],
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(Output { datacenters })
}

fn generate_tier(
	pegboard: bool,
	c: &GameNodeConfig,
	name: &str,
	numerator: u64,
	denominator: u64,
) -> Tier {
	let memory_per_core = if pegboard {
		c.memory_per_core_pb()
	} else {
		c.memory_per_core_nomad()
	};

	Tier {
		tier_name_id: name.into(),
		rivet_cores_numerator: numerator as u32,
		rivet_cores_denominator: denominator as u32,
		cpu: c.cpu_per_core() * numerator / denominator,
		memory: memory_per_core * numerator / denominator,
		// Allow oversubscribing memory by 50% of the reserved
		// memory
		memory_max: u64::min(
			(memory_per_core * numerator / denominator) * 3 / 2,
			c.memory,
		),
		disk: c.disk_per_core() * numerator / denominator,
		bandwidth: c.bandwidth_per_core() * numerator / denominator,
	}
}
