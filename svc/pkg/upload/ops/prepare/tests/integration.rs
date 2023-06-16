use chirp_worker::prelude::*;
use proto::backend;

use std::collections::HashMap;

const TEST_BUCKET_NAME: &str = "bucket-build";

#[worker_test]
async fn empty(ctx: TestCtx) {
	// Generate random files
	let files = (0..16)
		.map(|i| {
			(
				format!("dir/file-{}.txt", i),
				format!("file {}", i).into_bytes(),
			)
		})
		.collect::<HashMap<String, Vec<u8>>>();
	tracing::info!(?files, "generated files");

	tracing::info!("creating upload");
	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: TEST_BUCKET_NAME.into(),
		files: files
			.iter()
			.map(|(k, v)| backend::upload::PrepareFile {
				path: k.clone(),
				mime: Some("text/plain".into()),
				content_length: v.len() as u64,
				..Default::default()
			})
			.collect(),
	})
	.await
	.unwrap();
	let upload_id = upload_prepare_res.upload_id.unwrap();

	// TODO: Replace raw SQL query
	// Check upload exists
	let res = op!([ctx] upload_get {
		upload_ids: vec![upload_id],
	})
	.await
	.unwrap();
	assert!(!res.uploads.is_empty(), "upload not in database");

	tracing::info!(presigned_requests = ?upload_prepare_res.presigned_requests, "uploading files");
	for file in &upload_prepare_res.presigned_requests {
		let file_data = files.get(&file.path).unwrap();

		let res = reqwest::Client::new()
			.put(&file.url)
			.body(file_data.clone())
			.header("content-type", "text/plain")
			.send()
			.await
			.expect("failed to upload");
		if res.status().is_success() {
			tracing::info!(?file, "uploaded successfully");
		} else {
			panic!(
				"failed to upload ({}): {:?}",
				res.status(),
				res.text().await
			);
		}
	}

	tracing::info!("checking files exist");
	for _k in files.keys() {
		// TODO: Authenticate this
		// // Fetch directly from Minio
		// {
		// 	let url = format!(
		// 		"https://minio.rivet-gg.test/{}-{}/{}/{}",
		// 		util::env::namespace(),
		// 		TEST_BUCKET_NAME,
		// 		upload_id,
		// 		k,
		// 	);
		// 	tracing::info!(%url, "fetching from minio");
		// 	let res = reqwest::Client::new()
		// 		.get(url)
		// 		.send()
		// 		.await
		// 		.expect("failed to fetch minio");
		// 	if !res.status().is_success() {
		// 		panic!(
		// 			"failed to fetch file from minio ({}): {:?}",
		// 			res.status(),
		// 			res.text().await
		// 		);
		// 	}
		// }

		// TODO: This causes errors
		// // Fetch through ATS
		// {
		// 	let url = format!(
		// 		"http://proxy.traffic-server.service.consul:21100/s3-cache/{}-{}/{}/{}",
		// 		util::env::namespace(),
		// 		TEST_BUCKET_NAME,
		// 		upload_id,
		// 		k,
		// 	);
		// 	tracing::info!(%url, "fetching from ats proxy");
		// 	let res = reqwest::Client::new()
		// 		.get(url)
		// 		.send()
		// 		.await
		// 		.expect("failed to fetch ats proxy");
		// 	if !res.status().is_success() {
		// 		panic!(
		// 			"failed to fetch file from ats proxy ({}): {:?}",
		// 			res.status(),
		// 			res.text().await
		// 		);
		// 	}
		// }
	}
}
