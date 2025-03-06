use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let game_id = game_res.game_id.unwrap().as_uuid();

	let body = b"hello world";
	let res = op!([ctx] cdn_site_create {
		game_id: Some(game_id.into()),
		display_name: util::faker::display_name(),
		files: vec![backend::upload::PrepareFile {
			path: "test.txt".into(),
			mime: Some("text/plain".into()),
			content_length: body.len() as u64,
			..Default::default()
		}],
	})
	.await
	.unwrap();
	let upload_id = res.upload_id.unwrap().as_uuid();

	let presigned_request = res.presigned_requests.first().unwrap();
	tracing::info!(?presigned_request, "writing test file");
	let res = reqwest::Client::new()
		.put(&presigned_request.url)
		.body(body.to_vec())
		.header("content-type", "text/plain")
		.send()
		.await
		.expect("failed to upload");
	if res.status().is_success() {
		tracing::info!("uploaded successfully");
	} else {
		panic!(
			"failed to upload ({}): {:?}",
			res.status(),
			res.text().await
		);
	}

	op!([ctx] upload_complete {
		upload_id: Some(upload_id.into()),
		bucket: Some("bucket-cdn".into()),
	})
	.await
	.unwrap();

	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![upload_id.into()],
	})
	.await
	.unwrap();

	let _upload_data = upload_res.uploads.first().expect("upload not created");
}
