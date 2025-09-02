pub mod actors;
pub mod ctx;
pub mod ns;
pub mod runner;
pub mod test_helpers;

pub use actors::*;
pub use ctx::*;
pub use ns::*;
pub use test_helpers::*;

use std::future::Future;
use std::time::Duration;

pub fn run<F, Fut>(opts: TestOpts, test_fn: F)
where
	F: FnOnce(TestCtx) -> Fut,
	Fut: Future<Output = ()>,
{
	let runtime = tokio::runtime::Runtime::new().expect("failed to build runtime");

	runtime.block_on(async {
		let ctx = TestCtx::new_with_opts(opts).await.expect("build testctx");
		tokio::time::timeout(Duration::from_secs(10), test_fn(ctx))
			.await
			.expect("test timed out");
	});
}
