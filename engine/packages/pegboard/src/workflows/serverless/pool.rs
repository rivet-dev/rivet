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
	runners: Vec<Id>,
}

#[workflow]
pub async fn pegboard_serverless_pool(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	ctx.loope(LifecycleState::default(), |ctx, state| {
		let input = input.clone();
		async move {
			// 1. Remove completed connections
			let completed_runners = ctx
				.activity(GetCompletedInput {
					runners: state.runners.clone(),
				})
				.await?;

			state.runners.retain(|r| !completed_runners.contains(r));

			// 2. Get desired count -> drain and start counts
			let ReadDesiredOutput::Desired(desired_count) = ctx
				.activity(ReadDesiredInput {
					namespace_id: input.namespace_id,
					runner_name: input.runner_name.clone(),
				})
				.await?
			else {
				return Ok(Loop::Break(()));
			};

			let drain_count = state.runners.len().saturating_sub(desired_count);
			let start_count = desired_count.saturating_sub(state.runners.len());

			// 3. Drain old runners
			if drain_count != 0 {
				// TODO: Implement smart logic of draining runners with the lowest allocated actors
				let draining_runners = state.runners.iter().take(drain_count).collect::<Vec<_>>();

				for wf_id in draining_runners {
					ctx.signal(runner::Drain {})
						.to_workflow_id(*wf_id)
						.send()
						.await?;
				}
			}

			// 4. Dispatch new runner workflows
			if start_count != 0 {
				for _ in 0..start_count {
					let wf_id = ctx
						.workflow(runner::Input {
							pool_wf_id: ctx.workflow_id(),
							namespace_id: input.namespace_id,
							runner_name: input.runner_name.clone(),
						})
						.tag("namespace_id", input.namespace_id)
						.tag("runner_name", input.runner_name.clone())
						.dispatch()
						.await?;

					state.runners.push(wf_id);
				}
			}

			// Wait for Bump or runner update signals until we tick again
			match ctx.listen::<Main>().await? {
				Main::RunnerDrainStarted(sig) => {
					state.runners.retain(|wf_id| *wf_id != sig.runner_wf_id);
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
	Desired(usize),
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

	Ok(ReadDesiredOutput::Desired(desired_count))
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
