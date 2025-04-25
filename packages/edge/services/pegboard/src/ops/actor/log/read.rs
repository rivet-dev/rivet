use chirp_workflow::prelude::*;

use crate::types::LogsStreamType;

#[derive(Debug)]
pub struct Input {
	pub actor_ids: Vec<Uuid>,
	pub stream_types: Vec<LogsStreamType>,
	pub count: i64,
	pub order_by: Order,
	pub query: Query,
	pub search_text: Option<String>,
	pub search_case_sensitive: Option<bool>,
	pub search_enable_regex: Option<bool>,
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
pub async fn pegboard_actor_log_read(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
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

	// Prepare search parameters
	let search_text = input.search_text.as_deref().unwrap_or("");
	let apply_search = !search_text.is_empty();
	let enable_regex = input.search_enable_regex.unwrap_or(false);
	let case_sensitive = input.search_case_sensitive.unwrap_or(false);

	// Pre-format the regex strings with or without case sensitivity
	let regex_text = if case_sensitive {
		search_text.to_string()
	} else {
		format!("(?i){}", search_text)
	};

	// Direction for ordering
	let order_direction = match input.order_by {
		Order::Asc => "ASC",
		Order::Desc => "DESC",
	};

	// ?? = escaped ?
	let query = formatdoc!(
		"
		SELECT
			ts,
			message,
			stream_type,
			toString(actor_id) as actor_id_str
		FROM
			db_pegboard_actor_log.actor_logs
		WHERE
			actor_id IN ?
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
			-- Search filtering with conditional logic
			AND (
				NOT ? -- NOT apply_search (always true when search not applied)
				OR (
					CASE 
						WHEN ? THEN -- enable_regex
							-- Using pre-formatted regex string
							match(message, ?)
						ELSE 
							-- Toggle for case sensitivity without regex
							CASE 
								WHEN ? THEN position(message, ?) > 0
								ELSE positionCaseInsensitive(message, ?) > 0
							END
					END
				)
			)
		-- Use dynamic direction directly in the ORDER BY clause
		ORDER BY ts {order_direction}
		LIMIT
			?
		"
	);

	// Build query with all parameters and safety restrictions
	let query_builder = clickhouse
		.query(&query)
		.bind(&input.actor_ids)
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
		.bind(before_nts.unwrap_or(0))
		// Search parameters
		.bind(apply_search)
		.bind(enable_regex)
		.bind(regex_text)
		.bind(case_sensitive)
		.bind(search_text)
		.bind(search_text.to_lowercase())
		// Limit
		.bind(input.count);

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
