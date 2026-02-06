use gas::prelude::*;

use super::conn;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {
	pub pool_wf_id: Id,
	pub namespace_id: Id,
	pub runner_name: String,
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

/// Runs alongside the connection workflow to process signals. This is required because the connection
/// workflow cannot listen for signals while in an activity.
#[workflow]
pub async fn pegboard_serverless_receiver(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	ctx.activity(InitStateInput {}).await?;

	let conn_wf_id = ctx
		.workflow(conn::Input {
			pool_wf_id: input.pool_wf_id,
			receiver_wf_id: ctx.workflow_id(),
			namespace_id: input.namespace_id,
			runner_name: input.runner_name.clone(),
		})
		.dispatch()
		.await?;

	ctx.listen::<Drain>().await?;

	ctx.activity(MarkAsDrainingInput {}).await?;

	// If the connection is between retries, this will be received
	ctx.signal(conn::Drain {})
		.to_workflow_id(conn_wf_id)
		.send()
		.await?;

	// if the connection is currently running an outbound req, this will be received
	ctx.msg(conn::Drain {})
		.topic(("workflow_id", conn_wf_id))
		.send()
		.await?;

	// Wait for connection wf to complete so this wf's state remains readable
	ctx.workflow::<conn::Input>(conn_wf_id).output().await?;

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

#[signal("pegboard_serverless_runner_drain")]
pub struct Drain {}
