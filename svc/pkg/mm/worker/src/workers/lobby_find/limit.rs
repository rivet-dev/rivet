use chirp_worker::prelude::*;
use proto::backend::{self, pkg::mm::msg::lobby_find_fail::ErrorCode, pkg::*};
use serde_json::json;

use super::fail;
/// Limit the number of matchmaker players in this namespace for a given IP.
///
/// This is limited to the game's namespace instead of globally across all games
/// since we need to let developers configure the limits.
///
/// Returns false if called `fail`.
#[tracing::instrument(skip(redis_mm))]
pub async fn check_remote_addresses(
	ctx: &OperationContext<mm::msg::lobby_find::Message>,
	redis_mm: &mut RedisConn,
	analytics_events: &mut Vec<analytics::msg::event_create::Event>,
	namespace_id: Uuid,
	query_id: Uuid,
	mm_ns_config: &backend::matchmaker::NamespaceConfig,
	players: &[super::Player],
) -> GlobalResult<bool> {
	let backend::matchmaker::NamespaceConfig {
		max_players_per_client,
		// TODO:
		// max_players_per_client_vpn,
		// max_players_per_client_proxy,
		// max_players_per_client_tor,
		// max_players_per_client_hosting,
		..
	} = *mm_ns_config;

	// Filter to remote addresses from players
	let remote_addresses = players
		.iter()
		.filter_map(|p| {
			p.client_info
				.as_ref()
				.and_then(|ci| ci.remote_address.clone())
		})
		.collect::<Vec<_>>();

	// Fetch existing player counts
	let fetch_perf = ctx.perf().start("fetch-remote-address-player-count").await;
	let mut pipe = redis::pipe();
	for remote_address in &remote_addresses {
		pipe.scard(util_mm::key::ns_remote_address_player_ids(
			namespace_id,
			remote_address,
		));
	}
	let player_counts = pipe.query_async::<_, Vec<u32>>(redis_mm).await?;
	fetch_perf.end();

	// Validate player count
	for (remote_address, player_count) in remote_addresses.iter().zip(player_counts) {
		tracing::info!(
			%remote_address,
			player_count,
			"players for remote address"
		);

		analytics_events.push(analytics::msg::event_create::Event {
			name: "mm.players.count_for_remote_address".into(),
			properties_json: Some(serde_json::to_string(&json!({
				"namespace_id": namespace_id,
				"remote_address": remote_address,
				"player_count": player_count,
			}))?),
			..Default::default()
		});

		if player_count >= max_players_per_client {
			tracing::warn!(
				%remote_address,
				%player_count,
				"too many players for remote address"
			);
			return fail(
				ctx,
				namespace_id,
				query_id,
				ErrorCode::TooManyPlayersFromSource,
				true,
			)
			.await
			.map(|_| false);
		}
	}

	tracing::info!("checked all players");

	Ok(true)
}
