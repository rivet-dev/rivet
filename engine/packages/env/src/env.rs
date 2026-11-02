//! Shared environment facade for clocks, rand streams, and Tokio tasks.

use crate::{Clock, RngStream, TaskHandle};
use std::{convert::Infallible, future::Future, pin::Pin, sync::Arc};

/// Monotonic boot time includes suspension; use it for authority deadlines. Unix time
/// is for wall timestamps only. Both clocks are in milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Clocks {
	pub boot_ms: u64,
	pub unix_ms: i64,
}

#[derive(
	Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct NodeId(pub bytes::Bytes);

/// Runtime capabilities shared by production and simulation.
///
/// Select the backend once at startup and pass the environment to components.
/// Configuration, credentials, membership, retries, and ownership policy remain
/// application concerns.
pub trait Env: Send + Sync + 'static {
	fn clock(&self) -> Arc<dyn Clock>;

	/// Backends without local disk return Unsupported on filesystem operations.
	fn fs(&self) -> crate::fs::Fs {
		crate::fs::Fs::default()
	}

	/// Backends without host hardware facts return Unsupported on host reads.
	fn host(&self) -> crate::host::Host {
		crate::host::Host::default()
	}

	/// CPU nanoseconds consumed by the current OS thread. Sample before/after one synchronous
	/// Future::poll only; values cannot be compared across awaits/threads or used as lease time.
	/// Backends without metering return an explicit error so callers omit uncertain load samples.
	fn thread_cpu_time_ns(&self) -> Result<u64, crate::ClockError> {
		Err(crate::ClockError::Source(
			"thread CPU metering unavailable".into(),
		))
	}

	/// Reopening a label shares its draw sequence. Give independent component
	/// instances distinct stable labels; do not allocate labels per request.
	fn rng(&self, label: &str) -> RngStream;

	/// Dropping the handle detaches the task. Its owner must join it to observe
	/// failure, and cancel/drain it at shutdown (for example with `TaskScope`).
	/// Simulation additionally cancels every task when its node crashes.
	#[track_caller]
	fn spawn(
		&self,
		fut: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
	) -> TaskHandle;

	/// Execute bounded synchronous CPU work. It must not perform I/O, consult host
	/// clocks, or detach work; simulations execute it as one deterministic step.
	/// Already-running work cannot be cancelled and must bound its own workload.
	fn spawn_blocking(&self, _work: Box<dyn FnOnce() + Send>) -> anyhow::Result<TaskHandle> {
		anyhow::bail!("environment does not support blocking work")
	}

	/// Runs a synchronous adapter, such as SQLite, whose callbacks await injected
	/// async I/O. Unlike CPU work this cannot run as one simulation step. Backends
	/// without a blocking I/O executor return an explicit capability error.
	fn spawn_blocking_io(&self, _work: Box<dyn FnOnce() + Send>) -> anyhow::Result<TaskHandle> {
		anyhow::bail!("environment does not support blocking I/O callbacks")
	}
	/// Sends an owned I/O request from a blocking adapter to its async owner.
	/// Available only inside `spawn_blocking_io`; callbacks never poll I/O on
	/// the blocking thread. Operations retain their own deadlines and fences.
	fn submit_io(
		&self,
		_future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
	) -> anyhow::Result<()> {
		anyhow::bail!("environment does not support blocking I/O callbacks")
	}

	/// Terminate at the runtime boundary. Awaiting this never resumes the caller.
	/// This is fail-stop termination, without graceful cleanup or telemetry flush.
	fn exit(&self, code: i32) -> Pin<Box<dyn Future<Output = Infallible> + Send + '_>>;
}

impl dyn Env {
	/// Create a task scope whose tasks are spawned on this environment.
	pub fn task_scope(self: &Arc<Self>) -> crate::TaskScope {
		crate::TaskScope::new(self.clone())
	}
}
