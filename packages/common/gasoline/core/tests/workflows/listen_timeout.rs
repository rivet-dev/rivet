use super::signal_test::TestSignal;
use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct ListenTimeoutInput {
	pub timeout_ms: u64,
}

#[workflow(ListenTimeoutWorkflow)]
pub async fn listen_timeout_workflow(
	ctx: &mut WorkflowCtx,
	input: &ListenTimeoutInput,
) -> Result<bool> {
	let result = ctx
		.listen_with_timeout::<TestSignal>(input.timeout_ms)
		.await?;

	let timed_out = result.is_none();
	match result {
		Some(signal) => tracing::info!(?signal, "Received signal"),
		None => tracing::info!("Timeout reached"),
	}

	Ok(timed_out)
}
