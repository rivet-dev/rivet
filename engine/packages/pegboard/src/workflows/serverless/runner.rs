use std::collections::HashMap;

use super::{connection, pool};
use futures_util::FutureExt;
use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {
	pub pool_wf_id: Id,
	pub namespace_id: Id,
	pub runner_name: String,
	pub namespace_name: String,
	pub url: String,
	pub headers: HashMap<String, String>,
	pub request_lifespan: u32,
	pub slots_per_runner: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct State {
	pub is_draining: bool,
}

impl State {
	fn new() -> Self {
		Self { is_draining: false }
	}
}

#[workflow]
pub async fn pegboard_serverless_runner(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	ctx.activity(InitStateInput {}).await?;

	let conn_wf_id = ctx
		.workflow(connection::Input {
			runner_wf_id: ctx.workflow_id(),
			namespace_id: input.namespace_id,
			runner_name: input.runner_name.clone(),
			namespace_name: input.namespace_name.clone(),
			url: input.url.clone(),
			headers: input.headers.clone(),
			request_lifespan: input.request_lifespan,
			slots_per_runner: input.slots_per_runner,
		})
		.dispatch()
		.await?;

	ctx.repeat(|ctx| {
		let pool_wf_id = input.pool_wf_id;

		async move {
			match ctx.listen::<Tunnel>().await? {
				Tunnel::ConnectionDrainStarted(_) => {
					ctx.signal(pool::RunnerDrainStarted {
						runner_wf_id: ctx.workflow_id(),
					})
					.to_workflow_id(pool_wf_id)
					.send()
					.await?;

					// If the drain started we can stop our runner
					return Ok(Loop::Break(()));
				}
				Tunnel::Drain(_) => {
					ctx.msg(connection::Drain {})
						.tag("workflow_id", conn_wf_id)
						.send()
						.await?;

					ctx.activity(MarkAsDrainingInput {}).await?;
				}
			}

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	// Runner is drained, connection already started draining, we can exit

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InitStateInput {}

#[activity(InitState)]
async fn init_state(ctx: &ActivityCtx, input: &InitStateInput) -> Result<()> {
	let mut state = ctx.state::<Option<State>>()?;

	*state = Some(State::new());

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct MarkAsDrainingInput {}

#[activity(MarkAsDraining)]
async fn mark_as_draining(ctx: &ActivityCtx, input: &MarkAsDrainingInput) -> Result<()> {
	let mut state = ctx.state::<State>()?;
	state.is_draining = true;

	Ok(())
}

/// Forward a signal from connection to the pool
#[signal("pegboard_serverless_runner_conn_drain_started")]
pub struct ConnectionDrainStarted {}

#[signal("pegboard_serverless_runner_drain")]
pub struct Drain {}

join_signal!(Tunnel {
	Drain,
	ConnectionDrainStarted,
});
