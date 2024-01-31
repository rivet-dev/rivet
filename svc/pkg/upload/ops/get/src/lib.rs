use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct UploadRow {
	bucket: String,
	upload_id: Uuid,
	create_ts: i64,
	content_length: i64,
	complete_ts: Option<i64>,
	deleted_ts: Option<i64>,
	user_id: Option<Uuid>,
	provider: i64,
}

impl From<UploadRow> for backend::upload::Upload {
	fn from(val: UploadRow) -> Self {
		Self {
			bucket: val.bucket,
			upload_id: Some(val.upload_id.into()),
			create_ts: val.create_ts,
			content_length: val.content_length as u64,
			complete_ts: val.complete_ts,
			deleted_ts: val.deleted_ts,
			user_id: val.user_id.map(Into::into),
			provider: val.provider as i32,
		}
	}
}

#[operation(name = "upload-get")]
async fn handle(
	ctx: OperationContext<upload::get::Request>,
) -> GlobalResult<upload::get::Response> {
	let _crdb = ctx.crdb().await?;

	let upload_ids = ctx
		.upload_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let uploads = ctx
		.cache()
		.fetch_all_proto("upload", upload_ids, {
			let ctx = ctx.clone();
			move |mut cache, upload_ids| {
				let ctx = ctx.clone();
				async move {
					let uploads = sql_fetch_all!(
						[ctx, UploadRow]
						"
						SELECT
							bucket,
							upload_id,
							create_ts,
							content_length,
							complete_ts,
							deleted_ts,
							user_id,
							provider
						FROM db_upload.uploads
						WHERE upload_id = ANY($1)
						",
						upload_ids,
					)
					.await?;

					for row in uploads {
						cache.resolve(&row.upload_id.clone(), row.into());
					}

					Ok(cache)
				}
			}
		})
		.await?;

	Ok(upload::get::Response { uploads })
}
