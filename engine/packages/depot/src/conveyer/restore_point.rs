//! Restore point lifecycle helpers for the depot conveyer.

mod pinned;
mod recompute;
mod resolve;
mod restore;
mod shared;
#[cfg(debug_assertions)]
pub mod test_hooks;
#[cfg(not(debug_assertions))]
mod test_hooks;

use anyhow::Result;

use super::{
	Db,
	error::SqliteStorageError,
	metrics,
	types::{
		PinStatus, ResolvedRestoreTarget, ResolvedVersionstamp, RestorePointId, SnapshotSelector,
	},
};

pub use pinned::{create_restore_point, delete_restore_point, restore_point_status};
pub use resolve::{resolve_restore_point, resolve_restore_target};
pub use restore::restore_database;

impl Db {
	pub async fn create_restore_point(&self, selector: SnapshotSelector) -> Result<RestorePointId> {
		let node_id = self.node_id.to_string();
		let result = create_restore_point(
			&self.udb,
			self.sqlite_bucket_id(),
			self.database_id.clone(),
			selector,
		)
		.await;
		metrics::SQLITE_RESTORE_POINT_CREATE_TOTAL
			.with_label_values(&[
				node_id.as_str(),
				restore_point_create_outcome(result.as_ref().err()),
			])
			.inc();
		result
	}

	pub async fn restore_point_status(
		&self,
		restore_point: RestorePointId,
	) -> Result<Option<PinStatus>> {
		restore_point_status(
			&self.udb,
			self.sqlite_bucket_id(),
			self.database_id.clone(),
			restore_point,
		)
		.await
	}

	pub async fn delete_restore_point(&self, restore_point: RestorePointId) -> Result<()> {
		delete_restore_point(
			&self.udb,
			self.sqlite_bucket_id(),
			self.database_id.clone(),
			restore_point,
		)
		.await
	}

	pub async fn resolve_restore_point(
		&self,
		restore_point: RestorePointId,
	) -> Result<ResolvedVersionstamp> {
		let node_id = self.node_id.to_string();
		let _timer = metrics::SQLITE_RESTORE_POINT_RESOLVE_DURATION
			.with_label_values(&[node_id.as_str()])
			.start_timer();
		let result = resolve_restore_point(
			&self.udb,
			self.sqlite_bucket_id(),
			self.database_id.clone(),
			restore_point,
		)
		.await;
		metrics::SQLITE_RESTORE_POINT_RESOLVE_TOTAL
			.with_label_values(&[
				node_id.as_str(),
				restore_point_resolve_outcome(result.as_ref().err()),
			])
			.inc();
		result
	}

	pub async fn resolve_restore_target(
		&self,
		selector: SnapshotSelector,
	) -> Result<ResolvedRestoreTarget> {
		let node_id = self.node_id.to_string();
		let _timer = metrics::SQLITE_RESTORE_POINT_RESOLVE_DURATION
			.with_label_values(&[node_id.as_str()])
			.start_timer();
		let result = resolve_restore_target(
			&self.udb,
			self.sqlite_bucket_id(),
			self.database_id.clone(),
			selector,
		)
		.await;
		metrics::SQLITE_RESTORE_POINT_RESOLVE_TOTAL
			.with_label_values(&[
				node_id.as_str(),
				restore_point_resolve_outcome(result.as_ref().err()),
			])
			.inc();
		result
	}

	pub async fn restore_database(&self, selector: SnapshotSelector) -> Result<RestorePointId> {
		restore_database(
			&self.udb,
			self.sqlite_bucket_id(),
			self.database_id.clone(),
			selector,
		)
		.await
	}
}

fn restore_point_create_outcome(err: Option<&anyhow::Error>) -> &'static str {
	if err.is_none() { "ok" } else { "err" }
}

fn restore_point_resolve_outcome(err: Option<&anyhow::Error>) -> &'static str {
	match err.and_then(|err| err.downcast_ref::<SqliteStorageError>()) {
		None => "ok",
		Some(SqliteStorageError::RestoreTargetExpired) => "expired",
		Some(SqliteStorageError::RestorePointNotFound) => "not_found",
		Some(SqliteStorageError::BranchNotReachable) => "unreachable",
		Some(_) => "err",
	}
}
