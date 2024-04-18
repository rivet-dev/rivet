use std::collections::HashMap;

use chirp_worker::prelude::*;
use proto::backend;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use upload_prepare::CHUNK_SIZE;

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

#[worker_test]
async fn multipart(ctx: TestCtx) {
	// TODO: This test takes a long time to complete (each part takes ~40 seconds to create)
	return;

	let mut rng = thread_rng();
	// Create random parts
	let mut parts = (0..3)
		.map(|i| {
			tracing::info!("generating part {}", i + 1);

			(&mut rng)
				.sample_iter(&Alphanumeric)
				.take(CHUNK_SIZE as usize)
				.map(char::from)
				.collect::<String>()
		})
		.collect::<Vec<_>>();

	tracing::info!("creating upload");
	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: TEST_BUCKET_NAME.into(),
		files: vec![backend::upload::PrepareFile {
			path: "file.txt".to_string(),
			mime: Some("text/plain".into()),
			content_length: parts
				.iter()
				.fold(0, |acc, x| {
					acc + x.len()
				}) as u64,
			multipart: true,
			..Default::default()
		}],
	})
	.await
	.unwrap();
	let upload_id = upload_prepare_res.upload_id.unwrap();

	tracing::info!(presigned_requests = ?upload_prepare_res.presigned_requests, "uploading files");
	for file in &upload_prepare_res.presigned_requests {
		let part_data = std::mem::take(parts.get_mut((file.part_number - 1) as usize).unwrap());

		let res = reqwest::Client::new()
			.put(&file.url)
			.body(part_data)
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

	op!([ctx] upload_complete {
		upload_id: Some(upload_id),
		bucket: None,
	})
	.await
	.unwrap();
}
