use chirp_worker::prelude::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let res = op!([ctx] mm_lobby_for_run_id {
		run_ids: vec![lobby_res.lobby_id.unwrap(), Uuid::new_v4().into()],
	})
	.await
	.unwrap();
	assert_eq!(1, res.lobbies.len());
}
