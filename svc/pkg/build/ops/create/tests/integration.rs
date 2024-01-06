use chirp_worker::prelude::*;

// TODO: Add back
// #[worker_test]
// async fn basic(ctx: TestCtx) {
// 	let game_res = op!([ctx] faker_game {
// 		..Default::default()
// 	})
// 	.await
// 	.unwrap();
// 	let game_id = game_res.game_id.unwrap();

// 	let build_path = format!("/tmp/{}", Uuid::new_v4());

// 	todo!("fs_path doesn't work");

// 	let faker_res = op!([ctx] faker_build {
// 		image: backend::faker::Image::HangIndefinitely as i32,
// 		// fs_path: Some(build_path.clone()),
// 	})
// 	.await
// 	.unwrap();

// 	let tag = format!("rivet-game:{}", util::faker::ident());
// 	let res = op!([ctx] build_create {
// 		game_id: Some(game_id),
// 		display_name: util::faker::display_name(),
// 		image_tag: Some(tag.clone()),
// 		image_file: Some(backend::upload::PrepareFile {
// 			path: "image.tar".into(),
// 			mime: Some(faker_res.content_type.clone()),
// 			content_length: faker_res.content_length,
// 			..Default::default()
// 		}),
// 		..Default::default()
// 	})
// 	.await
// 	.unwrap();
// 	let upload_id = res.upload_id.unwrap();

// 	// Upload image
// 	let res = reqwest::Client::new()
// 		.put(&res.image_presigned_request.as_ref().unwrap().url)
// 		.header(
// 			reqwest::header::CONTENT_TYPE,
// 			faker_res.content_type.clone(),
// 		)
// 		.header(reqwest::header::CONTENT_LENGTH, faker_res.content_length)
// 		.body(tokio::fs::File::open(&build_path).await.unwrap())
// 		.send()
// 		.await
// 		.unwrap()
// 		.error_for_status()
// 		.unwrap();

// 	op!([ctx] upload_complete {
// 		upload_id: Some(upload_id),
// 		bucket: Some("bucket-build".into()),
// 	})
// 	.await
// 	.unwrap();

// 	let upload_res = op!([ctx] upload_get {
// 		upload_ids: vec![upload_id],
// 	})
// 	.await
// 	.unwrap();

// 	let _upload_data = upload_res.uploads.first().expect("upload not created");

// 	// Test not unique
// 	let res = op!([ctx] build_create {
// 		game_id: Some(game_id),
// 		display_name: util::faker::display_name(),
// 		image_tag: Some(tag.clone()),
// 		image_file: Some(backend::upload::PrepareFile {
// 			path: "image.tar".into(),
// 			mime: Some("application/x-tar".into()),
// 			content_length: 123,
// 			..Default::default()
// 		}),
// 		..Default::default()
// 	})
// 	.await
// 	.expect_err("should not allow duplicate tags");
// }

#[worker_test]
async fn default_build(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let game_id = game_res.game_id.unwrap();

	op!([ctx] build_create {
		game_id: Some(game_id),
		display_name: util::faker::display_name(),
		default_build_kind: Some("game-multiplayer".into()),
		..Default::default()
	})
	.await
	.unwrap();
}
