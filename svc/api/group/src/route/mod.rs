use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_group_server::models;

use uuid::Uuid;

mod groups;

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
		// Groups
		"groups": {
			GET: groups::get_suggested_groups(),
			POST: groups::create(body: models::CreateGroupRequest),
		},
		"groups" / Uuid / "profile": {
			GET: groups::profile(),
			POST: groups::update_profile(body: models::UpdateGroupProfileRequest),
		},
		"groups" / "profile" / "validate": {
			POST: groups::validate_profile(body: models::ValidateGroupProfileRequest),
		},
		"groups" / "search": {
			GET: groups::search(
				query: groups::SearchQuery,
				rate_limit: {
					buckets: [
						{ count: 128 },
					],
				},
			),
		},
		"groups" / "avatar-upload" / "prepare": {
			POST: groups::prepare_avatar_upload(body: models::PrepareGroupAvatarUploadRequest),
		},
		"groups" / Uuid / "avatar-upload" / Uuid / "complete": {
			POST: groups::complete_avatar_upload(body: models::CompleteGroupAvatarUploadRequest),
		},
		"groups" / Uuid / "leave": {
			POST: groups::leave(body: models::LeaveGroupRequest),
		},
		"groups" / Uuid / "transfer-owner": {
			POST: groups::transfer_owner(
				body: models::TransferGroupOwnershipRequest,
			),
		},
		"groups" / Uuid / "kick" / Uuid: {
			POST: groups::kick_member(body: models::KickGroupMemberRequest),
		},
		"groups" / Uuid / "bans": {
			GET: groups::bans(query: groups::ListBansQuery),
		},
		"groups" / Uuid / "bans" / Uuid: {
			POST: groups::ban(body: models::BanGroupIdentityRequest),
			DELETE: groups::unban(),
		},
		"groups" / Uuid / "members": {
			GET: groups::members(query: groups::ListMembersQuery),
		},
		"groups" / Uuid / "join-requests": {
			GET: groups::join_requests(query: groups::ListJoinRequestsQuery),
		},

		// Join requests
		"groups" / Uuid / "join-request": {
			POST: groups::request_join(body: models::CreateGroupJoinRequestRequest),
		},
		"groups" / Uuid / "join-request" / Uuid: {
			POST: groups::resolve_join_request(
				body: models::ResolveGroupJoinRequestRequest,
			),
		},

		// Invites
		"groups" / Uuid / "invites": {
			POST: groups::create_invite(
				body: models::CreateGroupInviteRequest,
			),
		},
		"invites" / String / "consume": {
			POST: groups::consume_invite(
				body: models::ConsumeGroupInviteRequest,
				rate_limit: {
					buckets: [
						{ count: 2 },
					],
				},
			),
		},
		"invites" / String: {
			GET: groups::get_invite(),
		},
	},
}
