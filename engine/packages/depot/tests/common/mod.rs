#![allow(dead_code)]

use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::{Context, Result};
use depot::{
	cold_tier::{ColdTier, FilesystemColdTier},
	conveyer::Db,
};
use gas::prelude::Id;
use rivet_pools::NodeId;
use tempfile::{Builder, TempDir};
use universaldb::utils::IsolationLevel::Snapshot;

pub async fn test_db(prefix: &str) -> Result<universaldb::Database> {
	let path = Builder::new().prefix(prefix).tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

pub async fn test_db_with_dir(prefix: &str) -> Result<(Arc<universaldb::Database>, TempDir)> {
	let dir = Builder::new().prefix(prefix).tempdir()?;
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(dir.path().to_path_buf()).await?;

	Ok((Arc::new(universaldb::Database::new(Arc::new(driver))), dir))
}

pub async fn test_db_arc(prefix: &str) -> Result<Arc<universaldb::Database>> {
	Ok(Arc::new(test_db(prefix).await?))
}

pub fn make_db(
	db: Arc<universaldb::Database>,
	bucket_id: Id,
	database_id: impl Into<String>,
) -> Db {
	Db::new(db, bucket_id, database_id.into(), NodeId::new())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierMode {
	Disabled,
	Filesystem,
}

impl TierMode {
	pub fn label(self) -> &'static str {
		match self {
			TierMode::Disabled => "cold_disabled",
			TierMode::Filesystem => "cold_filesystem",
		}
	}
}

pub struct TestDb {
	pub db: Db,
	pub udb: Arc<universaldb::Database>,
	pub bucket_id: Id,
	pub database_id: String,
	pub cold_tier: Option<Arc<dyn ColdTier>>,
	_udb_dir: TempDir,
	_cold_dir: Option<TempDir>,
}

impl TestDb {
	pub fn make_db(&self, bucket_id: Id, database_id: impl Into<String>) -> Db {
		let database_id = database_id.into();
		match &self.cold_tier {
			Some(cold_tier) => Db::new_with_cold_tier(
				self.udb.clone(),
				bucket_id,
				database_id,
				NodeId::new(),
				cold_tier.clone(),
			),
			None => Db::new(self.udb.clone(), bucket_id, database_id, NodeId::new()),
		}
	}
}

pub async fn build_test_db(prefix: &str, tier: TierMode) -> Result<TestDb> {
	let (udb, udb_dir) = test_db_with_dir(prefix).await?;
	let bucket_id = Id::new_v1(1);
	let database_id = format!("{prefix}-db");

	let (db, cold_tier, cold_dir) = match tier {
		TierMode::Disabled => (
			Db::new(udb.clone(), bucket_id, database_id.clone(), NodeId::new()),
			None,
			None,
		),
		TierMode::Filesystem => {
			let dir = tempfile::tempdir()?;
			let tier: Arc<dyn ColdTier> = Arc::new(FilesystemColdTier::new(dir.path()));
			let db = Db::new_with_cold_tier(
				udb.clone(),
				bucket_id,
				database_id.clone(),
				NodeId::new(),
				tier.clone(),
			);
			(db, Some(tier), Some(dir))
		}
	};

	Ok(TestDb {
		db,
		udb,
		bucket_id,
		database_id,
		cold_tier,
		_udb_dir: udb_dir,
		_cold_dir: cold_dir,
	})
}

pub async fn test_matrix<F>(prefix: &str, body: F) -> Result<()>
where
	F: Fn(TierMode, TestDb) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>,
{
	for tier in [TierMode::Disabled, TierMode::Filesystem] {
		let ctx = build_test_db(prefix, tier)
			.await
			.with_context(|| format!("[{}] failed to build TestDb", tier.label()))?;
		body(tier, ctx)
			.await
			.with_context(|| format!("[{}] body failed", tier.label()))?;
	}

	Ok(())
}

pub async fn read_value(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
	db.run(move |tx| {
		let key = key.clone();
		async move {
			Ok(tx
				.informal()
				.get(&key, Snapshot)
				.await?
				.map(Vec::<u8>::from))
		}
	})
	.await
}
