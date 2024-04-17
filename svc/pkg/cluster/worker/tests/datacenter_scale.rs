use chirp_worker::prelude::*;

#[worker_test]
async fn datacenter_scale(_ctx: TestCtx) {
	if !util::feature::server_provision() {
		return;
	}

	todo!();
}
