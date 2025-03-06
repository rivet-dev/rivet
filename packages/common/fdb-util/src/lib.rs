use std::{path::{PathBuf, Path}, result::Result::Ok, time::Duration};

use anyhow::*;
use foundationdb::{self as fdb, options::DatabaseOption};

lazy_static::lazy_static! {
	/// Must only be created once per program and must not be dropped until the program is over otherwise all
	/// FDB calls will fail with error code 1100.
	static ref FDB_NETWORK: fdb::api::NetworkAutoStop = unsafe { fdb::boot() };
}

pub fn handle(fdb_cluster_path: &Path) -> Result<fdb::Database> {
	let db = fdb::Database::from_path(
		&fdb_cluster_path
			.to_str()
			.context("bad fdb_cluster_path")?
			.to_string(),
	)
	.context("failed to create FDB database")?;
	db.set_option(DatabaseOption::TransactionRetryLimit(10))?;

	Ok(db)
}

/// Starts the network thread and spawns a health check task.
pub fn init(fdb_cluster_path: &Path) {
	// Initialize lazy static
	let _network = &*FDB_NETWORK;

	tokio::spawn(fdb_health_check(fdb_cluster_path.to_path_buf()));
}

pub async fn fdb_health_check(fdb_cluster_path: PathBuf) -> Result<()> {
	let db = handle(&fdb_cluster_path)?;

	loop {
		match ::tokio::time::timeout(
			Duration::from_secs(3),
			db.run(|trx, _maybe_committed| async move { Ok(trx.get(b"", true).await?) }),
		)
		.await
		{
			Ok(res) => {
				res?;
			}
			Err(_) => tracing::error!("fdb missed ping"),
		}

		::tokio::time::sleep(Duration::from_secs(3)).await;
	}
}
