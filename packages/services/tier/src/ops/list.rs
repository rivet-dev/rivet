use chirp_workflow::prelude::*;
use server_spec::types::ServerSpec;

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
	let server_spec = ctx
		.op(cluster::ops::datacenter::server_spec_get::Input {
			datacenter_ids: input.datacenter_ids.clone(),
			pegboard: input.pegboard,
		})
		.await?;

	let datacenters = server_spec
		.datacenters
		.iter()
		.map(|dc| Datacenter {
			datacenter_id: dc.datacenter_id,
			tiers: vec![
				generate_tier(input.pegboard, &dc.spec, "basic-4d1", 4, 1),
				generate_tier(input.pegboard, &dc.spec, "basic-2d1", 2, 1),
				generate_tier(input.pegboard, &dc.spec, "basic-1d1", 1, 1),
				generate_tier(input.pegboard, &dc.spec, "basic-1d2", 1, 2),
				generate_tier(input.pegboard, &dc.spec, "basic-1d4", 1, 4),
				generate_tier(input.pegboard, &dc.spec, "basic-1d8", 1, 8),
				generate_tier(input.pegboard, &dc.spec, "basic-1d16", 1, 16),
			],
		})
		.collect();

	Ok(Output { datacenters })
}

fn generate_tier(
	pegboard: bool,
	c: &ServerSpec,
	name: &str,
	numerator: u32,
	denominator: u32,
) -> Tier {
	let memory_per_core = if pegboard {
		c.memory_per_core_pb_container()
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
