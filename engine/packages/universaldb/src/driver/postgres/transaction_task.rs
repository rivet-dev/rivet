use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use deadpool_postgres::Transaction;
use tokio::sync::{mpsc, oneshot};
use tokio_postgres::IsolationLevel;

use crate::{
	options::ConflictRangeType,
	tx_ops::Operation,
	value::{KeyValue, Slice, Values},
};

use super::{commit, shared::PostgresShared};

/// Narrowest row a byte bounded page is sized for. Such a page asks Postgres for at most its byte
/// budget divided by this many rows, which bounds the rows read past the budget and then left out.
const PAGE_MIN_ROW_BYTES: usize = 64;

/// Fewest rows a byte bounded page asks Postgres for.
const PAGE_MIN_ROWS: usize = 16;

/// Once a page has shown how large the rows are, the next page asks Postgres for this many times the
/// rows that would fill its byte budget at that size. The slack lets a run of smaller rows still
/// fill most of the budget.
const PAGE_ROW_SLACK: usize = 2;

pub enum TransactionCommand {
	// Read operations
	Get {
		key: Vec<u8>,
		response: oneshot::Sender<Result<Option<Slice>>>,
	},
	GetKey {
		key: Vec<u8>,
		or_equal: bool,
		offset: i32,
		response: oneshot::Sender<Result<Option<Slice>>>,
	},
	GetRange {
		begin: Vec<u8>,
		begin_or_equal: bool,
		begin_offset: i32,
		end: Vec<u8>,
		end_or_equal: bool,
		end_offset: i32,
		limit: Option<usize>,
		/// Soft cap on the key plus value bytes of the page. See `RangeOption::page_target_bytes`.
		target_bytes: Option<usize>,
		reverse: bool,
		response: oneshot::Sender<Result<Values>>,
	},
	// Transaction control
	Commit {
		operations: Vec<Operation>,
		conflict_ranges: Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>,
		response: oneshot::Sender<Result<()>>,
	},
	GetEstimatedRangeSize {
		begin: Vec<u8>,
		end: Vec<u8>,
		response: oneshot::Sender<Result<i64>>,
	},
}

/// TransactionTask runs in a separate tokio task to own a single pinned PostgreSQL `REPEATABLE READ`
/// snapshot connection for the lifetime of a follower transaction.
///
/// Reads go directly against this snapshot (they never involve the leader). Commits delegate to
/// [`commit::submit`], which enqueues the request on the leader and awaits the result. The
/// `read_version` is captured from the cached watermark before the snapshot is opened, so no write
/// with `commit_version <= read_version` can be invisible to the snapshot.
pub struct TransactionTask {
	shared: Arc<PostgresShared>,
	receiver: mpsc::UnboundedReceiver<TransactionCommand>,
	/// Mean key plus value bytes of the rows in the last byte bounded page this transaction read.
	/// It sizes the row limit of the next page. A wrong guess only makes that page shorter or makes
	/// Postgres read more rows past the budget, and the page after it corrects the guess.
	page_row_bytes: Option<usize>,
}

impl TransactionTask {
	pub fn new(
		shared: Arc<PostgresShared>,
		receiver: mpsc::UnboundedReceiver<TransactionCommand>,
	) -> Self {
		Self {
			shared,
			receiver,
			page_row_bytes: None,
		}
	}

	pub async fn run(mut self) {
		// Capture the read version BEFORE opening the snapshot so the snapshot reflects every write
		// with commit_version <= read_version.
		let read_version = self.shared.read_version();

		let mut conn = match self.shared.pool.get().await {
			Ok(conn) => conn,
			Err(_) => {
				self.fail_receiver().await;
				return;
			}
		};

		let tx = match conn
			.build_transaction()
			.isolation_level(IsolationLevel::RepeatableRead)
			.read_only(true)
			.start()
			.await
		{
			Ok(tx) => tx,
			Err(_) => {
				self.fail_receiver().await;
				return;
			}
		};

		while let Some(cmd) = self.receiver.recv().await {
			match cmd {
				TransactionCommand::Get { key, response } => {
					let result = self.handle_get(&tx, &key).await;
					let _ = response.send(result);
				}
				TransactionCommand::GetKey {
					key,
					or_equal,
					offset,
					response,
				} => {
					let result = self.handle_get_key(&tx, &key, or_equal, offset).await;
					let _ = response.send(result);
				}
				TransactionCommand::GetRange {
					begin,
					begin_or_equal,
					begin_offset,
					end,
					end_or_equal,
					end_offset,
					limit,
					target_bytes,
					reverse,
					response,
				} => {
					let result = self
						.handle_get_range(
							&tx,
							begin,
							begin_or_equal,
							begin_offset,
							end,
							end_or_equal,
							end_offset,
							limit,
							target_bytes,
							reverse,
						)
						.await;
					let _ = response.send(result);
				}
				TransactionCommand::Commit {
					operations,
					conflict_ranges,
					response,
				} => {
					// The read snapshot is read-only, so end it and hand the pooled connection back
					// before awaiting the leader. Holding it across the submit lets parked commits
					// occupy every slot in the pool the leader drain loop draws from, so the commits
					// they are waiting on can never be applied.
					let _ = tx.commit().await;
					drop(conn);

					let result =
						commit::submit(&self.shared, read_version, operations, conflict_ranges)
							.await;
					let _ = response.send(result);
					return;
				}
				TransactionCommand::GetEstimatedRangeSize {
					begin,
					end,
					response,
				} => {
					let result = self
						.handle_get_estimated_range_size(&tx, &begin, &end)
						.await;
					let _ = response.send(result);
				}
			}
		}

		// If the channel is closed, the snapshot transaction is rolled back when dropped.
	}

