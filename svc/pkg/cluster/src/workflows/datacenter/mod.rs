use std::convert::TryInto;

use chirp_workflow::prelude::*;
use futures_util::FutureExt;
use rivet_operation::prelude::{proto::backend, Message};
use serde_json::json;

pub mod scale;
pub mod tls_issue;

use crate::types::{BuildDeliveryMethod, Pool, PoolType, PoolUpdate, Provider, TlsState};

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
}

#[workflow]
pub(crate) async fn cluster_datacenter(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	ctx.activity(InsertDbInput {
		cluster_id: input.cluster_id,
		datacenter_id: input.datacenter_id,
		name_id: input.name_id.clone(),
		display_name: input.display_name.clone(),

		provider: input.provider.clone(),
		provider_datacenter_id: input.provider_datacenter_id.clone(),
		provider_api_token: input.provider_api_token.clone(),

		pools: input.pools.clone(),

		build_delivery_method: input.build_delivery_method.clone(),
		prebakes_enabled: input.prebakes_enabled,
	})
	.await?;

	// Wait for TLS issuing process
	ctx.workflow(tls_issue::Input {
		datacenter_id: input.datacenter_id,
		renew: false,
	})
	.await?;

	ctx.msg(
		json!({
			"datacenter_id": input.datacenter_id,
		}),
		CreateComplete {},
	)
	.await?;

	// Scale initially
	ctx.workflow(scale::Input {
		datacenter_id: input.datacenter_id,
	})
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
					})
					.await?;

					// Scale
					ctx.signal(ctx.workflow_id(), Scale {}).await?;
				}
				Main::Scale(_) => {
					ctx.workflow(scale::Input { datacenter_id }).await?;
				}
				Main::ServerCreate(sig) => {
					ctx.dispatch_tagged_workflow(
						&json!({
							"server_id": sig.server_id,
						}),
						crate::workflows::server::Input {
							datacenter_id,
							server_id: sig.server_id,
							pool_type: sig.pool_type,
							tags: sig.tags,
						},
					)
					.await?;
				}
				Main::TlsRenew(_) => {
					ctx.dispatch_workflow(tls_issue::Input {
						datacenter_id,
						renew: true,
					})
					.await?;
				}
			}
			Ok(Loop::Continue)
		}
		.boxed()
	})
	.await
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct InsertDbInput {
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

#[activity(InsertDb)]
async fn insert_db(ctx: &ActivityCtx, input: &InsertDbInput) -> GlobalResult<()> {
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

		async move {
			sql_execute!(
				[ctx, @tx tx]
				"
				INSERT INTO db_cluster.datacenters (
					datacenter_id,
					cluster_id,
					name_id,
					display_name,
					provider2,
					provider_datacenter_id,
					provider_api_token,
					pools2,
					build_delivery_method2,
					prebakes_enabled,
					create_ts,

					-- Backwards compatibility
					provider,
					pools,
					build_delivery_method
				)
				VALUES (
					$1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
					0, b'', 0
				)
				",
				input.datacenter_id,
				input.cluster_id,
				&input.name_id,
				&input.display_name,
				serde_json::to_string(&input.provider)?,
				&input.provider_datacenter_id,
				&input.provider_api_token,
				pools_buf,
				serde_json::to_string(&input.build_delivery_method)?,
				input.prebakes_enabled,
				util::timestamp::now(),
			)
			.await?;

			// Insert TLS record
			sql_execute!(
				[ctx, @tx tx]
				"
				INSERT INTO db_cluster.datacenter_tls (
					datacenter_id,
					state2,
					expire_ts,

					-- Backwards compatibility
					state
				)
				VALUES ($1, $2, 0, 0)
				",
				input.datacenter_id,
				serde_json::to_string(&TlsState::Creating)?,
			)
			.await?;

			Ok(())
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[signal("cluster-datacenter-update")]
pub struct Update {
	pub pools: Vec<PoolUpdate>,
	pub prebakes_enabled: Option<bool>,
}

#[signal("cluster-datacenter-scale")]
pub struct Scale {}

#[signal("cluster-datacenter-tls-renew")]
pub struct TlsRenew {}

#[signal("cluster-datacenter-server-create")]
pub struct ServerCreate {
	pub server_id: Uuid,
	pub pool_type: PoolType,
	pub tags: Vec<String>,
}

join_signal!(Main, [Update, Scale, ServerCreate, TlsRenew]);

#[message("cluster-datacenter-create-complete")]
pub struct CreateComplete {}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {
	datacenter_id: Uuid,
	pools: Vec<PoolUpdate>,
	prebakes_enabled: Option<bool>,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<()> {
	// Get current pools
	let (pools, pools2) = sql_fetch_one!(
		[ctx, (Vec<u8>, Option<sqlx::types::Json<Vec<Pool>>>,)]
		"
		SELECT pools, pools2 FROM db_cluster.datacenters
		WHERE datacenter_id = $1
		",
		input.datacenter_id,
	)
	.await?;
	// Handle backwards compatibility
	let mut pools = if let Some(pools) = pools2 {
		pools.0
	} else {
		let proto = backend::cluster::Pools::decode(pools.as_slice())?.pools;

		proto
			.into_iter()
			.map(TryInto::try_into)
			.collect::<GlobalResult<Vec<_>>>()?
	};

	for pool in &input.pools {
		let current_pool = unwrap!(
			pools.iter_mut().find(|p| p.pool_type == pool.pool_type),
			"attempting to update pool that doesn't exist in current config"
		);

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
	}

	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.datacenters
		SET
			pools2 = $2,
			prebakes_enabled = coalesce($3, prebakes_enabled)
		WHERE datacenter_id = $1
		",
		input.datacenter_id,
		serde_json::to_string(&pools)?,
		input.prebakes_enabled,
	)
	.await?;

	// Purge cache
	ctx.cache()
		.purge("cluster.datacenters2", [input.datacenter_id])
		.await?;

	Ok(())
}
