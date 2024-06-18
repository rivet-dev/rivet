use std::collections::{HashMap, HashSet};

use futures_util::stream::StreamExt;
use proto::backend::pkg::*;
use rivet_operation::prelude::*;

// NOTE: There's a bug in mm-lobby-cleanup that will upsert rows
#[derive(Debug, sqlx::FromRow)]
struct LobbyRow {
	namespace_id: Uuid,
	lobby_id: Option<Uuid>,
	region_id: Option<Uuid>,
	create_ts: Option<i64>,
	stop_ts: Option<i64>,
}

#[derive(Default)]
struct RegionAggregate {
	query_start: i64,
	query_end: i64,

	/// Total time in milliseconds for each (namespace_id, region_id)
	total_time: HashMap<(Uuid, Uuid), i64>,

	/// Lobbies that are included in the aggregation.
	processed_lobby_ids: HashSet<Uuid>,
}

impl RegionAggregate {
	fn process_lobby(&mut self, lobby_row: &LobbyRow) {
		// Unwrap values or ignore row
		let (lobby_id, region_id, create_ts) = if let (Some(a), Some(b), Some(c)) =
			(lobby_row.lobby_id, lobby_row.region_id, lobby_row.create_ts)
		{
			(a, b, c)
		} else {
			tracing::warn!(?lobby_row, "missing data in lobby row history");
			return;
		};

		// Check it's not already registered
		if self.processed_lobby_ids.contains(&lobby_id) {
			tracing::info!(%lobby_id, "lobby already processed");
			return;
		}

		// Derive start and stop ts
		let start_ts = create_ts;
		let stop_ts = lobby_row.stop_ts.unwrap_or(self.query_end);

		// Filter out lobbies that did not overlap with the given range
		if start_ts > self.query_end || stop_ts <= self.query_start {
			return;
		}

		// Add duration masked with the query range
		let duration = i64::min(stop_ts, self.query_end) - i64::max(start_ts, self.query_start);
		*self
			.total_time
			.entry((lobby_row.namespace_id, region_id))
			.or_insert(0) += duration;
		self.processed_lobby_ids.insert(lobby_id);
	}
}

#[operation(name = "mm-lobby-runtime-aggregate")]
async fn handle(
	ctx: OperationContext<mm::lobby_runtime_aggregate::Request>,
) -> GlobalResult<mm::lobby_runtime_aggregate::Response> {
	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();
	tracing::info!(?namespace_ids, "namespaces");

	let mut region_aggregate = RegionAggregate {
		query_start: ctx.query_start,
		query_end: ctx.query_end,
		..Default::default()
	};

	// Aggregate all lobbies that finished during the given query span.
	//
	// We do this after querying the lobbies that are still running in order to
	// ensure that we capture all lobbies in all states that may have stopped
	// while generating this aggregation.
	//
	// `LobbyAggregate` handles deduplication of aggregated lobbies from the
	// previous step.
	//
	// Use AS OF SYSTEM TIME to reduce contention.
	// https://www.cockroachlabs.com/docs/v22.2/performance-best-practices-overview#use-as-of-system-time-to-decrease-conflicts-with-long-running-queries
	let crdb = ctx.crdb().await?;
	let mut lobby_rows = sql_fetch!(
		[ctx, LobbyRow, &crdb]
		"
		SELECT namespace_id, lobby_id, region_id, create_ts, stop_ts
		FROM db_mm_state.lobbies AS OF SYSTEM TIME '-5s'
		WHERE namespace_id = ANY($1) AND (
			-- Lobbies stopped during the query window
			(stop_ts > $2 AND stop_ts <= $3) OR
			-- Lobbies created during the query window, these may already be stopped after query_end
			(create_ts > $2 AND create_ts <= $3) OR
			-- Lobbies still running that overlap with the query window
			(stop_ts IS NULL AND create_ts <= $3)
		)
		",
		&namespace_ids,
		ctx.query_start,
		ctx.query_end,
	);
	while let Some(lobby_row) = lobby_rows.next().await {
		let lobby_row = lobby_row?;
		region_aggregate.process_lobby(&lobby_row);
	}
	tracing::info!(
		total_time = ?region_aggregate.total_time,
		processed_len = ?region_aggregate.processed_lobby_ids.len(),
		"aggregated all lobbies"
	);

	// Build response
	let usage = region_aggregate
		.total_time
		.into_iter()
		.map(|((namespace_id, region_id), total_time)| {
			mm::lobby_runtime_aggregate::response::NamespaceUsage {
				namespace_id: Some(namespace_id.into()),
				region_id: Some(region_id.into()),
				total_time,
			}
		})
		.collect::<Vec<_>>();

	Ok(mm::lobby_runtime_aggregate::Response { usage })
}
