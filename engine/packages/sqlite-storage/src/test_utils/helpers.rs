//! Shared test helpers for sqlite-storage integration tests.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use tempfile::Builder;
use tokio::sync::mpsc;
use universaldb::Subspace;
use uuid::Uuid;

use crate::engine::SqliteEngine;
use crate::types::DirtyPage;
use crate::udb;

async fn open_test_db(path: &Path) -> Result<Arc<universaldb::Database>> {
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path.to_path_buf()).await?;
	let db = Arc::new(universaldb::Database::new(Arc::new(driver)));

	Ok(db)
}

pub async fn test_db() -> Result<(Arc<universaldb::Database>, Subspace)> {
	let (db, subspace, _path) = test_db_with_path().await?;

	Ok((db, subspace))
}

pub async fn test_db_with_path() -> Result<(Arc<universaldb::Database>, Subspace, PathBuf)> {
	let path = Builder::new().prefix("sqlite-storage-").tempdir()?.keep();
	let db = open_test_db(&path).await?;
	let subspace = Subspace::new(&("sqlite-storage", Uuid::new_v4().to_string()));

	Ok((db, subspace, path))
}

pub async fn reopen_test_db(path: impl AsRef<Path>) -> Result<Arc<universaldb::Database>> {
	open_test_db(path.as_ref()).await
}

pub fn checkpoint_test_db(db: &universaldb::Database) -> Result<PathBuf> {
	let path = Builder::new()
		.prefix("sqlite-storage-checkpoint-")
		.tempdir()?
		.keep();
	std::fs::remove_dir_all(&path)?;
	db.checkpoint(&path)?;

	Ok(path)
}

pub async fn setup_engine() -> Result<(SqliteEngine, mpsc::UnboundedReceiver<String>)> {
	let (db, subspace) = test_db().await?;
	Ok(SqliteEngine::new(db, subspace))
}

pub async fn read_value(engine: &SqliteEngine, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
	udb::get_value(
		engine.db.as_ref(),
		&engine.subspace,
		engine.op_counter.as_ref(),
		key,
	)
	.await
}

pub async fn scan_prefix_values(
	engine: &SqliteEngine,
	prefix: Vec<u8>,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	udb::scan_prefix_values(
		engine.db.as_ref(),
		&engine.subspace,
		engine.op_counter.as_ref(),
		prefix,
	)
	.await
}

pub fn assert_op_count(engine: &SqliteEngine, expected: usize) {
	assert_eq!(
		udb::op_count(&engine.op_counter),
		expected,
		"unexpected op count"
	);
}

pub fn clear_op_count(engine: &SqliteEngine) {
	udb::clear_op_count(&engine.op_counter);
}

pub fn test_page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; crate::types::SQLITE_PAGE_SIZE as usize],
	}
}
