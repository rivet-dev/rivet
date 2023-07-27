use std::{collections::HashSet, time::Duration};

use futures_util::{StreamExt, TryStreamExt};
use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde_json::json;

const MAX_UPLOAD_SIZE: u64 = util::file_size::gigabytes(10);

#[operation(name = "upload-prepare")]
async fn handle(
	ctx: OperationContext<upload::prepare::Request>,
) -> GlobalResult<upload::prepare::Response> {
	let crdb = ctx.crdb("db-upload").await?;
	let provider = if let Some(provider) = ctx.provider {
		let proto_provider = internal_unwrap_owned!(
			backend::upload::Provider::from_i32(provider),
			"invalid upload provider"
		);

		match proto_provider {
			backend::upload::Provider::Minio => s3_util::Provider::Minio,
			backend::upload::Provider::Backblaze => s3_util::Provider::Backblaze,
			backend::upload::Provider::Aws => s3_util::Provider::Aws,
		}
	} else {
		s3_util::Provider::default()?
	};

	let s3_client_external =
		s3_util::Client::from_env_opt(&ctx.bucket, provider, s3_util::EndpointKind::External)
			.await?;

	let total_content_length = ctx.files.iter().fold(0, |acc, x| acc + x.content_length);
	tracing::info!(len = ?ctx.files.len(), ?total_content_length, "file info");
	internal_assert!(
		total_content_length < MAX_UPLOAD_SIZE,
		"file size must be < 10 gb"
	);

	let user_id = ctx.user_id.map(|x| x.as_uuid());

	// Check that there's no duplicate paths
	let mut registered_paths = HashSet::new();
	for file in &ctx.files {
		tracing::info!(?file, "signing url");

		internal_assert!(
			registered_paths.insert(file.path.clone()),
			"duplicate file path"
		);
	}

	// Prepare columns for files
	let upload_id = Uuid::new_v4();
	let _upload_ids = ctx.files.iter().map(|_| upload_id).collect::<Vec<_>>();
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
	let nsfw_score_thresholds = ctx
		.files
		.iter()
		.map(|x| x.nsfw_score_threshold)
		.collect::<Vec<_>>();

	// Insert in to database
	sqlx::query(indoc!(
		"
		WITH
			_insert_upload AS (
				INSERT INTO uploads (upload_id, create_ts, content_length, bucket, user_id)
				VALUES ($1, $2, $3, $4, $5)
				RETURNING 1
			),
			_insert_files AS (
				INSERT INTO upload_files (upload_id, path, mime, content_length, nsfw_score_threshold)
				SELECT $1, rows.*
				FROM unnest($6, $7, $8, $9) AS rows
				RETURNING 1
			)
		SELECT 1
		"
	))
	// Upload
	.bind(upload_id)
	.bind(ctx.ts())
	.bind(total_content_length as i64)
	.bind(&ctx.bucket)
	.bind(user_id)
	// Files
	.bind(paths)
	.bind(mimes)
	.bind(content_lengths)
	.bind(nsfw_score_thresholds)
	.execute(&crdb)
	.await?;

	// Create an upload URL with the given token
	let presigned_requests = futures_util::stream::iter(ctx.files.iter().cloned())
		.map(move |file| {
			let s3_client_external = s3_client_external.clone();
			async move {
				tracing::info!(?file, "signing url");

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
						s3_util::aws_sdk_s3::presigning::config::PresigningConfig::builder()
							.expires_in(Duration::from_secs(60 * 60))
							.build()?,
					)
					.await?;

				GlobalResult::Ok(backend::upload::PresignedUploadRequest {
					path: file.path.clone(),
					url: presigned_upload_req.uri().to_string(),
				})
			}
		})
		.buffer_unordered(32)
		.try_collect::<Vec<_>>()
		.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
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
