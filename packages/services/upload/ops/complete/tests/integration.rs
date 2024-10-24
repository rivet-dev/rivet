use chirp_worker::prelude::*;
use proto::backend;

const TEST_BUCKET: &str = "bucket-build";
const TEST_BODY: &[u8] = b"test file";

#[worker_test]
async fn basic(ctx: TestCtx) {
	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: TEST_BUCKET.into(),
		files: vec![
			backend::upload::PrepareFile {
				path: "upload.txt".into(),
				mime: Some("text/plain".into()),
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
		upload_id: Some(upload_id),
		bucket: Some("wrong-bucket".into()),
	})
	.await
	.unwrap_err();

	op!([ctx] upload_complete {
		upload_id: Some(upload_id),
		bucket: None,
	})
	.await
	.unwrap();

	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![upload_id],
	})
	.await
	.unwrap();

	let _upload_data = upload_res.uploads.first().expect("upload not created");
}

#[worker_test]
async fn profanity_filter(ctx: TestCtx) {
	assert!(
		!test_image_has_profanity(
			&ctx,
			include_bytes!("./static/a.jpeg"),
			"image.jpeg",
			"image/jpeg",
			0.1
		)
		.await
	);

	assert!(
		test_image_has_profanity(
			&ctx,
			include_bytes!("./static/b.jpeg"),
			"image.jpeg",
			"image/jpeg",
			0.005
		)
		.await
	);

	assert!(
		!test_image_has_profanity(
			&ctx,
			include_bytes!("./static/c.tiff"),
			"image.tiff",
			"image/tiff",
			0.1,
		)
		.await
	);
}

async fn test_image_has_profanity(
	ctx: &TestCtx,
	file_bytes: &[u8],
	file_path: &str,
	mime: &str,
	threshold: f32,
) -> bool {
	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: TEST_BUCKET.into(),
		files: vec![
			backend::upload::PrepareFile {
				path: file_path.into(),
				mime: Some(mime.into()),
				content_length: file_bytes.len() as u64,
				nsfw_score_threshold: Some(threshold),
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
		.body(file_bytes.to_owned())
		.header("content-type", mime)
		.send()
		.await
		.expect("failed to upload")
		.error_for_status()
		.unwrap();

	let res = op!([ctx] upload_complete {
		upload_id: Some(upload_id),
		bucket: None,
	})
	.await;
	match res {
		Ok(_) => false,
		Err(err) if err.is(formatted_error::code::UPLOAD_NSFW_CONTENT_DETECTED) => true,
		Err(err) => panic!("{:?}", err),
	}
}

// #[worker_test]
// #[ignore]
// async fn many_files(ctx: TestCtx) {
// 	let files = (0..16891)
// 		.map(|i| backend::upload::PrepareFile {
// 			path: format!("file-{i}.txt"),
// 			mime: Some("text/plain".into()),
// 			content_length: TEST_BODY.len() as u64,
// 			..Default::default()
// 		})
// 		.collect::<Vec<_>>();

// 	let upload_prepare_res = op!([ctx] upload_prepare {
// 		bucket: TEST_BUCKET.into(),
// 		files: files,
// 	})
// 	.await
// 	.unwrap();
// 	let upload_id = upload_prepare_res.upload_id.unwrap();

// 	tracing::info!("writing test files");
// 	let semaphore = Arc::new(Semaphore::new(32));
// 	let mut join_set = JoinSet::new();
// 	for (i, req) in upload_prepare_res.presigned_requests.iter().enumerate() {
// 		let permit = semaphore.clone().acquire_owned().await.unwrap();

// 		let len = upload_prepare_res.presigned_requests.len();
// 		let url = ctx.url.clone();
// 		join_set.spawn(async move {
// 			let res = reqwest::Client::new()
// 				.put(&url)
// 				.body(TEST_BODY.to_vec())
// 				.header("content-type", "text/plain")
// 				.send()
// 				.await
// 				.expect("failed to upload");
// 			if res.status().is_success() {
// 				tracing::info!("uploaded successfully ({i}/{len})",);
// 			} else {
// 				panic!(
// 					"failed to upload ({}): {:?}",
// 					res.status(),
// 					res.text().await
// 				);
// 			}

// 			drop(permit);
// 		});
// 	}
// 	while let Some(x) = join_set.join_next().await {
// 		x.unwrap();
// 	}

// 	op!([ctx] upload_complete {
// 		upload_id: Some(upload_id),
// 		bucket: None,
// 	})
// 	.await
// 	.unwrap();

// 	let upload_res = op!([ctx] upload_get {
// 		upload_ids: vec![upload_id],
// 	})
// 	.await
// 	.unwrap();

// 	let _upload_data = upload_res.uploads.first().expect("upload not created");
// }
