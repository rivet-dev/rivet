use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn lobby_stop(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let _nomad_config = nomad_util::config_from_env().unwrap();

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	let lobby_get = op!([ctx] mm_lobby_get {
		lobby_ids: vec![lobby_id.into()],
		include_stopped: false,
	})
	.await
	.unwrap();
	let lobby_data = lobby_get.lobbies.first().unwrap();
	let run_id = lobby_data.run_id.as_ref().unwrap().as_uuid();

	let run_get = op!([ctx] job_run_get {
		run_ids: vec![run_id.into()],
	})
	.await
	.unwrap();
	let run_data = run_get.runs.first().unwrap();
	let run_meta = match run_data.run_meta.as_ref().unwrap().kind.as_ref().unwrap() {
		backend::job::run_meta::Kind::Nomad(x) => x.clone(),
	};
	tracing::info!(alloc_id = ?run_meta.alloc_id, "alloc id");

	let mut stop_run_sub = subscribe!([ctx] job_run::msg::stop(run_id)).await.unwrap();
	let mut cleanup_sub = subscribe!([ctx] mm::msg::lobby_cleanup(lobby_id))
		.await
		.unwrap();
	msg!([ctx] mm::msg::lobby_stop(lobby_id) {
		lobby_id: Some(lobby_res.lobby_id.unwrap()),
	})
	.await
	.unwrap();
	stop_run_sub.next().await.unwrap();
	cleanup_sub.next().await.unwrap();
}
