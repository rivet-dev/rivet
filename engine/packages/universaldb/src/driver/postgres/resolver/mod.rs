mod apply;
mod lease;

use std::{
	collections::HashMap,
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::{Context, Result};
use tokio::sync::broadcast;
use tokio_util::task::AbortOnDropHandle;

use crate::{conflict_tracker::TransactionConflictTracker, transaction::TXN_TIMEOUT};

use lease::LEASE_TTL_SECS;

use super::shared::{
	ELECTION_CHANNEL, LEASE_ID, LeaseInfo, PostgresShared, WATERMARK_CHANNEL, commit_channel,
};

/// Max commits resolved+applied per batch (group commit). Amortizes the resolver, Postgres
/// round-trips, and fsync across the batch.
const DRAIN_BATCH_SIZE: i64 = 256;

/// How often a leader renews its lease. Must be comfortably under `LEASE_TTL_SECS`.
const RENEW_INTERVAL: Duration = Duration::from_secs(3);

/// Backstop poll cadence so a missed `udb_commit` NOTIFY cannot stall the drain indefinitely.
const POLL_BACKSTOP: Duration = Duration::from_millis(50);

/// How long a candidate waits before retrying election when another node holds the lease.
const ELECTION_RETRY: Duration = Duration::from_secs(2);

/// Spawn the per-process resolver task. Every node runs this; only the elected leader drains the
/// commit queue. The returned handle is aborted when the owning driver drops, which stops lease
/// renewal so the lease expires and another node can take over (node-death / failover path).
pub fn spawn(shared: Arc<PostgresShared>) -> tokio::task::JoinHandle<()> {
	tokio::spawn(run(shared))
}

async fn run(shared: Arc<PostgresShared>) {
	// A departing leader NOTIFYs this channel after releasing its lease so we elect immediately
	// rather than waiting out the full `ELECTION_RETRY` tick.
	let mut election_rx = shared.listener.listen(ELECTION_CHANNEL).await;

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
				wait_for_election_retry(&shared, &mut election_rx).await;
			}
			Err(err) => {
				tracing::warn!(?err, "failed udb lease acquire attempt");
				wait_for_election_retry(&shared, &mut election_rx).await;
			}
		}
	}
}

/// Wait before retrying the election: either the `ELECTION_RETRY` backstop elapses, or a departing
/// leader wakes us via `ELECTION_CHANNEL` so handoff is near-instant.
async fn wait_for_election_retry(
	shared: &Arc<PostgresShared>,
	election_rx: &mut broadcast::Receiver<String>,
) {
	tokio::select! {
		_ = tokio::time::sleep(ELECTION_RETRY) => {}
		res = election_rx.recv() => {
			if matches!(res, Err(broadcast::error::RecvError::Closed)) {
				// The listener recreates the channel on reconnect; re-subscribe.
				*election_rx = shared.listener.listen(ELECTION_CHANNEL).await;
			}
		}
	}
}

/// Best-effort graceful leadership handoff invoked on shutdown. If this node currently holds the
/// lease, expire it and wake a standby so it takes over immediately instead of waiting out the TTL.
/// Safe to call on a follower: the fenced release matches no row and nothing is notified. The
/// caller must already have stopped lease renewal before calling this.
pub async fn handoff(shared: &Arc<PostgresShared>) {
	match lease::release(&shared.pool, &shared.node_id).await {
		Ok(true) => {
			tracing::info!(node_id = %shared.node_id, "released udb leader lease for graceful handoff");
			notify_election(shared).await;
		}
		Ok(false) => {}
		Err(err) => {
			tracing::warn!(?err, "failed to release udb lease on shutdown");
		}
	}
}

/// Wake standby candidates so the next election fires immediately after a graceful release.
async fn notify_election(shared: &Arc<PostgresShared>) {
	let conn = match shared.pool.get().await {
		Ok(conn) => conn,
		Err(err) => {
			tracing::debug!(?err, "failed to get connection for election notify");
			return;
		}
	};

	if let Err(err) = conn
		.execute("SELECT pg_notify($1, '')", &[&ELECTION_CHANNEL])
		.await
	{
		tracing::debug!(?err, "failed to notify election channel");
	}
}

