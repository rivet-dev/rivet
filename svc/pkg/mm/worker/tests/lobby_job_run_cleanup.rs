use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn lobby_job_run_cleanup(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let faker_lobby = op!([ctx] faker_mm_lobby {}).await.unwrap();
	let lobby_id = faker_lobby.lobby_id.unwrap().as_uuid();
	let run_id = faker_lobby.run_id.unwrap().as_uuid();

	// Check that cleaning up a job also cleans up the lobby
	let mut cleanup_sub = subscribe!([ctx] mm::msg::lobby_cleanup(lobby_id))
		.await
		.unwrap();
	msg!([ctx] job_run::msg::cleanup(run_id) {
		run_id: Some(run_id.into()),
		..Default::default()
	})
	.await
	.unwrap();
	cleanup_sub.next().await.unwrap();
}
