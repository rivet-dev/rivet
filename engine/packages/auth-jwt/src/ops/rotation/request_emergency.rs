use anyhow::{Context, Result, ensure};
use gas::prelude::*;

use crate::{
	ABSOLUTE_KEY_LIMIT, KeyId,
	ops::{key_ring, rotation::WORKFLOW_TAGS},
	validate_emergency_request,
	workflows::key_rotation,
};

#[derive(Debug)]
pub struct Input {
	pub request_id: [u8; 16],
	pub expected_generation: u64,
	pub revoke_kids: Vec<KeyId>,
}

#[operation]
pub async fn auth_jwt_rotation_request_emergency(ctx: &OperationCtx, input: &Input) -> Result<()> {
	ensure!(
		input.revoke_kids.len() <= ABSOLUTE_KEY_LIMIT,
		"too many JWT key ids in emergency request"
	);
	let snapshot = ctx
		.op(key_ring::get_latest::Input)
		.await?
		.optional()?
		.context("JWT key ring is not initialized")?;
	validate_emergency_request(
		&snapshot.ring,
		input.expected_generation,
		&input.revoke_kids,
	)?;
	let workflow_id = ctx
		.find_workflow::<key_rotation::Workflow>(&WORKFLOW_TAGS[..])
		.await?
		.context("JWT key-rotation workflow is not running")?;
	ctx.signal(key_rotation::EmergencyRotate {
		request_id: input.request_id,
		expected_generation: input.expected_generation,
		revoke_kids: input
			.revoke_kids
			.iter()
			.map(|kid| *kid.as_bytes())
			.collect(),
	})
	.to_workflow_id(workflow_id)
	.send()
	.await?;
	Ok(())
}
