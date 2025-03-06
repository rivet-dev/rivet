use std::{
	path::{Path, PathBuf},
	result::Result::Ok,
	time::Duration,
};

use anyhow::*;
use foundationdb::{self as fdb, future::FdbValue, options::DatabaseOption};

/// Makes the code blatantly obvious if its using a snapshot read.
pub const SNAPSHOT: bool = true;
pub const SERIALIZABLE: bool = false;

lazy_static::lazy_static! {
	/// Must only be created once per program and must not be dropped until the program is over otherwise all
	/// FDB calls will fail with error code 1100.
	static ref FDB_NETWORK: fdb::api::NetworkAutoStop = unsafe { fdb::boot() };
}

pub trait FormalKey {
	type Value;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value>;

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>>;
}

pub trait FormalChunkedKey {
	type Value;
	type ChunkKey;

	fn chunk(&self, chunk: usize) -> Self::ChunkKey;

	/// Assumes chunks are in order.
	fn combine(&self, chunks: Vec<FdbValue>) -> Result<Self::Value>;

	fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>>;
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

async fn fdb_health_check(fdb_cluster_path: PathBuf) -> Result<()> {
	let db = handle(&fdb_cluster_path)?;

	loop {
		match tokio::time::timeout(
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

		tokio::time::sleep(Duration::from_secs(3)).await;
	}
}

pub mod prelude {
	pub use std::{borrow::Cow, result::Result::Ok};

	pub use foundationdb::{
		future::FdbValue,
		tuple::{PackError, PackResult, TupleDepth, TuplePack, TupleUnpack, VersionstampOffset},
	};

	pub use super::{FormalChunkedKey, FormalKey};
}
