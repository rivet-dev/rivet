use anyhow::Context;
use gas::prelude::*;
use rivet_envoy_protocol as protocol;

pub(crate) async fn refresh_command_wrapper(
	ctx: &StandaloneCtx,
	command_wrapper: &mut protocol::CommandWrapper,
) -> Result<()> {
	let protocol::Command::CommandStartActor(start) = &mut command_wrapper.inner else {
		return Ok(());
	};

	let actor_id =
		Id::parse(&command_wrapper.checkpoint.actor_id).context("invalid command actor id")?;
	start.hibernating_requests = ctx
		.op(pegboard::ops::actor::hibernating_request::list::Input { actor_id })
		.await?
		.into_iter()
		.map(|request| protocol::HibernatingRequest {
			gateway_id: request.gateway_id,
			request_id: request.request_id,
		})
		.collect();

	Ok(())
}
