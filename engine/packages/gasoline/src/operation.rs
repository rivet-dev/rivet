use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;

use crate::ctx::OperationCtx;

#[async_trait]
pub trait Operation {
	type Input: OperationInput;
	type Output: Debug + Send;

	const NAME: &'static str;
	const TIMEOUT: std::time::Duration;

	async fn run(ctx: &OperationCtx, input: &Self::Input) -> Result<Self::Output>;
}

pub trait OperationInput: Debug + Send {
	type Operation: Operation;
}
