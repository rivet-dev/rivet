use chirp_workflow::prelude::*;
use rivet_operation::prelude::proto::backend;

use crate::types::LogsStreamType;

#[derive(Debug)]
pub struct Input {
	pub actor_id: Uuid,
	pub stream_type: LogsStreamType,
}

#[derive(Debug)]
pub struct Output {
	pub upload_id: Uuid,
}

#[derive(clickhouse::Row, serde::Deserialize)]
pub struct LogEntry {
	pub message: Vec<u8>,
}

#[operation]
pub async fn pegboard_actor_log_read(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let file_name = match input.stream_type {
		LogsStreamType::StdOut => "stdout.txt",
		LogsStreamType::StdErr => "stderr.txt",
	};

	let mut entries_cursor = ctx
		.clickhouse()
		.await?
		.query(indoc!(
			"
			SELECT message
			FROM db_pegboard_actor_log.actor_logs
			WHERE
				actor_id = ? AND
				stream_type = ?
			ORDER BY ts ASC
			"
		))
		.bind(input.actor_id)
		.bind(input.stream_type as i8)
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
		bucket: "bucket-actor-log-export".into(),
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
		bucket: Some("bucket-pegboard-log-export".into()),
	})
	.await?;

	Ok(Output {
		upload_id: unwrap!(upload_res.upload_id).as_uuid(),
	})
}
