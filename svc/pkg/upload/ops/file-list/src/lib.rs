use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct FileRow {
	upload_id: Uuid,
	path: String,
	mime: Option<String>,
	content_length: i64,
}

impl From<FileRow> for backend::upload::UploadFile {
	fn from(val: FileRow) -> Self {
		Self {
			upload_id: Some(val.upload_id.into()),
			path: val.path,
			mime: val.mime,
			content_length: val.content_length as u64,
		}
	}
}

#[operation(name = "upload-file-list", timeout = 60)]
pub async fn handle(
	ctx: OperationContext<upload::file_list::Request>,
) -> GlobalResult<upload::file_list::Response> {
	let crdb = ctx.crdb("db-upload").await?;

	let upload_ids = ctx
		.upload_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let files = sqlx::query_as::<_, FileRow>(indoc!(
		"
		SELECT upload_id, path, mime, content_length
		FROM upload_files
		WHERE upload_id = ANY($1)
		"
	))
	.bind(upload_ids)
	.fetch_all(&crdb)
	.await?
	.into_iter()
	.map(Into::<backend::upload::UploadFile>::into)
	.collect();

	Ok(upload::file_list::Response { files })
}
