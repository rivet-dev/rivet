use indoc::indoc;
use rivet_operation::prelude::*;
use serde_json::json;
use std::{collections::HashSet, fmt::Display, time::Duration};

// This API key is safe to hardcode. It will not change and is intended to be public.
const POSTHOG_API_KEY: &str = "phc_1lUNmul6sAdFzDK1VHXNrikCfD7ivQZSpf2yzrPvr4m";

#[tracing::instrument]
pub async fn run_from_env(ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("telemetry-beacon").await?;
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("telemetry-beacon");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"telemetry-beacon".into(),
		Duration::from_secs(300),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);

	let team_ids = sqlx::query_as::<_, (Uuid,)>(indoc!(
		"
		SELECT team_id
		FROM dev_teams
		"
	))
	.fetch_all(&ctx.crdb("db-team-dev").await?)
	.await?
	.into_iter()
	.map(|(team_id,)| Into::<common::Uuid>::into(team_id))
	.collect::<Vec<_>>();

	let namespaces = sqlx::query_as::<_, (Uuid, Uuid)>(indoc!(
		"
		SELECT namespace_id, game_id
		FROM game_namespaces
		"
	))
	.fetch_all(&ctx.crdb("db-game").await?)
	.await?;

	let game_ids = namespaces
		.iter()
		.map(|x| x.1)
		.collect::<HashSet<Uuid>>()
		.into_iter()
		.map(Into::<common::Uuid>::into)
		.collect::<Vec<_>>();
	let namespace_ids = namespaces
		.iter()
		.map(|x| Into::<common::Uuid>::into(x.0))
		.collect::<Vec<_>>();

	let teams = op!([ctx] team_get {
		team_ids: team_ids.clone(),
	})
	.await?;

	let team_members = op!([ctx] team_member_count {
		team_ids: team_ids,
	})
	.await?;

	let games = op!([ctx] game_get {
		game_ids: game_ids,
	})
	.await?;

	let namespaces = op!([ctx] game_namespace_get {
		namespace_ids: namespace_ids.clone(),
	})
	.await?;

	let version_ids = namespaces
		.namespaces
		.iter()
		.filter_map(|x| x.version_id)
		.collect::<Vec<_>>();

	let versions = op!([ctx] game_version_get {
		version_ids: version_ids.clone(),
	})
	.await?;

	let cloud_versions = op!([ctx] cloud_version_get {
		version_ids: version_ids,
	})
	.await?;

	let player_counts = op!([ctx] mm_player_count_for_namespace {
		namespace_ids: namespace_ids.clone(),
	})
	.await?;

	// TODO: Registered players
	// TODO: MAU

	let mut events = Vec::new();

	{
		// We include both the cluster ID and the namespace ID in the distinct_id in case the config is
		// copied to a new namespace with a different name accidentally
		let mut event = async_posthog::Event::new(
			"cluster_beacon",
			&format!(
				"cluster:{}:{}",
				util::env::namespace(),
				util::env::cluster_id()
			),
		);

		event.insert_prop(
			"$set",
			&json!({
				"ns_id": util::env::namespace(),
				"cluster_id": util::env::namespace(),
			}),
		)?;

		events.push(event);
	}

	for team in &teams.teams {
		let team_id = team.team_id.unwrap().as_uuid();

		let member_count = team_members
			.teams
			.iter()
			.find(|x| x.team_id == team.team_id)
			.map_or(0, |x| x.member_count);

		let mut event =
			async_posthog::Event::new("team_beacon", &build_distinct_id(format!("team:{team_id}")));
		event.insert_prop(
			"$set",
			json!({
				"ns_id": util::env::namespace(),
				"cluster_id": util::env::cluster_id(),
				"team_id": team_id,
				"display_name": team.display_name,
				"create_ts": team.create_ts,
				"member_count": member_count,
			}),
		)?;
		events.push(event);
	}

	for game in &games.games {
		let game_id = game.game_id.unwrap().as_uuid();

		let mut event =
			async_posthog::Event::new("game_beacon", &build_distinct_id(format!("game:{game_id}")));
		event.insert_prop(
			"$set",
			json!({
				"ns_id": util::env::namespace(),
				"cluster_id": util::env::cluster_id(),
				"game_id": game_id,
				"name_id": game.name_id,
				"display_name": game.display_name,
				"create_ts": game.create_ts,
				"url": game.url,
			}),
		)?;
		events.push(event);
	}

	for ns in &namespaces.namespaces {
		let ns_id = ns.namespace_id.unwrap().as_uuid();

		let player_count = player_counts
			.namespaces
			.iter()
			.find(|x| x.namespace_id == ns.namespace_id)
			.map_or(0, |x| x.player_count);

		let version = versions
			.versions
			.iter()
			.find(|x| x.version_id == ns.version_id)
			.map(|version| {
				let config = cloud_versions
					.versions
					.iter()
					.find(|x| x.version_id == version.version_id)
					.and_then(|x| x.config.as_ref())
					.map(|config| {
						json!({
							"cdn": config.cdn.as_ref().map(|_| json!({})),
							"matchmaker": config.matchmaker.as_ref().map(|_| json!({})),
							"kv": config.kv.as_ref().map(|_| json!({})),
							"identity": config.identity.as_ref().map(|_| json!({})),
						})
					});

				json!({
					"version_id": version.version_id.unwrap().as_uuid(),
					"create_ts": version.create_ts,
					"display_name": version.display_name,
					"config": config,
				})
			});

		let mut event = async_posthog::Event::new(
			"namespace_beacon",
			&build_distinct_id(format!("ns:{ns_id}")),
		);
		event.insert_prop(
			"$set",
			json!({
				"ns_id": util::env::namespace(),
				"cluster_id": util::env::cluster_id(),
				"namespace_id": ns_id,
				"name_id": ns.name_id,
				"display_name": ns.display_name,
				"version": version,
			}),
		)?;
		event.insert_prop("player_count", player_count)?;
		events.push(event);
	}

	// Send events
	tracing::info!(events_len = ?events.len(), "sending events");
	async_posthog::client(POSTHOG_API_KEY)
		.capture_batch(events)
		.await?;
	tracing::info!("event sent");

	Ok(())
}

fn build_distinct_id(entity: impl Display) -> String {
	format!(
		"cluster:{}:{}:{entity}",
		util::env::namespace(),
		util::env::cluster_id()
	)
}
