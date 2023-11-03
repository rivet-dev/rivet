use chirp_worker::prelude::*;
use futures_util::StreamExt;
use proto::backend::{self, pkg::*};
use std::collections::{HashMap, HashSet};

#[derive(sqlx::FromRow)]
struct LobbyRow {
	namespace_id: Uuid,
	lobby_id: Uuid,
	region_id: Uuid,
	lobby_group_id: Uuid,
	create_ts: i64,
	stop_ts: Option<i64>,
}

#[derive(serde::Serialize)]
struct CsvRow<'a> {
	lobby_id: Uuid,
	namespace_id: Uuid,
	namespace_name: &'a str,
	region_id: Uuid,
	region_name: &'a str,
	tier_name: &'a str,
	version_id: Uuid,
	version_name: &'a str,
	lobby_group_id: Uuid,
	lobby_group_name: &'a str,
	start_ts: i64,
	stop_ts: Option<i64>,
	total_uptime_ms: i64,
	query_uptime_ms: i64,
}

struct RegionCache {
	region: backend::region::Region,
}

struct VersionCache {
	game_version: backend::game::Version,
	mm_version_config: backend::matchmaker::VersionConfig,
	mm_version_config_meta: backend::matchmaker::VersionConfigMeta,
}

struct LobbyGroupCache {
	version_id: Uuid,
	lobby_group_config: backend::matchmaker::LobbyGroup,
	tier_name_id: String,
}

