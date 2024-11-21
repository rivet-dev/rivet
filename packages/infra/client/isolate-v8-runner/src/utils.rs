use std::{result::Result::Ok, time::Duration};

use anyhow::*;
use foundationdb::{self as fdb, options::DatabaseOption};

pub fn var(name: &str) -> Result<String> {
	std::env::var(name).with_context(|| format!("missing env var: {name}"))
}

pub fn fdb_handle() -> Result<fdb::Database> {
	let fdb_cluster_path = var("FDB_CLUSTER_PATH")?;
	let db =
		fdb::Database::from_path(&fdb_cluster_path).context("failed to create FDB database")?;
	db.set_option(DatabaseOption::TransactionRetryLimit(10))?;

	Ok(db)
}

pub async fn fdb_health_check() -> Result<()> {
	let db = fdb_handle()?;

	loop {
		match tokio::time::timeout(
			Duration::from_secs(3),
			db.run(|trx, _maybe_committed| async move { Ok(trx.get(b"", false).await?) }),
		)
		.await
		{
			Ok(res) => {
				res?;
			}
			Err(_) => tracing::error!("fdb missed ping"),
		}

		tokio::time::sleep(Duration::from_secs(3)).await;
	}
}
