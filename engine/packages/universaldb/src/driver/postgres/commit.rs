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

	let id: i64 = conn
		.query_one(
			"INSERT INTO udb_commit_requests (epoch, read_version, payload, reply_channel)
			 VALUES ($1, $2, $3, $4)
			 RETURNING id",
			&[&lease.epoch, &read_version, &payload, &reply_channel],
		)
		.await
		.context("failed to enqueue commit request")?
		.get(0);

	// Wake the leader's drain loop.
	if let Err(err) = conn
		.execute(
			"SELECT pg_notify($1, $2)",
			&[&commit_channel(&lease.leader_addr), &id.to_string()],
		)
		.await
	{
		tracing::debug!(
			?err,
			"failed to notify leader; relying on its poll backstop"
		);
	}

	// Release the connection before waiting so a long wait does not pin a pool slot. The request
	// row is durable, so await_result re-acquires a connection per poll.
	drop(conn);

	await_result(shared, id, lease.epoch, &mut reply_rx).await
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

/// Poll the request row until it reaches a terminal status, woken by reply NOTIFYs with a polling
/// backstop. Bails as retryable if the leader epoch advances (our request is now orphaned and will
/// never be applied, so it is definitively not committed).
async fn await_result(
	shared: &Arc<PostgresShared>,
	id: i64,
	submit_epoch: i64,
	reply_rx: &mut tokio::sync::broadcast::Receiver<String>,
) -> Result<()> {
	loop {
		// Re-acquire a connection per poll: the request row is durable, so a transient pool/query
		// error just means we retry the poll rather than failing a possibly-applied commit.
		match read_status(shared, id).await {
			Ok(Some(Status::Committed)) => return Ok(()),
			Ok(Some(Status::Conflict)) => return Err(DatabaseError::NotCommitted.into()),
			Ok(Some(Status::Pending)) => {}
			Ok(None) => {
				// The row was GC'd before we observed a terminal status. Treat as not committed
				// and let the retry loop resubmit.
				return Err(DatabaseError::NotCommitted.into());
			}
			Err(err) => {
				tracing::debug!(?err, "transient error polling commit status, retrying");
			}
		}

		// If a new leader took over, our old-epoch request will never be claimed.
		if let Some(current) = shared.current_lease() {
			if current.epoch != submit_epoch {
				return Err(DatabaseError::NotCommitted.into());
			}
		}

		tokio::select! {
			res = reply_rx.recv() => {
				match res {
					Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
					Err(tokio::sync::broadcast::error::RecvError::Closed) => {
						*reply_rx = shared
							.listener
							.listen(&reply_channel(&shared.node_id))
							.await;
					}
				}
			}
			_ = tokio::time::sleep(RESULT_POLL_INTERVAL) => {}
		}
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
