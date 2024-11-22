use std::{collections::HashSet, time::Duration};

use futures_util::{FutureExt, StreamExt, TryStreamExt};
use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde_json::json;

pub const CHUNK_SIZE: u64 = util::file_size::mebibytes(100);
const MAX_UPLOAD_SIZE: u64 = util::file_size::gigabytes(100);
/// Minimum size for AWS multipart file uploads.
///
/// See AWS error code `EntityTooSmall`
///
/// https://docs.aws.amazon.com/AmazonS3/latest/API/API_CompleteMultipartUpload.html
const MIN_MULTIPART_FILE_SIZE: u64 = util::file_size::mebibytes(5);

struct PrepareResult {
	multipart: Option<MultipartUpdate>,
	fut: std::pin::Pin<
		Box<
			dyn std::future::Future<Output = GlobalResult<backend::upload::PresignedUploadRequest>>
				+ Send,
		>,
	>,
}

struct MultipartUpdate {
	path: String,
	multipart_upload_id: String,
}

#[operation(name = "upload-prepare")]
async fn handle(
	ctx: OperationContext<upload::prepare::Request>,
) -> GlobalResult<upload::prepare::Response> {
	// This client is used for making requests directly to S3
	let s3_client_internal = s3_util::Client::with_bucket_and_endpoint(
		ctx.config(),
		&ctx.bucket,
		s3_util::EndpointKind::Internal,
	)
	.await?;

	// This client is used for presigning requests using the public endopint
	let s3_client_external = s3_util::Client::with_bucket_and_endpoint(
		ctx.config(),
		&ctx.bucket,
		s3_util::EndpointKind::External,
	)
	.await?;

	// Validate upload sizes
	let total_content_length = ctx.files.iter().fold(0, |acc, x| acc + x.content_length);
	tracing::info!(len=%ctx.files.len(), %total_content_length, "file info");
	ensure!(
		total_content_length < MAX_UPLOAD_SIZE,
		"uploads must be < 100 gb"
	);

	let user_id = ctx.user_id.map(|x| x.as_uuid());

	// Check that there's no duplicate paths
	let mut registered_paths = HashSet::new();
	for file in &ctx.files {
		tracing::info!(?file, "signing url");

		ensure!(
			registered_paths.insert(file.path.clone()),
			"duplicate file path"
		);
	}

	// Prepare columns for files
	let upload_id = Uuid::new_v4();
	let paths = ctx
		.files
		.iter()
		.map(|x| x.path.as_str())
		.collect::<Vec<_>>();
	let mimes = ctx
		.files
		.iter()
		.map(|x| x.mime.as_deref())
		.collect::<Vec<_>>();
	let content_lengths = ctx
		.files
		.iter()
		.map(|x| x.content_length as i64)
		.collect::<Vec<_>>();

	// Insert in to database
	sql_execute!(
		[ctx]
		"
		WITH
			_insert_upload AS (
				INSERT INTO db_upload.uploads (upload_id, create_ts, content_length, bucket, user_id, provider)
				VALUES ($1, $2, $3, $4, $5, $6)
				RETURNING 1
			),
			_insert_files AS (
				INSERT INTO db_upload.upload_files (upload_id, path, mime, content_length)
				SELECT $1, rows.*
				FROM unnest($7, $8, $9) AS rows
				RETURNING 1
			)
		SELECT 1
		",
		upload_id,
		ctx.ts(),
		total_content_length as i64,
		&ctx.bucket,
		user_id,
		// Hardcoded to AWS since we don't use this feature anymore
		backend::upload::Provider::Aws as i64,
		paths,
		mimes,
		content_lengths,
	)
	.await?;

	// Create iterators to be joined later
	let (presigned_requests_init, multipart_updates) =
		futures_util::stream::iter(ctx.files.iter().cloned())
			.map(move |file| {
				let s3_client_internal = s3_client_internal.clone();
				let s3_client_external = s3_client_external.clone();

				handle_multipart_upload(s3_client_internal, s3_client_external, upload_id, file)
					.boxed()
			})
			.buffer_unordered(16)
			.try_collect::<Vec<_>>()
			.await?
			.into_iter()
			.flatten()
			.map(|res| (res.fut, res.multipart))
			.unzip::<_, _, Vec<_>, Vec<_>>();

	// Split columns for query
	let (multipart_paths, multipart_upload_ids) = multipart_updates
		.into_iter()
		.flatten()
		.map(|mp| (mp.path, mp.multipart_upload_id))
		.unzip::<_, _, Vec<_>, Vec<_>>();

	let (presigned_requests, _) = tokio::try_join!(
		// Create presigned requests
		futures_util::stream::iter(presigned_requests_init)
			.buffer_unordered(16)
			.try_collect::<Vec<_>>(),
		// Set multipart upload ids
		async {
			sql_execute!(
				[ctx]
				"
				UPDATE db_upload.upload_files
				SET multipart_upload_id = v.multipart_upload_id
				FROM (
					SELECT unnest($2) AS path,
						   unnest($3) AS multipart_upload_id
				) AS v
				WHERE
					upload_files.upload_id = $1 AND
					upload_files.path = v.path
				",
				upload_id,
				multipart_paths,
				multipart_upload_ids,
			)
			.await
			.map_err(Into::<GlobalError>::into)
		},
	)?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				event_id: Some(Uuid::new_v4().into()),
				name: "upload.prepare".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"user_id": user_id,
					"upload_id": upload_id,
					"bucket": ctx.bucket,
					"files": ctx.files.len(),
					"total_content_length": total_content_length,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(upload::prepare::Response {
		upload_id: Some(upload_id.into()),
		presigned_requests,
	})
}

