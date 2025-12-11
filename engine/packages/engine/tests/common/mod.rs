#![allow(dead_code, unused_variables, unused_imports)]

pub mod actors;
pub mod api;
pub mod ctx;
pub mod test_helpers;
pub mod test_runner;

pub use actors::*;
pub use ctx::*;
pub use rivet_api_types as api_types;
pub const TEST_RUNNER_NAME: &'static str = "test-runner";
pub use test_helpers::*;
pub use test_runner::*;

use std::future::Future;
use std::time::Duration;

pub fn run<F, Fut>(opts: TestOpts, test_fn: F)
where
	F: FnOnce(TestCtx) -> Fut,
	Fut: Future<Output = ()>,
{
	let runtime = tokio::runtime::Runtime::new().expect("failed to build runtime");
	runtime.block_on(async {
		let timeout = Duration::from_secs(opts.timeout_secs);
		let ctx = TestCtx::new_with_opts(opts).await.expect("build testctx");
		tokio::time::timeout(timeout, test_fn(ctx))
			.await
			.expect("test timed out");
	});
}
