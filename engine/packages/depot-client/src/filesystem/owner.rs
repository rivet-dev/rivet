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
		*self.phase.lock() = LeasePhase::Released;
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
	pub async fn shutdown(self) -> anyhow::Result<()> {
		*self.lease.phase.lock() = LeasePhase::Closing;
		let result = self.filesystem.close().await;
		self.lease.release();
		self.completion.wait().await;
		result?;
		Ok(())
	}
}
impl Drop for OpenedFilesystem {
	fn drop(&mut self) {
		self.filesystem
			.fence_with(crate::fences::FenceReason::OwnerReleased);
		self.lease.release();
	}
}


