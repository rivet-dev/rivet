use chirp_worker::prelude::*;
use proto::backend;

const TEST_BODY: &[u8] = b"test file";

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		skip_namespaces_and_versions: true,
		..Default::default()
	})
	.await
	.unwrap();

	// Create the upload
	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: "bucket-user-avatar".into(),
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

	op!([ctx] custom_user_avatar_upload_complete {
		game_id: game_res.game_id,
		upload_id: Some(upload_id),
	})
	.await
	.unwrap();

	let res = op!([ctx] custom_user_avatar_list_for_game {
		game_id: game_res.game_id,
	})
	.await
	.unwrap();
	assert_eq!(1, res.custom_avatars.len());
}
