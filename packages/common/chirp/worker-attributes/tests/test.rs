use chirp_worker::prelude::*;

#[derive(prost::Message, Clone)]
struct TestRequest {}

#[derive(prost::Message, Clone)]
struct TestResponse {}

#[chirp_worker_attributes::worker]
async fn worker(ctx: &OperationContext<TestMessage>) -> GlobalResult<TestResponse> {
	tracing::info!(body = ?req, "hello, world!");

	Ok(TestResponse {})
}
