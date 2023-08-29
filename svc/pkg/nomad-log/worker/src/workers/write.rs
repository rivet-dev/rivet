use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[derive(clickhouse::Row, serde::Serialize, Debug)]
struct LogEntry<'a> {
	alloc: &'a str,
	task: &'a str,
	stream_type: u8,
	ts: i64,
	idx: u32,
	message: &'a [u8],
}

#[worker(name = "nomad-log-write")]
async fn worker(ctx: &OperationContext<nomad_log::msg::entries::Message>) -> GlobalResult<()> {
	let client = clickhouse::Client::default()
		.with_url("http://http.clickhouse.service.consul:8123")
		.with_user("chirp")
		.with_password(util::env::read_secret(&["clickhouse", "users", "chirp", "password"]).await?)
		.with_database("db_nomad_logs");

	// Insert logs
	let mut insert = client.insert("logs")?;
	for entry in &ctx.entries {
		let entry_clickhouse = LogEntry {
			alloc: &ctx.alloc,
			task: &ctx.task,
			stream_type: ctx.stream_type as u8,
			ts: entry.ts,
			idx: entry.idx,
			message: &entry.message[..],
		};
		insert.write(&entry_clickhouse).await?;
	}
	insert.end().await?;

	Ok(())
}
