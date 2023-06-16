use std::collections::HashSet;

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_cloud_server::models;
use rivet_convert::cloud::analytics_lobby_summary_from_lobby;
use rivet_operation::prelude::*;

use crate::{assert, auth::Auth};

// MARK: GET /games/{}/namespaces/{}/analytics/matchmaker/live
pub async fn matchmaker_live(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetNamespaceAnalyticsMatchmakerLiveResponse> {
	ctx.auth().check_game_read(ctx.op_ctx(), game_id).await?;
	let game_ns = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	// Fetch lobby IDs
	let lobby_ids = {
		let lobby_list_res = op!([ctx] mm_lobby_list_for_namespace {
			namespace_ids: vec![namespace_id.into()],
		})
		.await?;
		let lobby_ids = internal_unwrap_owned!(lobby_list_res.namespaces.first());

		lobby_ids.lobby_ids.clone()
	};

	// Fetch all lobbies
	let lobbies = {
		let (lobby_get_res, player_count_res) = tokio::try_join!(
			op!([ctx] mm_lobby_get {
				lobby_ids: lobby_ids.clone(),
				include_stopped: false,
			}),
			op!([ctx] mm_lobby_player_count {
				lobby_ids: lobby_ids.clone(),
			}),
		)?;

		// Match lobby data with player counts
		lobby_get_res
			.lobbies
			.iter()
			.filter_map(|lobby| {
				player_count_res
					.lobbies
					.iter()
					.find(|pc| pc.lobby_id == lobby.lobby_id)
					.map(|pc| (lobby.clone(), pc.clone()))
			})
			.collect::<Vec<_>>()
	};

	// Read the versions of all running lobbies. We don't use
	// `game_ns.version_id` since we need to include versions of outdated
	// lobbies.
	let versions = {
		let lobby_group_ids = lobbies
			.iter()
			.filter_map(|(lobby, _)| lobby.lobby_group_id)
			.map(|x| x.as_uuid())
			.collect::<HashSet<_>>();

		// Resolve all version IDs from the lobby groups
		let resolve_res = op!([ctx] mm_config_lobby_group_resolve_version {
			lobby_group_ids: lobby_group_ids
				.iter()
				.cloned()
				.map(Into::into)
				.collect(),
		})
		.await?;
		let version_ids = resolve_res
			.versions
			.iter()
			.filter_map(|x| x.version_id)
			.map(|x| x.as_uuid())
			.collect::<HashSet<_>>();

		// Fetch all versions
		let versions_res = op!([ctx] mm_config_version_get {
			version_ids: version_ids
				.iter()
				.cloned()
				.map(Into::into)
				.collect(),
		})
		.await?;

		versions_res.versions
	};

	// Convert to analytics lobby
	let lobbies = lobbies
		.into_iter()
		// Find the lobby group
		.map(|(lobby, player_count)| {
			for version in &versions {
				let config = internal_unwrap!(version.config);
				let config_meta = internal_unwrap!(version.config_meta);

				// Find matching lobby group
				for (i, lobby_group_meta) in config_meta.lobby_groups.iter().enumerate() {
					if lobby_group_meta.lobby_group_id == lobby.lobby_group_id {
						let lobby_group = internal_unwrap_owned!(config.lobby_groups.get(i));

						// Check if this lobby belongs to a version different that's already deployed
						let is_outdated = game_ns.version_id != version.version_id;

						let lobby = analytics_lobby_summary_from_lobby(
							lobby,
							player_count,
							lobby_group,
							is_outdated,
						)?;

						return GlobalResult::Ok(lobby);
					}
				}
			}

			// No lobby group found
			internal_panic!("lobby group not found")
		})
		.filter_map(|res| match res {
			Ok(x) => Some(x),
			Err(err) => {
				tracing::error!(?err, "failed to construct lobby");
				None
			}
		})
		.collect::<Vec<_>>();

	Ok(models::GetNamespaceAnalyticsMatchmakerLiveResponse { lobbies })
}
