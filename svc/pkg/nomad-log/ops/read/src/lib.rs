use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(clickhouse::Row, serde::Deserialize)]
struct LogEntry {
	ts: i64,
	idx: u32,
	message: Vec<u8>,
}

#[operation(name = "nomad-log-read")]
async fn handle(
	ctx: OperationContext<nomad_log::read::Request>,
) -> GlobalResult<nomad_log::read::Response> {
	let clickhouse_url = std::env::var("CLICKHOUSE_URL")?;
	let clickhouse = clickhouse::Client::default()
		.with_url(clickhouse_url)
		.with_user("chirp")
		.with_password(util::env::read_secret(&["clickhouse", "users", "chirp", "password"]).await?)
		.with_database("db_nomad_logs");

	let req_query = internal_unwrap!(ctx.query);

	let entries = match req_query {
		nomad_log::read::request::Query::All(_) => query_all(ctx.body(), &clickhouse).await?,
		nomad_log::read::request::Query::BeforeTs(ts_query) => {
			query_before_ts(ctx.body(), &clickhouse, ts_query).await?
		}
		nomad_log::read::request::Query::AfterTs(ts_query) => {
			query_after_ts(ctx.body(), &clickhouse, ts_query).await?
		}
	};

	Ok(nomad_log::read::Response { entries })
}

async fn query_all(
	req: &nomad_log::read::Request,
	clickhouse: &clickhouse::Client,
) -> GlobalResult<Vec<backend::nomad_log::LogEntry>> {
	let mut entries_cursor = clickhouse
		.query(indoc!(
			"
			SELECT ts, idx, message
			FROM logs
			WHERE alloc = ? AND task = ? AND stream_type = ?
			ORDER BY ts ASC, idx ASC
			LIMIT ?
			"
		))
		.bind(&req.alloc)
		.bind(&req.task)
		.bind(req.stream_type as u8)
		.bind(req.count)
		.fetch::<LogEntry>()?;

	let mut entries = Vec::new();
	while let Some(entry) = entries_cursor.next().await? {
		entries.push(convert_entry(entry));
	}

	Ok(entries)
}

async fn query_before_ts(
	req: &nomad_log::read::Request,
	clickhouse: &clickhouse::Client,
	ts_query: &nomad_log::read::request::TimestampQuery,
) -> GlobalResult<Vec<backend::nomad_log::LogEntry>> {
	let mut entries_cursor = clickhouse
		.query(indoc!(
			"
			SELECT ts, idx, message
			FROM logs
			WHERE alloc = ? AND task = ? AND stream_type = ? AND ts <= fromUnixTimestamp64Milli(?)
			ORDER BY ts DESC, idx DESC
			LIMIT ?
			"
		))
		.bind(&req.alloc)
		.bind(&req.task)
		.bind(req.stream_type as u8)
		.bind(ts_query.ts)
		.bind(req.count)
		.fetch::<LogEntry>()?;

	let mut entries = Vec::new();
	while let Some(entry) = entries_cursor.next().await? {
		// Filter the log idx if the ts is the same
		if entry.ts < ts_query.ts || entry.idx <= ts_query.idx {
			entries.push(convert_entry(entry));
		}
	}

	// Sort in asc order
	entries.sort_by_key(|x| (x.ts, x.idx));

	Ok(entries)
}

async fn query_after_ts(
	req: &nomad_log::read::Request,
	clickhouse: &clickhouse::Client,
	ts_query: &nomad_log::read::request::TimestampQuery,
) -> GlobalResult<Vec<backend::nomad_log::LogEntry>> {
	let mut entries_cursor = clickhouse
		.query(indoc!(
			"
			SELECT ts, idx, message
			FROM logs
			WHERE alloc = ? AND task = ? AND stream_type = ? AND ts >= fromUnixTimestamp64Milli(?)
			ORDER BY ts ASC, idx ASC
			LIMIT ?
			"
		))
		.bind(&req.alloc)
		.bind(&req.task)
		.bind(req.stream_type as u8)
		.bind(ts_query.ts)
		.bind(req.count)
		.fetch::<LogEntry>()?;

	let mut entries = Vec::new();
	while let Some(entry) = entries_cursor.next().await? {
		// Filter the log idx if the ts is the same
		if entry.ts > ts_query.ts || entry.idx >= ts_query.idx {
			entries.push(convert_entry(entry));
		}
	}

	Ok(entries)
}

fn convert_entry(entry: LogEntry) -> backend::nomad_log::LogEntry {
	backend::nomad_log::LogEntry {
		ts: entry.ts,
		idx: entry.idx,
		message: entry.message,
	}
}
