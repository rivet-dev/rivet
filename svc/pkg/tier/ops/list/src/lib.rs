use linode::util::JobNodeConfig;
use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "tier-list")]
async fn handle(ctx: OperationContext<tier::list::Request>) -> GlobalResult<tier::list::Response> {
	let datacenters_res = chirp_workflow::compat::op(
		&ctx,
		cluster::ops::datacenter::get::Input {
			datacenter_ids: ctx.region_ids.iter().map(|id| id.as_uuid()).collect(),
		},
	)
	.await?;

	let hardware = datacenters_res
		.datacenters
		.iter()
		.map(|dc| {
			let job_pool = unwrap!(
				dc.pools
					.iter()
					.find(|pool| pool.pool_type == cluster::types::PoolType::Job),
				"no job pool"
			);

			// Choose the first hardware in the list
			let hardware = unwrap!(job_pool.hardware.first(), "no hardware")
				.provider_hardware
				.clone();

			Ok((dc.datacenter_id, hardware))
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	let instance_types_res = chirp_workflow::compat::op(
		&ctx,
		linode::ops::instance_type_get::Input {
			hardware_ids: hardware
				.iter()
				.map(|(_, hardware)| hardware.clone())
				.collect::<Vec<_>>(),
		},
	)
	.await?;

	let regions = hardware
		.into_iter()
		.map(|(datacenter_id, hardware)| {
			let instance_type = unwrap!(
				instance_types_res
					.instance_types
					.iter()
					.find(|it| it.hardware_id == hardware),
				"datacenter hardware stats not found"
			);
			let _config = JobNodeConfig::from_linode(instance_type);

			let config = JobNodeConfig::from_linode(&linode::types::InstanceType {
				hardware_id: "".to_string(),
				vcpus: 8,
				memory: 2u64.pow(14),
				disk: 2u64.pow(15) * 10,
				transfer: 6_000,
			});

			Ok(tier::list::response::Region {
				region_id: Some(datacenter_id.into()),
				tiers: vec![
					generate_tier(&config, "basic-4d1", 4, 1),
					generate_tier(&config, "basic-2d1", 2, 1),
					generate_tier(&config, "basic-1d1", 1, 1),
					generate_tier(&config, "basic-1d2", 1, 2),
					generate_tier(&config, "basic-1d4", 1, 4),
					generate_tier(&config, "basic-1d8", 1, 8),
					generate_tier(&config, "basic-1d16", 1, 16),
				],
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(tier::list::Response { regions })
}

fn generate_tier(
	c: &JobNodeConfig,
	name: &str,
	numerator: u64,
	denominator: u64,
) -> backend::region::Tier {
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
