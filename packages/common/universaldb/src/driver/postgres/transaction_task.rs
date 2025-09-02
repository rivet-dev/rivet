use deadpool_postgres::Pool;
use tokio::sync::{mpsc, oneshot};
use tokio_postgres::IsolationLevel;

use crate::{
	FdbError, FdbResult,
	atomic::apply_atomic_op,
	options::{ConflictRangeType, MutationType},
	versionstamp::substitute_versionstamp_if_incomplete,
};

#[derive(Debug, Clone, Copy)]
pub enum TransactionIsolationLevel {
	Serializable,
	RepeatableReadReadOnly,
}

#[derive(Debug)]
pub enum TransactionCommand {
	// Read operations
	Get {
		key: Vec<u8>,
		response: oneshot::Sender<FdbResult<Option<Vec<u8>>>>,
	},
	GetKey {
		key: Vec<u8>,
		or_equal: bool,
		offset: i32,
		response: oneshot::Sender<FdbResult<Option<Vec<u8>>>>,
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
		response: oneshot::Sender<FdbResult<Vec<(Vec<u8>, Vec<u8>)>>>,
	},
	// Write operations
	Set {
		key: Vec<u8>,
		value: Vec<u8>,
		response: oneshot::Sender<FdbResult<()>>,
	},
	Clear {
		key: Vec<u8>,
		response: oneshot::Sender<FdbResult<()>>,
	},
	ClearRange {
		begin: Vec<u8>,
		end: Vec<u8>,
		response: oneshot::Sender<FdbResult<()>>,
	},
	AtomicOp {
		key: Vec<u8>,
		param: Vec<u8>,
		op_type: MutationType,
		response: oneshot::Sender<FdbResult<()>>,
	},
	// Transaction control
	Commit {
		has_conflict_ranges: bool,
		response: oneshot::Sender<FdbResult<()>>,
	},
	// Conflict ranges
	AddConflictRange {
		begin: Vec<u8>,
		end: Vec<u8>,
		conflict_type: ConflictRangeType,
		response: oneshot::Sender<FdbResult<()>>,
	},
	GetEstimatedRangeSize {
		begin: Vec<u8>,
		end: Vec<u8>,
		response: oneshot::Sender<FdbResult<i64>>,
	},
}

/// TransactionTask runs in a separate tokio task to manage a PostgreSQL transaction.
///
/// This design is necessary because PostgreSQL transactions have lifetime constraints
/// that don't work well with the FoundationDB-style API. Specifically:
/// - The transaction must outlive all references to it
/// - We can't store the transaction in a mutex due to lifetime issues with the connection
/// - The synchronous `set`/`clear` methods in the TransactionDriver trait can't await
///
/// By running in a separate task and communicating via channels, we avoid these lifetime
/// issues while maintaining a single serializable transaction for all operations.
pub struct TransactionTask {
	pool: Pool,
	receiver: mpsc::Receiver<TransactionCommand>,
	isolation_level: TransactionIsolationLevel,
}

impl TransactionTask {
	pub fn new(
		pool: Pool,
		receiver: mpsc::Receiver<TransactionCommand>,
		isolation_level: TransactionIsolationLevel,
	) -> Self {
		Self {
			pool,
			receiver,
			isolation_level,
		}
	}

