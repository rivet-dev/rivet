//! Local-directory object store for single-node engines, built on `object_store`'s
//! `LocalFileSystem`.
//!
//! `LocalFileSystem` cannot apply `PutMode::Update` or `delete_if_match`, which topology
//! heartbeats, leases and the tablet manifest require. This adapter serves those by checking
//! the stored ETag and writing while it holds the object's lock, which every mutation of that
//! object takes, so a check and its write cannot interleave with another write. The locks live
//! in this process, so `open` takes an exclusive lock on a sibling `.rivet-store-lock` file and
//! refuses a directory another engine process holds.
//!
//! `LocalFileSystem` runs file I/O as blocking jobs that keep running when their caller is
//! cancelled, so every mutation hands its object locks and the directory ownership to the
//! blocking job and releases them only when the file operation itself finishes.

use std::{
	collections::hash_map::DefaultHasher,
	fmt,
	hash::{Hash, Hasher},
	path::Path as FsPath,
	sync::Arc,
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use object_store::{
	CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
	ObjectStoreExt, PutMode, PutMultipartOptions, PutOptions, PutPayload, PutResult, RenameOptions,
	UploadPart,
	local::{LocalFileSystem, LocalOperationGuard, LocalUpload},
	path::Path,
};
use tokio::sync::{Mutex, OwnedMutexGuard};

/// Lock stripes shared by all objects. Striping keeps a small lease or heartbeat write from
/// queueing behind an unrelated SST upload and its fsync.
const LOCK_STRIPES: usize = 64;

pub struct LocalObjectStore {
	inner: LocalFileSystem,
	locks: ObjectLocks,
	/// Exclusive ownership of the directory, shared with every blocking mutation so a second
	/// engine cannot open it while their writes are still running.
	_directory_lock: Arc<std::fs::File>,
}

/// Per-object write locks, striped by path hash.
#[derive(Clone, Debug)]
struct ObjectLocks(Arc<[Arc<Mutex<()>>]>);

impl ObjectLocks {
	fn new() -> Self {
		Self(
			(0..LOCK_STRIPES)
				.map(|_| Arc::new(Mutex::new(())))
				.collect(),
		)
	}

	fn stripe(&self, location: &Path) -> usize {
		let mut hasher = DefaultHasher::new();
		location.hash(&mut hasher);
		(hasher.finish() % LOCK_STRIPES as u64) as usize
	}

	async fn lock(&self, location: &Path) -> OwnedMutexGuard<()> {
		self.0[self.stripe(location)].clone().lock_owned().await
	}

	/// Locks both objects of a copy or rename in stripe order, so two such operations cannot
	/// deadlock.
	async fn lock_pair(&self, from: &Path, to: &Path) -> Vec<OwnedMutexGuard<()>> {
		let mut stripes = vec![self.stripe(from), self.stripe(to)];
		stripes.sort_unstable();
		stripes.dedup();
		let mut guards = Vec::with_capacity(stripes.len());
		for stripe in stripes {
			guards.push(self.0[stripe].clone().lock_owned().await);
		}
		guards
	}
}

#[derive(Debug)]
struct MutationOwnership {
	_directory: Arc<std::fs::File>,
	_objects: Vec<OwnedMutexGuard<()>>,
}

fn ownership(
	directory: &Arc<std::fs::File>,
	objects: Vec<OwnedMutexGuard<()>>,
) -> LocalOperationGuard {
	Arc::new(MutationOwnership {
		_directory: directory.clone(),
		_objects: objects,
	})
}

impl LocalObjectStore {
	/// Opens the store rooted at `root`, creating the directory when it does not exist.
	#[allow(
		clippy::disallowed_methods,
		reason = "the local object store adapter owns its root directory and its ownership lock, as remote adapters own their connections"
	)]
	pub fn open(root: &FsPath) -> Result<Arc<Self>> {
		std::fs::create_dir_all(root)
			.with_context(|| format!("create local object store directory `{}`", root.display()))?;
		let root = root
			.canonicalize()
			.context("resolve local object store directory")?;
		let mut lock_path = root.as_os_str().to_os_string();
		lock_path.push(".rivet-store-lock");
		let directory_lock = std::fs::OpenOptions::new()
			.read(true)
			.write(true)
			.create(true)
			.truncate(false)
			.open(&lock_path)
			.context("open local object store ownership lock")?;
		directory_lock.try_lock().with_context(|| {
			format!(
				"local object store `{}` is already in use by another engine",
				root.display()
			)
		})?;
		let inner = LocalFileSystem::new_with_prefix(&root)
			.with_context(|| format!("open local object store `{}`", root.display()))?
			.with_fsync(true)
			.with_automatic_cleanup(true);
		Ok(Arc::new(Self {
			inner,
			locks: ObjectLocks::new(),
			_directory_lock: Arc::new(directory_lock),
		}))
	}

	/// The stored ETag, or `None` when the object does not exist.
	async fn current_etag(
		inner: &LocalFileSystem,
		location: &Path,
	) -> object_store::Result<Option<String>> {
		match inner.head(location).await {
			Ok(meta) => Ok(meta.e_tag),
			Err(object_store::Error::NotFound { .. }) => Ok(None),
			Err(error) => Err(error),
		}
	}
}

impl fmt::Debug for LocalObjectStore {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "LocalObjectStore({})", self.inner)
	}
}

impl fmt::Display for LocalObjectStore {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "LocalObjectStore({})", self.inner)
	}
}

