use std::convert::{TryFrom, TryInto};
// TERMINOLOGY:
//
// server: a non-destroyed non-tainted server
// active server: a server that is being provisioned, is already provisioned, or is installed
// installed server: a server that has successfully installed the required software
// nomad server: a job server that has its nomad client connected to the leader
// draining server: a server that is currently draining, not drained
// drained server: a server that is finished draining
// tainted server: a tainted server
use std::{
	cmp::Ordering,
	collections::HashMap,
	future::Future,
	iter::{DoubleEndedIterator, Iterator},
	pin::Pin,
};

use chirp_worker::prelude::*;
use futures_util::{FutureExt, StreamExt, TryStreamExt};
use proto::backend::{self, pkg::*};

type MsgFuture = Pin<Box<dyn Future<Output = Result<(), chirp_client::error::ClientError>> + Send>>;

#[derive(sqlx::FromRow)]
struct ServerRow {
	server_id: Uuid,
	pool_type: i64,
	is_installed: bool,
	has_nomad_node: bool,
	is_draining: bool,
	is_drained: bool,
	is_tainted: bool,
}

struct Server {
	server_id: Uuid,
	pool_type: backend::cluster::PoolType,
	is_installed: bool,
	has_nomad_node: bool,
	drain_state: DrainState,
	is_tainted: bool,
}

impl TryFrom<ServerRow> for Server {
	type Error = GlobalError;

	fn try_from(value: ServerRow) -> GlobalResult<Self> {
		Ok(Server {
			server_id: value.server_id,
			pool_type: unwrap!(backend::cluster::PoolType::from_i32(value.pool_type as i32)),
			is_installed: value.is_installed,
			has_nomad_node: value.has_nomad_node,
			is_tainted: value.is_tainted,
			drain_state: if value.is_drained {
				DrainState::Complete
			} else if value.is_draining {
				DrainState::Draining
			} else {
				DrainState::None
			},
		})
	}
}

enum DrainState {
	None,
	Draining,
	Complete,
}

struct PoolCtx {
	datacenter_id: Uuid,
	provider: i32,
	pool_type: backend::cluster::PoolType,
	desired_count: usize,
}

#[worker(name = "cluster-datacenter-scale")]
async fn worker(
	ctx: &OperationContext<cluster::msg::datacenter_scale::Message>,
) -> GlobalResult<()> {
	let datacenter_id = unwrap_ref!(ctx.datacenter_id).as_uuid();

	let (datacenter_res, topology_res) = tokio::try_join!(
		op!([ctx] cluster_datacenter_get {
			datacenter_ids: vec![datacenter_id.into()],
		}),
		op!([ctx] cluster_datacenter_topology_get {
			datacenter_ids: vec![datacenter_id.into()],
		}),
	)?;

	let dc = unwrap!(datacenter_res.datacenters.first());

	let topology = unwrap!(topology_res.datacenters.first());
	let memory_by_server = topology
		.servers
		.iter()
		.map(|server| {
			Ok((
				unwrap_ref!(server.server_id).as_uuid(),
				unwrap_ref!(server.usage).memory,
			))
		})
		.collect::<GlobalResult<HashMap<_, _>>>()?;

	// Run everything in a locking transaction
	let msgs = rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.base();
		let dc = dc.clone();
		let memory_by_server = memory_by_server.clone();

		inner(ctx, tx, dc, memory_by_server).boxed()
	})
	.await?;

	// Publish all messages
	if !msgs.is_empty() {
		tracing::info!("transaction successful, publishing messages");

		futures_util::stream::iter(msgs)
			.buffer_unordered(16)
			.try_collect::<Vec<_>>()
			.await?;
	}

	Ok(())
}

