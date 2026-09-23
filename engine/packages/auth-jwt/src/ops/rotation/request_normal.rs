use anyhow::{Context, Result, ensure};
use gas::prelude::*;

use crate::{
	ops::rotation::{WORKFLOW_TAGS, status},
	workflows::key_rotation,
};

#[derive(Debug)]
pub struct Input {
	pub expected_generation: u64,
}

#[operation]
pub async fn auth_jwt_rotation_request_normal(ctx: &OperationCtx, input: &Input) -> Result<()> {
	let current = ctx
		.op(status::Input)
		.await?
		.context("JWT key ring is not initialized")?;
	ensure!(
		current.generation == input.expected_generation,
		"JWT key-ring generation changed"
	);
	ensure!(
		current.pending.is_none(),
		"JWT key ring already has a pending rotation"
	);
	let workflow_id = ctx
		.find_workflow::<key_rotation::Workflow>(&WORKFLOW_TAGS[..])
		.await?
		.context("JWT key-rotation workflow is not running")?;
	ctx.signal(key_rotation::StartRotation {
		expected_generation: input.expected_generation,
	})
	.to_workflow_id(workflow_id)
	.send()
	.await?;
	Ok(())
}
