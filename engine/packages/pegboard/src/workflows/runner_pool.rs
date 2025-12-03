use std::hash::{DefaultHasher, Hash, Hasher};

use futures_util::FutureExt;
use gas::prelude::*;
use rivet_types::{keys, runner_configs::RunnerConfigKind};

use super::serverless;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {
	pub namespace_id: Id,
	pub runner_name: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct LifecycleState {
	runners: Vec<RunnerState>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RunnerState {
	receiver_wf_id: Id,
	details_hash: u64,
}

#[workflow]
pub async fn pegboard_runner_pool(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	ctx.loope(LifecycleState::default(), |ctx, state| {
		let input = input.clone();
		async move {
			// Get desired count -> drain and start counts
			let ReadDesiredOutput::Desired {
				desired_count,
				details_hash,
			} = ctx.activity(ReadDesiredInput {
				namespace_id: input.namespace_id,
				runner_name: input.runner_name.clone(),
			})
			.await?
			else {
				return Ok(Loop::Break(()));
			};

			// Remove runners that have an outdated hash. This is done outside of the below draining mechanism
			// because we drain specific runners, not just a number of runners
			let (new, outdated) = std::mem::take(&mut state.runners)
				.into_iter()
				.partition::<Vec<_>, _>(|r| r.details_hash == details_hash);
			state.runners = new;

			for runner in outdated {
				// TODO: Spawn sub wf to process these so this is not blocking the loop
				ctx.signal(serverless::receiver::Drain {})
					.to_workflow_id(runner.receiver_wf_id)
					.send()
					.await?;
			}

			let drain_count = state.runners.len().saturating_sub(desired_count);
			let start_count = desired_count.saturating_sub(state.runners.len());

			// Drain unnecessary runners
			if drain_count != 0 {
				// TODO: Implement smart logic of draining runners with the lowest allocated actors
				let draining_runners = state.runners.iter().take(drain_count).collect::<Vec<_>>();

				// TODO: Spawn sub wf to process these so this is not blocking the loop
				for runner in draining_runners {
					ctx.signal(serverless::receiver::Drain {})
						.to_workflow_id(runner.receiver_wf_id)
						.send()
						.await?;
				}
			}

			// Dispatch new runner workflows
			if start_count != 0 {
				// TODO: Spawn sub wf to process these so this is not blocking the loop
				for _ in 0..start_count {
					let receiver_wf_id = ctx
						.workflow(serverless::receiver::Input {
							pool_wf_id: ctx.workflow_id(),
							namespace_id: input.namespace_id,
							runner_name: input.runner_name.clone(),
						})
						.tag("namespace_id", input.namespace_id)
						.tag("runner_name", input.runner_name.clone())
						.dispatch()
						.await?;

					state.runners.push(RunnerState {
						receiver_wf_id,
						details_hash,
					});
				}
			}

			// Wait for Bump or serverless signals until we tick again
			for sig in ctx.listen_n::<Main>(1024).await? {
				match sig {
					Main::OutboundConnDrainStarted(sig) => {
						state
							.runners
							.retain(|r| r.receiver_wf_id != sig.receiver_wf_id);
					}
					Main::Bump(_) => {}
				}
			}

			Ok(Loop::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ReadDesiredInput {
	namespace_id: Id,
	runner_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
enum ReadDesiredOutput {
	Desired {
		desired_count: usize,
		details_hash: u64,
	},
	Stop,
}

#[activity(ReadDesired)]
async fn read_desired(ctx: &ActivityCtx, input: &ReadDesiredInput) -> Result<ReadDesiredOutput> {
	let udb_pool = ctx.udb()?;
	let (runner_config_res, desired_slots) = tokio::try_join!(
		ctx.op(crate::ops::runner_config::get::Input {
			runners: vec![(input.namespace_id, input.runner_name.clone())],
			bypass_cache: false,
		}),
		udb_pool.run(|tx| async move {
			let tx = tx.with_subspace(keys::pegboard::subspace());

			let desired_slots = tx
				.read_opt(
					&keys::pegboard::ns::ServerlessDesiredSlotsKey {
						namespace_id: input.namespace_id,
						runner_name: input.runner_name.clone(),
					},
					universaldb::utils::IsolationLevel::Serializable,
				)
				.await?;

			Ok(desired_slots.unwrap_or_default())
		}),
	)?;
	let Some(runner_config) = runner_config_res.into_iter().next() else {
		return Ok(ReadDesiredOutput::Stop);
	};

	let RunnerConfigKind::Serverless {
		url,
		headers,

		slots_per_runner,
		min_runners,
		max_runners,
		runners_margin,
		..
	} = runner_config.config.kind
	else {
		return Ok(ReadDesiredOutput::Stop);
	};

	let adjusted_desired_slots = if desired_slots < 0 {
		tracing::error!(
			namespace_id=%input.namespace_id,
			runner_name=%input.runner_name,
			?desired_slots,
			"negative desired slots, scaling to 0"
		);
		0
	} else {
		desired_slots
	};

	// Won't overflow as these values are all in u32 range
	let desired_count = (runners_margin
		+ (adjusted_desired_slots as u32).div_ceil(slots_per_runner))
	.max(min_runners)
	.min(max_runners)
	.try_into()?;

	// Compute consistent hash of serverless details
	let mut hasher = DefaultHasher::new();
	url.hash(&mut hasher);
	let mut sorted_headers = headers.iter().collect::<Vec<_>>();
	sorted_headers.sort();
	sorted_headers.hash(&mut hasher);
	let details_hash = hasher.finish();

	Ok(ReadDesiredOutput::Desired {
		desired_count,
		details_hash,
	})
}

#[signal("pegboard_runner_pool_bump")]
#[derive(Debug)]
pub struct Bump {}

#[signal("pegboard_outbound_conn_drain_started")]
pub struct OutboundConnDrainStarted {
	pub receiver_wf_id: Id,
}

join_signal!(Main {
	Bump,
	OutboundConnDrainStarted,
});
