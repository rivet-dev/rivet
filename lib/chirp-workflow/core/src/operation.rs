use async_trait::async_trait;
use global_error::GlobalResult;

use crate::OperationCtx;

#[async_trait]
pub trait Operation {
	type Input: OperationInput;
	type Output: Send;

	fn name() -> &'static str;

	async fn run(ctx: &mut OperationCtx, input: &Self::Input) -> GlobalResult<Self::Output>;
}

pub trait OperationInput: Send {
	type Operation: Operation;
}
