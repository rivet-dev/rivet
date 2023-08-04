use std::collections::HashSet;

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
use rivet_api::models;
use rivet_convert::{convert, fetch};
use rivet_operation::prelude::*;

use crate::auth::Auth;

#[derive(Debug)]
struct FollowingEntry<'a> {
	user: &'a backend::user::User,
	presence: &'a backend::user::Presence,
}

// MARK: GET /activities
#[derive(Debug)]
enum ConsumerUpdate {
	Follow(common::Uuid),
	Unfollow(common::Uuid),
}

// TODO: Add message when a new game user session is created and don't refetch all recent games when receiving
// that message
pub async fn activities(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::IdentityListActivitiesResponse> {
	let (current_user_id, game_user) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	// Fetch users
	let followers_res = op!([ctx] user_follow_list {
		kind: user_follow::list::request::Kind::Mutual as i32,
		user_ids: vec![current_user_id.into()],
		limit: 32,
		anchor: None,
	})
	.await?;
	let follows = internal_unwrap_owned!(followers_res.follows.first())
		.follows
		.clone();

	// Fetch user presences
	let mut user_ids = follows
		.iter()
		.filter_map(|f| f.user_id)
		.map(|f| *f)
		.collect::<HashSet<_>>();

	// Wait for an update if needed
	let (update, update_ts) = if let Some(anchor) = watch_index.to_consumer()? {
		let follow_sub =
			tail_anchor!([ctx, anchor] user::msg::mutual_follow_create(current_user_id));
		let unfollow_sub =
			tail_anchor!([ctx, anchor] user::msg::mutual_follow_delete(current_user_id));

		// User presence subs
		let user_presence_subs_select =
			util::future::select_all_or_wait(user_ids.iter().cloned().map(|user_id| {
				tail_anchor!([ctx, anchor] user_presence::msg::update(user_id)).boxed()
			}));

		util::macros::select_with_timeout!({
			event = follow_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.user_b_id.map(ConsumerUpdate::Follow), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = unfollow_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.user_b_id.map(ConsumerUpdate::Unfollow), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = user_presence_subs_select => {
				if let TailAnchorResponse::Message(msg) = event? {
					(None, Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	// Remove/add new user
	match update {
		Some(ConsumerUpdate::Follow(user_id)) => {
			user_ids.insert(user_id.as_uuid());
		}
		Some(ConsumerUpdate::Unfollow(user_id)) => {
			user_ids.remove(&user_id.as_uuid());
		}
		_ => {}
	}

	// Fetch suggested players and add to user id list so we can fetch their presence data
	let suggested_players =
		fetch_suggested_players(ctx.op_ctx(), current_user_id, &game_user).await?;
	user_ids.extend(
		suggested_players
			.iter()
			.map(|game_user| Ok(**internal_unwrap!(game_user.user_id)))
			.collect::<GlobalResult<Vec<_>>>()?
			.into_iter(),
	);

	// Convert into Vec<common::Uuid>
	let user_ids = user_ids.into_iter().map(Into::into).collect::<Vec<_>>();

	// Fetch user data and new follower data (if updated)
	let (follows, users, presences_ctx, recent_games, suggested_groups) = tokio::try_join!(
		// Fetch follow list updates
		async {
			match update {
				// Refetch follow list if a new follow occurred
				Some(ConsumerUpdate::Follow(_)) => {
					let followers_res = op!([ctx] user_follow_list {
						kind: user_follow::list::request::Kind::Mutual as i32,
						user_ids: vec![current_user_id.into()],
						limit: 32,
						anchor: None,
					})
					.await
					.map_err(Into::<GlobalError>::into)?;

					GlobalResult::Ok(
						internal_unwrap_owned!(followers_res.follows.first())
							.follows
							.clone(),
					)
				}
				_ => Ok(follows),
			}
		},
		fetch::identity::users(ctx.op_ctx(), user_ids.clone()),
		fetch::identity::presence_data(ctx.op_ctx(), current_user_id, user_ids, true),
		fetch_recent_games(ctx.op_ctx(), current_user_id, &game_user),
		fetch_suggested_groups(ctx.op_ctx(), current_user_id),
	)?;

	// Build following state
	let mut follows = follows
		.iter()
		.filter(|f| {
			// Filter out whoever was unfollowed (since we don't re-fetch the follows list for unfollows)
			if let Some(ConsumerUpdate::Unfollow(unfollow_id)) = update {
				f.user_id != Some(unfollow_id)
			} else {
				true
			}
		})
		.map(|follow| {
			let user =
				internal_unwrap_owned!(users.users.iter().find(|u| u.user_id == follow.user_id));
			let presence = internal_unwrap_owned!(presences_ctx
				.res
				.users
				.iter()
				.find(|u| u.user_id == follow.user_id)
				.and_then(|p| p.presence.as_ref()));

			Ok(FollowingEntry { user, presence })
		})
		.collect::<GlobalResult<Vec<_>>>()?
		.into_iter()
		.filter(|f| f.presence.status != backend::user::Status::Offline as i32)
		.collect::<Vec<_>>();

	// Sort follows
	follows.sort_by_key(|f| f.presence.update_ts);

	let identities = follows
		.into_iter()
		.map(|follow| convert::identity::handle(current_user_id, follow.user, &presences_ctx, true))
		.collect::<GlobalResult<Vec<_>>>()?;

	// TODO: Add back
	let parties = Vec::new();
	// let parties = presences_ctx
	// 	.parties
	// 	.parties
	// 	.values()
	// 	.map(|party| {
	// 		(
	// 			party.party_id,
	// 			convert::party::summary(
	// 				&current_user_id,
	// 				party,
	// 				&presences_ctx.games_with_namespace_ids,
	// 				&presences_ctx.parties.members,
	// 				&presences_ctx.parties.member_users,
	// 				&presences_ctx.parties.threads,
	// 			),
	// 		)
	// 	})
	// 	.filter_map(|(party_id, result)| match result {
	// 		Ok(x) => Some(x),
	// 		Err(err) => {
	// 			tracing::error!(?err, ?party_id, "failed to fetch party for activity");
	// 			None
	// 		}
	// 	})
	// 	.collect::<Vec<_>>();

	let suggested_players = suggested_players
		.into_iter()
		.filter_map(|game_user| {
			if let Some(user) = users.users.iter().find(|u| u.user_id == game_user.user_id) {
				Some(convert::identity::handle(
					current_user_id,
					user,
					&presences_ctx,
					true,
				))
			} else {
				tracing::info!(?game_user, "game user's user does not exist");
				None
			}
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::IdentityListActivitiesResponse {
		identities,
		games: recent_games,
		parties,
		suggested_groups,
		suggested_players,

		watch: WatchResponse::new_as_model(update_ts),
	})
}

async fn fetch_recent_games(
	ctx: &OperationContext<()>,
	current_user_id: Uuid,
	game_user: &Option<game_user::get::response::GameUser>,
) -> GlobalResult<Vec<models::GameSummary>> {
	// Don't return recent games for game users
	if game_user.is_some() {
		return Ok(Vec::new());
	}

	let recent_session_res = op!([ctx] game_user_recent_session_list {
		user_ids: vec![current_user_id.into()],
	})
	.await?;
	let user = internal_unwrap_owned!(recent_session_res.users.first());

	// Fetch game IDs
	let ns_ids = user
		.sessions
		.iter()
		.filter_map(|x| x.namespace_id)
		.collect::<Vec<_>>();
	let games_res = op!([ctx] game_resolve_namespace_id {
		namespace_ids: ns_ids,
	})
	.await?;

	// Fetch games
	let game_ids = games_res
		.games
		.iter()
		.filter_map(|x| x.game_id)
		.map(|x| x.as_uuid())
		.collect::<Vec<_>>();
	let games = fetch::game::summaries(ctx, game_ids).await?;

	// Reorder the games by the recent session order
	let games = user
		.sessions
		.iter()
		// Session -> namespace
		.filter_map(|session| session.namespace_id)
		// Namespace -> game
		.filter_map(|namespace_id| {
			games_res
				.games
				.iter()
				.find(|x| x.namespace_ids.contains(&namespace_id))
				.and_then(|x| x.game_id)
				.map(|x| x.as_uuid())
		})
		// Game -> game summary
		.filter_map(|game_id| games.iter().find(|x| x.game_id == game_id))
		.cloned()
		.collect::<Vec<_>>();

	Ok(games)
}

async fn fetch_suggested_players(
	ctx: &OperationContext<()>,
	current_user_id: Uuid,
	game_user: &Option<game_user::get::response::GameUser>,
) -> GlobalResult<Vec<game_user::get::response::GameUser>> {
	let recommended_game_user_res = op!([ctx] game_user_recommend {
		count: 16,
	})
	.await?;

	// Remove self from list if present (game user)
	let game_user_ids = if let Some(game_user) = game_user {
		let game_user_id = internal_unwrap!(game_user.game_user_id);

		recommended_game_user_res
			.game_user_ids
			.iter()
			.filter(|gi| gi != &game_user_id)
			.cloned()
			.collect::<Vec<_>>()
	} else {
		recommended_game_user_res.game_user_ids.clone()
	};

	let game_user_res = op!([ctx] game_user_get {
		game_user_ids: game_user_ids,
	})
	.await?;

	// Remove self from list if present (normal user)
	Ok(game_user_res
		.game_users
		.into_iter()
		.filter(|gu| gu.user_id.map(|id| *id) != Some(current_user_id))
		.collect::<Vec<_>>())
}

async fn fetch_suggested_groups(
	ctx: &OperationContext<()>,
	current_user_id: Uuid,
) -> GlobalResult<Vec<models::GroupSummary>> {
	let teams_res = op!([ctx] team_recommend {
		count: 16,
	})
	.await?;

	fetch::group::summaries(
		ctx,
		current_user_id,
		teams_res
			.team_ids
			.iter()
			.map(|team_id| **team_id)
			.collect::<Vec<_>>(),
	)
	.await
}
