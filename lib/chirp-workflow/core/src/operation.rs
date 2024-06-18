use async_trait::async_trait;
use global_error::GlobalResult;

use crate::OperationCtx;

#[async_trait]
pub trait Operation {
	type Input: OperationInput;
	type Output: Send;

	const NAME: &'static str;
	const TIMEOUT: std::time::Duration;

	async fn run(ctx: &OperationCtx, input: &Self::Input) -> GlobalResult<Self::Output>;
}

pub trait OperationInput: Send {
	type Operation: Operation;
}
