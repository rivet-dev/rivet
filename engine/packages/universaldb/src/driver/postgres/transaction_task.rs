use anyhow::{Context, Result, anyhow, bail};
use deadpool_postgres::{Pool, Transaction};
use tokio::sync::{mpsc, oneshot};
use tokio_postgres::IsolationLevel;

use crate::{
	atomic::apply_atomic_op,
	error::DatabaseError,
	options::ConflictRangeType,
	tx_ops::Operation,
	value::{KeyValue, Slice, Values},
	versionstamp::substitute_versionstamp_if_incomplete,
};

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

/// TransactionTask runs in a separate tokio task to manage a PostgreSQL transaction.
///
/// This design is necessary because PostgreSQL transactions have lifetime constraints
/// that don't work well with the FoundationDB-style API. Specifically:
/// - The transaction must outlive all references to it
/// - We can't store the transaction in a mutex due to lifetime issues with the connection
///
/// By running in a separate task and communicating via channels, we avoid these lifetime
/// issues while maintaining a single serializable transaction for all operations.
pub struct TransactionTask {
	pool: Pool,
	receiver: mpsc::Receiver<TransactionCommand>,
	unstable_disable_lock_customization: bool,
}

impl TransactionTask {
	pub fn new(
		pool: Pool,
		receiver: mpsc::Receiver<TransactionCommand>,
		unstable_disable_lock_customization: bool,
	) -> Self {
		Self {
			pool,
			receiver,
			unstable_disable_lock_customization,
		}
	}

	pub async fn run(mut self) {
		// Get connection from pool
		let mut conn = match self.pool.get().await {
			Ok(conn) => conn,
			Err(_) => {
				// If we can't get a connection, respond to all pending commands with errors
				self.fail_receiver().await;
				return;
			}
		};

		let tx = match conn
			.build_transaction()
			.isolation_level(IsolationLevel::RepeatableRead)
			.start()
			.await
		{
			Ok(tx) => tx,
			Err(_) => {
				// If we can't start a transaction, respond to all pending commands with errors
				self.fail_receiver().await;
				return;
			}
		};

		let start_version = match tx
			.query_one("SELECT nextval('global_version_seq')", &[])
			.await
		{
			Ok(row) => row.get::<_, i64>(0),
			Err(err) => {
				tracing::error!(?err, "failed to get postgres txn start_version");
				self.fail_receiver().await;
				return;
			}
		};

		// Process commands
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
					let result = self
						.handle_commit(tx, start_version, operations, conflict_ranges)
						.await;

					let _ = response.send(result);
					// Exit after commit
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

		// If the channel is closed, the transaction will be rolled back when dropped
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
		reverse: bool,
	) -> Result<Values> {
		// Determine SQL operators based on key selector types
		// For begin selector:
		// first_greater_or_equal: or_equal = false, offset = 1 -> ">="
		// first_greater_than: or_equal = true, offset = 1 -> ">"
		let begin_op = if begin_offset == 1 {
			if begin_or_equal { ">" } else { ">=" }
		} else {
			// This shouldn't happen for begin in range queries
			">="
		};

		// For end selector:
		// first_greater_than: or_equal = true, offset = 1 -> "<="
		// first_greater_or_equal: or_equal = false, offset = 1 -> "<"
		let end_op = if end_offset == 1 {
			if end_or_equal { "<=" } else { "<" }
		} else {
			// This shouldn't happen for end in range queries
			"<"
		};

		// Build query with CTE that adds conflict range
		let query = if reverse {
			if let Some(limit) = limit {
				format!(
					"SELECT key, value FROM kv WHERE key {begin_op} $1 AND key {end_op} $2 ORDER BY key DESC LIMIT {limit}"
				)
			} else {
				format!(
					"SELECT key, value FROM kv WHERE key {begin_op} $1 AND key {end_op} $2 ORDER BY key DESC"
				)
			}
		} else if let Some(limit) = limit {
			format!(
				"SELECT key, value FROM kv WHERE key {begin_op} $1 AND key {end_op} $2 ORDER BY key LIMIT {limit}"
			)
		} else {
			format!(
				"SELECT key, value FROM kv WHERE key {begin_op} $1 AND key {end_op} $2 ORDER BY key"
			)
		};

		let stmt = tx
			.prepare_cached(&query)
			.await
			.map_err(map_postgres_error)?;

		tx.query(&stmt, &[&begin_key, &end_key])
			.await
			.map(|rows| {
				rows.into_iter()
					.map(|row| {
						let key: Vec<u8> = row.get(0);
						let value: Vec<u8> = row.get(1);
						KeyValue::new(key, value)
					})
					.collect()
			})
			.map(Values::new)
			.map_err(map_postgres_error)
	}

