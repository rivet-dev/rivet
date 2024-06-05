use anyhow::*;
use async_trait::async_trait;

use crate::OperationCtx;

#[async_trait]
pub trait Operation {
	type Input: OperationInput;
	type Output: Send;

	fn name() -> &'static str;

	async fn run(ctx: &mut OperationCtx, input: &Self::Input) -> Result<Self::Output>;
}

pub trait OperationInput: Send {
	type Operation: Operation;
}
