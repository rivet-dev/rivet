use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct EvictionTestInput {
	pub test_id: Uuid,
}

#[workflow(EvictionTestWorkflow)]
pub async fn eviction_test_workflow(
	ctx: &mut WorkflowCtx,
	input: &EvictionTestInput,
) -> Result<()> {
	ctx.msg(RunningMessage {
		value: "foo".to_string(),
	})
	.tag("test", input.test_id)
	.send()
	.await?;

	ctx.listen::<TestSignal2>().await?;

	ctx.msg(RunningMessage {
		value: "bar".to_string(),
	})
	.tag("test", input.test_id)
	.send()
	.await?;

	Ok(())
}

#[message("running_message")]
pub struct RunningMessage {
	pub value: String,
}

#[signal("test_signal2")]
#[derive(Debug)]
pub struct TestSignal2 {
	pub value: String,
}