async fn inner(
	ctx: OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	dc: backend::cluster::Datacenter,
	memory_by_server: HashMap<Uuid, u64>,
) -> GlobalResult<Vec<MsgFuture>> {
	let datacenter_id = unwrap_ref!(dc.datacenter_id).as_uuid();

	let servers = sql_fetch_all!(
		[ctx, ServerRow, @tx tx]
		"
		SELECT
			server_id, pool_type,
			(install_complete_ts IS NOT NULL) AS is_installed,
			(nomad_node_id IS NOT NULL) AS has_nomad_node,
			(drain_ts IS NOT NULL) AS is_draining,
			(drain_complete_ts IS NOT NULL) AS is_drained,
			(taint_ts IS NOT NULL) AS is_tainted
		FROM db_cluster.servers
		WHERE
			datacenter_id = $1 AND
			-- Filters out servers that are being destroyed/already destroyed
			cloud_destroy_ts IS NULL
		-- Newer servers will be destroyed first
		ORDER BY create_ts DESC
		FOR UPDATE
		",
		datacenter_id,
	)
	.await?;

	let mut servers = servers
		.into_iter()
		.map(TryInto::try_into)
		.collect::<GlobalResult<Vec<Server>>>()?;

	// Sort job servers by memory usage
	servers.sort_by_key(|server| memory_by_server.get(&server.server_id));

	// TODO: RVT-3732 Sort gg and ats servers by cpu usage
	// servers.sort_by_key

	let mut msgs = Vec::new();

	// NOTE: Can't parallelize because this is in a transaction
	for pool in &dc.pools {
		let pool_ctx = PoolCtx {
			datacenter_id,
			provider: dc.provider,
			pool_type: unwrap!(backend::cluster::PoolType::from_i32(pool.pool_type)),
			desired_count: pool.desired_count.min(pool.max_count) as usize,
		};

		scale_servers(&ctx, tx, &mut msgs, &servers, &pool_ctx).await?;

		match pool_ctx.pool_type {
			backend::cluster::PoolType::Job => {
				cleanup_tainted_job_servers(&ctx, tx, &mut msgs, &servers, &pool_ctx).await?
			}
			_ => cleanup_tainted_servers(&ctx, tx, &mut msgs, &servers, &pool_ctx).await?,
		}
	}

	destroy_drained_servers(&ctx, tx, &mut msgs, &servers).await?;

	Ok(msgs)
}

async fn scale_servers(
	ctx: &OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	msgs: &mut Vec<MsgFuture>,
	servers: &[Server],
	pctx: &PoolCtx,
) -> GlobalResult<()> {
	let servers_in_pool = servers
		.iter()
		.filter(|server| server.pool_type == pctx.pool_type)
		.filter(|server| !server.is_tainted);

	let active_servers_in_pool = servers_in_pool
		.clone()
		.filter(|server| matches!(server.drain_state, DrainState::None));
	let active_count = active_servers_in_pool.clone().count();

	match pctx.desired_count.cmp(&active_count) {
		Ordering::Less => match pctx.pool_type {
			backend::cluster::PoolType::Job => {
				scale_down_job_servers(ctx, tx, msgs, pctx, active_servers_in_pool, active_count)
					.await?
			}
			backend::cluster::PoolType::Gg => {
				scale_down_gg_servers(ctx, tx, msgs, pctx, active_servers_in_pool, active_count)
					.await?
			}
			backend::cluster::PoolType::Ats => {
				scale_down_ats_servers(ctx, tx, msgs, pctx, active_servers_in_pool, active_count)
					.await?
			}
		},
		Ordering::Greater => {
			scale_up_servers(ctx, tx, msgs, pctx, servers_in_pool, active_count).await?;
		}
		Ordering::Equal => {}
	}

	Ok(())
}

async fn scale_down_job_servers<'a, I: Iterator<Item = &'a Server>>(
	ctx: &OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	msgs: &mut Vec<MsgFuture>,
	pctx: &PoolCtx,
	active_servers: I,
	active_count: usize,
) -> GlobalResult<()> {
	tracing::info!(
		datacenter_id=?pctx.datacenter_id,
		desired=%pctx.desired_count,
		active=%active_count,
		"scaling down job"
	);

	let (nomad_servers, without_nomad_servers) =
		active_servers.partition::<Vec<_>, _>(|server| server.has_nomad_node);

	let destroy_count = (active_count - pctx.desired_count).min(without_nomad_servers.len());
	let drain_count = active_count - pctx.desired_count - destroy_count;

	// Destroy servers
	if destroy_count != 0 {
		tracing::info!(count=%destroy_count, "destroying servers");

		let destroy_candidates = without_nomad_servers
			.iter()
			.take(destroy_count)
			.map(|server| server.server_id);

		destroy_servers(ctx, tx, msgs, destroy_candidates).await?;
	}

	// Drain servers
	if drain_count != 0 {
		tracing::info!(count=%drain_count, "draining job servers");

		let drain_candidates = nomad_servers
			.iter()
			.rev()
			.take(drain_count)
			.map(|server| server.server_id);

		drain_servers(ctx, tx, msgs, drain_candidates).await?;
	}

	Ok(())
}

