use std::time::Duration;

use futures_util::stream::{StreamExt, TryStreamExt};
use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

#[derive(Debug, sqlx::FromRow)]
struct UploadRow {
	bucket: String,
	user_id: Option<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
struct FileRow {
	path: String,
	content_length: i64,
	multipart_upload_id: Option<String>,
}

#[operation(name = "upload-complete")]
async fn handle(
	ctx: OperationContext<upload::complete::Request>,
) -> GlobalResult<upload::complete::Response> {
	let upload_id = unwrap_ref!(ctx.upload_id).as_uuid();

	let (bucket, files, user_id) = fetch_files(&ctx, upload_id).await?;
	let files_len = files.len();

	if let Some(req_bucket) = &ctx.bucket {
		ensure_eq_with!(&bucket, req_bucket, DB_INVALID_BUCKET);
	}

	let s3_client = s3_util::Client::with_bucket(ctx.config(), &bucket).await?;

	validate_files(&s3_client, upload_id, files).await?;

	// Mark as complete
	sql_execute!(
		[ctx]
		"
		UPDATE db_upload.uploads
		SET complete_ts = $2
		WHERE upload_id = $1
		",
		upload_id,
		ctx.ts(),
	)
	.await?;

	ctx.cache().purge("upload", [upload_id]).await?;

	msg!([ctx] upload::msg::complete_complete(upload_id) {
		upload_id: Some(upload_id.into()),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				event_id: Some(Uuid::new_v4().into()),
				name: "upload.complete".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"user_id": user_id,
					"upload_id": upload_id,
					"bucket": bucket,
					"files_len": files_len,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(upload::complete::Response {})
}

async fn fetch_files(
	ctx: &OperationContext<upload::complete::Request>,
	upload_id: Uuid,
) -> GlobalResult<(String, Vec<FileRow>, Option<Uuid>)> {
	let (upload, files) = tokio::try_join!(
		sql_fetch_one!(
			[ctx, UploadRow]
			"
			SELECT bucket, user_id
			FROM db_upload.uploads
			WHERE upload_id = $1
			",
			upload_id,
		),
		sql_fetch_all!(
			[ctx, FileRow]
			"
			SELECT path, content_length, multipart_upload_id
			FROM db_upload.upload_files
			WHERE upload_id = $1
			",
			upload_id,
		)
	)?;

	tracing::info!(bucket=?upload.bucket, files_len = ?files.len(), "fetched files");

	Ok((upload.bucket, files, upload.user_id))
}

async fn validate_files(
	s3_client: &s3_util::Client,
	upload_id: Uuid,
	files: Vec<FileRow>,
) -> GlobalResult<()> {
	tracing::info!("validating files");

	let files_len = files.len();
	futures_util::stream::iter(files.into_iter().enumerate())
		.map(|(i, file_row)| async move {
			if let Some(multipart_upload_id) = &file_row.multipart_upload_id {
				tracing::info!(?file_row, "completing multipart upload");

				// Fetch all parts
				let parts_res = s3_client
					.list_parts()
					.bucket(s3_client.bucket())
					.key(format!("{}/{}", upload_id, file_row.path))
					.upload_id(multipart_upload_id.clone())
					.send()
					.await?;
				let parts = parts_res.parts();

				s3_client
					.complete_multipart_upload()
					.bucket(s3_client.bucket())
					.key(format!("{}/{}", upload_id, file_row.path))
					.upload_id(multipart_upload_id)
					.multipart_upload(
						s3_util::aws_sdk_s3::types::CompletedMultipartUpload::builder()
							.set_parts(Some(parts.iter().map(|part| {
								Ok(s3_util::aws_sdk_s3::types::CompletedPart::builder()
									.part_number(unwrap!(part.part_number()))
									.set_e_tag(part.e_tag().map(|s| s.to_owned()))
									.build()
							)}).collect::<GlobalResult<Vec<_>>>()?))
							.build()
					)
					.send()
					.await?;
			}

			// Fetch & validate file metadata
			let mut fail_idx = 0;
			let head_obj = loop {
				let head_obj_res = s3_client
					.head_object()
					.bucket(s3_client.bucket())
					.key(format!("{}/{}", upload_id, file_row.path))
					.send()
					.await;
				match head_obj_res {
					Ok(x) => break x,
					Err(err) => {
						fail_idx += 1;

						if fail_idx > 4 {
							tracing::error!(?fail_idx, "head object failed too many times");
							return Err(err.into());
						} else {
							tracing::warn!(?fail_idx, "head object failed, retrying due to likely benign error from backblaze with malformed last-modified header");
							tokio::time::sleep(Duration::from_millis(500)).await;
						}
					}
				}
			};

			// This should never be triggered since we use prepared uploads, but
			// we validate it regardless
			ensure_eq!(
				file_row.content_length,
				unwrap!(head_obj.content_length),
				"incorrect content length"
			);

			if i % 1000 == 0 {
				tracing::info!("fetched file metadata ({i}/{files_len})")
			}

			GlobalResult::Ok(())
		})
		.buffer_unordered(32)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}
