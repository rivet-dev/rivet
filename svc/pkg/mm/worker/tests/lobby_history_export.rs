use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn default(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let lobby_res2 = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	grace_period().await;

	let request_id = Uuid::new_v4();
	let res = msg!([ctx] mm::msg::lobby_history_export(request_id) -> mm::msg::lobby_history_export_complete {
		request_id: Some(request_id.into()),
		namespace_ids: vec![lobby_res.namespace_id.unwrap(), lobby_res2.namespace_id.unwrap()],
		query_start: 0,
		query_end: util::timestamp::now()
	})
	.await
	.unwrap();

	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![res.upload_id.unwrap()],
	})
	.await
	.unwrap();
	let upload = upload_res.uploads.first().unwrap();

	// TODO: Check the outputed CSV
}

/// mm-lobby-history returns stale responses for performance purposes.
/// This waits for changes to propagate.
async fn grace_period() {
	tokio::time::sleep(std::time::Duration::from_secs(5)).await;
}
