//! Wraps `tokio::signal` to translate OS termination into injectable shutdown events.

use crate::{Env, TaskHandle, TaskScope, capabilities::shutdown::ShutdownTrigger};
use std::sync::Arc;

/// The bootstrap owner retains this scope until every service has drained.
pub fn listen(env: Arc<dyn Env>, trigger: ShutdownTrigger) -> std::io::Result<Signals> {
	let scope = env.task_scope();
	#[cfg(unix)]
	let task = {
		use tokio::signal::unix::{SignalKind, signal};
		let mut term = signal(SignalKind::terminate())?;
		let mut interrupt = signal(SignalKind::interrupt())?;
		let task_env = env.clone();
		scope.spawn(async move {
			loop {
				let signal = crate::select!(task_env.rng("production.signals");
					signal = term.recv() => signal,
					signal = interrupt.recv() => signal,
				);
				if signal.is_none() {
					task_env.exit(1).await;
				}
				trigger.request(task_env.clock().as_ref());
			}
		})
	};
	#[cfg(windows)]
	let task = {
		let mut interrupt = tokio::signal::windows::ctrl_c()?;
		let task_env = env.clone();
		scope.spawn(async move {
			while interrupt.recv().await.is_some() {
				trigger.request(task_env.clock().as_ref());
			}
		})
	};
	Ok(Signals { scope, task })
}

pub struct Signals {
	scope: TaskScope,
	task: TaskHandle,
}

impl Signals {
	pub async fn shutdown(self) -> Result<(), crate::tokio::task::JoinError> {
		self.scope.shutdown().await;
		self.task.await
	}
}
