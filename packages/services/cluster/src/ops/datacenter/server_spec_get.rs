use chirp_workflow::prelude::*;
use server_spec::types::ServerSpec;
use std::collections::{HashMap, HashSet};

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
	pub spec: ServerSpec,
}

#[operation]
pub async fn datacenter_server_spec_list(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
	let cluster_configs = ctx.config().server()?.rivet.clusters();

	let datacenters_res = ctx
		.op(crate::ops::datacenter::get::Input {
			datacenter_ids: input.datacenter_ids.clone(),
		})
		.await?;

	let pool_type = if input.pegboard {
		crate::types::PoolType::Pegboard
	} else {
		crate::types::PoolType::Job
	};

	// Lookup hardware IDs for each dc
	let hardware_ids = datacenters_res
		.datacenters
		.iter()
		.filter(|x| match x.provider {
			crate::types::Provider::Manual => false,
			crate::types::Provider::Linode => true,
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
		.filter(|dc| dc.provider == crate::types::Provider::Linode)
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

	let linode_instance_types =
		if ctx.config().server()?.linode.is_some() && !linode_hardware_ids.is_empty() {
			ctx.op(linode::ops::instance_type_get::Input {
				hardware_ids: linode_hardware_ids,
			})
			.await?
			.instance_types
		} else {
			Vec::new()
		};

	// Build tiers
	let mut datacenters_output = Vec::new();
	for datacenter in datacenters_res.datacenters {
		let spec = match datacenter.provider {
			crate::types::Provider::Manual => {
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
			crate::types::Provider::Linode => {
				let hardware_id = unwrap!(hardware_ids.get(&datacenter.datacenter_id));

				let instance_type = unwrap!(
					linode_instance_types
						.iter()
						.find(|it| it.hardware_id == *hardware_id),
					"datacenter linode hardware stats not found"
				);

				ServerSpec::from_linode(instance_type)
			}
		};

		datacenters_output.push(Datacenter {
			datacenter_id: datacenter.datacenter_id,
			spec,
		})
	}

	Ok(Output {
		datacenters: datacenters_output,
	})
}
