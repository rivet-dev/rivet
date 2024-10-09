use chirp_workflow::prelude::*;

use nix::sys::signal::Signal;
use pegboard::protocol;

/// How long to wait after last ping before forcibly removing a client from the database and deleting its
/// workflow, evicting all containers. Note that the client may still be running and can reconnect.
const CLIENT_LOST_THRESHOLD_MS: i64 = util::duration::minutes(2);
/// How long to wait after creating and not receiving a starting state before forcibly stopping container.
const CONTAINER_START_THRESHOLD_MS: i64 = util::duration::seconds(30);
/// How long to wait after stopping and not receiving a stop state before manually setting container as
/// stopped.
const CONTAINER_STOP_THRESHOLD_MS: i64 = util::duration::seconds(30);

#[derive(sqlx::FromRow)]
struct ContainerRow {
	container_id: Uuid,
	client_id: Uuid,
	failed_start: bool,
	failed_stop: bool,
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(ts: i64, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("pegboard-gc");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		rivet_connection::Connection::new(client, pools, cache),
		"pegboard-gc",
	)
	.await?;

	let (dead_client_rows, failed_container_rows) = tokio::try_join!(
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
			[ctx, ContainerRow]
			"
			SELECT
				container_id,
				client_id,
				running_ts IS NULL AS failed_start,
				stopping_ts IS NOT NULL AS failed_stop
			FROM db_pegboard.containers
			WHERE
				(
					(create_ts < $1 AND running_ts IS NULL) OR
					stopping_ts < $2
				) AND
				stop_ts IS NULL AND
				exit_ts IS NULL
			",
			ts - CONTAINER_START_THRESHOLD_MS,
			ts - CONTAINER_STOP_THRESHOLD_MS,
		),
	)?;

	for (client_id,) in dead_client_rows {
		tracing::warn!(?client_id, "dead client");

		ctx.signal(pegboard::workflows::client::Destroy {})
			.tag("client_id", client_id)
			.send()
			.await?;
	}

	for row in &failed_container_rows {
		if row.failed_stop {
			tracing::warn!(container_id=?row.container_id, "container failed to stop");

			// Manually set stopped state
			ctx.signal(pegboard::workflows::client::ContainerStateUpdate {
				state: protocol::ContainerState::Stopped,
			})
			.tag("container_id", row.container_id)
			.send()
			.await?;
		} else if row.failed_start {
			tracing::warn!(container_id=?row.container_id, "container failed to start");

			ctx.signal(protocol::Command::SignalContainer {
				container_id: row.container_id,
				signal: Signal::SIGKILL as i32,
			})
			.tag("client_id", row.client_id)
			.send()
			.await?;
		}
	}

	// Manually set stop ts for failed stop containers
	let failed_stop_container_ids = failed_container_rows
		.iter()
		.filter(|row| row.failed_stop)
		.map(|row| row.container_id)
		.collect::<Vec<_>>();
	if !failed_stop_container_ids.is_empty() {
		sql_fetch_all!(
			[ctx, ContainerRow]
			"
			UPDATE db_pegboard.containers
			SET stop_ts = $2
			WHERE container_id = ANY($1)
			",
			failed_stop_container_ids,
			ts,
		)
		.await?;
	}

	Ok(())
}
