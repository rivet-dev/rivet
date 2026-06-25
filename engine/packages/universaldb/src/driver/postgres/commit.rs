use std::{
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::{Context, Result};

use crate::{error::DatabaseError, options::ConflictRangeType, tx_ops::Operation};

use super::{
	codec,
	shared::{LeaseInfo, PostgresShared, commit_channel, reply_channel},
};

/// How long to wait for a leader to be elected before giving up a submit as retryable.
const LEADER_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
/// Backstop poll cadence while waiting for a commit result, in case a reply NOTIFY is missed.
const RESULT_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Submit a follower transaction's commit to the leader and await the result.
///
/// `read_version` is the watermark captured when this transaction opened its read snapshot. A pure
/// snapshot read-only transaction (no operations and no read conflict ranges) submits nothing.
pub async fn submit(
	shared: &Arc<PostgresShared>,
	read_version: i64,
	operations: Vec<Operation>,
	conflict_ranges: Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>,
) -> Result<()> {
	// A transaction with no writes and no serializable read ranges has nothing to order or
	// validate; it never needs the leader.
	if operations.is_empty()
		&& conflict_ranges
			.iter()
			.all(|(_, _, kind)| matches!(kind, ConflictRangeType::Write))
	{
		return Ok(());
	}

	let lease = wait_for_leader(shared).await?;
	let payload =
		codec::encode_commit_request(read_version.max(0) as u64, &conflict_ranges, &operations)
			.context("failed to encode commit request")?;
	let reply_channel = reply_channel(&shared.node_id);

	// Subscribe to our reply channel before inserting so we cannot miss the leader's NOTIFY.
	let mut reply_rx = shared.listener.listen(&reply_channel).await;

	let conn = shared
		.pool
		.get()
		.await
		.context("failed to get connection for commit submit")?;

	// Enqueue the request and wake the leader's drain loop in one round-trip. The autocommit
	// statement durably inserts the row and fires the NOTIFY together, so there is no separate
	// notify round-trip or second pool acquire.
	let id: i64 = conn
		.query_one(
			"WITH ins AS (
				INSERT INTO udb_commit_requests (epoch, read_version, payload, reply_channel)
				VALUES ($1, $2, $3, $4)
				RETURNING id
			)
			SELECT pg_notify($5, id::text), id FROM ins",
			&[
				&lease.epoch,
				&read_version,
				&payload,
				&reply_channel,
				&commit_channel(&lease.leader_addr),
			],
		)
		.await
		.context("failed to enqueue and notify commit request")?
		.get(1);

	// Release the connection before waiting so a long wait does not pin a pool slot. The request
	// row is durable, so await_result re-acquires a connection per poll.
	drop(conn);

	let submit_start = Instant::now();
	let result = await_result(shared, id, lease.epoch, &mut reply_rx).await;
	tracing::debug!(
		id,
		epoch = lease.epoch,
		wait_ms = submit_start.elapsed().as_millis() as u64,
		ok = result.is_ok(),
		"udb commit submit completed"
	);
	result
}

/// Wait for a known leader, returning a retryable error if none is elected in time.
async fn wait_for_leader(shared: &Arc<PostgresShared>) -> Result<LeaseInfo> {
	let deadline = Instant::now() + LEADER_WAIT_TIMEOUT;
	loop {
		if let Some(lease) = shared.current_lease() {
			return Ok(lease);
		}
		if Instant::now() >= deadline {
			return Err(DatabaseError::NotCommitted.into());
		}
		tokio::time::sleep(RESULT_POLL_INTERVAL).await;
	}
}

