use chirp_workflow::prelude::*;

use nix::sys::signal::Signal;
use pegboard::protocol;

/// How long to wait after last ping before forcibly removing a client from the database and deleting its
/// workflow, evicting all actors. Note that the client may still be running and can reconnect.
const CLIENT_LOST_THRESHOLD_MS: i64 = util::duration::minutes(2);
/// How long to wait after creating and not receiving a starting state before forcibly stopping actor.
const ACTOR_START_THRESHOLD_MS: i64 = util::duration::seconds(30);
/// How long to wait after stopping and not receiving a stop state before manually setting actor as
/// stopped.
const ACTOR_STOP_THRESHOLD_MS: i64 = util::duration::seconds(30);
/// How long to wait after stopped and not receiving an exit state before manually setting actor as
/// exited.
const ACTOR_EXIT_THRESHOLD_MS: i64 = util::duration::seconds(5);

#[derive(sqlx::FromRow)]
struct ActorRow {
	actor_id: Uuid,
	client_id: Uuid,
	failed_start: bool,
	failed_stop: bool,
	failed_exit: bool,
}

pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
	loop {
		interval.tick().await;

		let ts = util::timestamp::now();
		run_from_env(config.clone(), pools.clone(), ts).await?;
	}
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	ts: i64,
) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("pegboard-gc");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		config,
		rivet_connection::Connection::new(client, pools, cache),
		"pegboard-gc",
	)
	.await?;

	let (dead_client_rows, failed_actor_rows) = tokio::try_join!(
		sql_fetch_all!(
			[ctx, (Uuid,)]
			"
			UPDATE db_pegboard.clients
			SET delete_ts = $2
			WHERE
				last_ping_ts < $1 AND
				delete_ts IS NULL
			RETURNING client_id
			",
			ts - CLIENT_LOST_THRESHOLD_MS,
			ts,
		),
		sql_fetch_all!(
			[ctx, ActorRow]
			"
			SELECT
				actor_id,
				client_id,
				running_ts IS NULL AS failed_start,
				stopping_ts IS NOT NULL AS failed_stop,
				stop_ts IS NOT NULL AS failed_exit
			FROM db_pegboard.actors
			WHERE
				exit_ts IS NULL AND
				lost_ts IS NULL AND
				(
					-- create_ts exceeded threshold and running_ts is null (failed start)
					(
						create_ts < $1 AND
						running_ts IS NULL AND
						stopping_ts IS NULL AND
						stop_ts IS NULL
					) OR
					-- stopping_ts exceeded threshold and stop_ts is null (failed stop)
					(
						stopping_ts < $2 AND
						stop_ts IS NULL
					) OR
					-- stop_ts exceeded threshold and exit_ts is null (failed exit)
					stop_ts < $3
				)
			",
			ts - ACTOR_START_THRESHOLD_MS,
			ts - ACTOR_STOP_THRESHOLD_MS,
			ts - ACTOR_EXIT_THRESHOLD_MS
		),
	)?;

	for (client_id,) in dead_client_rows {
		tracing::warn!(?client_id, "dead client");

		ctx.signal(pegboard::workflows::client::Destroy {})
			.tag("client_id", client_id)
			.send()
			.await?;
	}

	for row in &failed_actor_rows {
		if row.failed_exit {
			tracing::error!(actor_id=?row.actor_id, "actor failed to exit");

			ctx.signal(pegboard::workflows::client::ActorStateUpdate {
				state: protocol::ActorState::Lost,
			})
			.tag("actor_id", row.actor_id)
			.send()
			.await?;
		} else if row.failed_stop {
			tracing::error!(actor_id=?row.actor_id, "actor failed to stop");

			ctx.signal(pegboard::workflows::client::ActorStateUpdate {
				state: protocol::ActorState::Lost,
			})
			.tag("actor_id", row.actor_id)
			.send()
			.await?;
		} else if row.failed_start {
			tracing::error!(actor_id=?row.actor_id, "actor failed to start");

			ctx.signal(protocol::Command::SignalActor {
				actor_id: row.actor_id,
				signal: Signal::SIGKILL as i32,
				persist_state: false,
			})
			.tag("client_id", row.client_id)
			.send()
			.await?;
		}
	}

	// Manually set stop ts for failed stop actors
	let failed_stop_actor_ids = failed_actor_rows
		.iter()
		.filter(|row| row.failed_stop)
		.map(|row| row.actor_id)
		.collect::<Vec<_>>();
	if !failed_stop_actor_ids.is_empty() {
		sql_fetch_all!(
			[ctx, ActorRow]
			"
			UPDATE db_pegboard.actors
			SET stop_ts = $2
			WHERE actor_id = ANY($1)
			",
			failed_stop_actor_ids,
			ts,
		)
		.await?;
	}

	Ok(())
}