	async fn handle_get_estimated_range_size(
		&mut self,
		tx: &Transaction<'_>,
		begin: &[u8],
		end: &[u8],
	) -> Result<i64> {
		// Sample's 1% of the range
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

	async fn handle_commit(
		&mut self,
		tx: Transaction<'_>,
		start_version: i64,
		operations: Vec<Operation>,
		conflict_ranges: Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>,
	) -> Result<()> {
		// // Defer all constraint checks until commit
		// tx.execute("SET CONSTRAINTS ALL DEFERRED", &[])
		// 	.await
		// 	.map_err(map_postgres_error)?;

		let version_res = if self.unstable_disable_lock_customization {
			// Don't customize lock settings - just get the version
			tx.query_one("SELECT nextval('global_version_seq')", &[])
				.await
		} else {
			// Apply lock customization (default behavior)
			let (_, _, version_res) = tokio::join!(
				tx.execute("SET LOCAL lock_timeout = '0'", &[],),
				tx.execute("SET LOCAL deadlock_timeout = '10ms'", &[],),
				tx.query_one("SELECT nextval('global_version_seq')", &[]),
			);

			version_res
		};

		let commit_version = version_res
			.context("failed to get postgres txn commit_version")?
			.get::<_, i64>(0);

		let mut begins = Vec::with_capacity(conflict_ranges.len());
		let mut ends = Vec::with_capacity(conflict_ranges.len());
		let mut conflict_types = Vec::with_capacity(conflict_ranges.len());

		for (begin, end, conflict_type) in conflict_ranges {
			let conflict_type = match conflict_type {
				ConflictRangeType::Read => "read",
				ConflictRangeType::Write => "write",
			};

			begins.push(begin);
			ends.push(end);
			conflict_types.push(conflict_type);
		}

		let query = "
			INSERT INTO conflict_ranges (range_data, conflict_type, start_version, commit_version)
			SELECT
				bytearange(begin_key, end_key, '[)'),
				conflict_type::range_type,
				$4,
				$5
			FROM UNNEST($1::bytea[], $2::bytea[], $3::text[]) AS t(begin_key, end_key, conflict_type)";
		let stmt = tx.prepare_cached(query).await.map_err(map_postgres_error)?;

		// Insert all conflict ranges at once
		tx.execute(
			&stmt,
			&[
				&begins,
				&ends,
				&conflict_types,
				&start_version,
				&commit_version,
			],
		)
		.await
		.map_err(map_postgres_error)?;

		for op in operations {
			match op {
				Operation::Set { key, value } => {
					// TODO: versionstamps need to be calculated on the sql side, not in rust
					let value = substitute_versionstamp_if_incomplete(value.clone(), 0);

					// // Poor man's upsert, you cant use ON CONFLICT with deferred constraints
					// let query = "WITH updated AS (
					// 		UPDATE kv
					// 		SET value = $2
					// 		WHERE key = $1
					// 		RETURNING 1
					// 	)
					// 	INSERT INTO kv (key, value)
					// 	SELECT $1, $2
					// 	WHERE NOT EXISTS (SELECT 1 FROM updated)";
					let query = "INSERT INTO kv (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = $2";
					let stmt = tx.prepare_cached(query).await.map_err(map_postgres_error)?;

					tx.execute(&stmt, &[&key, &value])
						.await
						.map_err(map_postgres_error)?;
				}
				Operation::Clear { key } => {
					let query = "DELETE FROM kv WHERE key = $1";
					let stmt = tx.prepare_cached(query).await.map_err(map_postgres_error)?;

					tx.execute(&stmt, &[&key])
						.await
						.map_err(map_postgres_error)?;
				}
				Operation::ClearRange { begin, end } => {
					let query = "DELETE FROM kv WHERE key >= $1 AND key < $2";
					let stmt = tx.prepare_cached(query).await.map_err(map_postgres_error)?;

					tx.execute(&stmt, &[&begin, &end])
						.await
						.map_err(map_postgres_error)?;
				}
				Operation::AtomicOp {
					key,
					param,
					op_type,
				} => {
					// TODO: All operations need to be done on the sql side, not in rust

					// Get current value from database
					let current_query = "SELECT value FROM kv WHERE key = $1";
					let stmt = tx
						.prepare_cached(current_query)
						.await
						.map_err(map_postgres_error)?;

					let current_row = tx
						.query_opt(&stmt, &[&key])
						.await
						.map_err(map_postgres_error)?;

					// Extract current value or use None if key doesn't exist
					let current_value = current_row.map(|row| row.get::<_, Vec<u8>>(0));
					let current_slice = current_value.as_deref();

					// Apply atomic operation
					let new_value = apply_atomic_op(current_slice, &param, op_type);

					// Store the result
					if let Some(new_value) = new_value {
						// // Poor man's upsert, you cant use ON CONFLICT with deferred constraints
						// let update_query = "WITH updated AS (
						// 		UPDATE kv
						// 		SET value = $2
						// 		WHERE key = $1
						// 		RETURNING 1
						// 	)
						// 	INSERT INTO kv (key, value)
						// 	SELECT $1, $2
						// 	WHERE NOT EXISTS (SELECT 1 FROM updated)";
						let update_query = "INSERT INTO kv (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = $2";
						let stmt = tx
							.prepare_cached(update_query)
							.await
							.map_err(map_postgres_error)?;

						tx.execute(&stmt, &[&key, &new_value])
							.await
							.map_err(map_postgres_error)?;
					} else {
						let update_query = "DELETE FROM kv WHERE key = $1";
						let stmt = tx
							.prepare_cached(update_query)
							.await
							.map_err(map_postgres_error)?;

						tx.execute(&stmt, &[&key])
							.await
							.map_err(map_postgres_error)?;
					}
				}
			}
		}

		tx.commit().await.map_err(map_postgres_error)
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

/// Maps PostgreSQL error to DatabaseError
fn map_postgres_error(err: tokio_postgres::Error) -> anyhow::Error {
	let error_str = err.to_string();
	if error_str.contains("exclusion_violation")
		|| error_str.contains("violates exclusion constraint")
	{
		// Retryable - another transaction has a conflicting range
		DatabaseError::NotCommitted.into()
	} else if error_str.contains("serialization failure")
		|| error_str.contains("could not serialize")
		|| error_str.contains("deadlock detected")
	{
		// Retryable - transaction conflict
		DatabaseError::NotCommitted.into()
	} else if error_str.contains("current transaction is aborted") {
		// Returned by the rest of the commands in a txn if it failed for exclusion reasons
		DatabaseError::NotCommitted.into()
	} else {
		tracing::error!(%err, "postgres error");
		// Non-retryable error
		anyhow::Error::new(err)
	}
}