	pub async fn run(mut self) {
		// Get connection from pool
		let mut conn = match self.pool.get().await {
			Ok(conn) => conn,
			Err(_) => {
				// If we can't get a connection, respond to all pending commands with errors
				while let Some(cmd) = self.receiver.recv().await {
					match cmd {
						TransactionCommand::Get { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::GetKey { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::GetRange { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::Set { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::Clear { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::ClearRange { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::AtomicOp { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::Commit { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::AddConflictRange { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::GetEstimatedRangeSize { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
					}
				}
				return;
			}
		};

		// Start transaction with appropriate isolation level
		let tx = match self.isolation_level {
			TransactionIsolationLevel::Serializable => {
				conn.build_transaction()
					.isolation_level(IsolationLevel::Serializable)
					.start()
					.await
			}
			TransactionIsolationLevel::RepeatableReadReadOnly => {
				conn.build_transaction()
					.isolation_level(IsolationLevel::RepeatableRead)
					.read_only(true)
					.start()
					.await
			}
		};

		let tx = match tx {
			Ok(tx) => tx,
			Err(_) => {
				// If we can't start a transaction, respond to all pending commands with errors
				while let Some(cmd) = self.receiver.recv().await {
					match cmd {
						TransactionCommand::Get { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::GetKey { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::GetRange { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::Set { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::Clear { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::ClearRange { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::AtomicOp { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::Commit { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::AddConflictRange { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
						TransactionCommand::GetEstimatedRangeSize { response, .. } => {
							let _ = response.send(Err(FdbError::from_code(1510)));
						}
					}
				}
				return;
			}
		};

		// Set lock timeout to 0 for serializable transactions
		// This ensures conflict range acquisition fails immediately if there's a conflict
		if let TransactionIsolationLevel::Serializable = self.isolation_level {
			if let Err(_) = tx.execute("SET LOCAL lock_timeout = '0'", &[]).await {
				// If we can't set lock timeout, continue anyway
			}
		}

		// Process commands
		while let Some(cmd) = self.receiver.recv().await {
			match cmd {
				TransactionCommand::Get { key, response } => {
					let query = "SELECT value FROM kv WHERE key = $1";
					let result = match tx.prepare_cached(query).await {
						Ok(stmt) => tx
							.query_opt(&stmt, &[&key])
							.await
							.map_err(map_postgres_error)
							.map(|row| row.map(|r| r.get::<_, Vec<u8>>(0))),
						Err(e) => Err(map_postgres_error(e)),
					};

					let _ = response.send(result);
				}
				TransactionCommand::GetKey {
					key,
					or_equal,
					offset,
					response,
				} => {
					// Determine selector type and build appropriate query
					let query = match (or_equal, offset) {
						(false, 1) => {
							// first_greater_or_equal
							"SELECT key FROM kv WHERE key >= $1 ORDER BY key LIMIT 1"
						}
						(true, 1) => {
							// first_greater_than
							"SELECT key FROM kv WHERE key > $1 ORDER BY key LIMIT 1"
						}
						(false, 0) => {
							// last_less_than
							"SELECT key FROM kv WHERE key < $1 ORDER BY key DESC LIMIT 1"
						}
						(true, 0) => {
							// last_less_or_equal
							"SELECT key FROM kv WHERE key <= $1 ORDER BY key DESC LIMIT 1"
						}
						_ => {
							// For other offset values, we need more complex logic
							// This is a simplified fallback that may not handle all cases perfectly
							let _ = response.send(Err(FdbError::from_code(1510)));
							continue;
						}
					};

					let result = match tx.prepare_cached(query).await {
						Ok(stmt) => tx
							.query_opt(&stmt, &[&key])
							.await
							.map_err(map_postgres_error)
							.map(|row| row.map(|r| r.get::<_, Vec<u8>>(0))),
						Err(e) => Err(map_postgres_error(e)),
					};

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
					let base_select = if reverse {
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

					let query = match self.isolation_level {
						TransactionIsolationLevel::Serializable => {
							// Use CTE to atomically add conflict range and read data
							format!(
								"WITH conflict_range AS (
									INSERT INTO conflict_ranges (range_data, conflict_type) 
									VALUES (bytearange($1, $2, '[)'), 'read')
									ON CONFLICT DO NOTHING
								)
								{base_select}"
							)
						}
						TransactionIsolationLevel::RepeatableReadReadOnly => base_select,
					};

					let result = match tx.prepare_cached(&query).await {
						Ok(stmt) => tx
							.query(&stmt, &[&begin, &end])
							.await
							.map_err(map_postgres_error)
							.map(|rows| {
								rows.into_iter()
									.map(|row| {
										let key: Vec<u8> = row.get(0);
										let value: Vec<u8> = row.get(1);
										(key, value)
									})
									.collect()
							}),
						Err(e) => Err(map_postgres_error(e)),
					};

					let _ = response.send(result);
				}
				TransactionCommand::Set {
					key,
					value,
					response,
				} => {
					if let TransactionIsolationLevel::RepeatableReadReadOnly = self.isolation_level
					{
						tracing::error!("cannot set in read only txn");
						let _ = response.send(Err(FdbError::from_code(1510)));
						continue;
					};

					let value = substitute_versionstamp_if_incomplete(value, 0);

					let query = "INSERT INTO kv (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = $2";
					let result = match tx.prepare_cached(query).await {
						Ok(stmt) => tx
							.execute(&stmt, &[&key, &value])
							.await
							.map_err(map_postgres_error)
							.map(|_| ()),
						Err(e) => Err(map_postgres_error(e)),
					};

					let _ = response.send(result);
				}
				TransactionCommand::Clear { key, response } => {
					if let TransactionIsolationLevel::RepeatableReadReadOnly = self.isolation_level
					{
						tracing::error!("cannot set in read only txn");
						let _ = response.send(Err(FdbError::from_code(1510)));
						continue;
					};

					let query = "DELETE FROM kv WHERE key = $1";
					let result = match tx.prepare_cached(query).await {
						Ok(stmt) => tx
							.execute(&stmt, &[&key])
							.await
							.map_err(map_postgres_error)
							.map(|_| ()),
						Err(e) => Err(map_postgres_error(e)),
					};

					let _ = response.send(result);
				}
				TransactionCommand::ClearRange {
					begin,
					end,
					response,
				} => {
					if let TransactionIsolationLevel::RepeatableReadReadOnly = self.isolation_level
					{
						tracing::error!("cannot clear range in read only txn");
						let _ = response.send(Err(FdbError::from_code(1510)));
						continue;
					};

					// No conversion needed - we'll use bytea ranges directly

					// Use CTE to atomically add conflict range and delete data
					let query = "WITH conflict_range AS (
						INSERT INTO conflict_ranges (range_data, conflict_type) 
						VALUES (bytearange($1, $2, '[)'), 'write')
						ON CONFLICT DO NOTHING
					)
					DELETE FROM kv WHERE key >= $1 AND key < $2";

					let result = match tx.prepare_cached(query).await {
						Ok(stmt) => tx
							.execute(&stmt, &[&begin, &end])
							.await
							.map_err(map_postgres_error)
							.map(|_| ()),
						Err(e) => Err(map_postgres_error(e)),
					};

					let _ = response.send(result);
				}
				TransactionCommand::AtomicOp {
					key,
					param,
					op_type,
					response,
				} => {
					if let TransactionIsolationLevel::RepeatableReadReadOnly = self.isolation_level
					{
						tracing::error!("cannot apply atomic op in read only txn");
						let _ = response.send(Err(FdbError::from_code(1510)));
						continue;
					};

					// Get current value from database
					let current_query = "SELECT value FROM kv WHERE key = $1";
					let current_result = match tx.prepare_cached(current_query).await {
						Ok(stmt) => tx
							.query_opt(&stmt, &[&key])
							.await
							.map_err(map_postgres_error),
						Err(e) => Err(map_postgres_error(e)),
					};

					let result = match current_result {
						Ok(current_row) => {
							// Extract current value or use None if key doesn't exist
							let current_value = current_row.map(|row| row.get::<_, Vec<u8>>(0));
							let current_slice = current_value.as_deref();

							// Apply atomic operation
							let new_value = apply_atomic_op(current_slice, &param, op_type);

							// Store the result
							if let Some(new_value) = new_value {
								let update_query = "INSERT INTO kv (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = $2";
								match tx.prepare_cached(update_query).await {
									Ok(stmt) => tx
										.execute(&stmt, &[&key, &new_value])
										.await
										.map_err(map_postgres_error)
										.map(|_| ()),
									Err(e) => Err(map_postgres_error(e)),
								}
							} else {
								let update_query = "DELETE FROM kv WHERE key = $1";
								match tx.prepare_cached(update_query).await {
									Ok(stmt) => tx
										.execute(&stmt, &[&key])
										.await
										.map_err(map_postgres_error)
										.map(|_| ()),
									Err(e) => Err(map_postgres_error(e)),
								}
							}
						}
						Err(e) => Err(e),
					};

					let _ = response.send(result);
				}
				TransactionCommand::Commit {
					has_conflict_ranges,
					response,
				} => {
					if has_conflict_ranges {
						if let TransactionIsolationLevel::RepeatableReadReadOnly =
							self.isolation_level
						{
							tracing::error!("cannot release conflict ranges in read only txn");
							let _ = response.send(Err(FdbError::from_code(1510)));
							continue;
						};

						// Release all conflict ranges in a single query
						let query = "DELETE FROM conflict_ranges WHERE txn_id = txid_current()";

						if let Err(err) = tx.execute(query, &[]).await.map_err(map_postgres_error) {
							let _ = response.send(Err(err));
							return;
						}
					}

					let result = tx.commit().await.map_err(map_postgres_error);
					let _ = response.send(result);
					// Exit after commit
					return;
				}
				TransactionCommand::AddConflictRange {
					begin,
					end,
					conflict_type,
					response,
				} => {
					if let TransactionIsolationLevel::RepeatableReadReadOnly = self.isolation_level
					{
						tracing::error!("cannot add conflict range in read only txn");
						let _ = response.send(Err(FdbError::from_code(1510)));
						continue;
					};

					let conflict_type = match conflict_type {
						ConflictRangeType::Read => "read",
						ConflictRangeType::Write => "write",
					};

					// Try to add the conflict range
					let result = tx
						.execute(
							"INSERT INTO conflict_ranges (range_data, conflict_type) VALUES (bytearange($1, $2, '[)'), $3::text::range_type)",
							&[&begin, &end, &conflict_type],
						)
						.await
						.map_err(map_postgres_error)
						.map(|_| ());

					let _ = response.send(result);
				}
				TransactionCommand::GetEstimatedRangeSize {
					begin,
					end,
					response,
				} => {
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

					let result = match tx.prepare_cached(query).await {
						Ok(stmt) => match tx.query_opt(&stmt, &[&begin, &end]).await {
							Ok(Some(row)) => Ok(row.get::<_, i64>(0)),
							Ok(None) => Ok(0),
							Err(e) => Err(map_postgres_error(e)),
						},
						Err(e) => Err(map_postgres_error(e)),
					};

					let _ = response.send(result);
				}
			}
		}

		// If the channel is closed, the transaction will be rolled back when dropped
	}
}

/// Maps PostgreSQL errors to FdbError codes
fn map_postgres_error(err: tokio_postgres::Error) -> FdbError {
	let error_str = err.to_string();
	if error_str.contains("exclusion_violation")
		|| error_str.contains("violates exclusion constraint")
	{
		// Retryable - another transaction has a conflicting range
		FdbError::from_code(1020)
	} else if error_str.contains("serialization failure")
		|| error_str.contains("could not serialize")
		|| error_str.contains("deadlock detected")
	{
		// Retryable - transaction conflict
		FdbError::from_code(1020)
	} else if error_str.contains("current transaction is aborted") {
		// Returned by the rest of the commands in a txn if it failed for exclusion reasons
		FdbError::from_code(1020)
	} else {
		tracing::error!(%err, "postgres error");
		// Non-retryable error
		FdbError::from_code(1510)
	}
}
