use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(clickhouse::Row, serde::Deserialize)]
struct LogEntry {
	message: Vec<u8>,
}

#[operation(name = "ds-log-export")]
pub async fn handle(
	ctx: OperationContext<ds_log::export::Request>,
) -> GlobalResult<ds_log::export::Response> {
	let request_id = unwrap_ref!(ctx.request_id).as_uuid();
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();

	let stream_type = unwrap!(backend::ds::log::StreamType::from_i32(ctx.stream_type));
	let file_name = match stream_type {
		backend::ds::log::StreamType::StdOut => "stdout.txt",
		backend::ds::log::StreamType::StdErr => "stderr.txt",
	};

	let mut entries_cursor = ctx
		.clickhouse()
		.await?
		.query(indoc!(
			"
			SELECT
				message
			FROM
				db_ds_log.server_logs
			WHERE
				server_id = ?
				AND stream_type = ?
			ORDER BY
				ts
			ASC
			"
		))
		.bind(server_id)
		.bind(ctx.stream_type as i8)
		.fetch::<LogEntry>()?;

	let mut lines = 0;
	let mut buf = Vec::new();
	while let Some(mut entry) = entries_cursor.next().await? {
		buf.append(&mut entry.message);
		buf.push(b'\n');
		lines += 1;
	}

	tracing::info!(?lines, bytes = ?buf.len(), "read all logs");

	// Upload log
	let mime = "text/plain";
	let content_length = buf.len();
	let upload_res = op!([ctx] upload_prepare {
		bucket: "bucket-ds-log-export".into(),
		files: vec![
			backend::upload::PrepareFile {
				path: file_name.into(),
				mime: Some(mime.into()),
				content_length: content_length as u64,
				..Default::default()
			},
		],
	})
	.await?;

	let presigned_req = unwrap!(upload_res.presigned_requests.first());
	let res = reqwest::Client::new()
		.put(&presigned_req.url)
		.body(buf)
		.header(reqwest::header::CONTENT_TYPE, mime)
		.header(reqwest::header::CONTENT_LENGTH, content_length)
		.send()
		.await?;
	if res.status().is_success() {
		tracing::info!("uploaded successfully");
	} else {
		let status = res.status();
		let text = res.text().await;
		tracing::error!(?status, ?text, "failed to upload");
		bail!("failed to upload");
	}

	op!([ctx] upload_complete {
		upload_id: upload_res.upload_id,
		bucket: Some("bucket-ds-log-export".into()),
	})
	.await?;

	Ok(ds_log::export::Response {
		request_id: Some(request_id.into()),
		upload_id: upload_res.upload_id,
	})
}
