//! Injectable facade for `tokio::fs`, its `tokio::io` file traits, and `std::fs::File` locking.

use std::{
	future::Future,
	io,
	path::{Path, PathBuf},
	pin::Pin,
	sync::Arc,
};
use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite};

pub type FsFuture<'a, T> = Pin<Box<dyn Future<Output = io::Result<T>> + Send + 'a>>;
pub type File = Box<dyn FileHandle>;
pub type ReadDir = Box<dyn DirectoryHandle>;

/// Only the file operations currently needed by Engine, using upstream I/O traits.
/// Writes, set_len, and directory mutations do not implicitly sync.
pub trait FileHandle: AsyncRead + AsyncWrite + AsyncSeek + Unpin + Send + Sync {
	/// Attempts an exclusive OS lock once. Retain this handle until all protected I/O ends.
	/// Backends without locking must reject the request rather than imply ownership.
	fn try_lock(&self) -> FsFuture<'_, ()> {
		Box::pin(async {
			Err(io::Error::new(
				io::ErrorKind::Unsupported,
				"filesystem locking unavailable",
			))
		})
	}

	/// Reports the scope of this handle's exclusive lock; see [`LockScope`]. Backends that
	/// cannot name a scope reject the request, so callers fall back to evidence that needs none.
	fn lock_scope(&self) -> FsFuture<'_, LockScope> {
		Box::pin(async {
			Err(io::Error::new(
				io::ErrorKind::Unsupported,
				"lock scope unavailable",
			))
		})
	}

	fn sync_data(&self) -> FsFuture<'_, ()>;
	fn sync_all(&self) -> FsFuture<'_, ()>;
	fn set_len(&self, size: u64) -> FsFuture<'_, ()>;
}

/// Where an exclusive lock excludes other holders: one running kernel and one locked file.
///
/// A lock taken through [`FileHandle::try_lock`] is kernel state that lasts until its holder
/// closes the handle or exits. So when a process takes a lock and finds the scope it reports
/// equal to the scope a previous holder reported, that previous holder no longer holds the
/// lock, because it held it under the same kernel on the same file. A different scope, such
/// as the same disk attached to another machine or after a reboot, proves nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LockScope {
	/// Random per kernel boot, such as Linux's `/proc/sys/kernel/random/boot_id`.
	pub kernel_boot_id: [u8; 16],
	/// The locked file's device and inode numbers under that kernel.
	pub device: u64,
	pub inode: u64,
}

pub trait DirectoryHandle: Send + Sync {
	/// Directory iteration has upstream ordering (not necessarily sorted).
	fn next_entry(&mut self) -> FsFuture<'_, Option<DirEntry>>;
}

pub struct DirEntry {
	path: PathBuf,
}

impl DirEntry {
	pub fn new(path: PathBuf) -> Self {
		Self { path }
	}
	pub fn path(&self) -> PathBuf {
		self.path.clone()
	}
}

/// Space on the filesystem holding a path. Two paths on one filesystem report the same
/// `filesystem_id`, so callers sizing several directories can share one pool of space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskSpace {
	pub filesystem_id: u64,
	pub total_bytes: u64,
	/// Bytes an unprivileged process can still write.
	pub available_bytes: u64,
}

/// Backend implemented by production and simulated filesystems.
pub trait FileSystem: Send + Sync {
	fn open<'a>(&'a self, path: &'a Path, options: &'a OpenOptions) -> FsFuture<'a, File>;
	fn read<'a>(&'a self, path: &'a Path) -> FsFuture<'a, Vec<u8>>;
	fn write<'a>(&'a self, path: &'a Path, contents: &'a [u8]) -> FsFuture<'a, ()>;
	fn read_dir<'a>(&'a self, path: &'a Path) -> FsFuture<'a, ReadDir>;
	fn rename<'a>(&'a self, from: &'a Path, to: &'a Path) -> FsFuture<'a, ()>;
	fn remove_file<'a>(&'a self, path: &'a Path) -> FsFuture<'a, ()>;
	fn create_dir_all<'a>(&'a self, path: &'a Path) -> FsFuture<'a, ()>;

	/// Reads the space of the filesystem holding the existing `path`. Backends that cannot
	/// measure space reject the request rather than report a guess.
	fn disk_space<'a>(&'a self, _path: &'a Path) -> FsFuture<'a, DiskSpace> {
		Box::pin(async {
			Err(io::Error::new(
				io::ErrorKind::Unsupported,
				"disk space unavailable",
			))
		})
	}

	/// Sums the sizes of every file under the existing directory `path`, recursively.
	/// A missing directory holds zero bytes.
	fn directory_bytes<'a>(&'a self, _path: &'a Path) -> FsFuture<'a, u64> {
		Box::pin(async {
			Err(io::Error::new(
				io::ErrorKind::Unsupported,
				"directory usage unavailable",
			))
		})
	}
}

