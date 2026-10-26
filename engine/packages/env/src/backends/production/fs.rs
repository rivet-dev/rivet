//! Production adapter over `tokio::fs`, with nonblocking locks from `std::fs::File`.

#![allow(
	clippy::disallowed_methods,
	clippy::disallowed_types,
	reason = "production filesystem adapter owns host disk I/O"
)]

use crate::fs::{
	DirEntry, DirectoryHandle, DiskSpace, File, FileHandle, FileSystem, FsFuture, LockScope,
	OpenOptions, ReadDir,
};
#[cfg(target_os = "linux")]
use std::os::{fd::AsRawFd, unix::fs::MetadataExt};
use std::path::Path;
use tokio::fs;
use tracing::Instrument;

/// For production bootstrap and plain Tokio tests, never SimRuntime: Tokio
/// filesystem operations use blocking threads. Directory durability is explicit.
pub struct ProductionFs;

impl FileHandle for fs::File {
	fn try_lock(&self) -> FsFuture<'_, ()> {
		Box::pin(async {
			// The cloned descriptor shares the lock with the retained Tokio handle.
			// Dropping this clone does not release it while that original remains open.
			let file = self.try_clone().await?.into_std().await;
			file.try_lock().map_err(std::io::Error::from)
		})
	}

	fn lock_scope(&self) -> FsFuture<'_, LockScope> {
		Box::pin(async {
			let metadata = self.metadata().await?;
			lock_scope(self, &metadata).await
		})
	}

	fn sync_data(&self) -> FsFuture<'_, ()> {
		Box::pin(fs::File::sync_data(self).instrument(tracing::debug_span!("fs.sync_data")))
	}
	fn sync_all(&self) -> FsFuture<'_, ()> {
		Box::pin(fs::File::sync_all(self).instrument(tracing::debug_span!("fs.sync_all")))
	}
	fn set_len(&self, size: u64) -> FsFuture<'_, ()> {
		Box::pin(fs::File::set_len(self, size).instrument(tracing::debug_span!("fs.set_len")))
	}
}

impl DirectoryHandle for fs::ReadDir {
	fn next_entry(&mut self) -> FsFuture<'_, Option<DirEntry>> {
		Box::pin(async move {
			Ok(fs::ReadDir::next_entry(self)
				.await?
				.map(|entry| DirEntry::new(entry.path())))
		})
	}
}

impl FileSystem for ProductionFs {
	fn open<'a>(&'a self, path: &'a Path, options: &'a OpenOptions) -> FsFuture<'a, File> {
		Box::pin(
			async move {
				let file = fs::OpenOptions::new()
					.read(options.read)
					.write(options.write)
					.append(options.append)
					.truncate(options.truncate)
					.create(options.create)
					.create_new(options.create_new)
					.open(path)
					.await?;
				Ok(Box::new(file) as File)
			}
			.instrument(tracing::debug_span!("fs.open")),
		)
	}
	fn read<'a>(&'a self, path: &'a Path) -> FsFuture<'a, Vec<u8>> {
		Box::pin(fs::read(path).instrument(tracing::debug_span!("fs.read")))
	}
	fn write<'a>(&'a self, path: &'a Path, contents: &'a [u8]) -> FsFuture<'a, ()> {
		Box::pin(fs::write(path, contents).instrument(tracing::debug_span!("fs.write")))
	}
	fn read_dir<'a>(&'a self, path: &'a Path) -> FsFuture<'a, ReadDir> {
		Box::pin(
			async move { Ok(Box::new(fs::read_dir(path).await?) as ReadDir) }
				.instrument(tracing::debug_span!("fs.read_dir")),
		)
	}
	fn rename<'a>(&'a self, from: &'a Path, to: &'a Path) -> FsFuture<'a, ()> {
		Box::pin(fs::rename(from, to).instrument(tracing::debug_span!("fs.rename")))
	}
	fn remove_file<'a>(&'a self, path: &'a Path) -> FsFuture<'a, ()> {
		Box::pin(fs::remove_file(path).instrument(tracing::debug_span!("fs.remove_file")))
	}
	fn create_dir_all<'a>(&'a self, path: &'a Path) -> FsFuture<'a, ()> {
		Box::pin(fs::create_dir_all(path).instrument(tracing::debug_span!("fs.create_dir_all")))
	}

	fn disk_space<'a>(&'a self, path: &'a Path) -> FsFuture<'a, DiskSpace> {
		let path = path.to_path_buf();
		Box::pin(async move {
			tokio::task::spawn_blocking(move || statvfs(&path))
				.await
				.map_err(std::io::Error::other)?
		})
	}

	fn directory_bytes<'a>(&'a self, path: &'a Path) -> FsFuture<'a, u64> {
		let path = path.to_path_buf();
		Box::pin(async move {
			tokio::task::spawn_blocking(move || directory_bytes(&path))
				.await
				.map_err(std::io::Error::other)?
		})
	}
}