async fn scale_down_gg_servers<'a, I: Iterator<Item = &'a Server> + DoubleEndedIterator + Clone>(
	ctx: &OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	msgs: &mut Vec<MsgFuture>,
	pctx: &PoolCtx,
	active_servers: I,
	active_count: usize,
) -> GlobalResult<()> {
	tracing::info!(
		datacenter_id=?pctx.datacenter_id,
		desired=%pctx.desired_count,
		active=%active_count,
		"scaling down gg"
	);

	let drain_count = active_count - pctx.desired_count;

	// Drain servers
	if drain_count != 0 {
		tracing::info!(count=%drain_count, "draining gg servers");

		let drain_candidates = active_servers
			.rev()
			.take(drain_count)
			.map(|server| server.server_id);

		drain_servers(ctx, tx, msgs, drain_candidates).await?;
	}

	Ok(())
}

async fn scale_down_ats_servers<
	'a,
	I: Iterator<Item = &'a Server> + DoubleEndedIterator + Clone,
>(
	ctx: &OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	msgs: &mut Vec<MsgFuture>,
	pctx: &PoolCtx,
	active_servers: I,
	active_count: usize,
) -> GlobalResult<()> {
	tracing::info!(
		datacenter_id=?pctx.datacenter_id,
		desired=%pctx.desired_count,
		active=%active_count,
		"scaling down ats"
	);

	let drain_count = active_count - pctx.desired_count;

	// Drain servers
	if drain_count != 0 {
		tracing::info!(count=%drain_count, "draining ats servers");

		let drain_candidates = active_servers
			.rev()
			.take(drain_count)
			.map(|server| server.server_id);

		drain_servers(ctx, tx, msgs, drain_candidates).await?;
	}

	Ok(())
}

async fn scale_up_servers<'a, I: Iterator<Item = &'a Server> + Clone>(
	ctx: &OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	msgs: &mut Vec<MsgFuture>,
	pctx: &PoolCtx,
	servers: I,
	active_count: usize,
) -> GlobalResult<()> {
	let draining_servers = servers
		.clone()
		.filter(|server| matches!(server.drain_state, DrainState::Draining))
		.collect::<Vec<_>>();

	tracing::info!(
		datacenter_id=?pctx.datacenter_id,
		desired=%pctx.desired_count,
		active=%active_count,
		draining=%draining_servers.len(),
		pool_type=?pctx.pool_type,
		"scaling up"
	);

	let undrain_count = (pctx.desired_count - active_count).min(draining_servers.len());
	let provision_count = pctx.desired_count - active_count - undrain_count;

	// Undrain servers
	if undrain_count != 0 {
		tracing::info!(count=%undrain_count, "undraining servers");

		// Because job servers are ordered by memory usage, this will undrain the servers with the most memory
		// usage
		let undrain_candidates = draining_servers
			.iter()
			.take(undrain_count)
			.map(|server| server.server_id);

		undrain_servers(ctx, tx, msgs, undrain_candidates).await?;
	}

	// Create new servers
	if provision_count != 0 {
		tracing::info!(count=%provision_count, "provisioning servers");

		for _ in 0..provision_count {
			provision_server(ctx, tx, msgs, pctx).await?;
		}
	}

	Ok(())
}

