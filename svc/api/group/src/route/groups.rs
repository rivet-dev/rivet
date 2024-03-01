use std::str::FromStr;

use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use chirp_client::TailAnchorResponse;
use futures_util::FutureExt;
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_api::models as new_models;
use rivet_convert::{ApiInto, ApiTryInto};
use rivet_group_server::models;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{assert, auth::Auth, convert, fetch, utils};

const MAX_AVATAR_UPLOAD_SIZE: i64 = util::file_size::megabytes(2) as i64;

// MARK: GET /groups/{}/profile
enum TeamConsumerUpdate {
	Team,
	MemberCreate,
	MemberRemove,
	JoinRequestCreate,
	JoinRequestResolve,
}

pub async fn profile(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetGroupProfileResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Fetch team data
	let teams = op!([ctx] team_get {
		team_ids: vec![group_id.into()],
	})
	.await?;

	let team = unwrap!(teams.teams.first()).clone();

	// Wait for an update if needed
	let (update, update_ts) = if let Some(anchor) = watch_index.to_consumer()? {
		// Team events
		let team_update_sub = tail_anchor!([ctx, anchor] team::msg::update(group_id));

		// Member creation/deletion tails ONLY for the current requesting user (required for
		// `is_current_identity_member` property)
		let team_member_create_sub = tail_anchor!([ctx, anchor] team::msg::member_create_complete(group_id, user_ent.user_id));
		let team_member_remove_sub = tail_anchor!([ctx, anchor] team::msg::member_remove_complete(group_id, user_ent.user_id));

		// Join request creation/deletion tails ONLY for the current requesting user (required for
		// `is_current_identity_requesting_join` property)
		let team_join_request_create_sub = tail_anchor!([ctx, anchor] team::msg::join_request_create_complete(group_id, user_ent.user_id));
		let team_join_request_resolve_sub = tail_anchor!([ctx, anchor] team::msg::join_request_resolve_complete(group_id, user_ent.user_id));

		// Listen for updates
		util::macros::select_with_timeout!({
			event = team_update_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(Some(TeamConsumerUpdate::Team), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = team_member_create_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(Some(TeamConsumerUpdate::MemberCreate), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = team_member_remove_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(Some(TeamConsumerUpdate::MemberRemove), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = team_join_request_create_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(Some(TeamConsumerUpdate::JoinRequestCreate), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = team_join_request_resolve_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(Some(TeamConsumerUpdate::JoinRequestResolve), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	let (team, team_members_res, team_join_requests_res) = tokio::try_join!(
		// Fetch new team data (if updated)
		async {
			if let Some(TeamConsumerUpdate::Team) = update {
				let teams_res = op!([ctx] team_get {
					team_ids: vec![group_id.into()],
				})
				.await?;

				Ok(unwrap!(teams_res.teams.first()).clone())
			} else {
				Ok(team)
			}
		},
		op!([ctx] team_member_list {
			team_ids: vec![group_id.into()],
			limit: None,
			anchor: None,
		}),
		op!([ctx] team_join_request_list {
			team_ids: vec![group_id.into()],
		}),
	)?;
	let team_members_res = unwrap!(team_members_res.teams.first());

	let is_current_identity_member = team_members_res
		.members
		.iter()
		.flat_map(|u| u.user_id.as_ref().map(common::Uuid::as_uuid))
		.any(|id| id == user_ent.user_id);
	let is_current_identity_requesting_join = unwrap!(team_join_requests_res.teams.first())
		.join_requests
		.iter()
		.flat_map(|u| u.user_id.as_ref().map(common::Uuid::as_uuid))
		.any(|id| id == user_ent.user_id);

	// TODO:
	let _admin_role_id = Uuid::default();
	let owner_user_id = unwrap_ref!(team.owner_user_id).as_uuid();
	let team_id = unwrap_ref!(team.team_id).as_uuid();

	Ok(models::GetGroupProfileResponse {
		group: models::GroupProfile {
			group_id: team_id.to_string(),
			display_name: team.display_name.clone(),
			bio: team.bio.clone(),
			avatar_url: util::route::team_avatar(&team),
			external: models::GroupExternalLinks {
				profile: util::route::team_profile(team_id),
				chat: Default::default(),
			},

			is_current_identity_member,
			publicity: unwrap!(backend::team::Publicity::from_i32(team.publicity)).api_into(),
			member_count: team_members_res.members.len() as i32,
			is_developer: true,

			members: Vec::new(),
			join_requests: Vec::new(),
			is_current_identity_requesting_join,
			owner_identity_id: owner_user_id.to_string(),

			thread_id: Default::default(),
		},
		watch: convert::watch_response(WatchResponse::new(update_ts + 1)),
	})
}

// MARK: GET /groups/{}/members
#[derive(Debug)]
enum TeamMemberConsumerUpdate {
	MemberCreate(common::Uuid),
	MemberRemove(common::Uuid),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListMembersQuery {
	anchor: Option<String>,
	limit: Option<u32>,
}

pub async fn members(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	watch_index: WatchIndexQuery,
	query: ListMembersQuery,
) -> GlobalResult<models::GetGroupMembersResponse> {
	let (user, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Check if user is a member of this team
	let team_list_res = op!([ctx] user_team_list {
		user_ids: vec![user_ent.user_id.into()],
	})
	.await?;

	let user_team = unwrap!(team_list_res.users.first());
	let user_team_ids = user_team
		.teams
		.iter()
		.map(|t| Ok(unwrap_ref!(t.team_id).as_uuid()))
		.collect::<GlobalResult<Vec<_>>>()?;
	let has_team = user_team_ids.iter().any(|team_id| &group_id == team_id);

	ensure_with!(has_team || user.is_admin, GROUP_NOT_MEMBER);

	let team_members_res = op!([ctx] team_member_list {
		team_ids: vec![group_id.into()],
		limit: query.limit.or(Some(16)),
		anchor: query.anchor
			.map(|anchor| anchor.parse::<i64>())
			.transpose()?,
	})
	.await?;
	let team_members = unwrap!(team_members_res.teams.first());

	let mut user_ids = team_members
		.members
		.iter()
		.map(|member| Ok(unwrap!(member.user_id)))
		.collect::<GlobalResult<Vec<_>>>()?;

	// Wait for an update if needed
	let (update, update_ts) = if let Some(anchor) = watch_index.to_consumer()? {
		// Team events
		let team_member_create_sub =
			tail_anchor!([ctx, anchor] team::msg::member_create_complete(group_id, "*"));
		let team_member_remove_sub =
			tail_anchor!([ctx, anchor] team::msg::member_remove_complete(group_id, "*"));

		// User subs
		let user_subs_select =
			util::future::select_all_or_wait(user_ids.iter().cloned().map(|user_id| {
				tail_anchor!([ctx, anchor] user::msg::update(user_id.as_uuid())).boxed()
			}));

		// User presence subs
		let user_presence_subs_select =
			util::future::select_all_or_wait(user_ids.iter().cloned().map(|user_id| {
				tail_anchor!([ctx, anchor] user_presence::msg::update(user_id.as_uuid())).boxed()
			}));

		// Listen for updates
		util::macros::select_with_timeout!({
			event = team_member_create_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.user_id.map(TeamMemberConsumerUpdate::MemberCreate), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = team_member_remove_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.user_id.map(TeamMemberConsumerUpdate::MemberRemove), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = user_subs_select => {
				(None, event?.msg_ts())
			}
			event = user_presence_subs_select => {
				(None, event?.msg_ts())
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	// Handle member list changes
	match update {
		Some(TeamMemberConsumerUpdate::MemberCreate(user_id)) => {
			// New member is within the paginated range
			if team_members
				.anchor
				.map(|anchor| update_ts < anchor)
				.unwrap_or(true)
			{
				user_ids.push(user_id);
			}
		}
		Some(TeamMemberConsumerUpdate::MemberRemove(user_id)) => {
			// Will remove the removed member if they are within the paginated range
			user_ids.retain(|u| u != &user_id);
		}
		_ => {}
	}

	// NOTE: We don't use fetch::identities::handles here because the end model is `GroupMember` not `IdentityHandle`
	// Fetch team member and join request data
	let (users, presences_ctx, user_follows) = tokio::try_join!(
		fetch::identity::users(&ctx, user_ids.clone()),
		fetch::identity::presence_data(&ctx, user_ent.user_id, user_ids.clone(), false),
		fetch::identity::follows(
			&ctx,
			user_ent.user_id,
			user_ids
				.iter()
				.map(common::Uuid::as_uuid)
				.collect::<Vec<_>>()
		),
	)?;

	let raw_user_ent_id = Into::<common::Uuid>::into(user_ent.user_id);
	let members = users
		.users
		.iter()
		.map(|user| {
			let is_mutual_following = user_follows.follows.iter().any(|follow| {
				follow.follower_user_id.as_ref() == user.user_id.as_ref()
					&& follow.following_user_id.as_ref() == Some(&raw_user_ent_id)
					&& follow.is_mutual
			});

			Ok(models::GroupMember {
				identity: convert::identity::handle(
					user_ent.user_id,
					user,
					&presences_ctx,
					is_mutual_following,
				)?,
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::GetGroupMembersResponse {
		members,
		anchor: team_members.anchor.map(|anchor| anchor.to_string()),
		watch: convert::watch_response(WatchResponse::new(update_ts + 1)),
	})
}

// MARK: GET /groups/{}/join-requests
enum TeamJoinRequestConsumerUpdate {
	JoinRequestCreate(common::Uuid),
	JoinRequestResolve(common::Uuid),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListJoinRequestsQuery {
	anchor: Option<String>,
	limit: Option<u32>,
}

pub async fn join_requests(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	watch_index: WatchIndexQuery,
	_query: ListJoinRequestsQuery,
) -> GlobalResult<models::GetGroupJoinRequestsResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Verify the team exists
	let (teams_res, team_join_requests_res) = tokio::try_join!(
		op!([ctx] team_get {
			team_ids: vec![group_id.into()],
		}),
		op!([ctx] team_join_request_list {
			team_ids: vec![group_id.into()],
			// TODO: Re-add when pagination is added to join requests
			// limit: query.limit.or(Some(16)),
			// anchor: query.anchor
			// 	.map(|anchor| anchor.parse::<i64>())
			// 	.transpose()?,
		}),
	)?;
	let team = unwrap_with!(teams_res.teams.first(), GROUP_NOT_FOUND);
	let team_join_requests = unwrap!(team_join_requests_res.teams.first()).clone();
	let owner_user_id = unwrap_ref!(team.owner_user_id).as_uuid();

	// Verify request user's permissions
	ensure_eq_with!(
		user_ent.user_id,
		owner_user_id,
		GROUP_INSUFFICIENT_PERMISSIONS
	);

	let mut user_ids = team_join_requests
		.join_requests
		.iter()
		.map(|join_request| Ok(unwrap!(join_request.user_id)))
		.collect::<GlobalResult<Vec<_>>>()?;

	// Wait for an update if needed
	let (update, update_ts) = if let Some(anchor) = watch_index.to_consumer()? {
		// Team events
		let team_join_request_create_sub =
			tail_anchor!([ctx, anchor] team::msg::join_request_create_complete(group_id, "*"));
		let team_join_request_resolve_sub =
			tail_anchor!([ctx, anchor] team::msg::join_request_resolve_complete(group_id, "*"));

		// User subs
		let user_subs_select =
			util::future::select_all_or_wait(user_ids.iter().cloned().map(|user_id| {
				tail_anchor!([ctx, anchor] user::msg::update(user_id.as_uuid())).boxed()
			}));

		// User presence subs
		let user_presence_subs_select =
			util::future::select_all_or_wait(user_ids.iter().cloned().map(|user_id| {
				tail_anchor!([ctx, anchor] user_presence::msg::update(user_id.as_uuid())).boxed()
			}));

		// Listen for updates
		util::macros::select_with_timeout!({
			event = team_join_request_create_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.user_id.map(TeamJoinRequestConsumerUpdate::JoinRequestCreate), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = team_join_request_resolve_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.user_id.map(TeamJoinRequestConsumerUpdate::JoinRequestResolve), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = user_subs_select => {
				(None, event?.msg_ts())
			}
			event = user_presence_subs_select => {
				(None, event?.msg_ts())
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	// Handle join request list changes
	match update {
		Some(TeamJoinRequestConsumerUpdate::JoinRequestCreate(user_id)) => {
			// New join request is within the paginated range

			// TODO: Re-add when pagination is added to join requests
			// if team_join_requests
			// 	.anchor
			// 	.map(|anchor| team_member.join_ts < anchor)
			// 	.unwrap_or(true)
			// {
			// 	user_ids.push(user_id);
			// }

			// TODO: Remove when pagination is added
			user_ids.push(user_id);
		}
		Some(TeamJoinRequestConsumerUpdate::JoinRequestResolve(user_id)) => {
			// Remove the unbanned user id
			user_ids.retain(|u| u != &user_id);
		}
		_ => {}
	}

	// NOTE: We don't use fetch::identities::handles here because the end model is `GroupMember` not
	// `IdentityHandle`
	// Fetch team member and join request data
	let (users, presences_ctx, user_follows) = tokio::try_join!(
		fetch::identity::users(&ctx, user_ids.clone()),
		fetch::identity::presence_data(&ctx, user_ent.user_id, user_ids.clone(), false),
		fetch::identity::follows(
			&ctx,
			user_ent.user_id,
			user_ids
				.iter()
				.map(common::Uuid::as_uuid)
				.collect::<Vec<_>>()
		),
	)?;

	let raw_user_ent_id = Into::<common::Uuid>::into(user_ent.user_id);
	let join_requests = users
		.users
		.iter()
		.map(|user| {
			let join_request_ts = team_join_requests
				.join_requests
				.iter()
				.find(|join_request| user.user_id == join_request.user_id)
				.map(|jr| jr.ts)
				.unwrap_or(update_ts);
			let is_mutual_following = user_follows.follows.iter().any(|follow| {
				follow.follower_user_id.as_ref() == user.user_id.as_ref()
					&& follow.following_user_id.as_ref() == Some(&raw_user_ent_id)
					&& follow.is_mutual
			});

			Ok(models::GroupJoinRequest {
				identity: convert::identity::handle(
					user_ent.user_id,
					user,
					&presences_ctx,
					is_mutual_following,
				)?,
				ts: util::timestamp::to_chrono(join_request_ts)?,
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::GetGroupJoinRequestsResponse {
		join_requests,
		// TODO: Re-add when pagination is added to join requests
		// anchor: team_members.anchor.map(|anchor| anchor.to_string()),
		anchor: None,
		watch: convert::watch_response(WatchResponse::new(update_ts + 1)),
	})
}

// MARK: POST /groups/{}/profile
pub async fn update_profile(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	body: models::UpdateGroupProfileRequest,
) -> GlobalResult<models::UpdateGroupProfileResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	let res = op!([ctx] team_get {
		team_ids: vec![group_id.into()],
	})
	.await?;

	// Validate the team exists
	let team = unwrap!(res.teams.first());
	let owner_user_id = unwrap_ref!(team.owner_user_id).as_uuid();

	// Verify user's permissions
	ensure_eq_with!(
		user_ent.user_id,
		owner_user_id,
		GROUP_INSUFFICIENT_PERMISSIONS
	);

	let res = msg!([ctx] team::msg::profile_set(group_id) -> Result<team::msg::profile_set_complete, team::msg::profile_set_fail> {
		team_id: Some(group_id.into()),
		display_name: body.display_name.clone(),
		bio: body.bio.clone(),
		publicity: body.publicity.map(|publicity| ApiInto::<backend::team::Publicity>::api_into(publicity) as i32)
	})
	.await?;
	match res {
		Ok(_) => {}
		Err(msg) => {
			use team::msg::profile_set_fail::ErrorCode::*;

			let code = team::msg::profile_set_fail::ErrorCode::from_i32(msg.error_code);
			match unwrap!(code) {
				Unknown => bail!("unknown team profile set fail error code"),
				ValidationFailed => bail_with!(VALIDATION_ERROR),
			}
		}
	}

	Ok(models::UpdateGroupProfileResponse {})
}

// MARK: GET /groups
pub async fn get_suggested_groups(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::ListSuggestedGroupsResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Fetch recommendations
	let recommend_res = op!([ctx] team_recommend { count: 64 }).await?;

	// Fetch teams
	let groups = fetch::group::summaries(
		&ctx,
		user_ent.user_id,
		recommend_res
			.team_ids
			.iter()
			.map(common::Uuid::as_uuid)
			.collect::<Vec<_>>(),
	)
	.await?;

	Ok(models::ListSuggestedGroupsResponse {
		groups,
		watch: convert::watch_response(WatchResponse::new(ctx.chirp().ts() + 1)),
	})
}

// MARK: GET /groups/{}/leave
pub async fn leave(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	_body: models::LeaveGroupRequest,
) -> GlobalResult<models::LeaveGroupResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Validate the team exists
	let teams_res = op!([ctx] team_get {
		team_ids: vec![group_id.into()],
	})
	.await?;
	ensure_with!(!teams_res.teams.is_empty(), GROUP_NOT_FOUND);

	// Remove the team member
	msg!([ctx] team::msg::member_remove(group_id, user_ent.user_id) -> team::msg::member_remove_complete {
		team_id: Some(group_id.into()),
		user_id: Some(user_ent.user_id.into()),
		silent: false,
	})
	.await?;

	Ok(models::LeaveGroupResponse {})
}

// MARK: POST /groups
pub async fn create(
	ctx: Ctx<Auth>,
	body: models::CreateGroupRequest,
) -> GlobalResult<models::CreateGroupResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	let publicity = unwrap!(std::env::var("RIVET_ACCESS_KIND").ok());
	match publicity.as_str() {
		"public" => {}
		"private" => {
			ctx.auth().admin(ctx.op_ctx()).await?;
		}
		_ => bail!("invalid RIVET_ACCESS_KIND"),
	}

	let team_id = Uuid::new_v4();
	let create_res = msg!([ctx] team::msg::create(team_id) -> Result<team::msg::create_complete, team::msg::create_fail> {
		team_id: Some(team_id.into()),
		display_name: body.display_name.to_owned(),
		owner_user_id: Some(user_ent.user_id.into())
	})
	.await?;
	match create_res {
		Ok(_) => {}
		Err(msg) => {
			use team::msg::create_fail::ErrorCode::*;

			let code = team::msg::create_fail::ErrorCode::from_i32(msg.error_code);
			match unwrap!(code) {
				Unknown => bail!("unknown team create error code"),

				ValidationFailed => bail_with!(VALIDATION_ERROR),
			}
		}
	};

	Ok(models::CreateGroupResponse {
		group_id: team_id.to_string(),
	})
}

// MARK: POST /groups/{}/join-request
pub async fn request_join(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	_body: models::CreateGroupJoinRequestRequest,
) -> GlobalResult<models::CreateGroupJoinRequestResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Validate the team exists
	let teams_res = op!([ctx] team_get {
		team_ids: vec![group_id.into()],
	})
	.await?;
	let team = unwrap_with!(teams_res.teams.first(), GROUP_NOT_FOUND);

	// Validate team publicity
	let publicity = unwrap!(backend::team::Publicity::from_i32(team.publicity));
	ensure_eq_with!(
		publicity,
		backend::team::Publicity::Open,
		GROUP_CANNOT_REQUEST_JOIN
	);

	// Check if the user is banned
	let banned_users_res = op!([ctx] team_user_ban_get {
		members: vec![team::user_ban_get::request::Member {
			team_id: Some(group_id.into()),
			user_id: Some(user_ent.user_id.into()),
		}],
	})
	.await?;
	ensure_with!(
		banned_users_res.banned_users.is_empty(),
		GROUP_MEMBER_BANNED
	);

	// Create the team join request
	let res = msg!([ctx] team::msg::join_request_create(group_id, user_ent.user_id) -> Result<
			team::msg::join_request_create_complete,
			team::msg::join_request_create_fail
		> {
		team_id: Some(group_id.into()),
		user_id: Some(user_ent.user_id.into()),
		..Default::default()
	})
	.await?;

	if let Err(res) = res {
		let error_code = unwrap!(team::msg::join_request_create_fail::ErrorCode::from_i32(
			res.error_code
		));

		match error_code {
			team::msg::join_request_create_fail::ErrorCode::RequestAlreadyExists => {
				bail_with!(GROUP_JOIN_REQUEST_ALREADY_EXISTS)
			}
		}
	}

	Ok(models::CreateGroupJoinRequestResponse {})
}

// MARK: POST /groups/{}/join-request/{}
pub async fn resolve_join_request(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	identity_id: Uuid,
	body: models::ResolveGroupJoinRequestRequest,
) -> GlobalResult<models::ResolveGroupJoinRequestResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Validate the team exists
	let teams_res = op!([ctx] team_get {
		team_ids: vec![group_id.into()],
	})
	.await?;
	let team = unwrap_with!(teams_res.teams.first(), GROUP_NOT_FOUND);
	let owner_user_id = unwrap_ref!(team.owner_user_id).as_uuid();

	// Verify user's permissions
	ensure_eq_with!(
		user_ent.user_id,
		owner_user_id,
		GROUP_INSUFFICIENT_PERMISSIONS
	);

	// Resolve team request
	msg!([ctx] team::msg::join_request_resolve(group_id, identity_id) -> team::msg::join_request_resolve_complete {
		team_id: Some(group_id.into()),
		user_id: Some(identity_id.into()),
		resolution: body.resolution
	})
	.await?;

	Ok(models::ResolveGroupJoinRequestResponse {})
}

// MARK: GET /groups/search
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
	query: String,
	limit: Option<u32>,
	anchor: Option<String>,
}

pub async fn search(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: SearchQuery,
) -> GlobalResult<models::SearchGroupsResponse> {
	ctx.auth().user(ctx.op_ctx()).await?;

	ensure_with!(
		query.limit.map(|v| v != 0).unwrap_or(true),
		API_BAD_QUERY_PARAMETER,
		parameter = "count",
		error = "Must be greater than 0",
	);

	let team_search_res = op!([ctx] team_search {
		query: query.query.clone(),
		limit: query.limit.unwrap_or(32),
		anchor: query.anchor
			.map(|anchor| anchor.parse::<i64>())
			.transpose()?,
	})
	.await?;

	let team_res = op!([ctx] team_get {
		team_ids: team_search_res.team_ids.clone(),
	})
	.await?;

	let group_handles = team_res
		.teams
		.iter()
		.map(convert::group::handle)
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::SearchGroupsResponse {
		groups: group_handles,
		anchor: team_search_res.anchor.as_ref().map(ToString::to_string),
	})
}

// MARK: POST /groups/{}/profile
pub async fn validate_profile(
	ctx: Ctx<Auth>,
	body: models::ValidateGroupProfileRequest,
) -> GlobalResult<models::ValidateGroupProfileResponse> {
	ctx.auth().user(ctx.op_ctx()).await?;

	let res = op!([ctx] team_profile_validate {
		display_name: body.display_name.clone(),
		bio: body.bio.clone(),
	})
	.await?;

	Ok(models::ValidateGroupProfileResponse {
		errors: res
			.errors
			.into_iter()
			.map(ApiInto::api_into)
			.collect::<Vec<_>>(),
	})
}

// MARK: POST /groups/{}/transfer-owner
pub async fn transfer_owner(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	body: models::TransferGroupOwnershipRequest,
) -> GlobalResult<models::TransferGroupOwnershipResponse> {
	let (user, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	let new_owner_user_id = Uuid::from_str(body.new_owner_identity_id.as_str())?;

	// Verify user is not self
	ensure_with!(
		user_ent.user_id != new_owner_user_id,
		API_FORBIDDEN,
		reason = "Invalid user"
	);

	// Verify the team exists
	let teams_res = op!([ctx] team_get {
		team_ids: vec![group_id.into()],
	})
	.await?;
	let team = unwrap_with!(teams_res.teams.first(), GROUP_NOT_FOUND);
	let owner_user_id = unwrap_ref!(team.owner_user_id).as_uuid();

	// Verify request user's permissions
	ensure_eq_with!(
		user_ent.user_id,
		owner_user_id,
		GROUP_INSUFFICIENT_PERMISSIONS
	);

	// Verify that new owner is a group member
	ensure_with!(
		utils::group_member(&ctx, group_id, new_owner_user_id).await? || user.is_admin,
		GROUP_NOT_MEMBER
	);

	// Verify that new owner is registered
	assert::user_registered(&ctx, new_owner_user_id).await?;

	// Create the team member
	msg!([ctx] team::msg::owner_transfer(group_id) -> team::msg::update {
		team_id: Some(group_id.into()),
		new_owner_user_id: Some(new_owner_user_id.into()),
	})
	.await?;

	Ok(models::TransferGroupOwnershipResponse {})
}

// MARK: POST /groups/avatar-upload/prepare
pub async fn prepare_avatar_upload(
	ctx: Ctx<Auth>,
	body: new_models::GroupPrepareAvatarUploadRequest,
) -> GlobalResult<new_models::GroupPrepareAvatarUploadResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;
	assert::user_registered(&ctx, user_ent.user_id).await?;

	ensure!(body.content_length >= 0, "Upload invalid");
	ensure_with!(
		body.content_length < MAX_AVATAR_UPLOAD_SIZE,
		UPLOAD_TOO_LARGE
	);

	let ext = if body.path.ends_with(".png") {
		"png"
	} else if body.path.ends_with(".jpg") || body.path.ends_with(".jpeg") {
		"jpeg"
	} else {
		bail!("invalid file type (allowed: .png, .jpg)");
	};

	// Create the upload
	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: "bucket-team-avatar".to_owned(),
		files: vec![
			backend::upload::PrepareFile {
				path: format!("image.{}", ext),
				mime: Some(format!("image/{}", ext)),
				content_length: body.content_length.api_try_into()?,
				nsfw_score_threshold: Some(util_nsfw::score_thresholds::TEAM_AVATAR),
				..Default::default()
			},
		],
		user_id: Some(user_ent.user_id.into()),
	})
	.await?;

	let upload_id = unwrap_ref!(upload_prepare_res.upload_id).as_uuid();
	let presigned_request = unwrap!(upload_prepare_res.presigned_requests.first());

	Ok(new_models::GroupPrepareAvatarUploadResponse {
		upload_id,
		presigned_request: Box::new(presigned_request.clone().api_try_into()?),
	})
}

// MARK: POST /groups/avatar-upload/{}/complete
pub async fn complete_avatar_upload(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	upload_id: Uuid,
	_body: models::CompleteGroupAvatarUploadRequest,
) -> GlobalResult<models::CompleteGroupAvatarUploadResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Validate the team exists
	let teams_res = op!([ctx] team_get {
		team_ids: vec![group_id.into()],
	})
	.await?;
	let team = unwrap_with!(teams_res.teams.first(), GROUP_NOT_FOUND);
	let owner_user_id = unwrap_ref!(team.owner_user_id).as_uuid();

	// Verify user's permissions
	ensure_eq_with!(
		user_ent.user_id,
		owner_user_id,
		GROUP_INSUFFICIENT_PERMISSIONS
	);

	op!([ctx] team_avatar_upload_complete {
		team_id: Some(group_id.into()),
		upload_id: Some(upload_id.into()),
	})
	.await?;

	Ok(models::CompleteGroupAvatarUploadResponse {})
}

// MARK: POST /groups/{}/invites
pub async fn create_invite(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	body: models::CreateGroupInviteRequest,
) -> GlobalResult<models::CreateGroupInviteResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Validate the team exists
	let teams_res = op!([ctx] team_get {
		team_ids: vec![group_id.into()],
	})
	.await?;
	let _team = unwrap_with!(teams_res.teams.first(), GROUP_NOT_FOUND);
	ensure_with!(
		utils::group_member(&ctx, group_id, user_ent.user_id).await?,
		GROUP_NOT_FOUND
	);

	if let Some(ttl) = body.ttl {
		ensure!(ttl >= 0, "invalid parameter `ttl`");
		ensure!(ttl <= util::duration::days(30), "parameter `ttl` too large");
	}
	if let Some(use_count) = body.use_count {
		ensure!(use_count >= 0, "invalid parameter `use_count`");
		ensure!(use_count <= 5000, "parameter `use_count` too large");
	}

	let res = msg!([ctx] team_invite::msg::create(group_id) -> team_invite::msg::create_complete {
		team_id: Some(group_id.into()),
		ttl: body.ttl,
		max_use_count: body.use_count.map(|v| v.api_try_into()).transpose()?,
	})
	.await?;

	Ok(models::CreateGroupInviteResponse {
		code: res.code.clone(),
	})
}

// MARK: POST /invites/{}/consume
pub async fn consume_invite(
	ctx: Ctx<Auth>,
	code: String,
	_body: models::ConsumeGroupInviteRequest,
) -> GlobalResult<models::ConsumeGroupInviteResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	ensure_eq!(code.len(), 8, "invalid code");

	let (mut create_sub, mut fail_sub) = tokio::try_join!(
		subscribe!([ctx] team::msg::member_create("*", user_ent.user_id)),
		subscribe!([ctx] team_invite::msg::consume_fail(&code, user_ent.user_id)),
	)?;

	msg!([ctx] team_invite::msg::consume(&code, user_ent.user_id) {
		user_id: Some(user_ent.user_id.into()),
		code: code.clone(),
	})
	.await?;

	let (failure, team_id) = util::macros::select_with_timeout!([10 SEC] {
		event = fail_sub.next() => {
			let event = event?;

			(
				Some(unwrap!(
					team_invite::msg::consume_fail::ErrorCode::from_i32(event.error_code)
				)),
				event.team_id.as_ref().map(common::Uuid::as_uuid),
			)
		}
		event = create_sub.next() => {
			let event = event?;

			(
				None,
				Some(unwrap_ref!(event.team_id).as_uuid()),
			)
		}
	});

	// Timeout condition
	let failure = (failure.is_none() && team_id.is_none())
		.then_some(team_invite::msg::consume_fail::ErrorCode::Unknown)
		.or(failure);

	if let Some(failure) = failure {
		use team_invite::msg::consume_fail::ErrorCode;

		match failure {
			ErrorCode::Unknown => bail_with!(GROUP_FAILED_TO_CONSUME_INVITE),
			ErrorCode::InviteCodeInvalid => bail_with!(GROUP_INVITE_CODE_INVALID),
			ErrorCode::InviteExpired => {
				bail_with!(GROUP_INVITE_CODE_EXPIRED)
			}
			ErrorCode::InviteRevoked => {
				bail_with!(GROUP_INVITE_CODE_REVOKED)
			}
			ErrorCode::InviteAlreadyUsed => bail_with!(GROUP_INVITE_CODE_ALREADY_USED),
			ErrorCode::UserAlreadyTeamMember => {
				bail_with!(GROUP_ALREADY_MEMBER {
					metadata: json!({ "group_id": team_id })
				})
			}
			ErrorCode::TeamFull => {
				bail_with!(GROUP_FULL)
			}
			ErrorCode::UserBanned => {
				bail_with!(GROUP_MEMBER_BANNED);
			}
		}
	};

	Ok(models::ConsumeGroupInviteResponse {
		group_id: team_id.map(|id| id.to_string()),
	})
}

// MARK: GET /invites/{}
pub async fn get_invite(
	ctx: Ctx<Auth>,
	code: String,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetGroupInviteResponse> {
	ctx.auth().user(ctx.op_ctx()).await?;

	ensure_eq!(code.len(), 8, "invalid code");

	let invite_res = op!([ctx] team_invite_get {
		codes: vec![code.clone()],
	})
	.await?;

	if let Some(invite) = invite_res.invites.first() {
		if invite
			.expire_ts
			.map(|ts| util::timestamp::now() >= ts)
			.unwrap_or_default()
		{
			bail_with!(GROUP_INVITE_CODE_EXPIRED);
		} else if invite.revoke_ts.is_some() {
			bail_with!(GROUP_INVITE_CODE_REVOKED);
		}

		let team_id = unwrap!(invite.team_id);

		let team_res = op!([ctx] team_get {
			team_ids: vec![team_id],
		})
		.await?;
		let team = unwrap!(team_res.teams.first());

		Ok(models::GetGroupInviteResponse {
			group: convert::group::handle(team)?,
		})
	} else {
		bail_with!(GROUP_INVITE_CODE_INVALID);
	}
}

// MARK: POST /groups/{}/kick/{}
pub async fn kick_member(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	identity_id: Uuid,
	_body: models::KickGroupMemberRequest,
) -> GlobalResult<models::KickGroupMemberResponse> {
	let (user, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Verify user is not self
	ensure_with!(
		user_ent.user_id != identity_id,
		API_FORBIDDEN,
		reason = "Invalid user"
	);

	// Verify the team exists
	let teams_res = op!([ctx] team_get {
		team_ids: vec![group_id.into()],
	})
	.await?;
	let team = unwrap_with!(teams_res.teams.first(), GROUP_NOT_FOUND);
	let owner_user_id = unwrap_ref!(team.owner_user_id).as_uuid();

	// Verify request user's permissions
	ensure_eq_with!(
		user_ent.user_id,
		owner_user_id,
		GROUP_INSUFFICIENT_PERMISSIONS
	);

	// Verify that user is a group member
	ensure_with!(
		utils::group_member(&ctx, group_id, identity_id).await? || user.is_admin,
		GROUP_NOT_MEMBER
	);

	// Kick the team member
	msg!([ctx] team::msg::member_kick(group_id, identity_id) -> team::msg::member_kick_complete {
		team_id: Some(group_id.into()),
		user_id: Some(identity_id.into()),
		kicker_user_id: Some(user_ent.user_id.into()),
	})
	.await?;

	Ok(models::KickGroupMemberResponse {})
}

// MARK: POST /groups/{}/ban/{}
pub async fn ban(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	identity_id: Uuid,
	_body: models::BanGroupIdentityRequest,
) -> GlobalResult<models::BanGroupIdentityResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Verify user is not self
	ensure_with!(
		user_ent.user_id != identity_id,
		API_FORBIDDEN,
		reason = "Invalid user"
	);

	// Verify the team exists
	let teams_res = &op!([ctx] team_get {
		team_ids: vec![group_id.into()],
	})
	.await?;
	let team = unwrap_with!(teams_res.teams.first(), GROUP_NOT_FOUND);
	let owner_user_id = unwrap_ref!(team.owner_user_id).as_uuid();

	// Verify request user's permissions
	ensure_eq_with!(
		user_ent.user_id,
		owner_user_id,
		GROUP_INSUFFICIENT_PERMISSIONS
	);

	// Ban the user
	msg!([ctx] team::msg::user_ban(group_id, identity_id) -> team::msg::user_ban_complete {
		team_id: Some(group_id.into()),
		user_id: Some(identity_id.into()),
		banner_user_id: Some(user_ent.user_id.into()),
	})
	.await?;

	Ok(models::BanGroupIdentityResponse {})
}

// MARK: DELETE /groups/{}/ban/{}
pub async fn unban(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	identity_id: Uuid,
) -> GlobalResult<models::UnbanGroupIdentityResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Verify user is not self
	ensure_with!(
		user_ent.user_id != identity_id,
		API_FORBIDDEN,
		reason = "Invalid user"
	);

	// Verify the team exists
	let teams_res = &op!([ctx] team_get {
		team_ids: vec![group_id.into()],
	})
	.await?;
	let team = unwrap_with!(teams_res.teams.first(), GROUP_NOT_FOUND);
	let owner_user_id = unwrap_ref!(team.owner_user_id).as_uuid();

	// Verify request user's permissions
	ensure_eq_with!(
		user_ent.user_id,
		owner_user_id,
		GROUP_INSUFFICIENT_PERMISSIONS
	);

	// Unban the user
	msg!([ctx] team::msg::user_unban(group_id, identity_id) -> team::msg::user_unban_complete {
		team_id: Some(group_id.into()),
		user_id: Some(identity_id.into()),
		unbanner_user_id: Some(user_ent.user_id.into()),
	})
	.await?;

	Ok(models::UnbanGroupIdentityResponse {})
}

// MARK: GET /groups/{}/bans
enum TeamBanConsumerUpdate {
	Ban(common::Uuid),
	Unban(common::Uuid),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListBansQuery {
	anchor: Option<String>,
	limit: Option<u32>,
}

pub async fn bans(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	watch_index: WatchIndexQuery,
	_query: ListBansQuery,
) -> GlobalResult<models::GetGroupBansResponse> {
	let (_, user_ent) = ctx.auth().user(ctx.op_ctx()).await?;

	// Verify the team exists
	let (teams_res, team_bans_res) = tokio::try_join!(
		op!([ctx] team_get {
			team_ids: vec![group_id.into()],
		}),
		op!([ctx] team_user_ban_list {
			team_ids: vec![group_id.into()],
			// TODO: Re-add when pagination is added to bans
			// limit: query.limit.or(Some(16)),
			// anchor: query.anchor
			// 	.map(|anchor| anchor.parse::<i64>())
			// 	.transpose()?,
		}),
	)?;
	let team = unwrap_with!(teams_res.teams.first(), GROUP_NOT_FOUND);
	let team_bans = unwrap!(team_bans_res.teams.first()).clone();
	let owner_user_id = unwrap_ref!(team.owner_user_id).as_uuid();

	// Verify request user's permissions
	ensure_eq_with!(
		user_ent.user_id,
		owner_user_id,
		GROUP_INSUFFICIENT_PERMISSIONS
	);

	let mut user_ids = team_bans
		.banned_users
		.iter()
		.map(|ban| Ok(unwrap!(ban.user_id)))
		.collect::<GlobalResult<Vec<_>>>()?;

	// Wait for an update if needed
	let (update, update_ts) = if let Some(anchor) = watch_index.to_consumer()? {
		// Team events
		let team_ban_sub = tail_anchor!([ctx, anchor] team::msg::user_ban_complete(group_id, "*"));
		let team_unban_sub =
			tail_anchor!([ctx, anchor] team::msg::user_unban_complete(group_id, "*"));

		// User subs
		let user_subs_select =
			util::future::select_all_or_wait(user_ids.iter().cloned().map(|user_id| {
				tail_anchor!([ctx, anchor] user::msg::update(user_id.as_uuid())).boxed()
			}));

		// User presence subs
		let user_presence_subs_select =
			util::future::select_all_or_wait(user_ids.iter().cloned().map(|user_id| {
				tail_anchor!([ctx, anchor] user_presence::msg::update(user_id.as_uuid())).boxed()
			}));

		// Listen for updates
		util::macros::select_with_timeout!({
			event = team_ban_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.user_id.map(TeamBanConsumerUpdate::Ban), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = team_unban_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.user_id.map(TeamBanConsumerUpdate::Unban), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = user_subs_select => {
				(None, event?.msg_ts())
			}
			event = user_presence_subs_select => {
				(None, event?.msg_ts())
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	// Handle ban list changes
	match update {
		Some(TeamBanConsumerUpdate::Ban(user_id)) => {
			// New ban is within the paginated range

			// TODO: Re-add when pagination is added to bans
			// if team_bans
			// 	.anchor
			// 	.map(|anchor| team_member.join_ts < anchor)
			// 	.unwrap_or(true)
			// {
			// 	user_ids.push(user_id);
			// }

			// TODO: Remove when pagination is added
			user_ids.push(user_id);
		}
		Some(TeamBanConsumerUpdate::Unban(user_id)) => {
			// Remove the unbanned user id
			user_ids.retain(|u| u != &user_id);
		}
		_ => {}
	}

	// NOTE: We don't use fetch::identities::handles here because the end model is `BannedIdentity` not
	// `IdentityHandle`
	// Fetch team member and ban data
	let (users, presences_ctx, user_follows) = tokio::try_join!(
		fetch::identity::users(&ctx, user_ids.clone()),
		fetch::identity::presence_data(&ctx, user_ent.user_id, user_ids.clone(), false),
		fetch::identity::follows(
			&ctx,
			user_ent.user_id,
			user_ids
				.iter()
				.map(common::Uuid::as_uuid)
				.collect::<Vec<_>>()
		),
	)?;

	let raw_user_ent_id = Into::<common::Uuid>::into(user_ent.user_id);
	let banned_identities = users
		.users
		.iter()
		.map(|user| {
			// Determine ban ts
			let ban_ts = if let Some(TeamBanConsumerUpdate::Ban(_)) = update {
				update_ts
			} else {
				unwrap!(team_bans
					.banned_users
					.iter()
					.find(|ban| user.user_id == ban.user_id))
				.ban_ts
			};

			let is_mutual_following = user_follows.follows.iter().any(|follow| {
				follow.follower_user_id.as_ref() == user.user_id.as_ref()
					&& follow.following_user_id.as_ref() == Some(&raw_user_ent_id)
					&& follow.is_mutual
			});

			Ok(models::GroupBannedIdentity {
				identity: convert::identity::handle(
					user_ent.user_id,
					user,
					&presences_ctx,
					is_mutual_following,
				)?,
				ban_ts: util::timestamp::to_chrono(ban_ts)?,
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::GetGroupBansResponse {
		banned_identities,
		// TODO: Re-add when pagination is added to bans
		// anchor: team_members.anchor.map(|anchor| anchor.to_string()),
		anchor: None,
		watch: convert::watch_response(WatchResponse::new(update_ts + 1)),
	})
}
