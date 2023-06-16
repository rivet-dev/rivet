use chirp_worker::prelude::*;

#[derive(prost::Message, Clone)]
struct TestRequest {}

#[derive(prost::Message, Clone)]
struct TestResponse {}

#[derive(thiserror::Error, Debug)]
enum TestError {
	#[error("test")]
	Test,
}

#[chirp_worker_attributes::worker]
async fn worker(ctx: OperationContext<TestMessage>) -> Result<TestResponse, GlobalError> {
	tracing::info!(body = ?req, "hello, world!");

	do_something()?;

	Ok(TestResponse {})
}

fn do_something() -> Result<(), TestError> {
	Err(TestError::Test)
}
