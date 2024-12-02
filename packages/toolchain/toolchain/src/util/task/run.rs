use anyhow::*;
use tokio::sync::{broadcast, mpsc};

use super::{Task, TaskCtxInner, TaskEvent};

use crate::errors;

/// Run config passed to the task.
pub struct RunConfig {
	pub abort_rx: mpsc::Receiver<()>,
	pub shutdown_tx: broadcast::Sender<()>,
	pub shutdown_rx: broadcast::Receiver<()>,
	pub event_tx: mpsc::UnboundedSender<TaskEvent>,
}

/// Handlers used to interface with the task while running.
pub struct RunHandles {
	pub abort_tx: mpsc::Sender<()>,
	pub shutdown_rx: broadcast::Receiver<()>,
	pub event_rx: mpsc::UnboundedReceiver<TaskEvent>,
}

impl RunConfig {
	pub fn build() -> (RunConfig, RunHandles) {
		let (abort_tx, abort_rx) = mpsc::channel(1);
		let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
		let shutdown_rx2 = shutdown_tx.subscribe();
		let (event_tx, event_rx) = mpsc::unbounded_channel();

		(
			RunConfig {
				abort_rx,
				shutdown_tx,
				shutdown_rx: shutdown_rx2,
				event_tx,
			},
			RunHandles {
				abort_tx,
				shutdown_rx,
				event_rx,
			},
		)
	}
}

pub async fn run_task<T>(run_config: RunConfig, input: T::Input) -> Result<T::Output>
where
	T: Task,
{
	// Setup config
	let RunConfig {
		mut abort_rx,
		shutdown_tx,
		shutdown_rx,
		event_tx,
	} = run_config;

	// Create context
	let task_ctx = TaskCtxInner::new(event_tx, shutdown_rx);

	// Run task or wait for abort
	tokio::select! {
		result = T::run(task_ctx.clone(), input) => {
			// Write result
			task_ctx.result(&result)?;

			// Shutdown
			shutdown_tx.send(())?;

			result
		},
		_ = abort_rx.recv() => {
			// Shutdown
			shutdown_tx.send(())?;

			Err(errors::GracefulExit.into())
		},
	}
}
