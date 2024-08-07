use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(clickhouse::Row, serde::Deserialize)]
struct LogEntry {
	// In nanoseconds
	ts: i64,
	message: Vec<u8>,
}

#[operation(name = "ds-log-read")]
async fn handle(
	ctx: OperationContext<ds_log::read::Request>,
) -> GlobalResult<ds_log::read::Response> {
	let clickhouse = ctx.clickhouse().await?;

	let server_id = unwrap_ref!(ctx.server_id).as_uuid();
	let req_query = unwrap_ref!(ctx.query);

	let order_by = if ctx.order_asc { "ASC" } else { "DESC" };

	let entries = match req_query {
		ds_log::read::request::Query::All(_) => {
			query_all(ctx.body(), &clickhouse, server_id, order_by).await?
		}
		ds_log::read::request::Query::BeforeNts(nts) => {
			query_before_nts(ctx.body(), &clickhouse, server_id, *nts, order_by).await?
		}
		ds_log::read::request::Query::AfterNts(nts) => {
			query_after_nts(ctx.body(), &clickhouse, server_id, *nts, order_by).await?
		}
		ds_log::read::request::Query::NtsRange(query) => {
			query_nts_range(
				ctx.body(),
				&clickhouse,
				server_id,
				query.after_nts,
				query.before_nts,
				order_by,
			)
			.await?
		}
	};

	Ok(ds_log::read::Response { entries })
}

async fn query_all(
	req: &ds_log::read::Request,
	clickhouse: &ClickHousePool,
	run_id: Uuid,
	order_by: &str,
) -> GlobalResult<Vec<backend::ds::log::LogEntry>> {
	let mut entries_cursor = clickhouse
		.query(&formatdoc!(
			"
			SELECT
				ts,
				message
			FROM
				db_ds_log.server_logs
			WHERE
				server_id = ?
				AND stream_type = ?
			ORDER BY
				ts {order_by}
			LIMIT
				?
			"
		))
		.bind(run_id)
		.bind(req.stream_type as u8)
		.bind(req.count)
		.fetch::<LogEntry>()?;

	let mut entries = Vec::new();
	while let Some(entry) = entries_cursor.next().await? {
		entries.push(convert_entry(entry));
	}

	Ok(entries)
}

async fn query_before_nts(
	req: &ds_log::read::Request,
	clickhouse: &ClickHousePool,
	run_id: Uuid,
	nts: i64,
	order_by: &str,
) -> GlobalResult<Vec<backend::ds::log::LogEntry>> {
	let mut entries_cursor = clickhouse
		.query(&formatdoc!(
			"
			SELECT
				ts,
				message
			FROM
				db_ds_log.server_logs
			WHERE
				server_id = ?
				AND stream_type = ?
				AND ts < fromUnixTimestamp64Nano(?)
			ORDER BY
				ts {order_by}
			LIMIT
				?
			"
		))
		.bind(run_id)
		.bind(req.stream_type as u8)
		.bind(nts)
		.bind(req.count)
		.fetch::<LogEntry>()?;

	let mut entries = Vec::new();
	while let Some(entry) = entries_cursor.next().await? {
		entries.push(convert_entry(entry));
	}

	Ok(entries)
}

async fn query_after_nts(
	req: &ds_log::read::Request,
	clickhouse: &ClickHousePool,
	run_id: Uuid,
	nts: i64,
	order_by: &str,
) -> GlobalResult<Vec<backend::ds::log::LogEntry>> {
	let mut entries_cursor = clickhouse
		.query(&formatdoc!(
			"
			SELECT
				ts,
				message
			FROM
				db_ds_log.server_logs
			WHERE
				server_id = ?
				AND stream_type = ?
				AND ts > fromUnixTimestamp64Nano(?)
			ORDER BY
				ts {order_by}
			LIMIT
				?
			"
		))
		.bind(run_id)
		.bind(req.stream_type as u8)
		.bind(nts)
		.bind(req.count)
		.fetch::<LogEntry>()?;

	let mut entries = Vec::new();
	while let Some(entry) = entries_cursor.next().await? {
		entries.push(convert_entry(entry));
	}

	Ok(entries)
}

async fn query_nts_range(
	req: &ds_log::read::Request,
	clickhouse: &ClickHousePool,
	run_id: Uuid,
	after_nts: i64,
	before_nts: i64,
	order_by: &str,
) -> GlobalResult<Vec<backend::ds::log::LogEntry>> {
	let mut entries_cursor = clickhouse
		.query(&formatdoc!(
			"
			SELECT
				ts,
				message
			FROM
				db_ds_log.server_logs
			WHERE
				run_id = ?
				AND stream_type = ?
				AND ts > fromUnixTimestamp64Nano(?)
				AND ts < fromUnixTimestamp64Nano(?)
			ORDER BY
				ts {order_by}
			LIMIT
				?
			"
		))
		.bind(run_id)
		.bind(req.stream_type as u8)
		.bind(after_nts)
		.bind(before_nts)
		.bind(req.count)
		.fetch::<LogEntry>()?;

	let mut entries = Vec::new();
	while let Some(entry) = entries_cursor.next().await? {
		entries.push(convert_entry(entry));
	}

	Ok(entries)
}

fn convert_entry(entry: LogEntry) -> backend::ds::log::LogEntry {
	backend::ds::log::LogEntry {
		nts: entry.ts,
		message: entry.message,
	}
}
