use gas::prelude::*;
use gasoline as gas;

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct TestSignal {
	pub value: String,
}
