use std::{
	borrow::Cow,
	ops::Deref,
	path::{Path, PathBuf},
	result::Result::Ok,
	time::{Duration, Instant},
};

use anyhow::*;
use foundationdb::{
	self as fdb,
	future::FdbValue,
	options::DatabaseOption,
	tuple::{self, PackResult, TuplePack, TupleUnpack},
	KeySelector, RangeOption,
};

pub mod keys;
mod metrics;

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

/// Wrapper type around `foundationdb::tuple::Subspace` that records metrics.
pub struct Subspace {
	inner: tuple::Subspace,
}

impl Subspace {
	/// Creates a subspace with the given tuple.
	pub fn new<T: TuplePack>(t: &T) -> Self {
		Self {
			inner: tuple::Subspace::all().subspace(t),
		}
	}

	/// Returns a new Subspace whose prefix extends this Subspace with a given tuple encodable.
	pub fn subspace<T: TuplePack>(&self, t: &T) -> Self {
		Self {
			inner: self.inner.subspace(t),
		}
	}

	/// Returns the key encoding the specified Tuple with the prefix of this Subspace
	/// prepended.
	pub fn pack<T: TuplePack>(&self, t: &T) -> Vec<u8> {
		metrics::KEY_PACK_COUNT
			.with_label_values(&[std::any::type_name::<T>()])
			.inc();

		self.inner.pack(t)
	}

	/// Returns the key encoding the specified Tuple with the prefix of this Subspace
	/// prepended, with a versionstamp.
	pub fn pack_with_versionstamp<T: TuplePack>(&self, t: &T) -> Vec<u8> {
		metrics::KEY_PACK_COUNT
			.with_label_values(&[std::any::type_name::<T>()])
			.inc();

		self.inner.pack_with_versionstamp(t)
	}

	/// `unpack` returns the Tuple encoded by the given key with the prefix of this Subspace
	/// removed.  `unpack` will return an error if the key is not in this Subspace or does not
	/// encode a well-formed Tuple.
	pub fn unpack<'de, T: TupleUnpack<'de>>(&self, key: &'de [u8]) -> PackResult<T> {
		metrics::KEY_UNPACK_COUNT
			.with_label_values(&[std::any::type_name::<T>()])
			.inc();

		self.inner.unpack(key)
	}
}

impl Deref for Subspace {
	type Target = tuple::Subspace;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl<'a> From<&'a Subspace> for RangeOption<'static> {
	fn from(subspace: &Subspace) -> Self {
		let (begin, end) = subspace.range();

		Self {
			begin: KeySelector::first_greater_or_equal(Cow::Owned(begin)),
			end: KeySelector::first_greater_or_equal(Cow::Owned(end)),
			..Self::default()
		}
	}
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

async fn fdb_health_check(fdb_cluster_path: PathBuf) {
	let db = match handle(&fdb_cluster_path) {
		Ok(db) => db,
		Err(err) => {
			tracing::error!(?err, "failed to get fdb handle");
			return;
		}
	};

	loop {
		let start_instant = Instant::now();

		match tokio::time::timeout(
			Duration::from_secs(3),
			db.run(|trx, _maybe_committed| async move { Ok(trx.get(b"", true).await?) }),
		)
		.await
		{
			Ok(res) => {
				let dt = start_instant.elapsed().as_secs_f64();
				metrics::PING_DURATION.with_label_values(&[]).observe(dt);
				metrics::MISSED_PING.with_label_values(&[]).set(0);

				if let Err(err) = res {
					tracing::error!(?err, "error checking fdb ping");
				}
			}
			Err(_) => {
				metrics::MISSED_PING.with_label_values(&[]).set(1);

				tracing::error!("fdb missed ping")
			}
		}

		tokio::time::sleep(Duration::from_secs(3)).await;
	}
}

/// When using `add_conflict_range` to add a conflict for a single key, you cannot set both the start and end
/// keys to the same key. Instead, the end key must be the start key + a 0 byte.
/// See Python bindings: https://github.com/apple/foundationdb/blob/ec714791df4a6e4dafb5a926130d5789ce0c497a/bindings/python/fdb/impl.py#L633-L635
pub fn end_of_key_range(key: &[u8]) -> Vec<u8> {
	let mut end_key = Vec::with_capacity(key.len() + 1);
	end_key.extend_from_slice(key);
	end_key.push(0);
	end_key
}

pub mod prelude {
	pub use std::result::Result::Ok;

	pub use foundationdb::{
		future::FdbValue,
		tuple::{PackError, PackResult, TupleDepth, TuplePack, TupleUnpack, VersionstampOffset},
	};

	pub use super::{keys::*, FormalChunkedKey, FormalKey};
}