	async fn handle_get(&mut self, tx: &Transaction<'_>, key: &[u8]) -> Result<Option<Slice>> {
		let query = "SELECT value FROM kv WHERE key = $1";
		let stmt = tx.prepare_cached(query).await.map_err(map_postgres_error)?;

		tx.query_opt(&stmt, &[&key])
			.await
			.map(|row| row.map(|r| r.get::<_, Vec<u8>>(0).into()))
			.map_err(map_postgres_error)
	}

	async fn handle_get_key(
		&mut self,
		tx: &Transaction<'_>,
		key: &[u8],
		or_equal: bool,
		offset: i32,
	) -> Result<Option<Slice>> {
		// Determine selector type and build appropriate query
		let query = match (or_equal, offset) {
			// first_greater_or_equal
			(false, 1) => "SELECT key FROM kv WHERE key >= $1 ORDER BY key LIMIT 1",
			// first_greater_than
			(true, 1) => "SELECT key FROM kv WHERE key > $1 ORDER BY key LIMIT 1",
			// last_less_than
			(false, 0) => "SELECT key FROM kv WHERE key < $1 ORDER BY key DESC LIMIT 1",
			// last_less_or_equal
			(true, 0) => "SELECT key FROM kv WHERE key <= $1 ORDER BY key DESC LIMIT 1",
			_ => bail!("invalid or_equal + offset combo"),
		};

		let stmt = tx.prepare_cached(query).await.map_err(map_postgres_error)?;

		tx.query_opt(&stmt, &[&key])
			.await
			.map(|row| row.map(|r| r.get::<_, Vec<u8>>(0).into()))
			.map_err(map_postgres_error)
	}

	/// Reads one page of a range.
	///
	/// The page ends at the row limit, or after the row that brings it to `target_bytes`, whichever
	/// comes first. It always holds at least one row when the range has any, so a row larger than the
	/// byte budget still makes progress.
	async fn handle_get_range(
		&mut self,
		tx: &Transaction<'_>,
		begin_key: Vec<u8>,
		begin_or_equal: bool,
		begin_offset: i32,
		end_key: Vec<u8>,
		end_or_equal: bool,
		end_offset: i32,
		limit: Option<usize>,
		target_bytes: Option<usize>,
		reverse: bool,
	) -> Result<Values> {
		// Determine SQL operators based on key selector types
		let begin_op = if begin_offset == 1 {
			if begin_or_equal { ">" } else { ">=" }
		} else {
			">="
		};

		let end_op = if end_offset == 1 {
			if end_or_equal { "<=" } else { "<" }
		} else {
			"<"
		};

		let order = if reverse { "DESC" } else { "ASC" };

		// The limit and the byte budget are bound as parameters rather than written into the SQL so
		// the statement cache holds one entry per query shape instead of one per value.
		let (rows, row_cap) = if let Some(target_bytes) = target_bytes {
			// SQL has no byte limit, so the page is cut with a running total. `bytes_before` is the
			// size of every row ahead of a row, which keeps the first row and the row that reaches the
			// budget. The inner row limit bounds how many rows past the budget are read only to be
			// left out.
			let widest_byte_rows = target_bytes / PAGE_MIN_ROW_BYTES;
			let byte_rows = match self.page_row_bytes {
				Some(row_bytes) => (target_bytes / row_bytes.max(PAGE_MIN_ROW_BYTES))
					.saturating_mul(PAGE_ROW_SLACK)
					.min(widest_byte_rows),
				None => widest_byte_rows,
			}
			.max(PAGE_MIN_ROWS);
			let row_cap = limit.map_or(byte_rows, |limit| limit.min(byte_rows));

			let query = format!(
				"SELECT key, value FROM (
					SELECT key, value, SUM(octet_length(key) + octet_length(value)) OVER (
						ORDER BY key {order} ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING
					) AS bytes_before
					FROM (
						SELECT key, value FROM kv
						WHERE key {begin_op} $1 AND key {end_op} $2
						ORDER BY key {order}
						LIMIT $3::bigint
					) AS candidates
				) AS page
				WHERE bytes_before IS NULL OR bytes_before < $4::bigint
				ORDER BY key {order}"
			);
			let stmt = tx
				.prepare_cached(&query)
				.await
				.map_err(map_postgres_error)?;

			let rows = tx
				.query(
					&stmt,
					&[
						&begin_key,
						&end_key,
						&sql_bigint(row_cap),
						&sql_bigint(target_bytes),
					],
				)
				.await
				.map_err(map_postgres_error)?;

			(rows, row_cap)
		} else {
			let row_cap = limit.unwrap_or(usize::MAX);

			let query = format!(
				"SELECT key, value FROM kv
				WHERE key {begin_op} $1 AND key {end_op} $2
				ORDER BY key {order}
				LIMIT $3::bigint"
			);
			let stmt = tx
				.prepare_cached(&query)
				.await
				.map_err(map_postgres_error)?;

			let rows = tx
				.query(&stmt, &[&begin_key, &end_key, &sql_bigint(row_cap)])
				.await
				.map_err(map_postgres_error)?;

			(rows, row_cap)
		};

