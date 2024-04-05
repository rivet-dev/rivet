use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn datacenter_scale(_ctx: TestCtx) {
	if !util::feature::server_provision() {
		return;
	}

	// msg!([ctx] cluster::msg::datacenter_scale() {

	// })
	// .await
	// .unwrap();

	todo!();
}
