use chirp_workflow::prelude::*;
use rivet_operation::prelude::proto::backend;

#[workflow_test]
async fn prewarm_ats(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: backend::faker::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();
	let build_id = build_res.build_id.as_ref().unwrap().as_uuid();

	let region_res = op!([ctx] faker_region {
		..Default::default()
	})
	.await
	.unwrap();

	ctx.op(build::ops::prewarm_ats::Input {
		datacenter_ids: vec![region_res.region_id.unwrap().as_uuid()],
		build_ids: vec![build_id],
	})
	.await
	.unwrap();

	// TODO:
}
