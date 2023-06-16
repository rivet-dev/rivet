use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};

use rivet_party_server::models;
use uuid::Uuid;

mod activity;
mod parties;

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
		// MARK: Parties
		"parties" / Uuid / "summary": {
			GET: parties::summary(),
		},
		"parties" / "self" / "summary": {
			GET: parties::summary_self(),
		},
		"parties" / Uuid / "profile": {
			GET: parties::profile(),
		},
		"parties" / Uuid / "join-request" / "send": {
			POST: parties::send_join_request(body: models::SendJoinRequestRequest),
		},

		"parties" / "self" / "profile": {
			GET: parties::profile_self(),
		},
		"parties": {
			POST: parties::create_party(body: models::CreatePartyRequest),
		},
		"parties" / "self" / "invites": {
			POST: parties::create_party_invite(body: models::CreatePartyInviteRequest),
		},
		"parties" / "self" / "invites" / Uuid: {
			DELETE: parties::revoke_party_invite(),
		},
		"parties" / "join": {
			POST: parties::join_party(body: models::JoinPartyRequest),
		},
		"parties" / "self" / "leave": {
			POST: parties::leave_party(body: models::LeavePartyRequest),
		},
		"parties" / "self" / "publicity": {
			PUT: parties::set_party_publicity(body: models::SetPartyPublicityRequest),
		},
		"parties" / "self" / "members" / Uuid / "transfer-ownership": {
			POST: parties::transfer(body: models::TransferPartyOwnershipRequest),
		},
		"parties" / "self" / "members" / Uuid / "kick": {
			POST: parties::kick(body: models::KickMemberRequest),
		},
		"invites": {
			GET: parties::get_party_from_invite(query: parties::GetPartyFromInviteQuery),
		},

		// MARK: Activity idle
		"parties" / "self" / "activity": {
			DELETE: activity::set_idle(),
		},

		// MARK: Activity matchmaker
		"parties" / "self" / "activity" / "matchmaker" / "lobbies" / "join": {
			POST: activity::matchmaker::join_lobby(body: models::JoinMatchmakerLobbyForPartyRequest),
		},
		"parties" / "self" / "activity" / "matchmaker" / "lobbies" / "find": {
			POST: activity::matchmaker::find_lobby(body: models::FindMatchmakerLobbyForPartyRequest),
		},
		"parties" / "self" / "members" / "self" / "matchmaker" / "ready": {
			POST: activity::matchmaker::ready(body: models::MatchmakerSelfReadyRequest),
		},
	},
}
