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
		}
	}
}

#[operation(name = "upload-get")]
async fn handle(
	ctx: OperationContext<upload::get::Request>,
) -> GlobalResult<upload::get::Response> {
	let crdb = ctx.crdb("db-upload").await?;

	let upload_ids = ctx
		.upload_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let uploads = sqlx::query_as::<_, UploadRow>(indoc!(
		"
		SELECT
			bucket,
			upload_id,
			create_ts,
			content_length,
			complete_ts,
			deleted_ts,
			user_id
		FROM uploads
		WHERE upload_id = ANY($1)
		"
	))
	.bind(upload_ids)
	.fetch_all(&crdb)
	.await?
	.into_iter()
	.map(Into::<backend::upload::Upload>::into)
	.collect::<Vec<_>>();

	Ok(upload::get::Response { uploads })
}
