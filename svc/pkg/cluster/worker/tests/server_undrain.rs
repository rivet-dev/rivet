use chirp_worker::prelude::*;

#[worker_test]
async fn server_undrain(_ctx: TestCtx) {
	if !util::feature::server_provision() {
		return;
	}

	// Difficult test to write:
	// 1. Create job server
	// 2. Wait for nomad to register
	// 3. Create lobby on node
	// 4. Drain node
	// 5. Before the drain ends (somehow manage to postpone it), undrain the node
	// 6. Check that the node is not draining anymore

	// msg!([ctx] cluster::msg::server_undrain() {

	// })
	// .await
	// .unwrap();

	todo!();
}
