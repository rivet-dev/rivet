use chirp_worker::prelude::*;
use futures_util::stream::StreamExt;
use proto::backend::{self, pkg::*};

#[derive(clickhouse::Row, serde::Deserialize)]
struct LogEntry {
	message: Vec<u8>,
}

#[worker(name = "nomad-log-export")]
async fn worker(ctx: OperationContext<nomad_log::msg::export::Message>) -> GlobalResult<()> {
	let clickhouse_url = std::env::var("CLICKHOUSE_URL")?;
	let clickhouse = clickhouse::Client::default()
		.with_url(clickhouse_url)
		.with_user("chirp")
		.with_password(util::env::read_secret(&["clickhouse", "users", "chirp", "password"]).await?)
		.with_database("db_nomad_logs");

	let request_id = internal_unwrap!(ctx.request_id).as_uuid();

	let stream_type =
		internal_unwrap_owned!(backend::nomad_log::StreamType::from_i32(ctx.stream_type));
	let file_name = match stream_type {
		backend::nomad_log::StreamType::StdOut => "stdout.txt",
		backend::nomad_log::StreamType::StdErr => "stderr.txt",
	};

	let mut entries_cursor = clickhouse
		.query(indoc!(
			"
			SELECT message
			FROM logs
			WHERE alloc = ? AND task = ? AND stream_type = ? AND ts < ?
			ORDER BY ts ASC, idx ASC
			"
		))
		.bind(&ctx.alloc)
		.bind(&ctx.task)
		.bind(ctx.stream_type as i8)
		.bind(ctx.ts())
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
		bucket: "bucket-nomad-log-export".into(),
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

	let presigned_req = internal_unwrap_owned!(upload_res.presigned_requests.first());
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
		internal_panic!("failed to upload");
	}

	op!([ctx] upload_complete {
		upload_id: upload_res.upload_id,
		bucket: Some("bucket-nomad-log-export".into()),
	})
	.await?;

	msg!([ctx] nomad_log::msg::export_complete(request_id) {
		request_id: Some(request_id.into()),
		upload_id: upload_res.upload_id,
	})
	.await?;

	Ok(())
}
