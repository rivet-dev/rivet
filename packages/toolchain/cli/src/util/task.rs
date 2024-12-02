use anyhow::*;
use std::{io::Write, process::ExitCode};
use tokio::{
	sync::{broadcast, mpsc},
	task::block_in_place,
};
use toolchain::util::task::{self, TaskEvent};

/// Runs a task in a CLI-friendly way.
pub async fn run_task<T>(output_style: TaskOutputStyle, input: T::Input) -> Result<T::Output>
where
	T: task::Task,
{
	let (run_config, handles) = task::RunConfig::build();

	// Spawn aborter
	tokio::spawn(abort_handler(handles.abort_tx, handles.shutdown_rx));

	// Spawn event handler
	let event_join_handle = tokio::spawn(event_handler(handles.event_rx, output_style));

	// Run task
	let result = task::run_task::<T>(run_config, input).await;

	// Wait for logger to shut down
	match event_join_handle.await {
		Result::Ok(_) => {}
		Err(err) => eprintln!("error waiting for event handle: {err:?}"),
	};

	result
}

/// Runs a task and returns an exit code. Useful for commands that are a thin wrapper around a
/// task.
pub async fn run_task_simple<T>(input: T::Input) -> ExitCode
where
	T: task::Task,
{
	match run_task::<T>(TaskOutputStyle::PlainNoResult, input).await {
		Result::Ok(_) => ExitCode::SUCCESS,
		Err(e) => {
			eprintln!("Error: {e:?}");
			ExitCode::from(1)
		}
	}
}

pub async fn run_task_json(
	output_style: TaskOutputStyle,
	name: &str,
	input_json: &str,
) -> Result<task::RunTaskJsonOutput> {
	let (run_config, handles) = task::RunConfig::build();

	// Spawn aborter
	tokio::spawn(abort_handler(handles.abort_tx, handles.shutdown_rx));

	// Spawn event handler
	let event_join_handle = tokio::spawn(event_handler(handles.event_rx, output_style));

	// Run task
	let result = toolchain::tasks::run_task_json(run_config, name, input_json).await;

	// Wait for logger to shut down
	match event_join_handle.await {
		Result::Ok(_) => {}
		Err(err) => eprintln!("error waiting for event handle: {err:?}"),
	};

	Ok(result)
}

/// Handles aborting the task.
async fn abort_handler(abort_tx: mpsc::Sender<()>, mut shutdown_rx: broadcast::Receiver<()>) {
	tokio::select! {
		result = tokio::signal::ctrl_c() => {
			match result {
				Result::Ok(_) => {}
				Err(err) => {
					eprintln!("error waiting for ctrl c: {err:?}");
				}
			}

			// Abort task
			let _ = abort_tx.send(()).await;
		}
		_ = shutdown_rx.recv() => {
			// Stop waiting
		}
	}
}

/// Handles output from the task.
async fn event_handler(
	mut event_rx: mpsc::UnboundedReceiver<TaskEvent>,
	output_style: TaskOutputStyle,
) {
	let mut stdout = std::io::stdout();
	let mut stderr = std::io::stdout();
	while let Some(event) = event_rx.recv().await {
		block_in_place(|| {
			print_event(&mut stdout, &mut stderr, &event, output_style);
		});
	}
}

/// Prints an event depending on the output style.
fn print_event(
	stdout: &mut impl Write,
	stderr: &mut impl Write,
	event: &TaskEvent,
	output_style: TaskOutputStyle,
) {
	match output_style {
		TaskOutputStyle::None => {}
		TaskOutputStyle::Json => {
			if let Err(err) = serde_json::to_writer(&mut *stdout, event) {
				eprintln!("failed to serialize output: {err:?}");
			}
			writeln!(stdout).unwrap();
		}
		TaskOutputStyle::Plain => match event {
			TaskEvent::Log(x) => {
				if let Err(err) = writeln!(stderr, "{x}") {
					eprintln!("failed to write output: {err:?}");
				}
			}
			TaskEvent::Result { result } => {
				if let Err(err) = writeln!(stdout, "{}", serde_json::to_string(&result).unwrap()) {
					eprintln!("failed to serialize output: {err:?}");
				}
			}
		},
		TaskOutputStyle::PlainNoResult => match event {
			TaskEvent::Log(x) => {
				if let Err(err) = writeln!(stderr, "{x}") {
					eprintln!("failed to write output: {err:?}");
				}
			}
			_ => {}
		},
	}
}

#[derive(Copy, Clone)]
pub enum TaskOutputStyle {
	/// Does not output anything.
	None,
	/// Writes all events to stdout in JSON.
	Json,
	/// Writes logs to stderr and result to stdout.
	Plain,
	/// Writes logs to stderr but does not return result.
	PlainNoResult,
}

impl Default for TaskOutputStyle {
	fn default() -> Self {
		Self::Plain
	}
}