/// Leader entry point: publish our lease, compute the cold-window floor, then run renewal and
/// draining as two sibling tasks. They coordinate purely by completion: when either returns (lease
/// lost or error), the other is aborted and the leader steps down. Both operations are safe to
/// hard-abort, so no explicit cancellation signalling is needed. A renew is a single fenced
/// `UPDATE`; a drain batch runs in one Postgres transaction that rolls back cleanly when dropped,
/// leaving its claimed requests `pending` for the next leader.
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

	tracing::info!(
		epoch,
		recovery_version,
		cold_window_ms = TXN_TIMEOUT.as_millis() as u64,
		"udb leader entering lead loop"
	);

	// Renewal runs in its own task so the drain loop can never starve it: if renewal shared the
	// drain loop, a single long drain under sustained load would block renewal past the lease TTL
	// and the lease would be lost mid-drain, thrashing leadership. Both are held in abort-on-drop
	// handles so a hard abort of this `run` task (driver drop / node death) tears them down. A leaked
	// renew task would keep this dead leader's lease alive and block failover.
	let mut renew = AbortOnDropHandle::new(tokio::spawn(renew_loop(shared.clone(), epoch)));
	let mut drain = AbortOnDropHandle::new(tokio::spawn(drain_loop(
		shared.clone(),
		epoch,
		recovery_version,
		recovery_deadline,
	)));

	// Whichever task returns first (lease lost or error), step down; the other is aborted when its
	// handle drops at the end of this scope. A clean exit yields its inner result; a panic surfaces
	// through `?` as a join error.
	tokio::select! {
		res = &mut renew => res?,
		res = &mut drain => res?,
	}
}

/// Lease-renewal loop. Runs on its own task and pool connection so it cannot be starved by drain
/// work. Returns when the lease is definitively gone (epoch bumped by another node, or renewal
/// failing for the whole lease TTL), which causes [`lead`] to abort the drain task and step down.
async fn renew_loop(shared: Arc<PostgresShared>, epoch: i64) -> Result<()> {
	let mut interval = tokio::time::interval(RENEW_INTERVAL);
	interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	// The lease was just acquired/renewed (expires_at = now + TTL), so consume the immediate first
	// tick and renew after one interval.
	interval.tick().await;

	let mut last_renew = Instant::now();

	loop {
		interval.tick().await;

		let gap_ms = last_renew.elapsed().as_millis() as u64;
		let renew_start = Instant::now();
		match lease::renew(&shared.pool, &shared.node_id, epoch).await {
			Ok(true) => {
				last_renew = Instant::now();
				let renew_query_ms = renew_start.elapsed().as_millis() as u64;
				// With renewal on its own task this gap should track RENEW_INTERVAL closely; a large
				// gap now points at pool or Postgres contention.
				if gap_ms > RENEW_INTERVAL.as_millis() as u64 * 2 {
					tracing::warn!(
						epoch,
						gap_since_last_renew_ms = gap_ms,
						renew_query_ms,
						"udb leader renew was delayed (pool or postgres contention)"
					);
				} else {
					tracing::debug!(
						epoch,
						gap_since_last_renew_ms = gap_ms,
						renew_query_ms,
						"udb leader renewed lease"
					);
				}
			}
			Ok(false) => {
				tracing::warn!(
					epoch,
					gap_since_last_renew_ms = gap_ms,
					"udb leader lost lease on renew (epoch bumped by another node); stepping down"
				);
				return Ok(());
			}
			Err(err) => {
				// A transient renew error is tolerable within the TTL; keep retrying. Only give up
				// if we have been unable to renew for the whole lease TTL, at which point we can no
				// longer assume we hold the lease.
				if last_renew.elapsed() >= Duration::from_secs(LEASE_TTL_SECS as u64) {
					tracing::warn!(
						?err,
						epoch,
						"udb leader renew failing past lease TTL; assuming lease lost and stepping down"
					);
					return Ok(());
				}
				tracing::warn!(?err, epoch, "udb leader lease renew errored; will retry");
			}
		}
	}
}

