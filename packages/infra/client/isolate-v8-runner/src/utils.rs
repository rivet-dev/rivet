use std::{path::Path, result::Result::Ok, time::Duration};

use anyhow::*;
use foundationdb::{self as fdb, options::DatabaseOption};

pub fn var(name: &str) -> Result<String> {
	std::env::var(name).context(name.to_string())
}

pub fn fdb_handle() -> fdb::FdbResult<fdb::Database> {
	let cluster_file_path = Path::new("/etc/foundationdb/fdb.cluster");
	let db = fdb::Database::from_path(&cluster_file_path.display().to_string())?;
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
			Err(_) => eprintln!("db missed ping"),
		}

		tokio::time::sleep(Duration::from_secs(3)).await;
	}
}
