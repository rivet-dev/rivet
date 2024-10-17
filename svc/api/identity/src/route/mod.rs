use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_api::models;
use uuid::Uuid;

mod events;
mod identities;
mod links;

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

define_router! {
	cors: CorsConfigBuilder::public().build(),
	routes: {
		// Identities
		"identities": {
			POST: identities::setup_identity(
				opt_auth: true,
				body: models::IdentitySetupRequest,
			),
		},
		"identities" / Uuid / "profile": {
			GET: identities::profile(
				rate_limit: {
					key: "identity-get",
					buckets: [
						{ count: 128 },
					],
				},
			),
		},
		"identities" / "self" / "profile": {
			GET: identities::profile_self(
				rate_limit: {
					key: "identity-get",
					buckets: [
						{ count: 128 },
					],
				},
			),
			POST: identities::update_profile(body: models::IdentityUpdateProfileRequest),
		},
		"identities" / "batch" / "handle": {
			GET: identities::handles(
				query: identities::IdentityIdsQuery,
				rate_limit: {
					key: "identity-get",
					buckets: [
						{ count: 128 },
					],
				},
			),
		},
		"identities" / "batch" / "summary": {
			GET: identities::summaries(
				query: identities::IdentityIdsQuery,
				rate_limit: {
					key: "identity-get",
					buckets: [
						{ count: 128 },
					],
				},
			),
		},
		"identities" / "self" / "profile" / "validate": {
			POST: identities::validate_profile(body: models::IdentityUpdateProfileRequest),
		},
		"identities" / "search": {
			GET: identities::search(
				query: identities::SearchQuery,
				rate_limit: {
					buckets: [
						{ count: 128 },
					],
				},
			),
		},
		"identities" / "avatar-upload" / "prepare": {
			POST: identities::prepare_avatar_upload(body: models::IdentityPrepareAvatarUploadRequest),
		},
		"identities" / "avatar-upload" / Uuid / "complete": {
			POST: identities::complete_avatar_upload(body: serde_json::Value),
		},
		"identities" / "self" / "beta-signup": {
			POST: identities::beta_signup(body: models::IdentitySignupForBetaRequest),
		},
		"identities" / Uuid / "follow": {
			POST: identities::follow_identity(
				body: serde_json::Value,
				rate_limit: {
					buckets: [
						{ count: 4 },
					],
				},
			),
			DELETE: identities::unfollow_identity(
				rate_limit: {
					buckets: [
						{ count: 4 },
					],
				},
			),
		},
		"identities" / Uuid / "report": {
			POST: identities::report_identity(
				body: models::IdentityReportRequest,
				rate_limit: {
					buckets: [
						{ count: 4 },
					],
				},
			),
		},
		"identities" / Uuid / "followers": {
			GET: identities::followers(query: identities::FollowsQuery),
		},
		"identities" / Uuid / "following": {
			GET: identities::following(query: identities::FollowsQuery),
		},
		"identities" / "self" / "friends": {
			GET: identities::friends(query: identities::FollowsQuery),
		},
		"identities" / Uuid / "mutual-friends": {
			GET: identities::mutual_friends(query: identities::FollowsQuery),
		},
		"identities" / "self" / "recent-followers": {
			GET: identities::recent_followers(query: identities::FollowsQuery),
		},
		"identities" / "self" / "recent-followers" / Uuid / "ignore": {
			POST: identities::recent_follower_ignore(
				body: serde_json::Value,
			),
		},
		"identities" / "self" / "delete-request": {
			POST: identities::mark_deletion(body: serde_json::Value),
			DELETE: identities::unmark_deletion(),
		},

		// Events
		"events" / "live": {
			GET: events::events(),
		},

		// Links
		"game-links": {
			POST: links::prepare_game_link(
				body: serde_json::Value,
				rate_limit: {
					buckets: [
						{ count: 2 },
					],
				},
			),
			GET: links::get_game_link(
				query: links::GameLinkQuery,
				opt_auth: true,
				rate_limit: {
					buckets: [
						{ count: 32 },
					],
				},
			),
		},
		"game-links" / "complete": {
			POST: links::complete_game_link(
				body: models::IdentityCompleteGameLinkRequest,
				rate_limit: {
					buckets: [
						{ count: 2 },
					],
				},
			),
		},
		"game-links" / "cancel": {
			POST: links::cancel_game_link(
				body: models::IdentityCancelGameLinkRequest,
				rate_limit: {
					buckets: [
						{ count: 2 },
					],
				},
			),
		},
	},
}