async fn handle_multipart_upload(
	s3_client_internal: s3_util::Client,
	s3_client_external: s3_util::Client,
	upload_id: Uuid,
	file: backend::upload::PrepareFile,
) -> GlobalResult<Vec<PrepareResult>> {
	// If the file is too small for multipart uploads, fallback to normal file uploads
	if file.content_length < MIN_MULTIPART_FILE_SIZE {
		return Ok(handle_upload(s3_client_external, upload_id, file).await?);
	}

	// Create multipart upload
	let mut multipart_builder = s3_client_internal
		.create_multipart_upload()
		.bucket(s3_client_internal.bucket())
		.key(format!("{}/{}", upload_id, file.path));
	if let Some(mime) = &file.mime {
		multipart_builder = multipart_builder.content_type(mime.clone());
	}

	let multipart = multipart_builder.send().await?;
	let multipart_upload_id = unwrap!(multipart.upload_id()).to_string();

	let part_count = util::div_up!(file.content_length, CHUNK_SIZE);
	ensure!(part_count <= 10000, "too many parts");

	// S3's part number is 1-based
	Ok((1..=part_count)
		.map(|part_number| {
			let s3_client_external = s3_client_external.clone();
			let file = file.clone();
			let multipart_upload_id2 = multipart_upload_id.clone();
			let path = file.path.clone();

			let fut = async move {
				// Sign an upload request
				let offset = (part_number - 1) * CHUNK_SIZE;
				let content_length = (file.content_length - offset).min(CHUNK_SIZE);

				let upload_part_builder = s3_client_external
					.upload_part()
					.bucket(s3_client_external.bucket())
					.key(format!("{}/{}", upload_id, file.path))
					.content_length(content_length as i64)
					.upload_id(multipart_upload_id2)
					.part_number(part_number as i32);

				let presigned_upload_req = upload_part_builder
					.presigned(
						s3_util::aws_sdk_s3::presigning::PresigningConfig::builder()
							.expires_in(Duration::from_secs(60 * 60 * 6))
							.build()?,
					)
					.await?;

				GlobalResult::Ok(backend::upload::PresignedUploadRequest {
					path: file.path.clone(),
					url: presigned_upload_req.uri().to_string(),
					part_number: part_number as u32,
					byte_offset: offset,
					content_length,
				})
			}
			.boxed();

			PrepareResult {
				multipart: Some(MultipartUpdate {
					path,
					multipart_upload_id: multipart_upload_id.clone(),
				}),
				fut,
			}
		})
		.collect::<Vec<_>>())
}

async fn handle_upload(
	s3_client_external: s3_util::Client,
	upload_id: Uuid,
	file: backend::upload::PrepareFile,
) -> GlobalResult<Vec<PrepareResult>> {
	let fut = async move {
		// Sign an upload request
		let mut put_obj_builder = s3_client_external
			.put_object()
			.bucket(s3_client_external.bucket())
			.key(format!("{}/{}", upload_id, file.path))
			.content_length(file.content_length as i64);
		if let Some(mime) = &file.mime {
			put_obj_builder = put_obj_builder.content_type(mime.clone());
		}
		let presigned_upload_req = put_obj_builder
			.presigned(
				s3_util::aws_sdk_s3::presigning::PresigningConfig::builder()
					.expires_in(Duration::from_secs(60 * 60))
					.build()?,
			)
			.await?;

		GlobalResult::Ok(backend::upload::PresignedUploadRequest {
			path: file.path.clone(),
			url: presigned_upload_req.uri().to_string(),
			part_number: 0,
			byte_offset: 0,
			content_length: file.content_length,
		})
	}
	.boxed();

	Ok(vec![PrepareResult {
		multipart: None,
		fut,
	}])
}
