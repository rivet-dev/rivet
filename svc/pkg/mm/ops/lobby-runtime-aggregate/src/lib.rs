use futures_util::stream::StreamExt;
use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use std::collections::{HashMap, HashSet};

// NOTE: There's a bug in mm-lobby-cleanup that will upsert rows
#[derive(Debug, sqlx::FromRow)]
struct LobbyRow {
	namespace_id: Uuid,
	lobby_id: Option<Uuid>,
	region_id: Option<Uuid>,
	lobby_group_id: Option<Uuid>,
	create_ts: Option<i64>,
	stop_ts: Option<i64>,
}

#[derive(Default)]
struct LobbyAggregate {
	query_start: i64,
	query_end: i64,

	/// Total time in milliseconds for each (namespace_id, region_id, lobby_group_id)
	total_time: HashMap<(Uuid, Uuid, Uuid), i64>,

	/// Lobbies that are included in the aggregation.
	processed_lobby_ids: HashSet<Uuid>,
}

impl LobbyAggregate {
	fn process_lobby(&mut self, lobby_row: &LobbyRow) {
		// Unwrap values or ignore row
		let (lobby_id, region_id, lobby_group_id, create_ts) =
			if let (Some(a), Some(b), Some(c), Some(d)) = (
				lobby_row.lobby_id,
				lobby_row.region_id,
				lobby_row.lobby_group_id,
				lobby_row.create_ts,
			) {
				(a, b, c, d)
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
			.entry((lobby_row.namespace_id, region_id, lobby_group_id))
			.or_insert(0) += duration;
		self.processed_lobby_ids.insert(lobby_id);
	}

	fn lobby_group_ids(&self) -> HashSet<Uuid> {
		self.total_time
			.iter()
			.map(|((_, _, x), _)| *x)
			.collect::<HashSet<Uuid>>()
	}
}

#[operation(name = "mm-lobby-runtime-aggregate")]
async fn handle(
	ctx: OperationContext<mm::lobby_runtime_aggregate::Request>,
) -> GlobalResult<mm::lobby_runtime_aggregate::Response> {
	let _redis = ctx.redis_mm().await?;
	let crdb = ctx.crdb().await?;

	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();
	tracing::info!(?namespace_ids, "namespaces");

	let mut lobby_aggregate = LobbyAggregate {
		query_start: ctx.query_start,
		query_end: ctx.query_end,
		..LobbyAggregate::default()
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
	let mut lobby_rows = sql_fetch!(
		[ctx, LobbyRow, &crdb]
		"
		SELECT namespace_id, lobby_id, region_id, lobby_group_id, create_ts, stop_ts
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
		lobby_aggregate.process_lobby(&lobby_row);
	}
	tracing::info!(
		total_time = ?lobby_aggregate.total_time,
		processed_len = ?lobby_aggregate.processed_lobby_ids.len(),
		"aggregated all lobbies"
	);

	// Look up region tiers for all lobby groups
	let lobby_group_ids = lobby_aggregate
		.lobby_group_ids()
		.into_iter()
		.map(Into::<common::Uuid>::into)
		.collect::<Vec<_>>();
	let lg_resolve_res = op!([ctx] mm_config_lobby_group_resolve_version {
		lobby_group_ids: lobby_group_ids.clone(),
	})
	.await?;
	tracing::info!(
		lobby_group_ids_len = ?lobby_group_ids.len(),
		versions_len = ?lg_resolve_res.versions.len(),
		"resolved lobby group versions"
	);

	let version_ids = lg_resolve_res
		.versions
		.iter()
		.filter_map(|x| x.version_id.as_ref())
		.map(common::Uuid::as_uuid)
		.collect::<HashSet<_>>()
		.into_iter()
		.map(Into::<common::Uuid>::into)
		.collect::<Vec<_>>();
	let version_res = op!([ctx] mm_config_version_get {
		version_ids: version_ids.clone(),
	})
	.await?;
	ensure_eq!(
		version_ids.len(),
		version_res.versions.len(),
		"missing version ids"
	);
	tracing::info!(versions_len = ?version_res.versions.len(), "fetched mm versions");

	// Convert responses
	let mut tier_aggregates = HashMap::<(Uuid, Uuid, String, &str), i64>::new(); // (namespace_id, region_id, lobby_group_name_id, tier_name_id) -> time (ms)
	for ((namespace_id, region_id, lobby_group_id), total_time) in lobby_aggregate.total_time {
		let region_id_proto = Some(common::Uuid::from(region_id));
		let lgi_proto = Some(common::Uuid::from(lobby_group_id));

		// Find the version ID for the lobby group
		let version_id_proto = if let Some(version) = lg_resolve_res
			.versions
			.iter()
			.find(|x| x.lobby_group_id == lgi_proto)
		{
			&version.version_id
		} else {
			tracing::warn!(%lobby_group_id, "could not find matching version for lobby group");
			continue;
		};
		let version_id = unwrap_ref!(version_id_proto).as_uuid();

		// Find the matching version config
		let version_res = if let Some(x) = version_res
			.versions
			.iter()
			.find(|x| x.version_id == *version_id_proto)
		{
			x
		} else {
			tracing::warn!(%lobby_group_id, %version_id, "could not find matching version config for version id");
			continue;
		};
		let version_config = unwrap_ref!(version_res.config);
		let version_meta = unwrap_ref!(version_res.config_meta);

		// Resolve the configured tier name ID
		let lobby_group_idx = unwrap!(
			version_meta
				.lobby_groups
				.iter()
				.enumerate()
				.find(|(_, x)| x.lobby_group_id == lgi_proto),
			"could not find matching tier"
		)
		.0;
		let lobby_group_config = unwrap!(version_config.lobby_groups.get(lobby_group_idx));
		let lobby_group_region = unwrap!(
			lobby_group_config
				.regions
				.iter()
				.find(|x| x.region_id == region_id_proto),
			"could not find matching region id config"
		);
		let tier_name_id = lobby_group_region.tier_name_id.as_str();

		// Append to region + tier aggregate
		*tier_aggregates
			.entry((
				namespace_id,
				region_id,
				lobby_group_config.name_id.clone(),
				tier_name_id,
			))
			.or_insert(0) += total_time;
	}

	// Build response
	let region_tier_times = tier_aggregates
		.into_iter()
		.map(
			|((namespace_id, region_id, lobby_group_name_id, tier_name_id), total_time)| {
				mm::lobby_runtime_aggregate::response::RegionTierTime {
					namespace_id: Some(namespace_id.into()),
					region_id: Some(region_id.into()),
					lobby_group_name_id,
					tier_name_id: tier_name_id.to_string(),
					total_time,
				}
			},
		)
		.collect::<Vec<_>>();

	Ok(mm::lobby_runtime_aggregate::Response { region_tier_times })
}
