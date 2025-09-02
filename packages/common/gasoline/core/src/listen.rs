use async_trait::async_trait;

use crate::{ctx::ListenCtx, error::WorkflowResult};

/// A trait which allows listening for signals from the workflows database. This is used by
/// `WorkflowCtx::listen` and `WorkflowCtx::query_signal`. If you need a listener with state, use
/// `CustomListener`.
#[async_trait]
pub trait Listen: Sized {
	/// This function may be polled by the `WorkflowCtx`.
	async fn listen(ctx: &mut ListenCtx) -> WorkflowResult<Self>;
	fn parse(name: &str, body: &serde_json::value::RawValue) -> WorkflowResult<Self>;
}

/// A trait which allows listening for signals with a custom state. This is used by
/// `WorkflowCtx::custom_listener`.
#[async_trait]
pub trait CustomListener: Sized {
	type Output;

	/// This function may be polled by the `WorkflowCtx`.
	async fn listen(&self, ctx: &mut ListenCtx) -> WorkflowResult<Self::Output>;
	fn parse(name: &str, body: &serde_json::value::RawValue) -> WorkflowResult<Self::Output>;
}
