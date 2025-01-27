use chirp_workflow::prelude::*;
use futures_util::FutureExt;
use std::ops::Deref;

pub mod scale;
pub mod tls_issue;

use crate::types::{
	BuildDeliveryMethod, GuardPublicHostname, Pool, PoolUpdate, Provider, TlsState,
};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub cluster_id: Uuid,
	pub datacenter_id: Uuid,
	pub name_id: String,
	pub display_name: String,

	pub provider: Provider,
	pub provider_datacenter_id: String,
	pub provider_api_token: Option<String>,

	pub pools: Vec<Pool>,

	pub build_delivery_method: BuildDeliveryMethod,
	pub prebakes_enabled: bool,
	pub guard_public_hostname: Option<GuardPublicHostname>,
}

#[workflow]
pub(crate) async fn cluster_datacenter(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	{
		let v1 = InsertDbInputV1 {
			cluster_id: input.cluster_id,
			datacenter_id: input.datacenter_id,
			name_id: input.name_id.clone(),
			display_name: input.display_name.clone(),

			provider: input.provider,
			provider_datacenter_id: input.provider_datacenter_id.clone(),
			provider_api_token: input.provider_api_token.clone(),

			pools: input.pools.clone(),

			build_delivery_method: input.build_delivery_method,
			prebakes_enabled: input.prebakes_enabled,
		};

		match ctx.check_version(2).await? {
			1 => ctx.activity(v1).await?,
			_latest => {
				ctx.activity(InsertDbInputV2 {
					v1,
					guard_public_hostname: input.guard_public_hostname.clone(),
				})
				.await?
			}
		}
	}

	// Issue TLS
	if ctx.config().server()?.is_tls_enabled() {
		ctx.workflow(tls_issue::Input {
			datacenter_id: input.datacenter_id,
			renew: false,
		})
		.output()
		.await?;
	}

	ctx.msg(CreateComplete {})
		.tag("datacenter_id", input.datacenter_id)
		.send()
		.await?;

	// Scale initially
	ctx.workflow(scale::Input {
		datacenter_id: input.datacenter_id,
	})
	.output()
	.await?;

	ctx.repeat(|ctx| {
		let datacenter_id = input.datacenter_id;

		async move {
			match ctx.listen::<Main>().await? {
				Main::Update(sig) => {
					ctx.activity(UpdateDbInput {
						datacenter_id,
						pools: sig.pools,
						prebakes_enabled: sig.prebakes_enabled,
						guard_public_hostname: sig.guard_public_hostname,
					})
					.await?;

					// Scale
					ctx.workflow(scale::Input { datacenter_id })
						.output()
						.await?;
				}
				Main::Scale(_) => {
					ctx.workflow(scale::Input { datacenter_id })
						.output()
						.await?;
				}
				Main::TlsRenew(_) => {
					if ctx.config().server()?.is_tls_enabled() {
						ctx.workflow(tls_issue::Input {
							datacenter_id,
							renew: true,
						})
						.dispatch()
						.await?;
					}
				}
			}
			Ok(Loop::Continue)
		}
		.boxed()
	})
	.await
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct InsertDbInputV1 {
	cluster_id: Uuid,
	datacenter_id: Uuid,
	name_id: String,
	display_name: String,

	provider: Provider,
	provider_datacenter_id: String,
	provider_api_token: Option<String>,

	pools: Vec<Pool>,

	build_delivery_method: BuildDeliveryMethod,
	prebakes_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct InsertDbInputV2 {
	v1: InsertDbInputV1,
	guard_public_hostname: Option<GuardPublicHostname>,
}

impl Deref for InsertDbInputV2 {
	type Target = InsertDbInputV1;

	fn deref(&self) -> &Self::Target {
		&self.v1
	}
}

#[activity(InsertDb)]
async fn insert_db(ctx: &ActivityCtx, input: &InsertDbInputV1) -> GlobalResult<()> {
	insert_db_inner(
		ctx,
		&InsertDbInputV2 {
			v1: input.clone(),
			guard_public_hostname: None,
		},
	)
	.await
}

#[activity(InsertDbV2)]
async fn insert_db_v2(ctx: &ActivityCtx, input: &InsertDbInputV2) -> GlobalResult<()> {
	insert_db_inner(ctx, input).await
}

async fn insert_db_inner(ctx: &ActivityCtx, input: &InsertDbInputV2) -> GlobalResult<()> {
	let mut pools = input.pools.clone();

	// Constrain the desired count
	for pool in &mut pools {
		pool.desired_count = pool.desired_count.max(pool.min_count).min(pool.max_count);
	}

	let pools_buf = serde_json::to_string(&pools)?;

	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let input = input.clone();
		let pools_buf = pools_buf.clone();
		let (gph_dns_parent, gph_static) = input
			.guard_public_hostname
			.clone()
			.map_or((None, None), GuardPublicHostname::into_columns);

		async move {
			sql_execute!(
				[ctx, @tx tx]
				"
				INSERT INTO db_cluster.datacenters (
					datacenter_id,
					cluster_id,
					name_id,
					display_name,
					provider,
					provider_datacenter_id,
					provider_api_token,
					pools,
					pools2,
					build_delivery_method,
					prebakes_enabled,
					create_ts,
					guard_public_hostname_dns_parent,
					guard_public_hostname_static
				)
				VALUES (
					$1, $2, $3, $4, $5, $6, $7, '', $8, $9, $10, $11, $12, $13
				)
				",
				input.datacenter_id,
				input.cluster_id,
				&input.name_id,
				&input.display_name,
				input.provider as i64,
				&input.provider_datacenter_id,
				&input.provider_api_token,
				pools_buf,
				input.build_delivery_method as i64,
				input.prebakes_enabled,
				util::timestamp::now(),
				gph_dns_parent,
				gph_static
			)
			.await?;

			// Insert TLS record
			sql_execute!(
				[ctx, @tx tx]
				"
				INSERT INTO db_cluster.datacenter_tls (
					datacenter_id,
					state,
					expire_ts
				)
				VALUES ($1, $2, 0)
				",
				input.datacenter_id,
				TlsState::Creating as i64,
			)
			.await?;

			Ok(())
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[signal("cluster_datacenter_update")]
pub struct Update {
	pub pools: Vec<PoolUpdate>,
	pub prebakes_enabled: Option<bool>,
	pub guard_public_hostname: Option<GuardPublicHostname>,
}

#[signal("cluster_datacenter_scale")]
pub struct Scale {}

#[signal("cluster_datacenter_tls_renew")]
pub struct TlsRenew {}

join_signal!(Main {
	Update,
	Scale,
	TlsRenew,
});

#[message("cluster_datacenter_create_complete")]
pub struct CreateComplete {}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {
	datacenter_id: Uuid,
	pools: Vec<PoolUpdate>,
	prebakes_enabled: Option<bool>,
	guard_public_hostname: Option<GuardPublicHostname>,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<()> {
	// Get current pools
	let (pools,) = sql_fetch_one!(
		[ctx, (sqlx::types::Json<Vec<Pool>>,)]
		"
		SELECT pools2 FROM db_cluster.datacenters
		WHERE datacenter_id = $1
		",
		input.datacenter_id,
	)
	.await?;
	let mut pools = pools.0;

	for pool in &input.pools {
		if let Some(current_pool) = pools.iter_mut().find(|p| p.pool_type == pool.pool_type) {
			// Update pool config
			if !pool.hardware.is_empty() {
				current_pool.hardware.clone_from(&pool.hardware);
			}
			if let Some(desired_count) = pool.desired_count {
				current_pool.desired_count = desired_count;
			}
			if let Some(min_count) = pool.min_count {
				current_pool.min_count = min_count;
			}
			if let Some(max_count) = pool.max_count {
				current_pool.max_count = max_count;
			}
			if let Some(drain_timeout) = pool.drain_timeout {
				current_pool.drain_timeout = drain_timeout;
			}
		} else {
			tracing::info!(pool_type=?pool.pool_type, "creating new pool");

			ensure!(
				!pool.hardware.is_empty(),
				"must have `hardware` when creating a new pool"
			);

			let min_count = unwrap!(
				pool.min_count,
				"must have `min_count` when creating a new pool"
			);

			pools.push(Pool {
				pool_type: pool.pool_type,
				hardware: pool.hardware.clone(),
				desired_count: pool.desired_count.unwrap_or(min_count),
				min_count,
				max_count: unwrap!(
					pool.max_count,
					"must have `max_count` when creating a new pool"
				),
				drain_timeout: unwrap!(
					pool.drain_timeout,
					"must have `drain_timeout` when creating a new pool"
				),
			});
		};
	}

	let (gph_dns_parent, gph_static) = input
		.guard_public_hostname
		.clone()
		.map_or((None, None), GuardPublicHostname::into_columns);

	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.datacenters
		SET
			pools2 = $2,
			prebakes_enabled = coalesce($3, prebakes_enabled),
			guard_public_hostname_dns_parent = $4,
			guard_public_hostname_static = $5
		WHERE datacenter_id = $1
		",
		input.datacenter_id,
		serde_json::to_string(&pools)?,
		input.prebakes_enabled,
		gph_dns_parent,
		gph_static
	)
	.await?;

	// Purge cache
	ctx.cache()
		.purge("cluster.datacenters2", [input.datacenter_id])
		.await?;

	Ok(())
}