/// Drain loop. Processes one batch per iteration. Draining until the queue emptied in a single call
/// could run for many seconds under sustained load; processing one batch at a time keeps the loop
/// at a clean await point between batches. A non-empty queue still drains back-to-back with no idle
/// wait, so throughput is unchanged. It blocks on `select!` (wake NOTIFY or poll backstop) only
/// when the queue is empty. Returns when this leader's epoch is fenced out; otherwise [`lead`]
/// aborts it when renewal reports the lease is lost.
async fn drain_loop(
	shared: Arc<PostgresShared>,
	epoch: i64,
	recovery_version: u64,
	recovery_deadline: Instant,
) -> Result<()> {
	let tracker = TransactionConflictTracker::new();

	let mut wake_rx = shared
		.listener
		.listen(&commit_channel(&shared.node_id))
		.await;

	let mut poll_interval = tokio::time::interval(POLL_BACKSTOP);
	poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

	loop {
		match drain_batch(
			&shared,
			epoch,
			&tracker,
			recovery_version,
			recovery_deadline,
		)
		.await?
		{
			BatchOutcome::LostLease => {
				tracing::warn!(
					epoch,
					"udb leader stepping down: lost lease during drain (epoch fenced on watermark)"
				);
				return Ok(());
			}
			// More work may be pending; loop immediately to keep throughput up.
			BatchOutcome::Processed => continue,
			BatchOutcome::Empty => {
				tokio::select! {
					res = wake_rx.recv() => {
						match res {
							Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
							Err(broadcast::error::RecvError::Closed) => {
								wake_rx = shared
									.listener
									.listen(&commit_channel(&shared.node_id))
									.await;
							}
						}
					}
					_ = poll_interval.tick() => {}
				}
			}
		}
	}
}

/// The cold-window rejection floor for a freshly elected leader: the durable watermark
/// (`udb_lease.durable_version`) at election time.
///
/// Reasoning (do NOT change this back to `max(durable, seq_high)`):
///
/// A new leader starts with an empty conflict tracker, so it cannot detect a read-write conflict
/// against any committed write it does not already know about. The writes it is missing are exactly
/// the previous leader's winners, and every winner's write is applied to `kv` AND its
/// `commit_version` folded into `durable_version` in the SAME apply transaction. So every missing
/// write has `commit_version <= durable_version`. A committing transaction `T` is therefore safe
/// iff `T.read_version >= durable_version`: every write above its read_version was committed by THIS
/// leader and is in the tracker. Only `T.read_version < durable_version` can race a missing winner,
/// so that is the exact set the cold window must reject.
///
/// `udb_version_seq.last_value` (the sequence high-water) is NOT a valid floor. Every drained
/// request consumes a `nextval` BEFORE the conflict check, including conflicts and cold rejects, so
/// the sequence races far ahead of `durable_version` with versions that never produced any write.
/// Using `max(durable, seq_high)` rejects essentially every commit for the whole cold window
/// (followers read at `durable_version`, which is always `< seq_high`), turning each failover into
/// a 5s mass-reject storm. The gap `(durable_version, seq_high]` holds only thrown-away loser
/// versions, so nothing in it is a missing write to guard against.
///
/// Version ASSIGNMENT is unaffected: commit versions still come from `nextval('udb_version_seq')`
/// in `drain_batch`, which is always above the sequence high-water, so uniqueness and monotonicity
/// across failover are preserved independently of this floor.
async fn recovery_floor(shared: &Arc<PostgresShared>) -> Result<u64> {
	let durable = lease::current_durable_version(&shared.pool).await?;
	Ok(durable.max(0) as u64)
}

enum BatchOutcome {
	Empty,
	Processed,
	LostLease,
}

struct Reply {
	channel: String,
	/// The follower's reply payload, encoding the outcome so the waiter resolves without a status
	/// SELECT: `"<id>:committed:<commit_version>"` or `"<id>:conflict"`.
	payload: String,
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