fn precondition(location: &Path, reason: &'static str) -> object_store::Error {
	object_store::Error::Precondition {
		path: location.to_string(),
		source: reason.into(),
	}
}

#[async_trait]
impl ObjectStore for LocalObjectStore {
	async fn put_opts(
		&self,
		location: &Path,
		payload: PutPayload,
		opts: PutOptions,
	) -> object_store::Result<PutResult> {
		let lock = self.locks.lock(location).await;
		let inner = self
			.inner
			.clone()
			.with_operation_guard(ownership(&self._directory_lock, vec![lock]));
		match &opts.mode {
			PutMode::Overwrite | PutMode::Create => inner.put_opts(location, payload, opts).await,
			PutMode::Update(version) => {
				let Some(expected) = version.e_tag.as_deref() else {
					return Err(precondition(location, "conditional update without an ETag"));
				};
				match Self::current_etag(&inner, location).await? {
					Some(current) if current == expected => {}
					Some(_) => {
						return Err(precondition(
							location,
							"ETag changed before conditional update",
						));
					}
					None => {
						return Err(precondition(
							location,
							"object missing for conditional update",
						));
					}
				}
				let opts = PutOptions {
					mode: PutMode::Overwrite,
					..opts
				};
				inner.put_opts(location, payload, opts).await
			}
		}
	}

	async fn put_multipart_opts(
		&self,
		location: &Path,
		opts: PutMultipartOptions,
	) -> object_store::Result<Box<dyn MultipartUpload>> {
		Ok(Box::new(LockedUpload {
			inner: self
				.inner
				.clone()
				.with_operation_guard(self._directory_lock.clone())
				.create_multipart(location, opts)?,
			locks: self.locks.clone(),
			location: location.clone(),
			_directory_lock: self._directory_lock.clone(),
		}))
	}

	async fn get_opts(
		&self,
		location: &Path,
		options: GetOptions,
	) -> object_store::Result<GetResult> {
		self.inner.get_opts(location, options).await
	}

	async fn delete_if_match(&self, location: &Path, etag: &str) -> object_store::Result<()> {
		let lock = self.locks.lock(location).await;
		let inner = self
			.inner
			.clone()
			.with_operation_guard(ownership(&self._directory_lock, vec![lock]));
		match Self::current_etag(&inner, location).await? {
			None => Ok(()),
			Some(current) if current == etag => match inner.delete(location).await {
				Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
				Err(error) => Err(error),
			},
			Some(_) => Err(precondition(
				location,
				"ETag changed before conditional delete",
			)),
		}
	}

	fn delete_stream(
		&self,
		locations: BoxStream<'static, object_store::Result<Path>>,
	) -> BoxStream<'static, object_store::Result<Path>> {
		let inner = self.inner.clone();
		let locks = self.locks.clone();
		let directory_lock = self._directory_lock.clone();
		locations
			.then(move |location| {
				let inner = inner.clone();
				let locks = locks.clone();
				let directory_lock = directory_lock.clone();
				async move {
					let location = location?;
					let lock = locks.lock(&location).await;
					let inner = inner.with_operation_guard(ownership(&directory_lock, vec![lock]));
					match inner.delete(&location).await {
						Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(location),
						Err(error) => Err(error),
					}
				}
			})
			.boxed()
	}

	fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
		self.inner.list(prefix)
	}

	fn list_with_offset(
		&self,
		prefix: Option<&Path>,
		offset: &Path,
	) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
		self.inner.list_with_offset(prefix, offset)
	}

	async fn list_with_delimiter(&self, prefix: Option<&Path>) -> object_store::Result<ListResult> {
		self.inner.list_with_delimiter(prefix).await
	}

	async fn copy_opts(
		&self,
		from: &Path,
		to: &Path,
		options: CopyOptions,
	) -> object_store::Result<()> {
		let locks = self.locks.lock_pair(from, to).await;
		let inner = self
			.inner
			.clone()
			.with_operation_guard(ownership(&self._directory_lock, locks));
		inner.copy_opts(from, to, options).await
	}

	async fn rename_opts(
		&self,
		from: &Path,
		to: &Path,
		options: RenameOptions,
	) -> object_store::Result<()> {
		let locks = self.locks.lock_pair(from, to).await;
		let inner = self
			.inner
			.clone()
			.with_operation_guard(ownership(&self._directory_lock, locks));
		inner.rename_opts(from, to, options).await
	}
}

/// Multipart publication must serialize with conditional checks on the same object.
#[derive(Debug)]
struct LockedUpload {
	inner: LocalUpload,
	locks: ObjectLocks,
	location: Path,
	_directory_lock: Arc<std::fs::File>,
}

#[async_trait]
impl MultipartUpload for LockedUpload {
	fn put_part(&mut self, data: PutPayload) -> UploadPart {
		self.inner
			.put_part_with_guard(data, Some(self._directory_lock.clone()))
	}

	async fn complete(&mut self) -> object_store::Result<PutResult> {
		let lock = self.locks.lock(&self.location).await;
		let guard = ownership(&self._directory_lock, vec![lock]);
		self.inner.complete_with_guard(Some(guard)).await
	}

	async fn abort(&mut self) -> object_store::Result<()> {
		let lock = self.locks.lock(&self.location).await;
		let guard = ownership(&self._directory_lock, vec![lock]);
		self.inner.abort_with_guard(Some(guard)).await
	}
}