/// Wait for the commit result, resolved directly from the leader's reply NOTIFY payload on the happy
/// path. A polling `read_status` backstop covers a missed/lagged NOTIFY, and an epoch advance orphans
/// the request (it will never be applied, so it is definitively not committed).
async fn await_result(
	shared: &Arc<PostgresShared>,
	id: i64,
	submit_epoch: i64,
	reply_rx: &mut tokio::sync::broadcast::Receiver<String>,
) -> Result<()> {
	let start = Instant::now();
	// Diagnostics: count how the waiter is driven so we can tell whether the reply NOTIFY is doing
	// its job or whether commits are riding the slow poll backstop / getting orphaned by failover.
	let mut status_reads = 0u32;
	let mut notify_wakes = 0u32;
	let mut poll_wakes = 0u32;

	// The backstop runs on a fixed-cadence interval, not a per-iteration sleep: under a flood of
	// other commits' replies on this node's shared reply channel (which we skip past), a fresh
	// per-iteration sleep would keep resetting and never fire, starving the backstop if our own
	// NOTIFY was lost. An interval ticks on wall-clock cadence regardless of loop churn.
	let mut poll_interval = tokio::time::interval(RESULT_POLL_INTERVAL);
	poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	// Consume the immediate first tick so the first backstop is one interval out, after the reply
	// has had a chance to arrive.
	poll_interval.tick().await;

	loop {
		// Wait for our reply NOTIFY, falling back to a status read on a poll tick or a lagged
		// broadcast. The happy path resolves straight from the payload with no status SELECT.
		tokio::select! {
			res = reply_rx.recv() => {
				match res {
					Ok(payload) => {
						notify_wakes += 1;
						match parse_reply(&payload, id) {
							Some(ReplyOutcome::Committed) => {
								tracing::debug!(
									id,
									wait_ms = start.elapsed().as_millis() as u64,
									status_reads,
									notify_wakes,
									poll_wakes,
									"udb commit resolved: committed"
								);
								return Ok(());
							}
							Some(ReplyOutcome::Conflict) => {
								tracing::debug!(
									id,
									wait_ms = start.elapsed().as_millis() as u64,
									status_reads,
									notify_wakes,
									poll_wakes,
									"udb commit resolved: conflict"
								);
								return Err(DatabaseError::NotCommitted.into());
							}
							// Reply for another waiter on the shared channel; keep waiting for ours.
							None => continue,
						}
					}
					Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
						// We may have missed our own reply; fall through to the status backstop.
						notify_wakes += 1;
						tracing::debug!(id, lagged = n, "udb reply broadcast lagged");
					}
					Err(tokio::sync::broadcast::error::RecvError::Closed) => {
						tracing::warn!(id, "udb reply broadcast closed; re-subscribing");
						*reply_rx = shared
							.listener
							.listen(&reply_channel(&shared.node_id))
							.await;
						continue;
					}
				}
			}
			_ = poll_interval.tick() => {
				poll_wakes += 1;
			}
		}

		// Backstop path (poll tick or lagged broadcast): re-acquire a connection and read the durable
		// status. A transient pool/query error just means we retry rather than failing a
		// possibly-applied commit.
		status_reads += 1;
		match read_status(shared, id).await {
			Ok(Some(Status::Committed)) => {
				tracing::debug!(
					id,
					wait_ms = start.elapsed().as_millis() as u64,
					status_reads,
					notify_wakes,
					poll_wakes,
					"udb commit resolved: committed (backstop)"
				);
				return Ok(());
			}
			Ok(Some(Status::Conflict)) => {
				tracing::debug!(
					id,
					wait_ms = start.elapsed().as_millis() as u64,
					status_reads,
					notify_wakes,
					poll_wakes,
					"udb commit resolved: conflict (backstop)"
				);
				return Err(DatabaseError::NotCommitted.into());
			}
			Ok(Some(Status::Pending)) => {}
			Ok(None) => {
				// The row was GC'd before we observed a terminal status. Treat as not committed
				// and let the retry loop resubmit.
				tracing::warn!(
					id,
					wait_ms = start.elapsed().as_millis() as u64,
					status_reads,
					"udb commit row missing before terminal status (gc'd); treating as not committed"
				);
				return Err(DatabaseError::NotCommitted.into());
			}
			Err(err) => {
				tracing::debug!(?err, id, "transient error polling commit status, retrying");
			}
		}

		// If a new leader took over, our old-epoch request will never be claimed.
		if let Some(current) = shared.current_lease() {
			if current.epoch != submit_epoch {
				tracing::warn!(
					id,
					submit_epoch,
					current_epoch = current.epoch,
					wait_ms = start.elapsed().as_millis() as u64,
					notify_wakes,
					poll_wakes,
					"udb commit orphaned by leader failover; treating as not committed"
				);
				return Err(DatabaseError::NotCommitted.into());
			}
		}
	}
}

enum ReplyOutcome {
	Committed,
	Conflict,
}

/// Parse a leader reply payload (`"<id>:committed:<commit_version>"` or `"<id>:conflict"`). Returns
/// `None` when the payload is for a different waiter on the shared reply channel or is unparseable.
fn parse_reply(payload: &str, id: i64) -> Option<ReplyOutcome> {
	let mut parts = payload.split(':');
	let reply_id: i64 = parts.next()?.parse().ok()?;
	if reply_id != id {
		return None;
	}
	match parts.next()? {
		"committed" => Some(ReplyOutcome::Committed),
		"conflict" => Some(ReplyOutcome::Conflict),
		_ => None,
	}
}

enum Status {
	Pending,
	Committed,
	Conflict,
}

/// Read the current status of a commit request. `Ok(None)` means the row no longer exists.
async fn read_status(shared: &Arc<PostgresShared>, id: i64) -> Result<Option<Status>> {
	let conn = shared
		.pool
		.get()
		.await
		.context("failed to get connection for commit status poll")?;

	let row = conn
		.query_opt(
			"SELECT status FROM udb_commit_requests WHERE id = $1",
			&[&id],
		)
		.await
		.context("failed to read commit request status")?;

	let Some(row) = row else {
		return Ok(None);
	};

	let status: String = row.get(0);
	let status = match status.as_str() {
		"committed" => Status::Committed,
		"conflict" => Status::Conflict,
		// 'pending' or any in-flight state.
		_ => Status::Pending,
	};
	Ok(Some(status))
}