	// Claim a batch in id order and allocate every commit version in the SAME round-trip via an
	// inline `nextval`, instead of a separate version-allocation query. FOR UPDATE SKIP LOCKED holds
	// the rows for this txn so they are stamped terminal on COMMIT with no intermediate 'claimed'
	// state to clean up.
	//
	// Postgres does NOT guarantee `nextval` is evaluated in output (id) order, so the per-row `cv`
	// here may not be monotonic in id. The versions are collected, sorted, and re-assigned to rows in
	// id order below, exactly as the separate-query path did, because versionstamp monotonicity with
	// commit order is load-bearing (epoxy changelog catch-up + depot PITR).
	let rows = txn
		.query(
			"SELECT id, read_version, payload, reply_channel,
			        nextval('udb_version_seq') AS cv
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

	let batch_start = Instant::now();
	let cold_window = Instant::now() < recovery_deadline;
	let batch_len = rows.len();
	let mut max_winner_cv: i64 = 0;
	let mut replies = Vec::with_capacity(batch_len);
	let mut committed_count = 0u32;
	let mut conflict_count = 0u32;
	let mut cold_reject_count = 0u32;

	// Re-assign the inline-allocated versions to rows in id order (winners and losers alike; losers'
	// versions are harmlessly skipped) so versionstamps stay monotonic with commit order. The sort
	// keeps assignment monotonic regardless of how Postgres ordered the per-row `nextval` evaluation.
	let mut versions: Vec<i64> = rows.iter().map(|row| row.get::<_, i64>(4)).collect();
	versions.sort_unstable();

	// Resolve every request in memory in id order. Winners are collected with their version and
	// operations for the fold; the bulk status stamp is built for all rows at once.
	let mut winners: Vec<apply::Winner> = Vec::new();
	let mut stamp_ids: Vec<i64> = Vec::with_capacity(batch_len);
	let mut stamp_statuses: Vec<&str> = Vec::with_capacity(batch_len);
	let mut stamp_versions: Vec<Option<i64>> = Vec::with_capacity(batch_len);

	// Sub-phase timers to decompose batch_ms and confirm whether the service time is constant (pure
	// M/M/1 queueing) or itself inflates under load.
	let mut decode_dur = Duration::ZERO;
	let mut conflict_dur = Duration::ZERO;

	for (i, row) in rows.iter().enumerate() {
		let id: i64 = row.get(0);
		let read_version: i64 = row.get(1);
		let payload: Vec<u8> = row.get(2);
		let reply_channel: String = row.get(3);
		let commit_version = versions[i];

		let decode_start = Instant::now();
		let decoded = super::codec::decode_commit_request(&payload)
			.context("failed to decode commit payload")?;
		decode_dur += decode_start.elapsed();

		let start_version = read_version.max(0) as u64;

		// Cold-window guard: a commit whose read_version predates the recovery floor cannot be
		// safely resolved against this leader's empty window. Reject it as retryable.
		let cold_rejected = cold_window && start_version < recovery_version;
		let conflicted = if cold_rejected {
			cold_reject_count += 1;
			true
		} else {
			let conflict_start = Instant::now();
			let res = tracker
				.check_and_insert(
					start_version,
					commit_version.max(0) as u64,
					decoded.conflict_ranges,
				)
				.await;
			conflict_dur += conflict_start.elapsed();
			res
		};

		stamp_ids.push(id);
		if conflicted {
			if !cold_rejected {
				conflict_count += 1;
			}
			stamp_statuses.push("conflict");
			stamp_versions.push(None);
		} else {
			committed_count += 1;
			stamp_statuses.push("committed");
			stamp_versions.push(Some(commit_version));
			max_winner_cv = max_winner_cv.max(commit_version);
			winners.push(apply::Winner {
				commit_version: commit_version.max(0) as u64,
				operations: decoded.operations,
			});
		}

		let reply_payload = if conflicted {
			format!("{id}:conflict")
		} else {
			format!("{id}:committed:{commit_version}")
		};
		replies.push(Reply {
			channel: reply_channel,
			payload: reply_payload,
		});
	}

	let t_resolved = Instant::now();

	// Bulk-read the pre-batch value of every key a winner's atomic op reads in one query, then fold
	// all winners into a single materialized write-set in memory. This collapses the per-row apply
	// round-trips to a fixed count independent of batch size.
	let atomic_keys = apply::atomic_read_keys(&winners);
	let base = if atomic_keys.is_empty() {
		HashMap::new()
	} else {
		txn.query(
			"SELECT key, value FROM kv WHERE key = ANY($1::bytea[])",
			&[&atomic_keys],
		)
		.await
		.context("failed to bulk-read atomic op keys")?
		.into_iter()
		.map(|row| (row.get::<_, Vec<u8>>(0), row.get::<_, Vec<u8>>(1)))
		.collect()
	};

	let t_read = Instant::now();

	let apply::WriteSet {
		upserts,
		point_deletes,
		range_deletes,
	} = apply::fold_winners(winners, &base).context("failed to fold batch winners")?;

	let t_fold = Instant::now();

	let (upsert_keys, upsert_values): (Vec<Vec<u8>>, Vec<Vec<u8>>) = upserts.into_iter().unzip();
	let (range_begins, range_ends): (Vec<Vec<u8>>, Vec<Vec<u8>>) =
		range_deletes.into_iter().unzip();

	// Range deletes run in their OWN statement BEFORE the apply CTE. Postgres data-modifying CTE
	// sub-statements all observe the same snapshot and never see each other's effects, so a range
	// delete and an in-range upsert in one CTE would have unspecified results. `range_deletes` are
	// ranges (not materialized per-key) and can overlap an upsert key (a key inside a cleared range
	// that is also re-set), so the range clear must commit its effect first; the upsert then
	// re-inserts the key. Collapsed from a per-range loop into one statement over a pair of arrays.
	if !range_begins.is_empty() {
		txn.execute(
			"DELETE FROM kv USING unnest($1::bytea[], $2::bytea[]) AS r(b, e)
			 WHERE key >= r.b AND key < r.e",
			&[&range_begins, &range_ends],
		)
		.await
		.context("failed to clear ranges")?;
	}

	// Apply the rest of the batch in one CTE: point deletes, the kv upsert, the terminal status
	// stamp, and the epoch-fenced watermark advance. This is safe to fold because the write sets are
	// disjoint: `apply::WriteSet` guarantees each key appears at most once across `upserts` and
	// `point_deletes` (see apply.rs), and the three tables (`kv`, `udb_commit_requests`, `udb_lease`)
	// are independent. The watermark UPDATE is fenced on our epoch: a zombie old leader whose epoch
	// was bumped sees zero rows returned and must step down before any of its writes become visible.
	let new_durable: i64 = match txn
		.query_opt(
			"WITH pdel AS (
				DELETE FROM kv WHERE key = ANY($1::bytea[])
			), up AS (
				INSERT INTO kv (key, value)
				SELECT * FROM unnest($2::bytea[], $3::bytea[])
				ON CONFLICT (key) DO UPDATE SET value = excluded.value
			), stamp AS (
				UPDATE udb_commit_requests AS r
				   SET status = b.status, commit_version = b.cv
				 FROM unnest($4::bigint[], $5::text[], $6::bigint[]) AS b(id, status, cv)
				 WHERE r.id = b.id
			)
			UPDATE udb_lease
			   SET durable_version = GREATEST(durable_version, $7)
			 WHERE id = $8 AND epoch = $9
			 RETURNING durable_version",
			&[
				&point_deletes,
				&upsert_keys,
				&upsert_values,
				&stamp_ids,
				&stamp_statuses,
				&stamp_versions,
				&max_winner_cv,
				&LEASE_ID,
				&epoch,
			],
		)
		.await
		.context("failed to apply batch and advance watermark")?
	{
		Some(row) => row.get(0),
		None => {
			txn.rollback().await.ok();
			return Ok(BatchOutcome::LostLease);
		}
	};

