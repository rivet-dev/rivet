use chirp_worker::prelude::*;
use proto::backend;

const TEST_BODY: &[u8] = b"test file";

#[worker_test]
async fn empty(ctx: TestCtx) {
	let team_res = op!([ctx] faker_team {
		..Default::default()
	})
	.await
	.unwrap();
	let team_id = team_res.team_id.unwrap();

	// Create the upload
	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: "bucket-team-avatar".into(),
		files: vec![
			backend::upload::PrepareFile {
				path: "image.png".to_owned(),
				mime: Some("image/png".into()),
				content_length: TEST_BODY.len() as u64,
				..Default::default()
			},
		],
	})
	.await
	.unwrap();
	let upload_id = upload_prepare_res.upload_id.unwrap();
	let presigned_request = upload_prepare_res.presigned_requests.first().unwrap();

	tracing::info!("writing test files");
	let res = reqwest::Client::new()
		.put(&presigned_request.url)
		.body(TEST_BODY.to_vec())
		.header("content-type", "image/png")
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

	op!([ctx] team_avatar_upload_complete {
		team_id: Some(team_id),
		upload_id: Some(upload_id),
	})
	.await
	.unwrap();

	let uploads_res = op!([ctx] upload_get {
		upload_ids: vec![upload_id],
	})
	.await
	.unwrap();

	let upload = uploads_res.uploads.first().unwrap();

	assert!(upload.complete_ts.is_some(), "upload did not complete");
}
