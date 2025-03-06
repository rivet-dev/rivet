use chirp_workflow::prelude::*;

use crate::types::LogsStreamType;

#[derive(Debug)]
pub struct Input {
	pub actor_id: Uuid,
	pub stream_type: LogsStreamType,
	pub count: i64,
	pub order_by: Order,
	pub query: Query,
}

#[derive(Debug, Clone, Copy)]
pub enum Query {
	All,
	BeforeNts(i64),
	AfterNts(i64),
	Range(i64, i64),
}

#[derive(Debug, Clone, Copy)]
pub enum Order {
	Asc,
	Desc,
}

#[derive(Debug)]
pub struct Output {
	pub entries: Vec<LogEntry>,
}

#[derive(Debug, clickhouse::Row, serde::Deserialize)]
pub struct LogEntry {
	/// In nanoseconds.
	pub ts: i64,
	pub message: Vec<u8>,
}

#[operation]
pub async fn pegboard_actor_log_read(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let clickhouse = ctx.clickhouse().await?;

	let entries = match input.query {
		Query::All => query_all(&input, &clickhouse).await?,
		Query::BeforeNts(nts) => query_before_nts(&input, &clickhouse, nts).await?,
		Query::AfterNts(nts) => query_after_nts(&input, &clickhouse, nts).await?,
		Query::Range(after_nts, before_nts) => {
			query_nts_range(&input, &clickhouse, after_nts, before_nts).await?
		}
	};

	Ok(Output { entries })
}

async fn query_all(input: &Input, clickhouse: &ClickHousePool) -> GlobalResult<Vec<LogEntry>> {
	let order_by = match input.order_by {
		Order::Asc => "ASC",
		Order::Desc => "DESC",
	};

	clickhouse
		.query(&formatdoc!(
			"
			SELECT
				ts,
				message
			FROM
				db_pegboard_actor_log.actor_logs
			WHERE
				actor_id = ?
				AND stream_type = ?
			ORDER BY
				ts {order_by}
			LIMIT
				?
			"
		))
		.bind(input.actor_id)
		.bind(input.stream_type as u8)
		.bind(input.count)
		.fetch_all::<LogEntry>()
		.await
		.map_err(Into::into)
}

async fn query_before_nts(
	input: &Input,
	clickhouse: &ClickHousePool,
	nts: i64,
) -> GlobalResult<Vec<LogEntry>> {
	let order_by = match input.order_by {
		Order::Asc => "ASC",
		Order::Desc => "DESC",
	};

	clickhouse
		.query(&formatdoc!(
			"
			SELECT ts, message
			FROM db_pegboard_actor_log.actor_logs
			WHERE
				actor_id = ? AND
				stream_type = ? AND
				ts < fromUnixTimestamp64Nano(?)
			ORDER BY ts {order_by}
			LIMIT ?
			"
		))
		.bind(input.actor_id)
		.bind(input.stream_type as u8)
		.bind(nts)
		.bind(input.count)
		.fetch_all::<LogEntry>()
		.await
		.map_err(Into::into)
}

async fn query_after_nts(
	input: &Input,
	clickhouse: &ClickHousePool,
	nts: i64,
) -> GlobalResult<Vec<LogEntry>> {
	let order_by = match input.order_by {
		Order::Asc => "ASC",
		Order::Desc => "DESC",
	};

	clickhouse
		.query(&formatdoc!(
			"
			SELECT ts, message
			FROM db_pegboard_actor_log.actor_logs
			WHERE
				actor_id = ? AND
				stream_type = ? AND
				ts > fromUnixTimestamp64Nano(?)
			ORDER BY ts {order_by}
			LIMIT ?
			"
		))
		.bind(input.actor_id)
		.bind(input.stream_type as u8)
		.bind(nts)
		.bind(input.count)
		.fetch_all::<LogEntry>()
		.await
		.map_err(Into::into)
}

async fn query_nts_range(
	input: &Input,
	clickhouse: &ClickHousePool,
	after_nts: i64,
	before_nts: i64,
) -> GlobalResult<Vec<LogEntry>> {
	let order_by = match input.order_by {
		Order::Asc => "ASC",
		Order::Desc => "DESC",
	};

	clickhouse
		.query(&formatdoc!(
			"
			SELECT ts, message
			FROM db_pegboard_actor_log.actor_logs
			WHERE
				actor_id = ? AND
				stream_type = ? AND
				ts > fromUnixTimestamp64Nano(?) AND
				ts < fromUnixTimestamp64Nano(?)
			ORDER BY ts {order_by}
			LIMIT ?
			"
		))
		.bind(input.actor_id)
		.bind(input.stream_type as u8)
		.bind(after_nts)
		.bind(before_nts)
		.bind(input.count)
		.fetch_all::<LogEntry>()
		.await
		.map_err(Into::into)
}