/// Local filesystems whose `flock` is kernel state on one inode, held until the descriptor
/// closes. Network filesystems such as NFS, FUSE and anything else report no scope, because
/// their locks may be emulated, recovered after a lease, or local to one client.
#[cfg(target_os = "linux")]
const LOCAL_LOCK_FILESYSTEMS: [i64; 4] = [
	0xEF53,      // ext2, ext3 and ext4
	0x5846_5342, // xfs
	0x9123_683E, // btrfs
	0x0102_1994, // tmpfs
];

/// The running kernel's boot ID and the file's device and inode, for a file on a local
/// filesystem whose locks are kernel state. Only Linux exposes a boot ID, so other platforms
/// report no scope. A cloned VM snapshot shares its boot ID and disk with the original, so
/// running both is unsupported.
#[cfg(target_os = "linux")]
async fn lock_scope(file: &fs::File, metadata: &std::fs::Metadata) -> std::io::Result<LockScope> {
	let mut stats = std::mem::MaybeUninit::<libc::statfs>::uninit();
	// SAFETY: the descriptor stays open for the call and `stats` is a valid out pointer.
	let result = unsafe { libc::fstatfs(file.as_raw_fd(), stats.as_mut_ptr()) };
	if result != 0 {
		return Err(std::io::Error::last_os_error());
	}
	// SAFETY: fstatfs returned success, so it initialized `stats`.
	let stats = unsafe { stats.assume_init() };
	let filesystem = stats.f_type as i64;
	if !LOCAL_LOCK_FILESYSTEMS.contains(&filesystem) {
		return Err(std::io::Error::new(
			std::io::ErrorKind::Unsupported,
			format!("filesystem type {filesystem:#x} has no kernel-local lock scope"),
		));
	}

	let boot_id = fs::read_to_string("/proc/sys/kernel/random/boot_id").await?;
	let boot_id = uuid::Uuid::parse_str(boot_id.trim())
		.map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
	Ok(LockScope {
		kernel_boot_id: boot_id.into_bytes(),
		device: metadata.dev(),
		inode: metadata.ino(),
	})
}

#[cfg(not(target_os = "linux"))]
async fn lock_scope(_file: &fs::File, _metadata: &std::fs::Metadata) -> std::io::Result<LockScope> {
	Err(std::io::Error::new(
		std::io::ErrorKind::Unsupported,
		"kernel boot ID unavailable",
	))
}

/// Sums file sizes under `root` without following symbolic links.
fn directory_bytes(root: &Path) -> std::io::Result<u64> {
	let mut total = 0u64;
	let mut pending = vec![root.to_path_buf()];
	while let Some(dir) = pending.pop() {
		let entries = match std::fs::read_dir(&dir) {
			Ok(entries) => entries,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound && dir == root => {
				return Ok(0);
			}
			Err(error) => return Err(error),
		};
		for entry in entries {
			let entry = entry?;
			let kind = entry.file_type()?;
			if kind.is_dir() {
				pending.push(entry.path());
			} else if kind.is_file() {
				total = total.saturating_add(entry.metadata()?.len());
			}
		}
	}
	Ok(total)
}

/// Reads `statvfs` for an existing path. Available space is what an unprivileged writer
/// may use, excluding blocks reserved for root.
fn statvfs(path: &Path) -> std::io::Result<DiskSpace> {
	use std::os::unix::ffi::OsStrExt;

	let path = std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| {
		std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains a NUL byte")
	})?;
	let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
	// SAFETY: `path` is NUL-terminated and `stats` is a valid out pointer for one statvfs.
	let result = unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) };
	if result != 0 {
		return Err(std::io::Error::last_os_error());
	}
	// SAFETY: statvfs returned success, so it initialized `stats`.
	let stats = unsafe { stats.assume_init() };
	let fragment = stats.f_frsize as u64;
	Ok(DiskSpace {
		filesystem_id: stats.f_fsid as u64,
		total_bytes: (stats.f_blocks as u64).saturating_mul(fragment),
		available_bytes: (stats.f_bavail as u64).saturating_mul(fragment),
	})
}
