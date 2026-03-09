//! Test workflows that replicate the patterns used in real actor/runner workflows.
//!
//! These workflows heavily use loops and signals, which are the patterns most likely to expose
//! Postgres-specific concurrency issues in UDB.

use futures_util::FutureExt;
use gas::prelude::*;

// -- Signals --

#[signal("ping")]
#[derive(Debug)]
pub struct PingSignal {
	pub iteration: usize,
	pub payload: String,
}

#[signal("trigger")]
#[derive(Debug)]
pub struct TriggerSignal {}

// -- LoopSignal Workflow --
// Simulates the actor lifecycle pattern: loop that listens for signals, processes them, and
// continues until a condition is met.

#[derive(Debug, Serialize, Deserialize)]
pub struct LoopSignalInput {
	pub max_iterations: usize,
}

#[workflow(LoopSignalWorkflow)]
pub async fn loop_signal(ctx: &mut WorkflowCtx, input: &LoopSignalInput) -> Result<usize> {
	let max_iterations = input.max_iterations;

	let count = ctx
		.loope(0usize, move |ctx, state| {
			async move {
				if *state >= max_iterations {
					return Ok(Loop::Break(*state));
				}

				// Listen for a ping signal (this is the main stress point)
				let signal = ctx.listen::<PingSignal>().await?;

				tracing::debug!(
					iteration = signal.iteration,
					payload = %signal.payload,
					state = *state,
					"received ping signal"
				);

				// Do an activity to write back (simulates actor state updates)
				ctx.activity(ProcessSignalInput {
					iteration: signal.iteration,
					payload: signal.payload,
				})
				.await?;

				*state += 1;

				Ok(Loop::Continue)
			}
			.boxed()
		})
		.await?;

	Ok(count)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ProcessSignalInput {
	iteration: usize,
	payload: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessSignalOutput {
	processed: bool,
}

#[activity(ProcessSignal)]
async fn process_signal(
	_ctx: &ActivityCtx,
	input: &ProcessSignalInput,
) -> Result<ProcessSignalOutput> {
	// Simulate some processing work
	tracing::debug!(
		iteration = input.iteration,
		payload = %input.payload,
		"processing signal"
	);

	Ok(ProcessSignalOutput { processed: true })
}

// -- BusyLoop Workflow --
// Pure loop stress test. No signals, just rapid looping with activities to hammer the DB.

#[derive(Debug, Serialize, Deserialize)]
pub struct BusyLoopInput {
	pub iterations: usize,
}

#[workflow(BusyLoopWorkflow)]
pub async fn busy_loop(ctx: &mut WorkflowCtx, input: &BusyLoopInput) -> Result<usize> {
	let iterations = input.iterations;

	let count = ctx
		.loope(0usize, move |ctx, state| {
			async move {
				if *state >= iterations {
					return Ok(Loop::Break(*state));
				}

				// Do a lightweight activity each iteration
				ctx.activity(IncrementCounterInput {
					current: *state,
				})
				.await?;

				*state += 1;

				Ok(Loop::Continue)
			}
			.boxed()
		})
		.await?;

	Ok(count)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct IncrementCounterInput {
	current: usize,
}

#[activity(IncrementCounter)]
async fn increment_counter(
	_ctx: &ActivityCtx,
	input: &IncrementCounterInput,
) -> Result<usize> {
	Ok(input.current + 1)
}

// -- SignalChain Workflow --
// Listens for a trigger, then dispatches sub-workflows that signal each other in a chain.
// Tests concurrent signal publishing from within workflows.

#[derive(Debug, Serialize, Deserialize)]
pub struct SignalChainInput {
	pub chain_length: usize,
}

#[workflow(SignalChainWorkflow)]
pub async fn signal_chain(ctx: &mut WorkflowCtx, input: &SignalChainInput) -> Result<()> {
	// Wait for trigger
	ctx.listen::<TriggerSignal>().await?;

	tracing::debug!(chain_length = input.chain_length, "signal chain triggered");

	// Do a series of activities to simulate chain processing
	for i in 0..input.chain_length {
		ctx.activity(ChainStepInput { step: i }).await?;
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ChainStepInput {
	step: usize,
}

#[activity(ChainStep)]
async fn chain_step(_ctx: &ActivityCtx, input: &ChainStepInput) -> Result<()> {
	tracing::debug!(step = input.step, "chain step processing");
	Ok(())
}
