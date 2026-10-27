//! Synchronous SQLite calls for foreign runtimes whose host thread runs user code.
//!
//! A synchronous call blocks its caller, so it only runs where writes never wait on the network:
//! - The actor must opt into deferred commits. A write then commits locally and returns. A cold
//!   page read, or a write while storage is far behind, can still wait on depot.
//! - A transaction owned by host-thread code can only finish on that thread. A synchronous call
//!   fails fast with `sqlite.writer_busy` while one is open or waiting for the writer. Internal
//!   transactions, such as state saves, never need the host thread, so a synchronous call waits
//!   for them.

use std::future::Future;
use std::time::Duration;

use anyhow::Result;

use super::{BindParam, ExecuteResult, QueryResult, SqliteDb, SqliteTransaction};
use crate::error::SqliteRuntimeError;

impl SqliteDb {
	/// Returns a handle for code that runs on a foreign runtime's host thread. Transactions
	/// begun through it count as host-thread transactions, which synchronous calls never wait
	/// for.
	pub fn for_host_thread(&self) -> Self {
		let mut db = self.clone();
		db.host_thread = true;
		db
	}

	pub fn execute_sync(
		&self,
		sql: impl Into<String>,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		self.ensure_sync_supported()?;
		self.ensure_no_host_thread_transaction()?;
		let db = self.clone();
		let sql = sql.into();
		wait_on_runtime(async move { db.execute(sql, params).await })
	}

	pub fn exec_sync(&self, sql: impl Into<String>) -> Result<QueryResult> {
		self.ensure_sync_supported()?;
		self.ensure_no_host_thread_transaction()?;
		let db = self.clone();
		let sql = sql.into();
		wait_on_runtime(async move { db.exec(sql).await })
	}

	/// Begins a host-thread transaction. The caller must finish it with `commit_sync` or
	/// `rollback_sync`, or it rolls back at its timeout.
	pub fn begin_transaction_sync(
		&self,
		name: Option<String>,
		timeout: Option<Duration>,
	) -> Result<SqliteTransaction> {
		self.ensure_sync_supported()?;
		self.ensure_no_host_thread_transaction()?;
		let db = self.for_host_thread();
		wait_on_runtime(async move { db.begin_named_transaction(name.as_deref(), timeout).await })
	}

	/// Returns the ticket of the latest local commit in this generation. Pass it to
	/// `when_durable` to wait until that commit and every earlier one reach depot.
	pub fn commit_ticket(&self) -> u64 {
		#[cfg(feature = "sqlite-local")]
		{
			self.db.lock().as_ref().map_or(
				0,
				sqlite_embedded::database::NativeDatabaseHandle::commit_seq,
			)
		}
		#[cfg(not(feature = "sqlite-local"))]
		{
			0
		}
	}

	/// Resolves once the commit `ticket` names is durable. Rejects with
	/// `sqlite.durability_failed` once the generation's database broke, even for a ticket that
	/// was already durable, because the generation is stopping.
	pub async fn when_durable(&self, ticket: u64) -> Result<()> {
		#[cfg(feature = "sqlite-local")]
		{
			use sqlite_embedded::vfs::FlushError;

			let native_db = match self.db.lock().as_ref() {
				Some(native_db) => native_db.clone(),
				None if ticket == 0 => return Ok(()),
				None => {
					return Err(
						SqliteRuntimeError::InvalidCommitTicket { ticket, current: 0 }.build(),
					);
				}
			};
			native_db
				.wait_for_flush(ticket)
				.await
				.map_err(|error| match error {
					FlushError::InvalidSequence { requested, current } => {
						SqliteRuntimeError::InvalidCommitTicket {
							ticket: requested,
							current,
						}
						.build()
					}
					FlushError::RetryDeadlineExceeded { .. }
					| FlushError::HeadDiverged { .. }
					| FlushError::Aborted(_) => SqliteRuntimeError::DurabilityFailed {
						message: error.to_string(),
					}
					.build(),
				})
		}
		#[cfg(not(feature = "sqlite-local"))]
		{
			let _ = ticket;
			Err(sync_unsupported("this build has no local SQLite"))
		}
	}

	pub(super) fn ensure_sync_supported(&self) -> Result<()> {
		#[cfg(feature = "sqlite-local")]
		{
			if self.backend() != super::SqliteBackend::LocalNative {
				return Err(sync_unsupported("the actor uses remote SQLite"));
			}
			let Some(native_db) = self.db.lock().as_ref().cloned() else {
				return Err(sync_unsupported(
					"the database is not open; open it with an asynchronous call first",
				));
			};
			if native_db.commit_mode() != sqlite_embedded::vfs::CommitMode::Deferred {
				return Err(sync_unsupported(
					"the actor did not opt into deferred commits, so a write would wait on the network",
				));
			}
			// A runtime task that blocks on another runtime task can starve the runtime.
			if tokio::task::try_id().is_some() {
				return Err(sync_unsupported(
					"the caller is an asynchronous runtime task",
				));
			}
			Ok(())
		}
		#[cfg(not(feature = "sqlite-local"))]
		{
			Err(sync_unsupported("this build has no local SQLite"))
		}
	}

	fn ensure_no_host_thread_transaction(&self) -> Result<()> {
		if self.transaction_coordinator.host_thread_lease_count() > 0 {
			return Err(SqliteRuntimeError::WriterBusy.build());
		}
		Ok(())
	}
}

impl SqliteTransaction {
	pub fn execute_sync(
		&self,
		sql: impl Into<String>,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		self.db.ensure_sync_supported()?;
		let transaction = self.clone();
		let sql = sql.into();
		wait_on_runtime(async move { transaction.execute(sql, params).await })
	}

	pub fn exec_sync(&self, sql: impl Into<String>) -> Result<QueryResult> {
		self.db.ensure_sync_supported()?;
		let transaction = self.clone();
		let sql = sql.into();
		wait_on_runtime(async move { transaction.exec(sql).await })
	}

	pub fn commit_sync(&self) -> Result<()> {
		self.db.ensure_sync_supported()?;
		let transaction = self.clone();
		wait_on_runtime(async move { transaction.commit().await })
	}

	pub fn rollback_sync(&self) -> Result<()> {
		self.db.ensure_sync_supported()?;
		let transaction = self.clone();
		wait_on_runtime(async move { transaction.rollback().await })
	}
}

/// Drives `future` to completion on the calling thread. The caller must be a thread that entered
/// the runtime, not one of its tasks, because blocking a runtime task could stall the work the
/// future waits on.
#[cfg(feature = "sqlite-local")]
fn wait_on_runtime<T, F>(future: F) -> Result<T>
where
	F: Future<Output = Result<T>>,
{
	if tokio::task::try_id().is_some() {
		return Err(sync_unsupported(
			"the caller is an asynchronous runtime task",
		));
	}
	let runtime = tokio::runtime::Handle::try_current()
		.map_err(|_| sync_unsupported("the calling thread has no runtime"))?;
	runtime.block_on(future)
}

#[cfg(not(feature = "sqlite-local"))]
fn wait_on_runtime<T, F>(_future: F) -> Result<T>
where
	F: Future<Output = Result<T>>,
{
	Err(sync_unsupported("this build has no local SQLite"))
}

fn sync_unsupported(reason: &str) -> anyhow::Error {
	SqliteRuntimeError::SyncUnsupported {
		reason: reason.to_owned(),
	}
	.build()
}
