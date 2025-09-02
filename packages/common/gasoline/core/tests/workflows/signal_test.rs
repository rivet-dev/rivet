use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct SignalTestInput {}

#[workflow(SignalTestWorkflow)]
pub async fn signal_test_workflow(
	ctx: &mut WorkflowCtx,
	_input: &SignalTestInput,
) -> Result<String> {
	let signal = ctx.listen::<TestSignal>().await?;
	tracing::info!(?signal, "Received signal");

	Ok(signal.value)
}

#[signal("test_signal")]
#[derive(Debug)]
pub struct TestSignal {
	pub value: String,
}
