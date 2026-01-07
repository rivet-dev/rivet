use std::time::Duration;

use gas::prelude::*;
use rivet_types::actor::RunnerPoolError;

const SIGNAL_DEBOUNCE: Duration = Duration::from_millis(250);
const SIGNAL_BATCH_SIZE: usize = 1024;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {
	pub namespace_id: Id,
	pub runner_name: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct State {
	/// Persistent error state - set on error, cleared after consecutive successes.
	/// Used to track errors during backoff periods when no new requests are made.
	pub active_error: Option<ActiveError>,

	/// Count of consecutive successes since last error.
	/// Error is only cleared after reaching the configured threshold.
	pub consecutive_successes: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActiveError {
	pub timestamp: i64,
	pub error: RunnerPoolError,
}

#[workflow]
pub async fn pegboard_runner_pool_error_tracker(
	ctx: &mut WorkflowCtx,
	input: &Input,
) -> Result<()> {
	tracing::debug!(
		namespace_id = %input.namespace_id,
		runner_name = %input.runner_name,
		"starting error tracker"
	);

	ctx.activity(InitStateInput {}).await?;

	// Batch receive signals with debounce. This allows us to (a) not require polling if the pool
	// is idle and has no signals and (b) avoid a hot loop by debouncing signal processing.
	ctx.lupe()
		// Txn sizes can quickly get large in this workflow, need to commit loop more often
		.commit_interval(1)
		.run(|ctx, _| {
			Box::pin(async move {
				// Sleep until we receive a signal
				let signals_a = ctx.v(2).listen_n::<Main>(SIGNAL_BATCH_SIZE).await?;

				// Debounce rest of signals if we haven't already reached the batch size
				let remaining_signals = SIGNAL_BATCH_SIZE.saturating_sub(signals_a.len());
				let signals_b = if remaining_signals > 0 {
					ctx.listen_n_with_timeout::<Main>(SIGNAL_DEBOUNCE, remaining_signals)
						.await?
				} else {
					Vec::new()
				};

				let signals_inner = signals_a
					.into_iter()
					.chain(signals_b.into_iter())
					.map(|s| match s {
						Main::ReportSuccess(x) => MainInner::ReportSuccess(x),
						Main::ReportError(x) => MainInner::ReportError(x),
						Main::Shutdown(x) => MainInner::Shutdown(x),
					})
					.collect();

				// Process signals
				let shutdown = ctx
					.activity(ProcessSignalsInput {
						signals: signals_inner,
					})
					.await?;

				if shutdown {
					Ok(Loop::Break(()))
				} else {
					Ok(Loop::Continue)
				}
			})
		})
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct InitStateInput {}

#[activity(InitState)]
pub async fn init_state(ctx: &ActivityCtx, _input: &InitStateInput) -> Result<()> {
	let mut state = ctx.state::<Option<State>>()?;
	*state = Some(State::default());
	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct ProcessSignalsInput {
	pub signals: Vec<MainInner>,
}

/// Returns `true` if shutdown signal received.
#[activity(ProcessSignals)]
pub async fn process_signals(ctx: &ActivityCtx, input: &ProcessSignalsInput) -> Result<bool> {
	let mut state = ctx.state::<State>()?;
	let now = util::timestamp::now();

	for signal in &input.signals {
		match signal {
			MainInner::ReportError(report) => {
				state.active_error = Some(ActiveError {
					timestamp: now,
					error: report.error.clone(),
				});
				state.consecutive_successes = 0;
			}
			MainInner::ReportSuccess(_) => {
				state.consecutive_successes += 1;

				// Only clear error after threshold reached
				let threshold = ctx
					.config()
					.pegboard()
					.runner_pool_error_consecutive_successes_to_clear();
				if state.consecutive_successes >= threshold {
					if state.active_error.is_some() {
						tracing::debug!("clearing active error after consecutive successes");
					}
					state.active_error = None;
				}
			}
			MainInner::Shutdown(_) => {
				return Ok(true);
			}
		}
	}

	Ok(false)
}

#[derive(Debug, Clone, Hash)]
#[signal("pegboard_runner_pool_error_tracker_report_error")]
pub struct ReportError {
	pub error: RunnerPoolError,
}

#[derive(Debug, Clone, Hash)]
#[signal("pegboard_runner_pool_error_tracker_report_success")]
pub struct ReportSuccess {}

#[derive(Debug, Clone, Hash)]
#[signal("pegboard_runner_pool_error_tracker_shutdown")]
pub struct Shutdown {}

join_signal!(Main {
	ReportError,
	ReportSuccess,
	Shutdown,
});

// HACK: Cannot implement `Hash` on `Main`
#[derive(Debug, Serialize, Deserialize, Hash)]
pub enum MainInner {
	ReportError(ReportError),
	ReportSuccess(ReportSuccess),
	Shutdown(Shutdown),
}
