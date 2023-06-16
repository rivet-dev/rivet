use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	op!([ctx] faker_job_template {
		kind: Some(faker::job_template::request::Kind::EchoServer(Default::default())),
		..Default::default()
	})
	.await
	.unwrap();
}
