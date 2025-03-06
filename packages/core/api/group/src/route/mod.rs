use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_api::models as new_models;
use rivet_group_server::models;
use uuid::Uuid;

mod groups;

define_router! {
	cors: |config| CorsConfigBuilder::public().build(),
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
		"groups" / "avatar-upload" / "prepare": {
			POST: groups::prepare_avatar_upload(body: new_models::GroupPrepareAvatarUploadRequest),
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
