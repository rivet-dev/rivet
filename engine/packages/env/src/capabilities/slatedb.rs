//! Injectable SlateDB construction. The upstream database still owns its runtime tasks.

use crate::MutationGuard;
use anyhow::Result;
use bytes::Bytes;
use object_store::path::Path;
use std::{future::Future, num::NonZeroU64, ops::Bound, pin::Pin, sync::Arc};
use uuid::Uuid;

/// Stable, precommitted clone inputs. This adapter does not allocate destinations
/// or choose checkpoints after an uncertain result.
#[derive(Clone, Debug)]
pub struct ProjectedClone {
	pub source: Path,
	pub destination: Path,
	pub checkpoint_id: Uuid,
	pub manifest_id: u64,
	/// Physical half-open snapshot range. One range may cover multiple families; the
	/// caller then enforces logical ownership on reads and through its storage policy.
	pub range: (Bound<Bytes>, Bound<Bytes>),
}

/// Whether catalog history permits an empty namespace. This is not a writer fence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseCreation {
	/// Bootstrap, including retry before any writer was durably confirmed ready.
	Allow,
	/// A split clone or a previously confirmed writer must already have initialized storage.
	RequireExisting,
}

pub use super::slatedb_result::DeleteDatabaseResult;

pub use slatedb::config::{CompressionCodec, Settings};

/// Upstream cache/filter controls shared by the writer and its embedded compactor.
/// Scan options are consumed by the database owner for individual snapshot reads.
#[derive(Clone, Default)]
pub struct StoragePolicy {
	pub sst_block_size: Option<slatedb::config::SstBlockSize>,
	pub block_cache: slatedb::BlockCachePolicy,
	pub filters: Option<Vec<Arc<dyn slatedb::FilterPolicy>>>,
	pub compaction_filter: Option<Arc<dyn slatedb::CompactionFilterSupplier>>,
	pub scan: slatedb::config::ScanOptions,
}

/// Current usage of a factory's shared SlateDB caches, for metrics only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlateDbCacheUsage {
	pub block_bytes: u64,
	pub block_capacity_bytes: u64,
	pub meta_bytes: u64,
	pub meta_capacity_bytes: u64,
}

/// Reports the caches that every database of one factory shares.
pub trait SharedCacheUsage: Send + Sync {
	fn usage(&self) -> SlateDbCacheUsage;
}

/// What [`SlateDbFactory::drop_caches`] dropped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DroppedCaches {
	/// Bytes the shared block and metadata caches held. These caches are process-wide, so
	/// every database of the factory loses them, whatever prefix was named.
	pub memory_bytes: u64,
	/// Disk-cached object parts removed under the named prefix.
	pub disk_files: u64,
	pub disk_bytes: u64,
}

pub trait SlateDbFactory: Send + Sync {
	/// A bounded page from an exact retained checkpoint, without acquiring a writer.
	/// Returns at most `limit` rows and 256 KiB; a short page can be byte-limited.
	/// Only an empty page proves exhaustion. A single oversized row returns an error.
	fn checkpoint_page(
		&self,
		_path: Path,
		_checkpoint: Uuid,
		_manifest: u64,
		_range: (Bound<Bytes>, Bound<Bytes>),
		_limit: usize,
	) -> Pin<Box<dyn Future<Output = Result<Vec<(Bytes, Bytes)>>> + Send + '_>> {
		Box::pin(async { anyhow::bail!("checkpoint reads unavailable") })
	}
	/// The caches shared by every database this factory opens, or `None` when each database
	/// keeps SlateDB's own per-database caches.
	fn shared_cache(&self) -> Option<Arc<dyn SharedCacheUsage>> {
		None
	}

	/// Drops the cached data of every database this factory opened: the shared block and
	/// metadata caches, and the disk-cached object parts under `prefix`, or all of them when
	/// `prefix` is `None`. Open databases keep working and read what they need from object
	/// storage again. Backends without such caches report an error rather than nothing.
	fn drop_caches(
		&self,
		prefix: Option<Path>,
	) -> Pin<Box<dyn Future<Output = Result<DroppedCaches>> + Send + '_>> {
		let _ = prefix;
		Box::pin(async { anyhow::bail!("this SlateDB factory keeps no droppable caches") })
	}

	/// Delete an irrevocably retired namespace once only its retained checkpoint remains.
	/// The caller must prevent new writers and new clone sources from being admitted.
	/// Existing child pins must disappear through normal detach before this can succeed.
	/// An uncertain result is retried with the identical path and checkpoint; a partially
	/// deleted store resumes through SlateDB's deletion marker, never by assuming absence.
	fn delete_unpinned_database(
		&self,
		path: Path,
		retained_checkpoint: Uuid,
		guard: Arc<dyn MutationGuard>,
	) -> Pin<Box<dyn Future<Output = Result<DeleteDatabaseResult>> + Send + '_>>;

	/// The caller must retain this future until completion. An error may follow persisted
	/// fencing effects, but returning an error must join every partially constructed writer task.
	/// Dropping a pending open is not cleanup; neither timeout nor task cancellation proves closure.
	fn open(
		&self,
		path: Path,
		epoch: NonZeroU64,
		creation: DatabaseCreation,
	) -> Pin<Box<dyn Future<Output = Result<slatedb::Db>> + Send + '_>> {
		self.open_with_settings(
			path,
			epoch,
			creation,
			slatedb::config::Settings::default(),
			None,
		)
	}

	/// Opens with the owner's storage policy while preserving the injected clock,
	/// stores and leadership fence. An optional upstream recorder exposes storage
	/// telemetry only; it must never drive application decisions. Construction and
	/// cleanup guarantees match `open`.
	fn open_with_settings(
		&self,
		path: Path,
		epoch: NonZeroU64,
		creation: DatabaseCreation,
		settings: slatedb::config::Settings,
		recorder: Option<Arc<dyn slatedb_common::metrics::MetricsRecorder>>,
	) -> Pin<Box<dyn Future<Output = Result<slatedb::Db>> + Send + '_>>;

	/// Opens with explicit upstream storage policy. Backends must implement the
	/// capability or reject it; silently ignoring retention filters is unsafe.
	fn open_with_storage_policy(
		&self,
		_path: Path,
		_epoch: NonZeroU64,
		_creation: DatabaseCreation,
		_settings: slatedb::config::Settings,
		_recorder: Option<Arc<dyn slatedb_common::metrics::MetricsRecorder>>,
		_policy: StoragePolicy,
	) -> Pin<Box<dyn Future<Output = Result<slatedb::Db>> + Send + '_>> {
		Box::pin(async { anyhow::bail!("SlateDB storage policy capability is unavailable") })
	}

	/// Caller must first abort/join application invocations and resolve submitted
	/// writes. Flushes the memtable before retaining an idempotent final checkpoint.
	fn final_checkpoint<'a>(
		&'a self,
		db: &'a slatedb::Db,
		id: Uuid,
		guard: Arc<dyn MutationGuard>,
	) -> Pin<Box<dyn Future<Output = Result<slatedb::CheckpointCreateResult>> + Send + 'a>>;

	/// Creates or verifies the exact projected clone without opening a writer.
	/// An error may follow a durable effect; retry the identical request, never a
	/// newly allocated destination. Does not overwrite initialized child writers.
	fn clone_projected(
		&self,
		request: ProjectedClone,
		guard: Arc<dyn MutationGuard>,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}
