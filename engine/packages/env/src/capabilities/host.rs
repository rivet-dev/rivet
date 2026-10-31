//! Injectable facade for host hardware facts that Linux exposes through `/proc` and `/sys`:
//! CPU, memory, the block device under a path and that device's I/O counters.

use crate::fs::FsFuture;
use std::{io, path::Path, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuInfo {
	/// Logical CPUs online on the host, which may exceed what a cgroup quota lets this
	/// process use.
	pub logical_cpus: u32,
	/// Model name as the kernel reports it, when it reports one.
	pub model: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryInfo {
	pub total_bytes: u64,
	/// Memory limit of this process's cgroup. `None` when the cgroup has no limit.
	pub limit_bytes: Option<u64>,
}

/// The block device holding a filesystem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockDevice {
	/// Kernel device name, such as `nvme0n1p1`. It names the device in [`Host::disk_io`].
	pub name: String,
	/// Whether the device reports rotating media. `None` when the kernel does not say.
	pub rotational: Option<bool>,
}

/// Cumulative I/O counters of one block device since the host booted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiskIo {
	pub read_bytes: u64,
	pub written_bytes: u64,
	pub writes_completed: u64,
	/// Time the device had at least one request in flight.
	pub busy_ms: u64,
}

/// Backend implemented by the production host and simulations.
pub trait HostBackend: Send + Sync {
	fn cpu(&self) -> FsFuture<'_, CpuInfo>;
	fn memory(&self) -> FsFuture<'_, MemoryInfo>;

	/// Reads the block device under the filesystem holding the existing `path`. Returns
	/// `None` for filesystems without one, such as tmpfs or overlay.
	fn block_device<'a>(&'a self, path: &'a Path) -> FsFuture<'a, Option<BlockDevice>>;

	/// Reads the counters of a device named by [`BlockDevice::name`].
	fn disk_io<'a>(&'a self, device: &'a str) -> FsFuture<'a, DiskIo>;
}

/// Cloneable, injected host. An unavailable backend returns Unsupported rather than
/// reading the real host from an environment without this capability.
#[derive(Clone, Default)]
pub struct Host(Option<Arc<dyn HostBackend>>);

impl Host {
	pub fn new(backend: Arc<dyn HostBackend>) -> Self {
		Self(Some(backend))
	}

	fn backend(&self) -> io::Result<&dyn HostBackend> {
		self.0.as_deref().ok_or_else(|| {
			io::Error::new(io::ErrorKind::Unsupported, "host capability unavailable")
		})
	}

	pub async fn cpu(&self) -> io::Result<CpuInfo> {
		self.backend()?.cpu().await
	}

	pub async fn memory(&self) -> io::Result<MemoryInfo> {
		self.backend()?.memory().await
	}

	pub async fn block_device(&self, path: impl AsRef<Path>) -> io::Result<Option<BlockDevice>> {
		self.backend()?.block_device(path.as_ref()).await
	}

	pub async fn disk_io(&self, device: &str) -> io::Result<DiskIo> {
		self.backend()?.disk_io(device).await
	}
}
