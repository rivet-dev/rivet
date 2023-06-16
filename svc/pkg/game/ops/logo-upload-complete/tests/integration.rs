use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let game_id = game_res.game_id.unwrap();

	// Create the upload
	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: "bucket-game-logo".into(),
		files: vec![
			backend::upload::PrepareFile {
				path: "image.png".to_owned(),
				mime: Some("image/png".into()),
				content_length: 123,
				..Default::default()
			},
		],
	})
	.await
	.unwrap();

	let upload_id = upload_prepare_res.upload_id.unwrap();
	let presigned_request = upload_prepare_res.presigned_requests.first();
	let _presigned_request = presigned_request.unwrap();

	op!([ctx] game_logo_upload_complete {
		game_id: Some(game_id),
		upload_id: Some(upload_id)
	})
	.await
	.unwrap();

	let uploads_res = op!([ctx] upload_get {
		upload_ids: vec![upload_id]
	})
	.await
	.unwrap();

	let upload = uploads_res.uploads.first().unwrap();

	assert!(upload.complete_ts.is_some(), "Upload did not complete");
}
