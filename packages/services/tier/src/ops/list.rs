use chirp_workflow::prelude::*;
use server_spec::types::ServerSpec;
use std::collections::{HashMap, HashSet};

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
	let cluster_configs = ctx.config().server()?.rivet.clusters();

	let datacenters_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: input.datacenter_ids.clone(),
		})
		.await?;

	let pool_type = if input.pegboard {
		cluster::types::PoolType::Pegboard
	} else {
		cluster::types::PoolType::Job
	};

	// Lookup hardware IDs for each dc
	let hardware_ids = datacenters_res
		.datacenters
		.iter()
		.filter(|x| match x.provider {
			cluster::types::Provider::Manual => false,
			cluster::types::Provider::Linode => true,
		})
		.map(|dc| {
			let game_pool = unwrap!(
				dc.pools.iter().find(|pool| pool.pool_type == pool_type),
				"no {} pool",
				pool_type
			);

			// Choose the first hardware in the list, the rest are fallback hardware
			let hardware = unwrap!(game_pool.hardware.first(), "no hardware")
				.provider_hardware
				.clone();

			Ok((dc.datacenter_id, hardware))
		})
		.collect::<GlobalResult<HashMap<_, _>>>()?;

	// Fetch all hardware sizes from Linode
	let linode_hardware_ids = datacenters_res
		.datacenters
		.iter()
		.filter(|dc| dc.provider == cluster::types::Provider::Linode)
		.map(|dc| {
			let game_pool = unwrap!(
				dc.pools.iter().find(|pool| pool.pool_type == pool_type),
				"no {} pool",
				pool_type
			);

			// Choose the first hardware in the list, the rest are fallback hardware
			let hardware = unwrap!(game_pool.hardware.first(), "no hardware")
				.provider_hardware
				.clone();

			Ok(hardware)
		})
		.collect::<GlobalResult<HashSet<_>>>()?
		.into_iter()
		.collect::<Vec<_>>();

	let instance_types_res = ctx
		.op(linode::ops::instance_type_get::Input {
			hardware_ids: linode_hardware_ids,
		})
		.await?;

	// Build tiers
	let mut datacenters_output = Vec::new();
	for datacenter in datacenters_res.datacenters {
		let server_spec = match datacenter.provider {
			cluster::types::Provider::Manual => {
				// TODO(RVT-4026): Switch to being stored in CRDB
				// Look up hardware in config
				let (_, cluster_config) = unwrap!(
					cluster_configs
						.iter()
						.find(|(_, c)| c.id == datacenter.cluster_id),
					"could not find matching cluster config"
				);
				let (_, dc_config) = unwrap!(
					cluster_config
						.datacenters
						.iter()
						.find(|(_, dc_config)| dc_config.id == datacenter.datacenter_id),
					"could not find matching datacenter config"
				);
				let hardware = unwrap_ref!(
					dc_config.hardware,
					"hardware not specified for datacenter with manual provider"
				);

				ServerSpec {
					cpu_cores: hardware.cpu_cores,
					cpu: hardware.cpu,
					memory: hardware.memory,
					disk: hardware.disk,
					bandwidth: hardware.bandwidth,
				}
			}
			cluster::types::Provider::Linode => {
				let hardware_id = unwrap!(hardware_ids.get(&datacenter.datacenter_id));

				let instance_type = unwrap!(
					instance_types_res
						.instance_types
						.iter()
						.find(|it| it.hardware_id == *hardware_id),
					"datacenter linode hardware stats not found"
				);

				ServerSpec::from_linode(instance_type)
			}
		};

		datacenters_output.push(Datacenter {
			datacenter_id: datacenter.datacenter_id,
			tiers: vec![
				generate_tier(input.pegboard, &server_spec, "basic-4d1", 4, 1),
				generate_tier(input.pegboard, &server_spec, "basic-2d1", 2, 1),
				generate_tier(input.pegboard, &server_spec, "basic-1d1", 1, 1),
				generate_tier(input.pegboard, &server_spec, "basic-1d2", 1, 2),
				generate_tier(input.pegboard, &server_spec, "basic-1d4", 1, 4),
				generate_tier(input.pegboard, &server_spec, "basic-1d8", 1, 8),
				generate_tier(input.pegboard, &server_spec, "basic-1d16", 1, 16),
			],
		})
	}

	Ok(Output {
		datacenters: datacenters_output,
	})
}

fn generate_tier(
	pegboard: bool,
	c: &ServerSpec,
	name: &str,
	numerator: u32,
	denominator: u32,
) -> Tier {
	let memory_per_core = if pegboard {
		c.memory_per_core_pb()
	} else {
		c.memory_per_core_nomad()
	};

	Tier {
		tier_name_id: name.into(),
		rivet_cores_numerator: numerator,
		rivet_cores_denominator: denominator,
		cpu: c.cpu_per_core() * numerator / denominator,
		cpu_millicores: 1000 * numerator / denominator,
		memory: memory_per_core * numerator / denominator,
		// Allow oversubscribing memory by 50% of the reserved
		// memory
		memory_max: u32::min(
			(memory_per_core * numerator / denominator) * 3 / 2,
			c.memory,
		),
		disk: c.disk_per_core() * numerator / denominator,
		bandwidth: c.bandwidth_per_core() * numerator / denominator,
	}
}
