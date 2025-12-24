use gas::prelude::*;
use rivet_types::actor::RunnerPoolError;

/// Number of consecutive successes required to clear an active error.
/// Prevents a single success from clearing error during flapping.
const CONSECUTIVE_SUCCESSES_TO_CLEAR: u32 = 3;

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
	/// Error is only cleared after reaching CONSECUTIVE_SUCCESSES_TO_CLEAR threshold.
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

	ctx.lupe()
		// Txn sizes can quickly get large in this workflow, need to commit loop more often
		.commit_interval(1)
		.run(|ctx, _| {
			Box::pin(async move {
				let signals = ctx.listen_n_with_timeout::<Main>(100, 256).await?;

				let signals_inner = signals
					.into_iter()
					.map(|s| match s {
						Main::ReportSuccess(x) => MainInner::ReportSuccess(x),
						Main::ReportError(x) => MainInner::ReportError(x),
						Main::Shutdown(x) => MainInner::Shutdown(x),
					})
					.collect();

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
				if state.consecutive_successes >= CONSECUTIVE_SUCCESSES_TO_CLEAR {
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
