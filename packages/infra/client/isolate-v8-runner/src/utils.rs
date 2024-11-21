use std::{result::Result::Ok, time::Duration};

use anyhow::*;
use foundationdb::{self as fdb, options::DatabaseOption};

use crate::config::Config;

pub fn fdb_handle(config: &Config) -> Result<fdb::Database> {
	let db = fdb::Database::from_path(
		&config
			.fdb_cluster_path
			.to_str()
			.context("bad fdb_cluster_path")?
			.to_string(),
	)
	.context("failed to create FDB database")?;
	db.set_option(DatabaseOption::TransactionRetryLimit(10))?;

	Ok(db)
}

pub async fn fdb_health_check(config: Config) -> Result<()> {
	let db = fdb_handle(&config)?;

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
