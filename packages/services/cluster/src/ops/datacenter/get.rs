use std::convert::TryInto;

use chirp_workflow::prelude::*;

use crate::types::{BuildDeliveryMethod, Datacenter, Pool, Provider};

#[derive(Debug)]
pub struct Input {
	pub datacenter_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub datacenters: Vec<Datacenter>,
}

#[derive(sqlx::FromRow)]
struct DatacenterRow {
	datacenter_id: Uuid,
	cluster_id: Uuid,
	name_id: String,
	display_name: String,
	provider: i64,
	provider_datacenter_id: String,
	provider_api_token: Option<String>,
	pools2: sqlx::types::Json<Vec<Pool>>,
	build_delivery_method: i64,
	prebakes_enabled: bool,
	create_ts: i64,
	guard_public_hostname_dns_parent: Option<String>,
	guard_public_hostname_static: Option<String>,
}

impl DatacenterRow {
	fn into_datacenter(self, config: &rivet_config::Config) -> GlobalResult<Datacenter> {
		Ok(Datacenter {
			datacenter_id: self.datacenter_id,
			cluster_id: self.cluster_id,
			name_id: self.name_id,
			display_name: self.display_name,
			create_ts: self.create_ts,
			provider: unwrap!(Provider::from_repr(self.provider.try_into()?)),
			provider_datacenter_id: self.provider_datacenter_id,
			provider_api_token: self.provider_api_token,
			pools: self.pools2.0,
			build_delivery_method: unwrap!(BuildDeliveryMethod::from_repr(
				self.build_delivery_method.try_into()?
			)),
			prebakes_enabled: self.prebakes_enabled,
			guard_public_hostname: crate::types::GuardPublicHostname::from_columns(
				config,
				self.datacenter_id,
				self.guard_public_hostname_dns_parent,
				self.guard_public_hostname_static,
			)?,
		})
	}
}

#[operation]
pub async fn cluster_datacenter_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let datacenters = ctx
		.cache()
		.fetch_all_json("cluster.datacenters2", input.datacenter_ids.clone(), {
			let ctx = ctx.clone();
			move |mut cache, datacenter_ids| {
				let ctx = ctx.clone();
				async move {
					let dcs = get_dcs(ctx, datacenter_ids).await?;
					for dc in dcs {
						let dc_id = dc.datacenter_id;
						cache.resolve(&dc_id, dc);
					}

					Ok(cache)
				}
			}
		})
		.await?;

	Ok(Output { datacenters })
}

async fn get_dcs(ctx: OperationCtx, datacenter_ids: Vec<Uuid>) -> GlobalResult<Vec<Datacenter>> {
	let dc_rows = sql_fetch_all!(
		[ctx, DatacenterRow]
		"
		SELECT
			datacenter_id,
			cluster_id,
			name_id,
			display_name,
			provider,
			provider_datacenter_id,
			provider_api_token,
			pools2,
			build_delivery_method,
			prebakes_enabled,
			create_ts,
			guard_public_hostname_dns_parent,
			guard_public_hostname_static
		FROM db_cluster.datacenters
		WHERE datacenter_id = ANY($1)
		",
		datacenter_ids,
	)
	.await?;

	dc_rows
		.into_iter()
		.map(|row| row.into_datacenter(ctx.config()))
		.collect::<GlobalResult<Vec<_>>>()
}