async fn cleanup_tainted_job_servers(
	ctx: &OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	msgs: &mut Vec<MsgFuture>,
	servers: &[Server],
	pctx: &PoolCtx,
) -> GlobalResult<()> {
	// Includes tainted and normal servers
	let active_servers_in_pool = servers
		.iter()
		.filter(|server| server.pool_type == pctx.pool_type)
		.filter(|server| matches!(server.drain_state, DrainState::None));

	let active_tainted_servers_in_pool = active_servers_in_pool
		.clone()
		.filter(|server| server.is_tainted);
	let active_tainted_count = active_tainted_servers_in_pool.clone().count();

	// For job servers the "active" servers we count are ones with nomad successfully connected
	let active_untainted_count = active_servers_in_pool
		.clone()
		.filter(|server| server.has_nomad_node)
		.filter(|server| !server.is_tainted)
		.count();

	// Amount of tainted servers that need to be deleted or drained
	// tainted - (desired - running) -> tainted + running - desired
	let removal_count =
		(active_tainted_count + active_untainted_count).saturating_sub(pctx.desired_count);

	let (nomad_servers, without_nomad_servers) =
		active_tainted_servers_in_pool.partition::<Vec<_>, _>(|server| server.has_nomad_node);

	let destroy_count = removal_count.min(without_nomad_servers.len());
	let drain_count = removal_count - destroy_count;

	if destroy_count != 0 {
		tracing::info!(
			pool_type=?pctx.pool_type,
			desired_count=%pctx.desired_count,
			%active_untainted_count,
			%active_tainted_count,
			%destroy_count,
			"destroying tainted servers",
		);

		destroy_servers(
			ctx,
			tx,
			msgs,
			without_nomad_servers
				.iter()
				.take(destroy_count)
				.map(|server| server.server_id),
		)
		.await?;
	}

	if drain_count != 0 {
		tracing::info!(
			pool_type=?pctx.pool_type,
			desired_count=%pctx.desired_count,
			%active_untainted_count,
			%active_tainted_count,
			%drain_count,
			"draining tainted servers",
		);

		drain_servers(
			ctx,
			tx,
			msgs,
			nomad_servers
				.iter()
				.take(drain_count)
				.map(|server| server.server_id),
		)
		.await?;
	}

	Ok(())
}

async fn cleanup_tainted_servers(
	ctx: &OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	msgs: &mut Vec<MsgFuture>,
	servers: &[Server],
	pctx: &PoolCtx,
) -> GlobalResult<()> {
	// Includes tainted and normal servers
	let active_servers_in_pool = servers
		.iter()
		.filter(|server| server.pool_type == pctx.pool_type)
		.filter(|server| matches!(server.drain_state, DrainState::None));

	let active_tainted_servers_in_pool = active_servers_in_pool
		.clone()
		.filter(|server| server.is_tainted);
	let active_tainted_count = active_tainted_servers_in_pool.clone().count();

	// Count servers that have successfully installed
	let active_untainted_count = active_servers_in_pool
		.clone()
		.filter(|server| server.is_installed)
		.filter(|server| !server.is_tainted)
		.count();

	// Amount of tainted servers that need to be drained
	// tainted - (desired - running) -> tainted + running - desired
	let drain_count =
		(active_tainted_count + active_untainted_count).saturating_sub(pctx.desired_count);

	if drain_count != 0 {
		tracing::info!(
			pool_type=?pctx.pool_type,
			desired_count=%pctx.desired_count,
			%active_untainted_count,
			%active_tainted_count,
			%drain_count,
			"draining tainted servers",
		);

		drain_servers(
			ctx,
			tx,
			msgs,
			active_tainted_servers_in_pool
				.take(drain_count)
				.map(|server| server.server_id),
		)
		.await?;
	}

	Ok(())
}

// Destroys all drained servers (including tainted drained servers)
async fn destroy_drained_servers(
	ctx: &OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	msgs: &mut Vec<MsgFuture>,
	servers: &[Server],
) -> GlobalResult<()> {
	let drained_server_ids = servers
		.iter()
		.filter(|server| matches!(server.drain_state, DrainState::Complete))
		.map(|server| server.server_id)
		.collect::<Vec<_>>();

	if drained_server_ids.is_empty() {
		return Ok(());
	}

	tracing::info!(count=%drained_server_ids.len(), "destroying drained servers");

	destroy_servers(ctx, tx, msgs, drained_server_ids.into_iter()).await
}