	let t_applied = Instant::now();

	txn.commit().await.context("failed to commit drain batch")?;

	let t_committed = Instant::now();

	// Watermark advances strictly after the apply txn is durably committed and visible, so a
	// reader handed this read_version can never miss a write with commit_version <= read_version.
	shared.advance_durable_version(new_durable);

	notify_after_commit(&conn, new_durable, &replies).await;

	let t_notified = Instant::now();

	let tracker_len = tracker.len().await;

	tracing::info!(
		epoch,
		batch_len,
		committed = committed_count,
		conflict = conflict_count,
		cold_reject = cold_reject_count,
		cold_window,
		new_durable,
		batch_ms = batch_start.elapsed().as_millis() as u64,
		// Sub-phase decomposition of batch_ms (micros) to separate constant service time from
		// load-dependent service inflation.
		tracker_len,
		decode_us = decode_dur.as_micros() as u64,
		conflict_us = conflict_dur.as_micros() as u64,
		resolve_us = (t_resolved - batch_start).as_micros() as u64,
		read_us = (t_read - t_resolved).as_micros() as u64,
		fold_us = (t_fold - t_read).as_micros() as u64,
		apply_us = (t_applied - t_fold).as_micros() as u64,
		commit_us = (t_committed - t_applied).as_micros() as u64,
		notify_us = (t_notified - t_committed).as_micros() as u64,
		"udb leader processed commit batch"
	);

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
	let payloads: Vec<&str> = replies.iter().map(|r| r.payload.as_str()).collect();
	if let Err(err) = conn
		.execute(
			"SELECT pg_notify(c, p) FROM unnest($1::text[], $2::text[]) AS t(c, p)",
			&[&channels, &payloads],
		)
		.await
	{
		tracing::debug!(?err, "failed to notify commit replies");
	}
}
