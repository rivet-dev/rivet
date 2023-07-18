use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_operation::prelude::*;
use util::duration;

pub mod lobbies;
pub mod players;
pub mod regions;

pub async fn handle(
	shared_client: chirp_client::SharedClientHandle,
	pools: rivet_pools::Pools,
	cache: rivet_cache::Cache,
	ray_id: uuid::Uuid,
	request: Request<Body>,
) -> Result<Response<Body>, http::Error> {
	let response = Response::builder();

	// Handle route
	Router::handle(shared_client, pools, cache, ray_id, request, response).await
}

// Make sure to set `opt_auth: true` for public endpoints that use
// domain-based authentication
define_router! {
	cors: CorsConfigBuilder::public().build(),
	routes: {
		"lobbies" / "ready": {
			POST: lobbies::ready(
				body: serde_json::Value,
				rate_limit: {
					buckets: [
						{ count: 512 },
					],
				},
			),
		},
		"lobbies" / "join": {
			POST: lobbies::join(
				body: rivet_api::models::MatchmakerLobbiesJoinRequest,
				opt_auth: true,
				rate_limit: {
					key: "lobby-find",
					buckets: [
						{ count: 16, duration: duration::minutes(1), },
						{ count: 200, duration: duration::hours(1), },
					],
				},
			),
		},
		"lobbies" / "find": {
			POST: lobbies::find(
				body: rivet_api::models::MatchmakerLobbiesFindRequest,
				opt_auth: true,
				rate_limit: {
					key: "lobby-find",
					buckets: [
						{ count: 16, duration: duration::minutes(1), },
						{ count: 200, duration: duration::hours(1), },
					],
				},
			),
		},
		"lobbies" / "create": {
			POST: lobbies::create(
				body: rivet_api::models::MatchmakerLobbiesCreateRequest,
				opt_auth: true,
				rate_limit: {
					key: "lobby-create",
					buckets: [
						{ count: 3 },
					],
				},
			),
		},
		"lobbies" / "closed": {
			PUT: lobbies::set_closed(
				body: rivet_api::models::MatchmakerLobbiesSetClosedRequest,
				rate_limit: {
					buckets: [
						{ count: 1024 },
					],
				},
			),
		},
		"lobbies" / "state": {
			PUT: lobbies::set_state(
				body_as_bytes: true,
				rate_limit: {
					buckets: [
						{ count: 1024 },
					],
				},
			),
		},
		"lobbies" / Uuid / "state": {
			GET: lobbies::get_state(),
		},
		"lobbies" / "list": {
			GET: lobbies::list(
				opt_auth: true,
				rate_limit: {
					key: "lobby-list",
					buckets: [
						{ count: 4 },
					],
				},
			),
		},

		"players" / "connected": {
			POST: players::connected(
				body: rivet_api::models::MatchmakerPlayersConnectedRequest,
				rate_limit: {
					buckets: [
						{ count: 16384 },
					],
				},
			),
		},
		"players" / "disconnected": {
			POST: players::disconnected(
				body: rivet_api::models::MatchmakerPlayersConnectedRequest,
				rate_limit: {
					buckets: [
						{ count: 16384 },
					],
				},
			),
		},
		"players" / "statistics": {
			GET: players::statistics(
				query: players::GetStatisticsQuery,
				rate_limit: {
					buckets: [
						{ count: 4 },
					],
				},
			),
		},

		"regions": {
			GET: regions::list(
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 32 },
					],
				},
			),
		},
	},
}
