use std::sync::Arc;

use namespace::{
	keys,
	types::{
		SqliteNamespaceConfig, decode_sqlite_namespace_config, encode_sqlite_namespace_config,
	},
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;

async fn test_db() -> anyhow::Result<universaldb::Database> {
	let path = Builder::new()
		.prefix("pegboard-sqlite-namespace-config-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn sample_config() -> SqliteNamespaceConfig {
	SqliteNamespaceConfig {
		default_retention_ms: 3_600_000,
		default_checkpoint_interval_ms: 900_000,
		default_max_checkpoints: 12,
		allow_pitr_read: true,
		allow_pitr_destructive: false,
		allow_pitr_admin: true,
		allow_fork: true,
		pitr_max_bytes_per_actor: 64 * 1024 * 1024,
		pitr_namespace_budget_bytes: 1024 * 1024 * 1024,
		max_retention_ms: 86_400_000,
		admin_op_rate_per_min: 30,
		concurrent_admin_ops: 5,
		concurrent_forks_per_src: 3,
	}
}

#[test]
fn vbare_roundtrip_of_namespace_config() -> anyhow::Result<()> {
	let config = sample_config();
	let encoded = encode_sqlite_namespace_config(config.clone())?;
	let decoded = decode_sqlite_namespace_config(&encoded)?;

	assert_eq!(decoded, config);

	Ok(())
}

#[tokio::test]
async fn sqlite_config_key_roundtrip() -> anyhow::Result<()> {
	let db = test_db().await?;
	let namespace_id = rivet_util::Id::new_v1(1);
	let config = sample_config();

	db.run({
		let config = config.clone();
		move |tx| {
			let config = config.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				tx.write(&keys::sqlite_config_key(namespace_id), config)?;
				Ok(())
			}
		}
	})
	.await?;

	let stored = db
		.run(move |tx| async move {
			let tx = tx.with_subspace(keys::subspace());
			tx.read(&keys::sqlite_config_key(namespace_id), Snapshot).await
		})
		.await?;

	assert_eq!(stored, config);

	Ok(())
}
