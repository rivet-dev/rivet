use futures_util::{StreamExt, TryStreamExt};
use indoc::indoc;
use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;
use std::{
	collections::{HashMap, HashSet},
	time::Duration,
};

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
	.fetch_all(&ctx.crdb("db-team").await?)
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
		.map(|x| x.1.clone())
		.collect::<HashSet<Uuid>>()
		.into_iter()
		.map(|x| Into::<common::Uuid>::into(x))
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
		.filter_map(|x| x.version_id.clone())
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

	let mut teams = teams
		.teams
		.iter()
		.map(|team| {
			let games = games
				.games
				.iter()
				.filter(|x| x.developer_team_id == team.team_id)
				.map(|game| {
					let namespaces = namespaces
						.namespaces
						.iter()
						.filter(|x| x.game_id == game.game_id)
						.map(|ns| {
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

							let ns_id = ns.namespace_id.unwrap().as_uuid();

							let player_count = player_counts
								.namespaces
								.iter()
								.find(|x| x.namespace_id == ns.namespace_id)
								.map_or(0, |x| x.player_count);

							(
								ns_id,
								json!({
									"name_id": ns.name_id,
									"display_name": ns.display_name,
									"version": version,
									"player_count": player_count,
								}),
							)
						})
						.collect::<HashMap<_, _>>();

					let game_id = game.game_id.unwrap().as_uuid();

					(
						game_id,
						json!({
							"name_id": game.name_id,
							"display_name": game.display_name,
							"create_ts": team.create_ts,
							"url": game.url,
							"namespaces": namespaces,
						}),
					)
				})
				.collect::<HashMap<_, _>>();

			let team_id = team.team_id.unwrap().as_uuid();

			let member_count = team_members
				.teams
				.iter()
				.find(|x| x.team_id == team.team_id)
				.map_or(0, |x| x.member_count);

			(
				team_id,
				json!({
					"display_name": team.display_name,
					"create_ts": team.create_ts,
					"member_count": member_count,
					"games": games,
				}),
			)
		})
		.collect::<HashMap<_, _>>();

	// TODO: Report to PostHog

	Ok(())
}