#[worker(name = "mm-lobby-history-export")]
async fn worker(
	ctx: &OperationContext<mm::msg::lobby_history_export::Message>,
) -> GlobalResult<()> {
	let request_id = unwrap_ref!(ctx.request_id).as_uuid();
	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();
	tracing::info!(?namespace_ids, "namespaces");

	// TODO: This will iterate over all lobbies regardless of stop timestamp
	// Use AS OF SYSTEM TIME to reduce contention.
	// https://www.cockroachlabs.com/docs/v22.2/performance-best-practices-overview#use-as-of-system-time-to-decrease-conflicts-with-long-running-queries
	let crdb = ctx.crdb().await?;
	let mut all_lobbies = sql_fetch!(
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
		ORDER BY create_ts DESC
		",
		&namespace_ids,
		ctx.query_start,
		ctx.query_end,
	);

	// Cached metadata
	let mut regions = HashMap::<Uuid, RegionCache>::new();
	let mut namespaces = HashMap::<Uuid, backend::game::Namespace>::new();
	let mut versions = HashMap::<Uuid, VersionCache>::new();
	let mut lobby_groups = HashMap::<Uuid, LobbyGroupCache>::new();

	// Iterate over lobbies and write to CSV
	let mut csv_writer = csv::Writer::from_writer(vec![]);
	let mut processed_lobby_ids = HashSet::<Uuid>::new();
	while let Some(lobby_row) = all_lobbies.next().await {
		let LobbyRow {
			namespace_id,
			lobby_id,
			region_id,
			lobby_group_id,
			create_ts,
			stop_ts,
		} = lobby_row?;
		let lgi_proto = Some(common::Uuid::from(lobby_group_id));
		let region_id_proto = Some(common::Uuid::from(region_id));

		// Check it's not already registered
		if processed_lobby_ids.contains(&lobby_id) {
			continue;
		}
		processed_lobby_ids.insert(lobby_id);

		// Derive start and stop ts
		let start_ts = create_ts;
		let stop_ts_capped = stop_ts.unwrap_or(ctx.query_end);

		// Filter out lobbies that did not overlap with the given range
		if start_ts > ctx.query_end || stop_ts_capped <= ctx.query_start {
			continue;
		}

		// Calculate uptime
		let total_uptime = i64::saturating_sub(stop_ts_capped, start_ts);
		let query_uptime = i64::saturating_sub(
			i64::min(stop_ts_capped, ctx.query_end),
			i64::max(start_ts, ctx.query_start),
		);

		// Fetch namespace metadata
		let namespace = if let Some(namespace) = namespaces.get(&namespace_id) {
			namespace
		} else {
			let game_namespace_res = op!([ctx] game_namespace_get {
				namespace_ids: vec![namespace_id.into()],
			})
			.await?;
			let game_namespace = unwrap!(game_namespace_res.namespaces.first()).clone();

			namespaces.insert(namespace_id, game_namespace.clone());
			unwrap!(namespaces.get(&namespace_id))
		};

		// Fetch region metadata
		let region = if let Some(region) = regions.get(&region_id) {
			region
		} else {
			let region_res = op!([ctx] region_get {
				region_ids: vec![region_id.into()],
			})
			.await?;
			let region = unwrap!(region_res.regions.first()).clone();

			regions.insert(region_id, RegionCache { region });
			unwrap!(regions.get(&region_id))
		};

		// Fetch lobby group metadata
		let lobby_group = if let Some(lobby_group) = lobby_groups.get(&lobby_group_id) {
			lobby_group
		} else {
			let lg_resolve_res = op!([ctx] mm_config_lobby_group_resolve_version {
				lobby_group_ids: vec![lobby_group_id.into()],
			})
			.await?;
			let version_id = if let Some(x) = lg_resolve_res.versions.first() {
				unwrap_ref!(x.version_id).as_uuid()
			} else {
				tracing::warn!(?lobby_group_id, "could not resolve version for lobby group");
				continue;
			};

			// Fetch version metadata
			let version = if let Some(version) = versions.get(&version_id) {
				version
			} else {
				let game_version_res = op!([ctx] game_version_get {
					version_ids: vec![version_id.into()],
				})
				.await?;
				let game_version = unwrap!(game_version_res.versions.first()).clone();

				let mm_version_res = op!([ctx] mm_config_version_get {
					version_ids: vec![version_id.into()],
				})
				.await?;
				let mm_version = unwrap!(mm_version_res.versions.first());
				let mm_version_config = unwrap_ref!(mm_version.config).clone();
				let mm_version_config_meta = unwrap_ref!(mm_version.config_meta).clone();

				versions.insert(
					version_id,
					VersionCache {
						game_version,
						mm_version_config,
						mm_version_config_meta,
					},
				);
				unwrap!(versions.get(&version_id))
			};

			// Find the lobby group data
			let lobby_group_idx = if let Some((x, _)) = version
				.mm_version_config_meta
				.lobby_groups
				.iter()
				.enumerate()
				.find(|(_, x)| x.lobby_group_id == lgi_proto)
			{
				x
			} else {
				tracing::warn!(?version.mm_version_config_meta, %lobby_group_id, "could not find version meta");
				continue;
			};
			let lobby_group_config =
				unwrap!(version.mm_version_config.lobby_groups.get(lobby_group_idx)).clone();
			let lobby_group_region = unwrap!(
				lobby_group_config
					.regions
					.iter()
					.find(|x| x.region_id == region_id_proto),
				"could not find matching region id config"
			);
			let tier_name_id = lobby_group_region.tier_name_id.clone();

			lobby_groups.insert(
				lobby_group_id,
				LobbyGroupCache {
					version_id,
					lobby_group_config,
					tier_name_id,
				},
			);

			unwrap!(lobby_groups.get(&lobby_group_id))
		};

		let version = unwrap!(versions.get(&lobby_group.version_id));

		// Write to CSV
		csv_writer.serialize(CsvRow {
			lobby_id,
			namespace_id,
			namespace_name: &namespace.name_id,
			region_id,
			region_name: &region.region.name_id,
			tier_name: &lobby_group.tier_name_id,
			version_id: lobby_group.version_id,
			version_name: &version.game_version.display_name,
			lobby_group_id,
			lobby_group_name: &lobby_group.lobby_group_config.name_id,
			start_ts,
			stop_ts,
			total_uptime_ms: total_uptime,
			query_uptime_ms: query_uptime,
		})?;
	}

	let csv_buf = csv_writer.into_inner()?;
	tracing::info!(
		len = ?csv_buf.len(),
		lobby_count = ?processed_lobby_ids.len(),
		"csv serialized"
	);

	// Upload CSV
	let mime = "text/csv";
	let content_length = csv_buf.len();
	let upload_res = op!([ctx] upload_prepare {
		bucket: "bucket-lobby-history-export".into(),
		files: vec![
			backend::upload::PrepareFile {
				path: "export.csv".into(),
				mime: Some(mime.into()),
				content_length: content_length as u64,
				..Default::default()
			},
		],
	})
	.await?;

	let presigned_req = unwrap!(upload_res.presigned_requests.first());
	let res = reqwest::Client::new()
		.put(&presigned_req.url)
		.body(csv_buf)
		.header(reqwest::header::CONTENT_TYPE, mime)
		.header(reqwest::header::CONTENT_LENGTH, content_length)
		.send()
		.await?;
	if res.status().is_success() {
		tracing::info!("uploaded successfully");
	} else {
		let status = res.status();
		let text = res.text().await;
		tracing::error!(?status, ?text, "failed to upload csv");
		bail!("failed to upload csv");
	}

	op!([ctx] upload_complete {
		upload_id: upload_res.upload_id,
		bucket: Some("bucket-lobby-history-export".into()),
	})
	.await?;

	msg!([ctx] mm::msg::lobby_history_export_complete(request_id) {
		upload_id: upload_res.upload_id,
	})
	.await?;

	Ok(())
}
