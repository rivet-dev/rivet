use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow, Clone)]
struct FileRow {
	upload_id: Uuid,
	path: String,
	mime: Option<String>,
	content_length: i64,
	multipart_upload_id: Option<String>,
}

impl From<FileRow> for backend::upload::UploadFile {
	fn from(val: FileRow) -> Self {
		Self {
			upload_id: Some(val.upload_id.into()),
			path: val.path,
			mime: val.mime,
			content_length: val.content_length as u64,
			multipart_upload_id: val.multipart_upload_id,
		}
	}
}

#[operation(name = "upload-file-list", timeout = 60)]
pub async fn handle(
	ctx: OperationContext<upload::file_list::Request>,
) -> GlobalResult<upload::file_list::Response> {
	let crdb = ctx.crdb().await?;

	let upload_ids = ctx
		.upload_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let files = ctx
		.cache()
		.fetch_all_proto("upload_files", upload_ids, move |mut cache, upload_ids| {
			let crdb = crdb.clone();
			async move {
				let rows = sqlx::query_as::<_, FileRow>(indoc!(
					"
					SELECT upload_id, path, mime, content_length, multipart_upload_id
					FROM db_upload.upload_files
					WHERE upload_id = ANY($1)
					"
				))
				.bind(&upload_ids)
				.fetch_all(&crdb)
				.await?;

				// Cache the file list for each upload ID
				for upload_id in upload_ids {
					let files = rows
						.iter()
						.filter(|x| x.upload_id == upload_id)
						.cloned()
						.map(Into::<backend::upload::UploadFile>::into)
						.collect::<Vec<_>>();
					cache.resolve(&upload_id, upload::file_list::Response { files });
				}

				Ok(cache)
			}
		})
		.await?
		.into_iter()
		.flat_map(|x| x.files)
		.collect();

	Ok(upload::file_list::Response { files })
}
