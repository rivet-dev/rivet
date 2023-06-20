use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn {{snake name}}(ctx: TestCtx) {
	todo!();

	msg!([ctx] {{snake pkg}}::msg::{{snake name}}() {
	
	})
	.await
	.unwrap();
}
