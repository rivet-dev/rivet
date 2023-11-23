use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(clickhouse::Row, serde::Deserialize)]
struct LogEntry {
	ts: i64,
	message: Vec<u8>,
}

#[operation(name = "job-log-read")]
async fn handle(
	ctx: OperationContext<job_log::read::Request>,
) -> GlobalResult<job_log::read::Response> {
	let clickhouse = rivet_pools::utils::clickhouse::client()?
		.with_user("chirp")
		.with_password(util::env::read_secret(&["clickhouse", "users", "chirp", "password"]).await?)
		.with_database("db_job_log");

	let run_id = unwrap_ref!(ctx.run_id).as_uuid();
	let req_query = unwrap_ref!(ctx.query);

	let entries = match req_query {
		job_log::read::request::Query::All(_) => query_all(ctx.body(), &clickhouse, run_id).await?,
		job_log::read::request::Query::BeforeTs(ts) => {
			query_before_ts(ctx.body(), &clickhouse, run_id, *ts).await?
		}
		job_log::read::request::Query::AfterTs(ts) => {
			query_after_ts(ctx.body(), &clickhouse, run_id, *ts).await?
		}
	};

	Ok(job_log::read::Response { entries })
}

async fn query_all(
	req: &job_log::read::Request,
	clickhouse: &clickhouse::Client,
	run_id: Uuid,
) -> GlobalResult<Vec<backend::job::log::LogEntry>> {
	let mut entries_cursor = clickhouse
		.query(indoc!(
			"
			SELECT ts, message
			FROM run_logs
			WHERE run_id = ? AND task = ? AND stream_type = ?
			ORDER BY ts ASC
			LIMIT ?
			"
		))
		.bind(run_id)
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
	req: &job_log::read::Request,
	clickhouse: &clickhouse::Client,
	run_id: Uuid,
	ts: i64,
) -> GlobalResult<Vec<backend::job::log::LogEntry>> {
	let mut entries_cursor = clickhouse
		.query(indoc!(
			"
			SELECT ts, message
			FROM logs
			WHERE run_id = ? AND task = ? AND stream_type = ? AND ts <= fromUnixTimestamp64Milli(?)
			ORDER BY ts DESC
			LIMIT ?
			"
		))
		.bind(run_id)
		.bind(&req.task)
		.bind(req.stream_type as u8)
		.bind(ts)
		.bind(req.count)
		.fetch::<LogEntry>()?;

	let mut entries = Vec::new();
	while let Some(entry) = entries_cursor.next().await? {
		entries.push(convert_entry(entry));
	}

	// Sort in asc order
	entries.sort_by_key(|x| x.ts);

	Ok(entries)
}

async fn query_after_ts(
	req: &job_log::read::Request,
	clickhouse: &clickhouse::Client,
	run_id: Uuid,
	ts: i64,
) -> GlobalResult<Vec<backend::job::log::LogEntry>> {
	let mut entries_cursor = clickhouse
		.query(indoc!(
			"
			SELECT ts, message
			FROM logs
			WHERE run_id = ? AND task = ? AND stream_type = ? AND ts >= fromUnixTimestamp64Milli(?)
			ORDER BY ts ASC
			LIMIT ?
			"
		))
		.bind(run_id)
		.bind(&req.task)
		.bind(req.stream_type as u8)
		.bind(ts)
		.bind(req.count)
		.fetch::<LogEntry>()?;

	let mut entries = Vec::new();
	while let Some(entry) = entries_cursor.next().await? {
		entries.push(convert_entry(entry));
	}

	Ok(entries)
}

fn convert_entry(entry: LogEntry) -> backend::job::log::LogEntry {
	backend::job::log::LogEntry {
		ts: entry.ts,
		message: entry.message,
	}
}
