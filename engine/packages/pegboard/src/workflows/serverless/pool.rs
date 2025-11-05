use std::collections::HashMap;

use super::runner;
use futures_util::FutureExt;
use gas::{db::WorkflowData, prelude::*};
use rivet_types::{keys, runner_configs::RunnerConfigKind};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {
	pub namespace_id: Id,
	pub runner_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerlessRunnerConfig {
	url: String,
	headers: HashMap<String, String>,
	request_lifespan: u32,
	slots_per_runner: u32,
	min_runners: u32,
	max_runners: u32,
	runners_margin: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct LifecycleState {
	runners: Vec<Id>,
	config: ServerlessRunnerConfig,
	first_loop: bool,
}

impl LifecycleState {
	fn new(config: ServerlessRunnerConfig) -> Self {
		Self {
			runners: Vec::new(),
			config: config,
			first_loop: true,
		}
	}
}

#[workflow]
pub async fn pegboard_serverless_pool(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	let namespace_name = ctx
		.activity(ReadNamespaceNameInput {
			namespace_id: input.namespace_id,
		})
		.await?;

	let config = ctx
		.activity(ReadConfigInput {
			namespace_id: input.namespace_id,
			runner_name: input.runner_name.clone(),
		})
		.await?;

	ctx.loope(LifecycleState::new(config), |ctx, state| {
		let input = input.clone();
		let namespace_name = namespace_name.clone();

		async move {
			if state.first_loop {
				state.first_loop = false
			} else {
				// Wait for Bump or runner update signals until we tick
				match ctx.listen::<Main>().await? {
					Main::RunnerDrainStarted(sig) => {
						let pos_to_drain = state
							.runners
							.iter()
							.position(|wf_id| *wf_id == sig.runner_wf_id);
						if let Some(pos_to_drain) = pos_to_drain {
							state.runners.remove(pos_to_drain);
						}
					}
					Main::BumpConfig(_) => {
						// Update config
						state.config = ctx
							.activity(ReadConfigInput {
								namespace_id: input.namespace_id,
								runner_name: input.runner_name.clone(),
							})
							.await?;
					}
					Main::Bump(_) => {}
				}
			}

			// 1. Remove completed connections
			let completed_runners = ctx
				.activity(GetCompletedInput {
					runners: state.runners.clone(),
				})
				.await?;

			state.runners.retain(|r| !completed_runners.contains(r));

			// 2. Get desired count -> drain and start counts
			let desired_count = ctx
				.activity(ReadDesiredInput {
					namespace_id: input.namespace_id,
					runner_name: input.runner_name.clone(),
					slots_per_runner: state.config.slots_per_runner as i64,
					min_runners: state.config.min_runners as i64,
					max_runners: state.config.max_runners as i64,
					runners_margin: state.config.runners_margin as i64,
				})
				.await?;

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
							namespace_name: namespace_name.clone(),
							runner_name: input.runner_name.clone(),
							url: state.config.url.clone(),
							headers: state.config.headers.clone(),
							request_lifespan: state.config.request_lifespan,
							slots_per_runner: state.config.slots_per_runner,
						})
						.tag("namespace_id", input.namespace_id)
						.tag("runner_name", input.runner_name.clone())
						.dispatch()
						.await?;

					state.runners.push(wf_id);
				}
			}

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ReadNamespaceNameInput {
	namespace_id: Id,
}

#[activity(ReadNamespaceName)]
async fn read_namespace_name(ctx: &ActivityCtx, input: &ReadNamespaceNameInput) -> Result<String> {
	let res = ctx
		.op(namespace::ops::get_global::Input {
			namespace_ids: vec![input.namespace_id],
		})
		.await?;

	let namespace = res.first().context("runner namespace not found")?;

	Ok(namespace.name.clone())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ReadConfigInput {
	namespace_id: Id,
	runner_name: String,
}

#[activity(ReadConfig)]
async fn read_config(ctx: &ActivityCtx, input: &ReadConfigInput) -> Result<ServerlessRunnerConfig> {
	let run_configs = ctx
		.op(crate::ops::runner_config::get::Input {
			runners: vec![(input.namespace_id, input.runner_name.clone())],
			bypass_cache: false,
		})
		.await?;

	let res = run_configs
		.first()
		.context("couldn't find own runner config")?;

	match &res.config.kind {
		&RunnerConfigKind::Normal {} => {
			tracing::error!(
				namespace_id = ?input.namespace_id,
				runner_name = ?input.runner_name,
				"serverless pool running for non-serverless runner config"
			);

			Err(anyhow!(
				"serverless pool running for non-serverless runner config"
			))
		}
		&RunnerConfigKind::Serverless {
			ref url,
			ref headers,
			request_lifespan,
			slots_per_runner,
			min_runners,
			max_runners,
			runners_margin,
		} => Ok(ServerlessRunnerConfig {
			url: url.clone(),
			headers: headers.clone(),
			request_lifespan,
			slots_per_runner,
			min_runners,
			max_runners,
			runners_margin,
		}),
	}
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
	slots_per_runner: i64,
	min_runners: i64,
	max_runners: i64,
	runners_margin: i64,
}

#[activity(ReadDesired)]
async fn read_desired(ctx: &ActivityCtx, input: &ReadDesiredInput) -> Result<usize> {
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
			namespace_id = ?input.namespace_id,
			runner_name = ?input.runner_name,
			?desired_slots,
			"negative desired slots, scaling to 0"
		);
		0
	} else {
		desired_slots
	};

	// Won't overflow as these values are all in u32 range
	let desired_count: usize = (input.runners_margin
		+ rivet_util::math::div_ceil_i64(adjusted_desired_slots, input.slots_per_runner)
			.max(input.min_runners))
	.min(input.max_runners)
	.try_into()?;

	Ok(desired_count)
}

#[signal("pegboard_serverless_bump")]
#[derive(Debug)]
pub struct Bump {}

#[signal("pegboard_serverless_bump_config")]
#[derive(Debug)]
pub struct BumpConfig {}

#[signal("pegboard_serverless_runner_drain_started")]
pub struct RunnerDrainStarted {
	pub runner_wf_id: Id,
}

join_signal!(Main {
	Bump,
	BumpConfig,
	RunnerDrainStarted,
});
