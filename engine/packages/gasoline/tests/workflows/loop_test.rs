use futures_util::FutureExt;
use gas::prelude::*;
use gasoline as gas;

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct LoopWorkflowInput {
	pub iterations: usize,
}

#[workflow(LoopTestWorkflow)]
pub async fn loop_test_workflow(ctx: &mut WorkflowCtx, input: &LoopWorkflowInput) -> Result<usize> {
	let iterations = input.iterations;

	ctx.loope(0, move |_ctx, state| {
		async move {
			if *state >= iterations {
				return Ok(Loop::Break(()));
			}

			*state += 1;

			Ok(Loop::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(iterations)
}
