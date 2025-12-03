use std::hash::{DefaultHasher, Hash, Hasher};

use futures_util::FutureExt;
use gas::{db::WorkflowData, prelude::*};
use rivet_types::{keys, runner_configs::RunnerConfigKind};

use super::runner;

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
	/// Serverless runner wf id, not normal runner wf id.
	runner_wf_id: Id,
	details_hash: u64,
}

#[workflow]
pub async fn pegboard_serverless_pool(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
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

			let completed_runners = ctx
				.activity(GetCompletedInput {
					runners: state.runners.iter().map(|r| r.runner_wf_id).collect(),
				})
				.await?;

			// Remove completed connections
			state
				.runners
				.retain(|r| !completed_runners.contains(&r.runner_wf_id));

			// Remove runners that have an outdated hash. This is done outside of the below draining mechanism
			// because we drain specific runners, not just a number of runners
			let (new, outdated) = std::mem::take(&mut state.runners)
				.into_iter()
				.partition::<Vec<_>, _>(|r| r.details_hash == details_hash);
			state.runners = new;

			for runner in outdated {
				ctx.signal(runner::Drain {})
					.to_workflow_id(runner.runner_wf_id)
					.send()
					.await?;
			}

			let drain_count = state.runners.len().saturating_sub(desired_count);
			let start_count = desired_count.saturating_sub(state.runners.len());

			// Drain unnecessary runners
			if drain_count != 0 {
				// TODO: Implement smart logic of draining runners with the lowest allocated actors
				let draining_runners = state.runners.iter().take(drain_count).collect::<Vec<_>>();

				for runner in draining_runners {
					ctx.signal(runner::Drain {})
						.to_workflow_id(runner.runner_wf_id)
						.send()
						.await?;
				}
			}

			// Dispatch new runner workflows
			if start_count != 0 {
				for _ in 0..start_count {
					let runner_wf_id = ctx
						.workflow(runner::Input {
							pool_wf_id: ctx.workflow_id(),
							namespace_id: input.namespace_id,
							runner_name: input.runner_name.clone(),
						})
						.tag("namespace_id", input.namespace_id)
						.tag("runner_name", input.runner_name.clone())
						.dispatch()
						.await?;

					state.runners.push(RunnerState {
						runner_wf_id,
						details_hash,
					});
				}
			}

			// Wait for Bump or runner update signals until we tick again
			match ctx.listen::<Main>().await? {
				Main::RunnerDrainStarted(sig) => {
					state.runners.retain(|r| r.runner_wf_id != sig.runner_wf_id);
				}
				Main::Bump(_) => {}
			}

			Ok(Loop::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetCompletedInput {
	runners: Vec<Id>,
}

#[activity(GetCompleted)]
async fn get_completed(ctx: &ActivityCtx, input: &GetCompletedInput) -> Result<Vec<Id>> {
	Ok(ctx
		.get_workflows(input.runners.clone())
		.await?
		.into_iter()
		// When a workflow has output, it means it has completed
		.filter(WorkflowData::has_output)
		.map(|wf| wf.workflow_id)
		.collect())
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
	let runner_config_res = ctx
		.op(crate::ops::runner_config::get::Input {
			runners: vec![(input.namespace_id, input.runner_name.clone())],
			bypass_cache: false,
		})
		.await?;
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

	let desired_slots = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::pegboard::subspace());

			tx.read(
				&keys::pegboard::ns::ServerlessDesiredSlotsKey {
					namespace_id: input.namespace_id,
					runner_name: input.runner_name.clone(),
				},
				universaldb::utils::IsolationLevel::Serializable,
			)
			.await
		})
		.await?;

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

#[signal("pegboard_serverless_bump")]
#[derive(Debug)]
pub struct Bump {}

#[signal("pegboard_serverless_runner_drain_started")]
pub struct RunnerDrainStarted {
	pub runner_wf_id: Id,
}

join_signal!(Main {
	Bump,
	RunnerDrainStarted,
});
