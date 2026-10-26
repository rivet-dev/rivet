//! Tokio runtime capabilities, OS clocks, and rand streams.

#![allow(clippy::disallowed_methods)]
pub mod clock;
pub use clock::TokioClock;

use crate::{Clock, Env, NodeId, RngStream, TaskHandle};
use futures_util::FutureExt;
use parking_lot::Mutex;
use std::{collections::BTreeMap, convert::Infallible, future::Future, pin::Pin, sync::Arc};

pub struct ProductionEnv {
	handle: ::tokio::runtime::Handle,
	clock: Arc<dyn Clock>,
	seed: u64,
	streams: Mutex<BTreeMap<String, RngStream>>,
}

impl ProductionEnv {
	/// Construct inside the Tokio runtime that will own the tasks.
	pub fn new() -> Result<Self, crate::ClockError> {
		Ok(Self::with_clock(Arc::new(TokioClock::new()?)))
	}

	/// Share this clock with adapters that accept absolute deadlines.
	pub fn with_clock(clock: Arc<dyn Clock>) -> Self {
		Self {
			handle: ::tokio::runtime::Handle::current(),
			clock,
			seed: RngStream::production_seed(),
			streams: Mutex::new(BTreeMap::new()),
		}
	}
}

type IoRequest = Pin<Box<dyn Future<Output = ()> + Send>>;
thread_local! {
	static BLOCKING_IO: std::cell::RefCell<Option<tokio::sync::mpsc::Sender<IoRequest>>> = const {std::cell::RefCell::new(None)};
}
struct BlockingIoGuard;
impl Drop for BlockingIoGuard {
	fn drop(&mut self) {
		BLOCKING_IO.with(|requests| requests.borrow_mut().take());
	}
}

impl Env for ProductionEnv {
	fn fs(&self) -> crate::fs::Fs {
		crate::fs::Fs::new(Arc::new(fs::ProductionFs))
	}
	fn host(&self) -> crate::host::Host {
		crate::host::Host::new(Arc::new(host::ProductionHost))
	}
	fn thread_cpu_time_ns(&self) -> Result<u64, crate::ClockError> {
		clock::thread_cpu_time_ns()
	}
	fn clock(&self) -> Arc<dyn Clock> {
		self.clock.clone()
	}

	fn rng(&self, label: &str) -> RngStream {
		self.streams
			.lock()
			.entry(label.to_owned())
			.or_insert_with(|| RngStream::seeded(self.seed, &NodeId(Default::default()), label))
			.clone()
	}

	#[track_caller]
	fn spawn(&self, fut: Pin<Box<dyn Future<Output = ()> + Send>>) -> TaskHandle {
		TaskHandle::new(self.handle.spawn(fut))
	}

	fn spawn_blocking(&self, work: Box<dyn FnOnce() + Send>) -> anyhow::Result<TaskHandle> {
		Ok(TaskHandle::new(self.handle.spawn_blocking(work)))
	}

	fn spawn_blocking_io(&self, work: Box<dyn FnOnce() + Send>) -> anyhow::Result<TaskHandle> {
		// One synchronous callback can have only one outstanding request. The
		// channel stays live exactly as long as this blocking worker's guard.
		let (send, mut receive) = tokio::sync::mpsc::channel::<IoRequest>(1);
		let worker = self.handle.spawn_blocking(move || {
			BLOCKING_IO.with(|requests| *requests.borrow_mut() = Some(send));
			let _guard = BlockingIoGuard;
			work();
		});
		Ok(TaskHandle::new(self.handle.spawn(async move {
			while let Some(request) = receive.recv().await {
				if let Err(panic) = std::panic::AssertUnwindSafe(request).catch_unwind().await {
					// Dropping the request closes its reply. Join the awakened
					// worker before reporting the original I/O panic.
					drop(receive);
					let _ = worker.await;
					std::panic::resume_unwind(panic);
				}
			}
			if let Err(error) = worker.await {
				if error.is_panic() {
					std::panic::resume_unwind(error.into_panic());
				}
			}
		})))
	}
	fn submit_io(&self, future: IoRequest) -> anyhow::Result<()> {
		BLOCKING_IO.with(|requests| {
			let requests = requests.borrow();
			let send = requests
				.as_ref()
				.ok_or_else(|| anyhow::anyhow!("I/O requests require an Env blocking-I/O task"))?;
			send.try_send(future)
				.map_err(|_| anyhow::anyhow!("blocking I/O owner ended"))
		})
	}

	fn exit(&self, code: i32) -> Pin<Box<dyn Future<Output = Infallible> + Send + '_>> {
		Box::pin(async move { std::process::exit(code) })
	}
}

/// OS entropy for credentials and keys at the application's authentication boundary.
/// Keep this outside simulated state transitions; inject fixed credentials into tests.
#[allow(clippy::disallowed_types)]
pub fn secure_random(bytes: &mut [u8]) -> Result<(), ::rand::Error> {
	use ::rand::RngCore;
	::rand::rngs::OsRng.try_fill_bytes(bytes)
}

pub mod fs;
pub mod host;
pub mod http;

pub mod signals;

#[cfg(feature = "slatedb")]
pub mod slatedb;
