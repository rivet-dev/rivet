use std::collections::HashMap;

use chirp_worker::prelude::*;
use futures_util::stream::{StreamExt, TryStreamExt};
use proto::backend::pkg::*;

#[derive(sqlx::FromRow)]
struct UploadRow {
	upload_id: Uuid,
	bucket: String,
}

#[derive(sqlx::FromRow)]
struct FileRow {
	upload_id: Uuid,
	path: String,
}

struct BucketDeletions {
	client: s3_util::Client,
	keys: Vec<String>,
}

#[worker(name = "upload-delete")]
async fn worker(ctx: &OperationContext<upload::msg::delete::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb("db-upload").await?;

	let request_id = internal_unwrap!(ctx.request_id).as_uuid();
	let upload_ids = ctx
		.upload_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let uploads = sqlx::query_as::<_, UploadRow>(indoc!(
		"
		SELECT upload_id, bucket
		FROM uploads
		WHERE upload_id = ANY($1)
		"
	))
	.bind(&upload_ids)
	.fetch_all(&crdb)
	.await?;

	let upload_files = sqlx::query_as::<_, FileRow>(indoc!(
		"
		SELECT upload_id, path
		FROM upload_files
		WHERE upload_id = ANY($1)
		"
	))
	.bind(&upload_ids)
	.fetch_all(&crdb)
	.await?;

	// Compile uploads into hashmap by bucket
	let mut deletions: HashMap<String, BucketDeletions> =
		HashMap::with_capacity(upload_files.len());
	for upload_file in upload_files {
		let upload = internal_unwrap_owned!(uploads
			.iter()
			.find(|upload| upload.upload_id == upload_file.upload_id));
		let key = format!("{}/{}", upload_file.upload_id, upload_file.path);

		if let Some(x) = deletions.get_mut(&upload.bucket) {
			x.keys.push(key);
		} else {
			let client = s3_util::Client::from_env(&upload.bucket).await?;

			deletions.insert(
				upload.bucket.clone(),
				BucketDeletions {
					client,
					keys: vec![key],
				},
			);
		}
	}

	let counts = deletions
		.iter()
		.map(|(bucket, deletion)| (bucket, deletion.keys.len()))
		.collect::<HashMap<_, _>>();
	tracing::info!(deletions=?counts, "deleting");

	// Execute batch deletions per bucket
	futures_util::stream::iter(deletions)
		.map(|(_, deletion)| {
			let delete = s3_util::aws_sdk_s3::model::Delete::builder()
				.set_objects(Some(
					deletion
						.keys
						.iter()
						.map(|key| {
							s3_util::aws_sdk_s3::model::ObjectIdentifier::builder()
								.key(key)
								.build()
						})
						.collect::<Vec<_>>(),
				))
				.build();

			deletion
				.client
				.delete_objects()
				.bucket(deletion.client.bucket())
				.delete(delete)
				.send()
		})
		.buffer_unordered(32)
		.try_collect::<Vec<_>>()
		.await?;

	// Mark upload as deleted
	sqlx::query(indoc!(
		"
		UPDATE uploads
		SET deleted_ts = $2
		WHERE upload_id = ANY($1)
		"
	))
	.bind(&upload_ids)
	.bind(ctx.ts())
	.execute(&crdb)
	.await?;

	msg!([ctx] upload::msg::delete_complete(request_id) {
		request_id: ctx.request_id,
		upload_ids: ctx.upload_ids.clone(),
	})
	.await?;

	Ok(())
}
