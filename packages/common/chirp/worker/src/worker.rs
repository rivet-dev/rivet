use async_trait::async_trait;
use global_error::GlobalResult;
use rivet_operation::OperationContext;

// TODO: Create custom response type
#[async_trait]
pub trait Worker: Clone + Send + Sync + 'static {
	type Request: prost::Message + Default + Clone;
	type Response: prost::Message + Default + Clone;

	const NAME: &'static str;
	const TIMEOUT: std::time::Duration;

	async fn handle<'a>(
		&self,
		req: &OperationContext<Self::Request>,
	) -> GlobalResult<Self::Response>
	where
		Self::Response: 'a;
}
