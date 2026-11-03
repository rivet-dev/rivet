//! A caller's exclusive filesystem lease; the process volume owner retains and joins its runner.
use super::Filesystem;
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};
use tokio::sync::Notify;

pub(crate) struct Completion {
	done: AtomicBool,
	wake: Notify,
}
impl Completion {
	pub(crate) fn new() -> Arc<Self> {
		Arc::new(Self {
			done: AtomicBool::new(false),
			wake: Notify::new(),
		})
	}
	pub(crate) fn finish(&self) {
		self.done.store(true, Ordering::Release);
		self.wake.notify_waiters();
	}
	async fn wait(&self) {
		loop {
			let wake = self.wake.notified();
			tokio::pin!(wake);
			wake.as_mut().enable();
			if self.done.load(Ordering::Acquire) {
				return;
			}
			wake.await;
		}
	}
}
enum LeasePhase {
	Serving,
	Closing,
	Released,
}
pub(crate) struct Lease {
	phase: parking_lot::Mutex<LeasePhase>,
	/// Tells the process owner that the slot may be idle once the lease is released.
	on_release: parking_lot::Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}
impl Lease {
	fn release(&self) {
		{
			let mut phase = self.phase.lock();
			if matches!(*phase, LeasePhase::Released) {
				return;
			}
			*phase = LeasePhase::Released;
		}
		let on_release = self.on_release.lock().clone();
		if let Some(on_release) = on_release {
			on_release();
		}
	}

	pub(crate) fn active(&self) -> bool {
		match *self.phase.lock() {
			LeasePhase::Serving | LeasePhase::Closing => true,
			LeasePhase::Released => false,
		}
	}
}
/// Dropping the lease fences every cloned filesystem immediately. The process
/// owner keeps the slot until its runner and lifecycle close have been cleaned up.
pub struct OpenedFilesystem {
	filesystem: Arc<Filesystem>,
	lease: Arc<Lease>,
	completion: Arc<Completion>,
}

/// Revokes the exact owner's permission without retaining its exclusive lease. A missing
/// lease has already been released by its owner's destructor; this never targets a successor.
#[derive(Clone)]
pub struct FilesystemRetirement {
	filesystem: std::sync::Weak<Filesystem>,
	lease: std::sync::Weak<Lease>,
}

impl FilesystemRetirement {
	pub fn retire(&self) {
		if let Some(filesystem) = self.filesystem.upgrade() {
			filesystem.fence_with(crate::fences::FenceReason::OwnerReleased);
		}
		if let Some(lease) = self.lease.upgrade() {
			lease.release();
		}
	}
}

impl OpenedFilesystem {
	pub(crate) fn new(filesystem: Arc<Filesystem>, completion: Arc<Completion>) -> Self {
		Self {
			filesystem,
			lease: Arc::new(Lease {
				phase: parking_lot::Mutex::new(LeasePhase::Serving),
				on_release: parking_lot::Mutex::new(None),
			}),
			completion,
		}
	}

	/// Runs `signal` each time the lease is released, after the phase is updated.
	pub(crate) fn on_release(self, signal: Arc<dyn Fn() + Send + Sync>) -> Self {
		*self.lease.on_release.lock() = Some(signal);
		self
	}
	pub(crate) fn lease(&self) -> std::sync::Weak<Lease> {
		Arc::downgrade(&self.lease)
	}
	pub fn filesystem(&self) -> &Arc<Filesystem> {
		&self.filesystem
	}

	/// Allows a lane to revoke this lease without acquiring a request's actor or pages lock.
	pub fn retirement_handle(&self) -> FilesystemRetirement {
		FilesystemRetirement {
			filesystem: Arc::downgrade(&self.filesystem),
			lease: Arc::downgrade(&self.lease),
		}
	}

	/// Revoke this owner's serving permission without waiting for callers holding filesystem
	/// context to finish. Their cloned filesystem stays alive but is permanently fenced.
	pub fn retire(&self) {
		self.filesystem
			.fence_with(crate::fences::FenceReason::OwnerReleased);
		self.lease.release();
	}

	pub async fn shutdown(self) -> anyhow::Result<()> {
		{
			let mut phase = self.lease.phase.lock();
			if matches!(*phase, LeasePhase::Serving) {
				*phase = LeasePhase::Closing;
			}
		}
		let result = self.filesystem.close().await;
		self.lease.release();
		self.completion.wait().await;
		result?;
		Ok(())
	}
}
impl Drop for OpenedFilesystem {
	fn drop(&mut self) {
		self.retire();
	}
}
