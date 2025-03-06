use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(Debug, sqlx::FromRow)]
struct RegionRow {
	namespace_id: Uuid,
	region_id: Uuid,
	total_time: i64,
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

	let regions = ctx
		.cache()
		.immutable()
		.fetch_all_proto(
			"mm.lobby_runtime",
			namespace_ids,
			|mut cache, namespace_ids| {
				let query_start = ctx.query_start;
				let query_end = ctx.query_end;
				let ctx = ctx.base();
				async move {
					// Aggregate all lobbies that finished during the given query span.
					//
					// We do this after querying the lobbies that are still running in order to
					// ensure that we capture all lobbies in all states that may have stopped
					// while generating this aggregation.
					//
					// Use AS OF SYSTEM TIME to reduce contention.
					// https://www.cockroachlabs.com/docs/v22.2/performance-best-practices-overview#use-as-of-system-time-to-decrease-conflicts-with-long-running-queries
					let region_rows = sql_fetch_all!(
						[ctx, RegionRow]
						"
						SELECT
							namespace_id,
							region_id,
							SUM_INT(
								CASE
									-- Lobbies stopped during the query window
									WHEN stop_ts > $2 AND stop_ts <= $3 THEN
										stop_ts - GREATEST(create_ts, $2)
									-- Lobbies created during the query window, these may already be stopped after query_end
									WHEN create_ts > $2 AND create_ts <= $3 THEN
										LEAST(stop_ts, $3) - create_ts
									-- Lobbies still running that overlap with the query window
									WHEN stop_ts IS NULL AND create_ts <= $3 THEN
										$3 - create_ts
									ELSE 0
								END
							) AS total_time
						FROM db_mm_state.lobbies AS OF SYSTEM TIME '-5s'
						WHERE namespace_id = ANY($1)
							AND (
								(stop_ts > $2 AND stop_ts <= $3)
								OR (create_ts > $2 AND create_ts <= $3)
								OR (stop_ts IS NULL AND create_ts <= $3)
							)
						GROUP BY namespace_id, region_id
						",
						&namespace_ids,
						query_start,
						query_end,
					)
					.await?;

					for row in region_rows {
						cache.resolve(
							&row.namespace_id,
							mm::lobby_runtime_aggregate::response::NamespaceUsage {
								namespace_id: Some(row.namespace_id.into()),
								region_id: Some(row.region_id.into()),
								total_time: row.total_time,
							},
						);
					}

					Ok(cache)
				}
			},
		)
		.await?;

	Ok(mm::lobby_runtime_aggregate::Response { usage: regions })
}