		let mut results = Vec::with_capacity(rows.len());
		let mut results_bytes = 0usize;
		for row in rows {
			let key: Vec<u8> = row.get(0);
			let value: Vec<u8> = row.get(1);
			results_bytes = results_bytes.saturating_add(key.len() + value.len());
			results.push(KeyValue::new(key, value));
		}

		if target_bytes.is_some() && !results.is_empty() {
			self.page_row_bytes = Some(results_bytes / results.len());
		}

		// A page that filled its row limit or its byte budget may have stopped short of the end of
		// the range. Reporting more rows than there are only costs the caller one empty page.
		let more = !results.is_empty()
			&& (results.len() >= row_cap
				|| target_bytes.is_some_and(|target_bytes| results_bytes >= target_bytes));

		Ok(Values::with_more(results, more))
	}

	async fn handle_get_estimated_range_size(
		&mut self,
		tx: &Transaction<'_>,
		begin: &[u8],
		end: &[u8],
	) -> Result<i64> {
		// Sample 1% of the range.
		let query = "
			WITH range_stats AS (
				SELECT
					COUNT(*) as estimated_count,
					COALESCE(SUM(pg_column_size(key) + pg_column_size(value)), 0) as sample_size
				FROM kv TABLESAMPLE SYSTEM(1)
				WHERE key >= $1 AND key < $2
			),
			table_stats AS (
				SELECT reltuples::bigint as total_rows
				FROM pg_class
				WHERE relname = 'kv' AND relkind = 'r'
			)
			SELECT
				CASE
					WHEN r.estimated_count = 0 THEN 0
					ELSE (r.sample_size * 100)::bigint
				END as estimated_size
			FROM range_stats r, table_stats t";
		let stmt = tx.prepare_cached(query).await.map_err(map_postgres_error)?;

		tx.query_opt(&stmt, &[&begin, &end])
			.await
			.map(|row| row.map(|r| r.get::<_, i64>(0)).unwrap_or(0))
			.map_err(map_postgres_error)
	}

	async fn fail_receiver(&mut self) {
		while let Some(cmd) = self.receiver.recv().await {
			match cmd {
				TransactionCommand::Get { response, .. } => {
					let _ = response.send(Err(anyhow!("postgres transaction connection failed")));
				}
				TransactionCommand::GetKey { response, .. } => {
					let _ = response.send(Err(anyhow!("postgres transaction connection failed")));
				}
				TransactionCommand::GetRange { response, .. } => {
					let _ = response.send(Err(anyhow!("postgres transaction connection failed")));
				}
				TransactionCommand::Commit { response, .. } => {
					let _ = response.send(Err(anyhow!("postgres transaction connection failed")));
				}
				TransactionCommand::GetEstimatedRangeSize { response, .. } => {
					let _ = response.send(Err(anyhow!("postgres transaction connection failed")));
				}
			}
		}
	}
}

/// Converts a row or byte count to a SQL `bigint` parameter, saturating rather than wrapping.
fn sql_bigint(value: usize) -> i64 {
	i64::try_from(value).unwrap_or(i64::MAX)
}

/// Maps a PostgreSQL error from the read path to a `DatabaseError` where appropriate.
fn map_postgres_error(err: tokio_postgres::Error) -> anyhow::Error {
	let error_str = err.to_string();

	if error_str.contains("serialization failure")
		|| error_str.contains("could not serialize")
		|| error_str.contains("deadlock detected")
	{
		crate::error::DatabaseError::NotCommitted.into()
	} else if error_str.contains("current transaction is aborted") {
		crate::error::DatabaseError::NotCommitted.into()
	} else {
		tracing::error!(%err, "postgres error");
		anyhow::Error::new(err)
	}
}