async fn drain_servers<I: Iterator<Item = Uuid> + Clone>(
	ctx: &OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	msgs: &mut Vec<MsgFuture>,
	server_ids: I,
) -> GlobalResult<()> {
	tracing::info!(count=%server_ids.clone().count(), "draining servers");

	// Mark servers as draining in db
	sql_execute!(
		[ctx, @tx tx]
		"
		UPDATE db_cluster.servers
		SET drain_ts = $2
		WHERE server_id = ANY($1)
		",
		server_ids.clone().collect::<Vec<_>>(),
		util::timestamp::now(),
	)
	.await?;

	msgs.extend(server_ids.map(|server_id| {
		let ctx = ctx.base();
		async move {
			tracing::info!(%server_id, "draining server");

			msg!([ctx] cluster::msg::server_drain(server_id) {
				server_id: Some(server_id.into()),
			})
			.await
		}
		.boxed()
	}));

	Ok(())
}

async fn undrain_servers<I: Iterator<Item = Uuid> + Clone>(
	ctx: &OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	msgs: &mut Vec<MsgFuture>,
	server_ids: I,
) -> GlobalResult<()> {
	tracing::info!(count=%server_ids.clone().count(), "undraining servers");

	// Mark servers as not draining in db
	sql_execute!(
		[ctx, @tx tx]
		"
		UPDATE db_cluster.servers
		SET drain_ts = NULL
		WHERE server_id = ANY($1)
		",
		server_ids.clone().collect::<Vec<_>>(),
	)
	.await?;

	msgs.extend(server_ids.map(|server_id| {
		let ctx = ctx.base();
		async move {
			tracing::info!(%server_id, "undraining server");

			msg!([ctx] cluster::msg::server_undrain(server_id) {
				server_id: Some(server_id.into()),
			})
			.await
		}
		.boxed()
	}));

	Ok(())
}

async fn provision_server(
	ctx: &OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	msgs: &mut Vec<MsgFuture>,
	pctx: &PoolCtx,
) -> GlobalResult<()> {
	let server_id = Uuid::new_v4();

	// Write new server to db
	sql_execute!(
		[ctx, @tx tx]
		"
				INSERT INTO db_cluster.servers (
					server_id,
					datacenter_id,
					pool_type,
					create_ts
				)
				VALUES ($1, $2, $3, $4)
				",
		server_id,
		pctx.datacenter_id,
		pctx.pool_type as i64,
		util::timestamp::now(),
	)
	.await?;

	let ctx = ctx.base();
	let datacenter_id = pctx.datacenter_id;
	let provider = pctx.provider;
	let pool_type = pctx.pool_type;

	msgs.push(
		async move {
			tracing::info!(%server_id, "provisioning server");
			msg!([ctx] cluster::msg::server_provision(server_id) {
				datacenter_id: Some(datacenter_id.into()),
				server_id: Some(server_id.into()),
				pool_type: pool_type as i32,
				provider: provider,
				tags: Vec::new(),
			})
			.await
		}
		.boxed(),
	);

	Ok(())
}

async fn destroy_servers<I: Iterator<Item = Uuid> + Clone>(
	ctx: &OperationContext<()>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	msgs: &mut Vec<MsgFuture>,
	server_ids: I,
) -> GlobalResult<()> {
	tracing::info!(count=%server_ids.clone().count(), "destroying servers");

	// Mark servers for destruction in db
	sql_execute!(
		[ctx, @tx tx]
		"
		UPDATE db_cluster.servers
		SET cloud_destroy_ts = $2
		WHERE server_id = ANY($1)
		",
		server_ids.clone().collect::<Vec<_>>(),
		util::timestamp::now(),
	)
	.await?;

	msgs.extend(server_ids.map(|server_id| {
		let ctx = ctx.base();
		async move {
			tracing::info!(%server_id, "destroying server");

			msg!([ctx] cluster::msg::server_destroy(server_id) {
				server_id: Some(server_id.into()),
				force: false,
			})
			.await
		}
		.boxed()
	}));

	Ok(())
}
