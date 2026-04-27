use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use futures_util::{StreamExt, stream};
use gas::prelude::{Id, StandaloneCtx, util::timestamp};
use rivet_envoy_protocol as protocol;
use sqlite_storage::{engine::SqliteEngine, open::OpenConfig};

use crate::{conn::Conn, sqlite_runtime};

const SHUTDOWN_CLOSE_PARALLELISM: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveActor {
	pub actor_generation: u32,
	pub sqlite_generation: Option<u64>,
	pub state: ActiveActorState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveActorState {
	Starting,
	Running,
	Stopping,
}

pub async fn start_actor(
	ctx: &StandaloneCtx,
	conn: &Conn,
	checkpoint: &protocol::ActorCheckpoint,
	start: &mut protocol::CommandStartActor,
) -> Result<()> {
	let actor_id = Id::parse(&checkpoint.actor_id).context("invalid start actor id")?;
	let actor_id_string = actor_id.to_string();

	match conn
		.active_actors
		.entry_async(actor_id_string.clone())
		.await
	{
		scc::hash_map::Entry::Occupied(_) => {
			ensure!(false, "actor already active on envoy connection");
		}
		scc::hash_map::Entry::Vacant(entry) => {
			entry.insert_entry(ActiveActor {
				actor_generation: checkpoint.generation,
				sqlite_generation: None,
				state: ActiveActorState::Starting,
			});
		}
	}

	let result = async {
		let sqlite_open = conn
			.sqlite_engine
			.open(&actor_id_string, OpenConfig::new(timestamp::now()))
			.await?;
		let sqlite_generation = sqlite_open.generation;

		let populate_res = async {
			ensure!(start.sqlite_startup_data.is_none());
			ensure!(start.preloaded_kv.is_none());

			let hibernating_requests = ctx
				.op(pegboard::ops::actor::hibernating_request::list::Input { actor_id })
				.await?;
			start.hibernating_requests = hibernating_requests
				.into_iter()
				.map(|x| protocol::HibernatingRequest {
					gateway_id: x.gateway_id,
					request_id: x.request_id,
				})
				.collect();

			let db = ctx.udb()?;
			start.preloaded_kv = pegboard::actor_kv::preload::fetch_preloaded_kv(
				&db,
				ctx.config().pegboard(),
				actor_id,
				conn.namespace_id,
				&start.config.name,
			)
			.await?;

			start.sqlite_startup_data =
				Some(sqlite_runtime::protocol_sqlite_startup_data(sqlite_open));

			Ok(())
		}
		.await;

		// Close SQLite if start command population fails.
		if let Err(err) = populate_res {
			if let Err(close_err) = conn
				.sqlite_engine
				.close(&actor_id_string, sqlite_generation)
				.await
			{
				tracing::warn!(
					actor_id = %actor_id_string,
					?close_err,
					"failed to close sqlite db after start population failed"
				);
			}
			return Err(err);
		}

		Ok(sqlite_generation)
	}
	.await;

	match result {
		Ok(sqlite_generation) => {
			let update_result = conn
				.active_actors
				.update_async(&actor_id_string, |_, active| {
					active.actor_generation = checkpoint.generation;
					active.sqlite_generation = Some(sqlite_generation);
					active.state = ActiveActorState::Running;
				})
				.await;
			if update_result.is_none() {
				if let Err(close_err) = conn
					.sqlite_engine
					.close(&actor_id_string, sqlite_generation)
					.await
				{
					tracing::warn!(
						actor_id = %actor_id_string,
						?close_err,
						"failed to close sqlite db after active state disappeared"
					);
				}
				ensure!(false, "actor active state missing after start");
			}
			Ok(())
		}
		Err(err) => {
			conn.active_actors.remove_async(&actor_id_string).await;
			Err(err)
		}
	}
}

pub async fn stop_actor(conn: &Conn, checkpoint: &protocol::ActorCheckpoint) -> Result<()> {
	let actor_id = checkpoint.actor_id.clone();
	let update_result = conn
		.active_actors
		.update_async(&actor_id, |_, active| {
			if active.actor_generation == checkpoint.generation {
				active.state = ActiveActorState::Stopping;
				Ok(())
			} else {
				Err(active.actor_generation)
			}
		})
		.await
		.context("actor is not active on envoy connection")?;

	if let Err(active_generation) = update_result {
		ensure!(
			false,
			"stop actor generation {} did not match active generation {}",
			checkpoint.generation,
			active_generation
		);
	}
	Ok(())
}

pub async fn actor_stopped(conn: &Conn, checkpoint: &protocol::ActorCheckpoint) -> Result<()> {
	let actor_id = checkpoint.actor_id.clone();
	let active = conn
		.active_actors
		.get_async(&actor_id)
		.await
		.map(|entry| entry.get().clone())
		.context("actor stopped without active sqlite state")?;
	ensure!(
		active.actor_generation == checkpoint.generation,
		"stopped actor generation {} did not match active generation {}",
		checkpoint.generation,
		active.actor_generation
	);

	let sqlite_generation = active
		.sqlite_generation
		.context("actor stopped before sqlite finished opening")?;
	let close_res = conn
		.sqlite_engine
		.close(&actor_id, sqlite_generation)
		.await;
	if let Err(err) = &close_res {
		tracing::warn!(
			%actor_id,
			?err,
			"close failed in actor_stopped, force-evicting open_dbs entry"
		);
		// Process-wide engine: leaving a stale entry would block re-opening
		// the same actor on this process.
		conn.sqlite_engine.force_close(&actor_id).await;
	}
	// Generation-checked remove so a concurrent `start_actor` for a fresh
	// generation between the `get_async` above and this point does not have
	// its newly-inserted entry deleted by the stale stop.
	conn.active_actors
		.remove_if_async(&actor_id, |entry| {
			entry.actor_generation == checkpoint.generation
		})
		.await;

	close_res
}

pub async fn shutdown_conn_actors(conn: &Conn) {
	let mut active_actors = Vec::new();
	conn.active_actors.retain_sync(|actor_id, active| {
		active_actors.push((actor_id.clone(), active.clone()));
		false
	});

	stream::iter(active_actors.into_iter().map(|(actor_id, active)| {
		let sqlite_engine = conn.sqlite_engine.clone();
		close_actor_on_shutdown(sqlite_engine, actor_id, active.sqlite_generation)
	}))
	.buffer_unordered(SHUTDOWN_CLOSE_PARALLELISM)
	.for_each(|_| async {})
	.await;
}

async fn close_actor_on_shutdown(
	sqlite_engine: Arc<SqliteEngine>,
	actor_id: String,
	sqlite_generation: Option<u64>,
) {
	if let Some(generation) = sqlite_generation {
		if let Err(err) = sqlite_engine.close(&actor_id, generation).await {
			tracing::warn!(
				actor_id = %actor_id,
				?err,
				"close failed during envoy shutdown, force-evicting open_dbs entry"
			);
		} else {
			return;
		}
	}
	// Reach this point either when the actor never finished opening (no generation) or when
	// close errored above. Always evict so the process-wide engine doesn't keep a stale
	// entry that would block re-opening the same actor on this process.
	sqlite_engine.force_close(&actor_id).await;
}

pub async fn assert_sqlite_actor_active(
	conn: &Conn,
	actor_id: &str,
	sqlite_generation: u64,
) -> Result<ActiveActor> {
	// Stopping is accepted in addition to Running: the actor still owns its sqlite
	// generation until actor_stopped runs, and may flush a final commit while draining.
	let active = conn
		.active_actors
		.get_async(actor_id)
		.await
		.map(|entry| entry.get().clone())
		.context("sqlite actor is not active on envoy connection")?;

	let active_sqlite_generation = active
		.sqlite_generation
		.context("sqlite actor is still starting")?;
	ensure!(
		active_sqlite_generation == sqlite_generation,
		"sqlite request generation {} did not match active generation {}",
		sqlite_generation,
		active_sqlite_generation
	);

	Ok(active)
}
