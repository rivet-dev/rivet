use chirp_workflow::prelude::*;
use clickhouse_user_query::{QueryExpr, UserDefinedQueryBuilder};

use crate::schema::ACTOR_LOGS_SCHEMA;
use crate::types::LogsStreamType;

#[derive(Debug)]
pub struct Input {
	pub env_id: Uuid,
	pub actor_ids: Vec<Uuid>,
	pub stream_types: Vec<LogsStreamType>,
	pub count: i64,
	pub order_by: Order,
	pub query: Query,
	/// User-defined query expression for additional filtering
	pub user_query_expr: Option<QueryExpr>,
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
pub struct LogEntryRow {
	/// In nanoseconds.
	pub ts: i64,
	pub message: Vec<u8>,
	pub stream_type: u8,
	pub actor_id_str: String,
}

#[derive(Debug)]
pub struct LogEntry {
	/// In nanoseconds.
	pub ts: i64,
	pub message: Vec<u8>,
	pub stream_type: u8,
	pub actor_id: Uuid,
}

#[operation]
pub async fn pegboard_actor_log_read_with_query(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
	let clickhouse = ctx.clickhouse().await?;

	// Convert stream types to a vector of u8
	let stream_type_values: Vec<u8> = input.stream_types.iter().map(|&st| st as u8).collect();

	// Extract values from query enum
	let (is_all, is_before, is_after, before_nts, after_nts) = match input.query {
		Query::All => (true, false, false, None, None),
		Query::BeforeNts(nts) => (false, true, false, Some(nts), None),
		Query::AfterNts(nts) => (false, false, true, None, Some(nts)),
		Query::Range(after, before) => (false, true, true, Some(before), Some(after)),
	};

	// Direction for ordering
	let order_direction = match input.order_by {
		Order::Asc => "ASC",
		Order::Desc => "DESC",
	};

	// Build user query filter if provided
	let (user_query_where, user_query_builder) = if let Some(ref query_expr) = input.user_query_expr
	{
		let builder = UserDefinedQueryBuilder::new(&ACTOR_LOGS_SCHEMA, query_expr)
			.map_err(|e| GlobalError::raw(e))?;
		let where_clause = format!("AND ({})", builder.where_expr());
		(where_clause, Some(builder))
	} else {
		(String::new(), None)
	};

	// Convert actor IDs to strings for the query
	let actor_id_strings: Vec<String> = input.actor_ids.iter().map(|id| id.to_string()).collect();

	// Build the query
	let query = formatdoc!(
		"
		SELECT
			ts,
			message,
			stream_type,
			actor_id as actor_id_str
		FROM
			db_pegboard_actor_log.actor_logs3_with_metadata
		WHERE
			namespace = ?
			AND env_id = ?
			AND actor_id IN ?
			AND stream_type IN ?
			-- Apply timestamp filtering based on query type
			AND (
				? -- is_all
				OR (? AND ts < fromUnixTimestamp64Nano(?)) -- is_before
				OR (? AND ts > fromUnixTimestamp64Nano(?)) -- is_after
				OR (? AND ? AND 
					ts > fromUnixTimestamp64Nano(?) AND 
					ts < fromUnixTimestamp64Nano(?)) -- is_range
			)
			{user_query_where}
		-- Use dynamic direction directly in the ORDER BY clause
		ORDER BY ts {order_direction}
		LIMIT
			?
		"
	);

	// Build query with all parameters and safety restrictions
	let mut query_builder = clickhouse
		.query(&query)
		.bind(&ctx.config().server()?.rivet.namespace)
		.bind(input.env_id)
		.bind(&actor_id_strings)
		.bind(stream_type_values)
		// Query type parameters
		.bind(is_all)
		.bind(is_before)
		.bind(before_nts.unwrap_or(0))
		.bind(is_after)
		.bind(after_nts.unwrap_or(0))
		.bind(is_before) // First part of AND condition for range
		.bind(is_after) // Second part of AND condition for range
		.bind(after_nts.unwrap_or(0))
		.bind(before_nts.unwrap_or(0));

	// Bind user query parameters if present
	if let Some(builder) = user_query_builder {
		query_builder = builder.bind_to(query_builder);
	}

	// Add limit
	query_builder = query_builder.bind(input.count);

	let entries = query_builder
		.fetch_all::<LogEntryRow>()
		.await
		.map_err(|err| GlobalError::from(err))?
		.into_iter()
		.map(|x| {
			Ok(LogEntry {
				ts: x.ts,
				message: x.message,
				stream_type: x.stream_type,
				actor_id: unwrap!(
					Uuid::parse_str(&x.actor_id_str).ok(),
					"invalid actor log entry uuid"
				),
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(Output { entries })
}
