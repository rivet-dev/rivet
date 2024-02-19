use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn normal(ctx: TestCtx) {
	let res = op!([ctx] email_verification_create {
		email: "test@rivet.gg".into(),
	})
	.await
	.unwrap();
	let verification_id = res.verification_id.as_ref().unwrap().as_uuid();

	let (row_count,) = sql_fetch_one!(
		[ctx, (i64,)]
		"
		SELECT COUNT(*)
		FROM db_email_verification.verifications
		WHERE verification_id = $1
		",
		verification_id,
	)
	.await
	.unwrap();
	assert_eq!(row_count, 1);
}

#[worker_test]
async fn with_game(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {}).await.unwrap();
	let game_id = game_res.game_id.unwrap().as_uuid();

	// Upload the game logo
	upload_game_logo(&ctx, game_id, "https://images.unsplash.com/photo-1550745165-9bc0b252726f?ixlib=rb-4.0.3&dl=lorenzo-herrera-p0j-mE6mGo4-unsplash.jpg&w=1920&q=80&fm=jpg&crop=entropy&cs=tinysrgb").await;

	// Send verification email
	let res = op!([ctx] email_verification_create {
		email: "test@rivet.gg".into(),
		game_id: Some(game_id.into()),
	})
	.await
	.unwrap();
	let verification_id = res.verification_id.as_ref().unwrap().as_uuid();

	let (row_count,) = sql_fetch_one!(
		[ctx, (i64,)]
		"
		SELECT COUNT(*)
		FROM db_email_verification.verifications
		WHERE verification_id = $1
		",
		verification_id,
	)
	.await
	.unwrap();
	assert_eq!(row_count, 1);
}

#[worker_test]
async fn invalid_err(ctx: TestCtx) {
	op!([ctx] email_verification_create {
		email: "def.com".into(),
	})
	.await
	.expect_err("should err from invalid email");
}

async fn upload_game_logo(ctx: &TestCtx, game_id: Uuid, url: &str) {
	let bucket = "bucket-game-logo";
	let mime = "image/jpeg";

	tracing::info!(?url, "downloading file");
	let file_bytes = reqwest::get(url)
		.await
		.unwrap()
		.error_for_status()
		.unwrap()
		.bytes()
		.await
		.unwrap();

	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: bucket.into(),
		files: vec![
			backend::upload::PrepareFile {
				path: "logo.jpeg".into(),
				mime: Some(mime.into()),
				content_length: file_bytes.len() as u64,
				..Default::default()
			},
		],
	})
	.await
	.unwrap();
	let upload_id = upload_prepare_res.upload_id.unwrap();

	let presigned_request = upload_prepare_res.presigned_requests.first().unwrap();
	tracing::info!(?presigned_request, "writing test file");
	reqwest::Client::new()
		.put(&presigned_request.url)
		.body(file_bytes)
		.header("content-type", mime)
		.send()
		.await
		.expect("failed to upload")
		.error_for_status()
		.unwrap();

	op!([ctx] game_logo_upload_complete {
		game_id: Some(game_id.into()),
		upload_id: Some(upload_id),
	})
	.await
	.unwrap();
}
