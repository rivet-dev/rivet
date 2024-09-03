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
	collections::HashMap,
	convert::{TryFrom, TryInto},
	iter::{DoubleEndedIterator, Iterator},
};

use chirp_workflow::prelude::*;
use futures_util::{FutureExt, StreamExt, TryStreamExt};

use crate::types::{Datacenter, PoolType, Provider};

#[derive(sqlx::FromRow)]
struct ServerRow {
	server_id: Uuid,
	pool_type: i64,
	pool_type2: Option<sqlx::types::Json<PoolType>>,
	is_installed: bool,
	has_nomad_node: bool,
	is_draining: bool,
	is_drained: bool,
	is_tainted: bool,
}

struct Server {
	server_id: Uuid,
	pool_type: PoolType,
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
			// Handle backwards compatibility
			pool_type: if let Some(pool_type) = value.pool_type2 {
				pool_type.0
			} else {
				value.pool_type.try_into()?
			},
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
	provider: Provider,
	pool_type: PoolType,
	desired_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub datacenter_id: Uuid,
}

#[workflow]
pub async fn cluster_scale(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let diff = ctx
		.activity(CalculateDiffInput {
			datacenter_id: input.datacenter_id,
		})
		.await?;

	if !diff.actions.is_empty() {
		tracing::info!(actions=?diff.actions, "dispatching signals");

		futures_util::stream::iter(
			diff.actions
				.into_iter()
				.map(|action| action.dispatch(ctx.step(), input.datacenter_id).boxed()),
		)
		.buffer_unordered(16)
		.try_collect::<Vec<_>>()
		.await?;
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CalculateDiffInput {
	datacenter_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CalculateDiffOutput {
	actions: Vec<Action>,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
enum Action {
	Provision {
		server_id: Uuid,
		pool_type: PoolType,
	},
	Drain {
		server_id: Uuid,
	},
	Undrain {
		server_id: Uuid,
	},
	Destroy {
		server_id: Uuid,
	},
}

impl Action {
	async fn dispatch(self, mut ctx: WorkflowCtx, datacenter_id: Uuid) -> GlobalResult<()> {
		match self {
			Action::Provision {
				server_id,
				pool_type,
			} => {
				ctx.workflow(crate::workflows::server::Input {
					datacenter_id,
					server_id,
					pool_type,
					tags: Vec::new(),
				})
				.tag("server_id", server_id)
				.dispatch()
				.await?;
			}
			Action::Drain { server_id } => {
				ctx.signal(crate::workflows::server::Drain {})
					.tag("server_id", server_id)
					.send()
					.await?;
			}
			Action::Undrain { server_id } => {
				ctx.signal(crate::workflows::server::Undrain {})
					.tag("server_id", server_id)
					.send()
					.await?;
			}
			Action::Destroy { server_id } => {
				ctx.signal(crate::workflows::server::Destroy {})
					.tag("server_id", server_id)
					.send()
					.await?;
			}
		}

		Ok(())
	}
}

#[activity(CalculateDiff)]
async fn calculate_diff(
	ctx: &ActivityCtx,
	input: &CalculateDiffInput,
) -> GlobalResult<CalculateDiffOutput> {
	let (datacenter_res, topology_res) = tokio::try_join!(
		ctx.op(crate::ops::datacenter::get::Input {
			datacenter_ids: vec![input.datacenter_id],
		}),
		ctx.op(crate::ops::datacenter::topology_get::Input {
			datacenter_ids: vec![input.datacenter_id],
		}),
	)?;

	let dc = unwrap!(datacenter_res.datacenters.first());

	let topology = unwrap!(topology_res.datacenters.first());
	let memory_by_server = topology
		.servers
		.iter()
		.map(|server| Ok((server.server_id, server.usage.memory)))
		.collect::<GlobalResult<HashMap<_, _>>>()?;

	// Run everything in a locking transaction
	let actions = rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let dc = dc.clone();
		let memory_by_server = memory_by_server.clone();

		inner(ctx, tx, dc, memory_by_server).boxed()
	})
	.await?;

	Ok(CalculateDiffOutput { actions })
}

async fn inner(
	ctx: ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	dc: Datacenter,
	memory_by_server: HashMap<Uuid, u64>,
) -> GlobalResult<Vec<Action>> {
	let servers = sql_fetch_all!(
		[ctx, ServerRow, @tx tx]
		"
		SELECT
			server_id, pool_type, pool_type2,
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
		dc.datacenter_id,
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

	let mut actions = Vec::new();

	// NOTE: Can't parallelize because this is in a transaction
	for pool in &dc.pools {
		let pool_ctx = PoolCtx {
			datacenter_id: dc.datacenter_id,
			provider: dc.provider.clone(),
			pool_type: pool.pool_type.clone(),
			desired_count: pool.desired_count.max(pool.min_count).min(pool.max_count) as usize,
		};

		scale_servers(&ctx, tx, &mut actions, &servers, &pool_ctx).await?;
	}

	destroy_drained_servers(&ctx, tx, &mut actions, &servers).await?;

	Ok(actions)
}

async fn scale_servers(
	ctx: &ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	actions: &mut Vec<Action>,
	servers: &[Server],
	pctx: &PoolCtx,
) -> GlobalResult<()> {
	let servers_in_pool = servers
		.iter()
		.filter(|server| server.pool_type == pctx.pool_type)
		.filter(|server| !server.is_tainted);

	// Active servers may not be entirely installed. This is important as we cannot filter out servers that
	// aren't installed or provisioned yet here.
	let active_servers = servers_in_pool
		.clone()
		.filter(|server| matches!(server.drain_state, DrainState::None));
	let active_count = active_servers.clone().count();

	tracing::info!(desired=%pctx.desired_count, active=%active_count, "comparing {:?}", pctx.pool_type);

	// Scale up
	if pctx.desired_count > active_count {
		scale_up_servers(ctx, tx, actions, pctx, servers_in_pool, active_count).await?;
	}

	// Scale down
	match pctx.pool_type {
		PoolType::Job => {
			let (nomad_servers, without_nomad_servers) = active_servers
				.clone()
				.partition::<Vec<_>, _>(|server| server.has_nomad_node);

			if pctx.desired_count < nomad_servers.len() {
				scale_down_job_servers(
					ctx,
					tx,
					actions,
					pctx,
					nomad_servers,
					without_nomad_servers,
				)
				.await?;
			}
		}
		PoolType::Gg => {
			let installed_servers = active_servers.filter(|server| server.is_installed);
			let installed_count = installed_servers.clone().count();

			if pctx.desired_count < installed_count {
				scale_down_gg_servers(ctx, tx, actions, pctx, installed_servers, installed_count)
					.await?;
			}
		}
		PoolType::Ats => {
			let installed_servers = active_servers.filter(|server| server.is_installed);
			let installed_count = installed_servers.clone().count();

			if pctx.desired_count < installed_count {
				scale_down_ats_servers(ctx, tx, actions, pctx, installed_servers, installed_count)
					.await?;
			}
		}
	}

	// Cleanup
	match pctx.pool_type {
		PoolType::Job => cleanup_tainted_job_servers(ctx, tx, actions, servers, pctx).await?,
		_ => cleanup_tainted_servers(ctx, tx, actions, servers, pctx).await?,
	}

	Ok(())
}

async fn scale_down_job_servers(
	ctx: &ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	actions: &mut Vec<Action>,
	pctx: &PoolCtx,
	nomad_servers: Vec<&Server>,
	without_nomad_servers: Vec<&Server>,
) -> GlobalResult<()> {
	tracing::info!(
		datacenter_id=?pctx.datacenter_id,
		desired=%pctx.desired_count,
		nomad_servers=%nomad_servers.len(),
		"scaling down job"
	);

	let diff = nomad_servers.len().saturating_sub(pctx.desired_count);

	let destroy_count = match pctx.provider {
		// Never destroy servers when scaling down with Linode, always drain
		Provider::Linode => 0,
		#[allow(unreachable_patterns)]
		_ => diff.min(without_nomad_servers.len()),
	};
	let drain_count = diff - destroy_count;

	// Destroy servers
	if destroy_count != 0 {
		tracing::info!(count=%destroy_count, "destroying servers");

		let destroy_candidates = without_nomad_servers
			.iter()
			.take(destroy_count)
			.map(|server| server.server_id);

		destroy_servers(ctx, tx, actions, destroy_candidates).await?;
	}

	// Drain servers
	if drain_count != 0 {
		tracing::info!(count=%drain_count, "draining job servers");

		let drain_candidates = nomad_servers
			.iter()
			.rev()
			.take(drain_count)
			.map(|server| server.server_id);

		drain_servers(ctx, tx, actions, drain_candidates).await?;
	}

	Ok(())
}

async fn scale_down_gg_servers<'a, I: Iterator<Item = &'a Server> + DoubleEndedIterator + Clone>(
	ctx: &ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	actions: &mut Vec<Action>,
	pctx: &PoolCtx,
	installed_servers: I,
	installed_count: usize,
) -> GlobalResult<()> {
	tracing::info!(
		datacenter_id=?pctx.datacenter_id,
		desired=%pctx.desired_count,
		installed=%installed_count,
		"scaling down gg"
	);

	let drain_count = installed_count.saturating_sub(pctx.desired_count);

	// Drain servers
	if drain_count != 0 {
		tracing::info!(count=%drain_count, "draining gg servers");

		let drain_candidates = installed_servers
			.rev()
			.take(drain_count)
			.map(|server| server.server_id);

		drain_servers(ctx, tx, actions, drain_candidates).await?;
	}

	Ok(())
}

async fn scale_down_ats_servers<
	'a,
	I: Iterator<Item = &'a Server> + DoubleEndedIterator + Clone,
>(
	ctx: &ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	actions: &mut Vec<Action>,
	pctx: &PoolCtx,
	installed_servers: I,
	installed_count: usize,
) -> GlobalResult<()> {
	tracing::info!(
		datacenter_id=?pctx.datacenter_id,
		desired=%pctx.desired_count,
		installed=%installed_count,
		"scaling down ats"
	);

	let drain_count = installed_count.saturating_sub(pctx.desired_count);

	// Drain servers
	if drain_count != 0 {
		tracing::info!(count=%drain_count, "draining ats servers");

		let drain_candidates = installed_servers
			.rev()
			.take(drain_count)
			.map(|server| server.server_id);

		drain_servers(ctx, tx, actions, drain_candidates).await?;
	}

	Ok(())
}

async fn scale_up_servers<'a, I: Iterator<Item = &'a Server> + Clone>(
	ctx: &ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	actions: &mut Vec<Action>,
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

		undrain_servers(ctx, tx, actions, undrain_candidates).await?;
	}

	// Create new servers
	if provision_count != 0 {
		tracing::info!(count=%provision_count, "provisioning servers");

		for _ in 0..provision_count {
			provision_server(ctx, tx, actions, pctx).await?;
		}
	}

	Ok(())
}

async fn cleanup_tainted_job_servers(
	ctx: &ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	actions: &mut Vec<Action>,
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
			actions,
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
			actions,
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
	ctx: &ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	actions: &mut Vec<Action>,
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
			actions,
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
	ctx: &ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	actions: &mut Vec<Action>,
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

	destroy_servers(ctx, tx, actions, drained_server_ids.into_iter()).await
}

async fn drain_servers<I: Iterator<Item = Uuid> + Clone>(
	ctx: &ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	actions: &mut Vec<Action>,
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

	actions.extend(server_ids.map(|server_id| Action::Drain { server_id }));

	Ok(())
}

async fn undrain_servers<I: Iterator<Item = Uuid> + Clone>(
	ctx: &ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	actions: &mut Vec<Action>,
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

	actions.extend(server_ids.map(|server_id| Action::Undrain { server_id }));

	Ok(())
}

async fn provision_server(
	ctx: &ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	actions: &mut Vec<Action>,
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
			pool_type2,
			create_ts,

			-- Backwards compatibility
			pool_type
		)
		VALUES ($1, $2, $3, $4, 0)
		",
		server_id,
		pctx.datacenter_id,
		serde_json::to_string(&pctx.pool_type)?,
		util::timestamp::now(),
	)
	.await?;

	actions.push(Action::Provision {
		server_id,
		pool_type: pctx.pool_type.clone(),
	});

	Ok(())
}

async fn destroy_servers<I: Iterator<Item = Uuid> + Clone>(
	ctx: &ActivityCtx,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	actions: &mut Vec<Action>,
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

	actions.extend(server_ids.map(|server_id| Action::Destroy { server_id }));

	Ok(())
}
