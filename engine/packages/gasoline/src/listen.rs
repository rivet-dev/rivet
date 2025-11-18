use async_trait::async_trait;

use crate::{ctx::ListenCtx, error::WorkflowResult};

/// A trait which allows listening for signals from the workflows database. This is used by
/// `WorkflowCtx::listen` and `WorkflowCtx::query_signal`.
#[async_trait]
pub trait Listen: Sized {
	/// This function may be polled by the `WorkflowCtx`.
	async fn listen(ctx: &mut ListenCtx, limit: usize) -> WorkflowResult<Vec<Self>>;
	fn parse(name: &str, body: &serde_json::value::RawValue) -> WorkflowResult<Self>;
}
