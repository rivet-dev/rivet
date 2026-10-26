//! Holds a new database's bootstrap statements back so they commit with its first transaction.
//!
//! A fresh actor's internal schema is not needed until the first SQL statement runs. Staging it
//! here lets the first transaction, normally the actor's first state save, carry the schema and
//! its own writes in one commit. Every other statement first commits the staged schema on its
//! own, so nothing ever runs against a database that is missing it.

use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use parking_lot::Mutex;

use super::{ExecuteResult, SqliteBatchStatement, SqliteDb};
use crate::actor::internal_storage::schema::HAS_INTERNAL_SCHEMA_SQL;

#[derive(Default)]
pub(super) struct DeferredBootstrap {
	/// Forced-sync: read and replaced without awaiting. The exclusive transaction gate
	/// serializes the transactions that apply the statements.
	statements: Mutex<Option<Vec<SqliteBatchStatement>>>,
	/// Whether any statements are staged, so statements skip the lock once they commit.
	staged: AtomicBool,
	/// Set when a transaction starts applying the statements. A carrier that ends without a
	/// confirmed commit may still have applied them, for example when its commit reply is lost,
	/// so the next carrier checks for the schema before applying them again.
	verify: AtomicBool,
	/// Forced-sync: the key of the transaction applying the statements.
	carrier: Mutex<Option<String>>,
}

impl SqliteDb {
	/// Stages `statements` to commit with this database's first transaction.
	pub(crate) fn defer_bootstrap(&self, statements: Vec<SqliteBatchStatement>) {
		*self.deferred_bootstrap.statements.lock() = Some(statements);
		self.deferred_bootstrap
			.verify
			.store(false, Ordering::Release);
		self.deferred_bootstrap
			.staged
			.store(true, Ordering::Release);
	}

	/// Whether staged bootstrap statements have not committed yet.
	pub(crate) fn has_deferred_bootstrap(&self) -> bool {
		self.deferred_bootstrap.staged.load(Ordering::Acquire)
	}

	/// Commits the staged bootstrap statements in a transaction of their own. Statements that
	/// run outside a transaction call this first.
	pub(super) async fn commit_deferred_bootstrap(&self) -> Result<()> {
		if !self.has_deferred_bootstrap() {
			return Ok(());
		}
		let transaction = self
			.begin_transaction(None)
			.await
			.context("begin deferred sqlite bootstrap")?;
		transaction
			.commit()
			.await
			.context("commit deferred sqlite bootstrap")
	}

	/// Applies the staged bootstrap statements as the first statements of the transaction
	/// `key`, which has just begun. Rolls the transaction back if one fails.
	pub(super) async fn apply_deferred_bootstrap(&self, key: &str) -> Result<()> {
		if !self.has_deferred_bootstrap() {
			return Ok(());
		}
		let Some(statements) = self.deferred_bootstrap.statements.lock().clone() else {
			return Ok(());
		};

		let result = self.apply_bootstrap_statements(key, statements).await;
		if let Err(error) = result {
			if let Err(rollback_error) = self.finish_transaction_inner(key, false).await {
				tracing::warn!(
					?rollback_error,
					"rollback after a failed deferred sqlite bootstrap failed; the transaction was likely already released"
				);
			}
			return Err(error.context("apply deferred sqlite bootstrap"));
		}
		Ok(())
	}

	async fn apply_bootstrap_statements(
		&self,
		key: &str,
		statements: Vec<SqliteBatchStatement>,
	) -> Result<()> {
		if self.deferred_bootstrap.verify.load(Ordering::Acquire) {
			let probe = self
				.transaction_execute_inner(key, HAS_INTERNAL_SCHEMA_SQL.to_owned(), None)
				.await
				.context("check for a schema an earlier bootstrap may have committed")?;
			if !probe.rows.is_empty() {
				self.clear_deferred_bootstrap();
				return Ok(());
			}
		}

		self.deferred_bootstrap
			.verify
			.store(true, Ordering::Release);
		*self.deferred_bootstrap.carrier.lock() = Some(key.to_owned());
		self.transaction_execute_statements_inner(key, statements)
			.await
	}

	/// Runs a remote batch, which the remote side applies as one transaction. A staged bootstrap
	/// rides ahead of the batch's own statements unless an earlier attempt may have applied it,
	/// in which case it commits first after checking for the schema.
	pub(super) async fn remote_execute_batch_with_bootstrap(
		&self,
		statements: Vec<SqliteBatchStatement>,
	) -> Result<Vec<ExecuteResult>> {
		let bootstrap = self.deferred_bootstrap_for_batch();
		let Some(bootstrap) = bootstrap else {
			let _guard = self.begin_regular_operation().await?;
			return self.remote_execute_batch(statements).await;
		};

		let _guard = self.begin_admitted_operation().await?;
		// A transaction may have committed the statements while this batch waited for the
		// gate. None can be applying them now, because that needs the gate exclusively.
		if !self.has_deferred_bootstrap() {
			return self.remote_execute_batch(statements).await;
		}
		let carried = bootstrap.len();
		let mut combined = bootstrap;
		combined.extend(statements);
		let mut results = self.remote_execute_batch(combined).await?;
		self.clear_deferred_bootstrap();
		Ok(results.split_off(carried))
	}

	/// The staged statements for a remote batch to carry, or `None` when nothing is staged or
	/// an earlier attempt may already have applied them.
	fn deferred_bootstrap_for_batch(&self) -> Option<Vec<SqliteBatchStatement>> {
		if !self.has_deferred_bootstrap() || self.deferred_bootstrap.verify.load(Ordering::Acquire)
		{
			return None;
		}
		let statements = self.deferred_bootstrap.statements.lock().clone()?;
		self.deferred_bootstrap
			.verify
			.store(true, Ordering::Release);
		Some(statements)
	}

	/// Records that the transaction `key` committed. Clears the staged statements if it
	/// carried them.
	pub(super) fn deferred_bootstrap_committed(&self, key: &str) {
		let carried = self.deferred_bootstrap.carrier.lock().as_deref() == Some(key);
		if carried {
			self.clear_deferred_bootstrap();
		}
	}

	fn clear_deferred_bootstrap(&self) {
		self.deferred_bootstrap
			.staged
			.store(false, Ordering::Release);
		self.deferred_bootstrap.statements.lock().take();
		self.deferred_bootstrap.carrier.lock().take();
		self.deferred_bootstrap
			.verify
			.store(false, Ordering::Release);
	}
}