/// Cloneable, injected filesystem. An unavailable backend returns Unsupported,
/// rather than accessing host disk from an environment without this capability.
#[derive(Clone, Default)]
pub struct Fs(Option<Arc<dyn FileSystem>>);

impl Fs {
	pub fn new(backend: Arc<dyn FileSystem>) -> Self {
		Self(Some(backend))
	}

	fn backend(&self) -> io::Result<&dyn FileSystem> {
		self.0.as_deref().ok_or_else(|| {
			io::Error::new(
				io::ErrorKind::Unsupported,
				"filesystem capability unavailable",
			)
		})
	}

	pub fn open_options(&self) -> OpenOptions {
		OpenOptions {
			fs: self.clone(),
			read: false,
			write: false,
			append: false,
			truncate: false,
			create: false,
			create_new: false,
		}
	}

	pub async fn open(&self, path: impl AsRef<Path>) -> io::Result<File> {
		self.open_options().read(true).open(path).await
	}

	pub async fn read(&self, path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
		self.backend()?.read(path.as_ref()).await
	}

	pub async fn write(
		&self,
		path: impl AsRef<Path>,
		contents: impl AsRef<[u8]>,
	) -> io::Result<()> {
		self.backend()?
			.write(path.as_ref(), contents.as_ref())
			.await
	}

	pub async fn read_dir(&self, path: impl AsRef<Path>) -> io::Result<ReadDir> {
		self.backend()?.read_dir(path.as_ref()).await
	}

	pub async fn rename(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
		self.backend()?.rename(from.as_ref(), to.as_ref()).await
	}

	pub async fn remove_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
		self.backend()?.remove_file(path.as_ref()).await
	}

	pub async fn create_dir_all(&self, path: impl AsRef<Path>) -> io::Result<()> {
		self.backend()?.create_dir_all(path.as_ref()).await
	}

	pub async fn disk_space(&self, path: impl AsRef<Path>) -> io::Result<DiskSpace> {
		self.backend()?.disk_space(path.as_ref()).await
	}

	pub async fn directory_bytes(&self, path: impl AsRef<Path>) -> io::Result<u64> {
		self.backend()?.directory_bytes(path.as_ref()).await
	}
}

/// Options mirror `tokio::fs::OpenOptions`; the filesystem selects the backend.
#[derive(Clone)]
pub struct OpenOptions {
	fs: Fs,
	pub read: bool,
	pub write: bool,
	pub append: bool,
	pub truncate: bool,
	pub create: bool,
	pub create_new: bool,
}

impl OpenOptions {
	pub fn read(&mut self, read: bool) -> &mut Self {
		self.read = read;
		self
	}
	pub fn write(&mut self, write: bool) -> &mut Self {
		self.write = write;
		self
	}
	pub fn append(&mut self, append: bool) -> &mut Self {
		self.append = append;
		self
	}
	pub fn truncate(&mut self, truncate: bool) -> &mut Self {
		self.truncate = truncate;
		self
	}
	pub fn create(&mut self, create: bool) -> &mut Self {
		self.create = create;
		self
	}
	pub fn create_new(&mut self, create_new: bool) -> &mut Self {
		self.create_new = create_new;
		self
	}
	pub async fn open(&self, path: impl AsRef<Path>) -> io::Result<File> {
		self.fs.backend()?.open(path.as_ref(), self).await
	}
}
