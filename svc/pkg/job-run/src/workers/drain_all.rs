use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "job-run-drain-all")]
async fn worker(ctx: &OperationContext<job_run::msg::drain_all::Message>) -> GlobalResult<()> {
	// TODO: Disabled for now
	// chirp_workflow::compat::workflow(
	// 	ctx,
	// 	crate::workflows::drain_all::Input {
	// 		nomad_node_id: ctx.nomad_node_id.clone(),
	// 		drain_timeout: ctx.drain_timeout,
	// 	},
	// )
	// .await?
	// .dispatch()
	// .await?;

	Ok(())
}
