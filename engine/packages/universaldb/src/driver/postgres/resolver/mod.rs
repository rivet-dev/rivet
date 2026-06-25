mod apply;
mod lease;

use std::{
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::{Context, Result};

use crate::{conflict_tracker::TransactionConflictTracker, transaction::TXN_TIMEOUT};

use super::shared::{LEASE_ID, LeaseInfo, PostgresShared, WATERMARK_CHANNEL, commit_channel};

/// Max commits resolved+applied per batch (group commit). Amortizes the resolver, Postgres
/// round-trips, and fsync across the batch.
const DRAIN_BATCH_SIZE: i64 = 256;

/// How often a leader renews its lease. Must be comfortably under `LEASE_TTL_SECS`.
const RENEW_INTERVAL: Duration = Duration::from_secs(3);

/// Backstop poll cadence so a missed `udb_commit` NOTIFY cannot stall the drain indefinitely.
const POLL_BACKSTOP: Duration = Duration::from_millis(50);

/// How long a candidate waits before retrying election when another node holds the lease.
const ELECTION_RETRY: Duration = Duration::from_secs(2);

enum DrainOutcome {
	/// Processed zero or more requests; still leader.
	Drained,
	/// Lost the lease (epoch bumped by a new leader). Step down.
	LostLease,
}

/// Spawn the per-process resolver task. Every node runs this; only the elected leader drains the
/// commit queue. The returned handle is aborted when the owning driver drops, which stops lease
/// renewal so the lease expires and another node can take over (node-death / failover path).
pub fn spawn(shared: Arc<PostgresShared>) -> tokio::task::JoinHandle<()> {
	tokio::spawn(run(shared))
}

async fn run(shared: Arc<PostgresShared>) {
	loop {
		match lease::try_acquire(&shared.pool, &shared.node_id).await {
			Ok(Some(acquired)) => {
				tracing::info!(epoch = acquired.epoch, node_id = %shared.node_id, "acquired udb leader lease");
				if let Err(err) = lead(&shared, acquired.epoch).await {
					tracing::error!(?err, "udb leader loop errored, stepping down");
				}
				tracing::info!(epoch = acquired.epoch, "stepped down from udb leader");
			}
			Ok(None) => {
				tokio::time::sleep(ELECTION_RETRY).await;
			}
			Err(err) => {
				tracing::warn!(?err, "failed udb lease acquire attempt");
				tokio::time::sleep(ELECTION_RETRY).await;
			}
		}
	}
}

/// Leader main loop: hold the lease, drain the commit queue on wake or poll, and renew the lease.
async fn lead(shared: &Arc<PostgresShared>, epoch: i64) -> Result<()> {
	// Publish our own lease into the cache immediately so our local commits route to us.
	shared.set_lease(LeaseInfo {
		epoch,
		leader_addr: shared.node_id.clone(),
	});

	// The recovery floor: a freshly elected leader has a cold conflict window, so reject commits
	// whose read_version predates the floor until the window warms (one TXN_TIMEOUT), forcing
	// those followers to take a fresh read_version.
	let recovery_version = recovery_floor(shared).await?;
	let recovery_deadline = Instant::now() + TXN_TIMEOUT;

	let tracker = TransactionConflictTracker::new();

	let mut wake_rx = shared
		.listener
		.listen(&commit_channel(&shared.node_id))
		.await;

	let mut renew_interval = tokio::time::interval(RENEW_INTERVAL);
	renew_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	let mut poll_interval = tokio::time::interval(POLL_BACKSTOP);
	poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

	// Drain anything already queued before our first wake.
	if matches!(
		drain(shared, epoch, &tracker, recovery_version, recovery_deadline).await?,
		DrainOutcome::LostLease
	) {
		return Ok(());
	}

	loop {
		tokio::select! {
			_ = renew_interval.tick() => {
				if !lease::renew(&shared.pool, &shared.node_id, epoch).await? {
					tracing::warn!(epoch, "lost udb lease on renew");
					return Ok(());
				}
			}
			res = wake_rx.recv() => {
				match res {
					Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
					Err(tokio::sync::broadcast::error::RecvError::Closed) => {
						wake_rx = shared.listener.listen(&commit_channel(&shared.node_id)).await;
					}
				}
				if matches!(
					drain(shared, epoch, &tracker, recovery_version, recovery_deadline).await?,
					DrainOutcome::LostLease
				) {
					return Ok(());
				}
			}
			_ = poll_interval.tick() => {
				if matches!(
					drain(shared, epoch, &tracker, recovery_version, recovery_deadline).await?,
					DrainOutcome::LostLease
				) {
					return Ok(());
				}
			}
		}
	}
}

/// The version floor a freshly elected leader continues from: the higher of the durable watermark
/// and the sequence high-water. The LOGGED `udb_version_seq` is crash-safe, so this never regresses.
async fn recovery_floor(shared: &Arc<PostgresShared>) -> Result<u64> {
	let durable = lease::current_durable_version(&shared.pool).await?;

	let conn = shared
		.pool
		.get()
		.await
		.context("failed to get connection for recovery floor")?;
	let seq_high: i64 = conn
		.query_one("SELECT last_value FROM udb_version_seq", &[])
		.await
		.context("failed to read sequence high water")?
		.get(0);

	Ok(durable.max(seq_high).max(0) as u64)
}

/// Drain pending commit requests in id-ordered batches until none remain. Each batch resolves and
/// applies inside a single Postgres transaction (group commit), fenced on the leader's epoch.
async fn drain(
	shared: &Arc<PostgresShared>,
	epoch: i64,
	tracker: &TransactionConflictTracker,
	recovery_version: u64,
	recovery_deadline: Instant,
) -> Result<DrainOutcome> {
	loop {
		match drain_batch(shared, epoch, tracker, recovery_version, recovery_deadline).await? {
			BatchOutcome::Empty => return Ok(DrainOutcome::Drained),
			BatchOutcome::Processed => {}
			BatchOutcome::LostLease => return Ok(DrainOutcome::LostLease),
		}
	}
}

enum BatchOutcome {
	Empty,
	Processed,
	LostLease,
}

struct Reply {
	channel: String,
	id: i64,
}

async fn drain_batch(
	shared: &Arc<PostgresShared>,
	epoch: i64,
	tracker: &TransactionConflictTracker,
	recovery_version: u64,
	recovery_deadline: Instant,
) -> Result<BatchOutcome> {
	let mut conn = shared
		.pool
		.get()
		.await
		.context("failed to get connection for drain batch")?;
	let txn = conn
		.build_transaction()
		.start()
		.await
		.context("failed to start drain batch txn")?;

	// Claim a batch in id order. FOR UPDATE SKIP LOCKED holds the rows for this txn so they are
	// stamped terminal on COMMIT with no intermediate 'claimed' state to clean up.
	let rows = txn
		.query(
			"SELECT id, read_version, payload, reply_channel
			 FROM udb_commit_requests
			 WHERE status = 'pending' AND epoch = $1
			 ORDER BY id
			 LIMIT $2
			 FOR UPDATE SKIP LOCKED",
			&[&epoch, &DRAIN_BATCH_SIZE],
		)
		.await
		.context("failed to claim commit batch")?;

	if rows.is_empty() {
		txn.rollback().await.ok();
		return Ok(BatchOutcome::Empty);
	}

	let cold_window = Instant::now() < recovery_deadline;
	let mut max_winner_cv: i64 = 0;
	let mut replies = Vec::with_capacity(rows.len());

	for row in &rows {
		let id: i64 = row.get(0);
		let read_version: i64 = row.get(1);
		let payload: Vec<u8> = row.get(2);
		let reply_channel: String = row.get(3);

		let decoded = super::codec::decode_commit_request(&payload)
			.context("failed to decode commit payload")?;

		let commit_version: i64 = txn
			.query_one("SELECT nextval('udb_version_seq')", &[])
			.await
			.context("failed to get next commit version")?
			.get(0);

		let start_version = read_version.max(0) as u64;

		// Cold-window guard: a commit whose read_version predates the recovery floor cannot be
		// safely resolved against this leader's empty window. Reject it as retryable.
		let conflicted = if cold_window && start_version < recovery_version {
			true
		} else {
			tracker
				.check_and_insert(
					start_version,
					commit_version.max(0) as u64,
					decoded.conflict_ranges,
				)
				.await
		};

		if conflicted {
			txn.execute(
				"UPDATE udb_commit_requests SET status = 'conflict' WHERE id = $1",
				&[&id],
			)
			.await
			.context("failed to stamp conflict")?;
		} else {
			apply::apply(&txn, decoded.operations, commit_version.max(0) as u64)
				.await
				.context("failed to apply commit")?;
			txn.execute(
				"UPDATE udb_commit_requests SET status = 'committed', commit_version = $1 WHERE id = $2",
				&[&commit_version, &id],
			)
			.await
			.context("failed to stamp committed")?;
			max_winner_cv = max_winner_cv.max(commit_version);
		}

		replies.push(Reply {
			channel: reply_channel,
			id,
		});
	}

	// Advance the watermark, fenced on our epoch. A zombie old leader whose epoch was bumped sees
	// zero rows updated and must step down before any of its writes become visible.
	let new_durable: i64 = match txn
		.query_opt(
			"UPDATE udb_lease
			   SET durable_version = GREATEST(durable_version, $1)
			 WHERE id = $2 AND epoch = $3
			 RETURNING durable_version",
			&[&max_winner_cv, &LEASE_ID, &epoch],
		)
		.await
		.context("failed to advance watermark")?
	{
		Some(row) => row.get(0),
		None => {
			txn.rollback().await.ok();
			return Ok(BatchOutcome::LostLease);
		}
	};

	txn.commit().await.context("failed to commit drain batch")?;

	// Watermark advances strictly after the apply txn is durably committed and visible, so a
	// reader handed this read_version can never miss a write with commit_version <= read_version.
	shared.advance_durable_version(new_durable);

	notify_after_commit(&conn, new_durable, &replies).await;

	Ok(BatchOutcome::Processed)
}

/// Wake watermark listeners and the followers waiting on each processed request. Best-effort: a
/// missed NOTIFY is covered by the follower's polling backstop and the watermark refresh timer.
async fn notify_after_commit(
	conn: &deadpool_postgres::Client,
	new_durable: i64,
	replies: &[Reply],
) {
	if let Err(err) = conn
		.execute(
			"SELECT pg_notify($1, $2)",
			&[&WATERMARK_CHANNEL, &new_durable.to_string()],
		)
		.await
	{
		tracing::debug!(?err, "failed to notify watermark");
	}

	let channels: Vec<&str> = replies.iter().map(|r| r.channel.as_str()).collect();
	let ids: Vec<String> = replies.iter().map(|r| r.id.to_string()).collect();
	if let Err(err) = conn
		.execute(
			"SELECT pg_notify(c, p) FROM unnest($1::text[], $2::text[]) AS t(c, p)",
			&[&channels, &ids],
		)
		.await
	{
		tracing::debug!(?err, "failed to notify commit replies");
	}
}
