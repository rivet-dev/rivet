use std::convert::TryInto;

use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;
use rivet_party_server::models;
use serde::Deserialize;

use crate::{auth::Auth, convert, fetch, utils};

// MARK: GET /parties/{}/summary
pub async fn summary(
	ctx: Ctx<Auth>,
	party_id: Uuid,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetPartySummaryResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	utils::touch_user_presence(ctx.op_ctx().base(), current_user_id);

	// Wait for an update if needed
	let update_ts = if let Some(anchor) = watch_index.to_consumer()? {
		let party_update_sub = tail_anchor!([ctx, anchor] party::msg::update(party_id));

		util::macros::select_with_timeout!({
			event = party_update_sub => {
				event?.msg_ts()
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	let mut parties = fetch::party::summaries(&ctx, current_user_id, vec![party_id]).await?;
	let party = internal_unwrap_owned!(parties.pop());

	Ok(models::GetPartySummaryResponse {
		party,
		watch: convert::watch_response(WatchResponse::new(update_ts + 1)),
	})
}

// MARK: GET /parties/self/summary
pub async fn summary_self(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetPartySelfSummaryResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	utils::touch_user_presence(ctx.op_ctx().base(), current_user_id);

	// Wait for an update if needed
	let update_ts = if let Some(anchor) = watch_index.to_consumer()? {
		if let Some(party_id) = utils::get_current_party(ctx.op_ctx(), current_user_id).await? {
			// Watch both for party updates and the party member update

			let party_update_sub = tail_anchor!([ctx, anchor] party::msg::update(party_id));
			let party_member_update_sub =
				tail_anchor!([ctx, anchor] party::msg::member_update(current_user_id));

			util::macros::select_with_timeout!({
				event = party_update_sub => {
					event?.msg_ts()
				}
				event = party_member_update_sub => {
					event?.msg_ts()
				}
			})
		} else {
			// The user is not in a party, only watch for party member change
			let party_member_update_sub =
				tail_anchor!([ctx, anchor] party::msg::member_update(current_user_id));

			util::macros::select_with_timeout!({
				event = party_member_update_sub => {
					event?.msg_ts()
				}
			})
		}
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	// Fetch party member again since this may have changed
	let party =
		if let Some(party_id) = utils::get_current_party(ctx.op_ctx(), current_user_id).await? {
			// Fetch associated party
			let res = fetch::party::summaries(&ctx, current_user_id, vec![party_id]).await;
			match res {
				// Explicitly handle PARTY_PARTY_NOT_FOUND since indicates there was a race condition
				// between the party the user is in.
				Err(err) if err.is(formatted_error::code::PARTY_PARTY_NOT_FOUND) => None,
				x => {
					let mut parties = x?;
					parties.pop()
				}
			}
		} else {
			None
		};

	Ok(models::GetPartySelfSummaryResponse {
		party,
		watch: convert::watch_response(WatchResponse::new(update_ts + 1)),
	})
}

// MARK: GET /parties/{}/profile
pub async fn profile(
	ctx: Ctx<Auth>,
	party_id: Uuid,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetPartyProfileResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	utils::touch_user_presence(ctx.op_ctx().base(), current_user_id);

	// Wait for an update if needed
	let update_ts = if let Some(anchor) = watch_index.to_consumer()? {
		let party_update_sub = tail_anchor!([ctx, anchor] party::msg::update(party_id));

		util::macros::select_with_timeout!({
			event = party_update_sub => {
				event?.msg_ts()
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	let party = fetch::party::profile(&ctx, current_user_id, party_id).await?;

	Ok(models::GetPartyProfileResponse {
		party,
		watch: convert::watch_response(WatchResponse::new(update_ts + 1)),
	})
}

// MARK: GET /parties/self/profile
pub async fn profile_self(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetPartySelfProfileResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	utils::touch_user_presence(ctx.op_ctx().base(), current_user_id);

	// Wait for an update if needed
	let update_ts = if let Some(anchor) = watch_index.to_consumer()? {
		if let Some(party_id) = utils::get_current_party(ctx.op_ctx(), current_user_id).await? {
			// Watch both for party updates and the party member update

			let party_update_sub = tail_anchor!([ctx, anchor] party::msg::update(party_id));
			let party_member_update_sub =
				tail_anchor!([ctx, anchor] party::msg::member_update(current_user_id));

			util::macros::select_with_timeout!({
				event = party_update_sub => {
					event?.msg_ts()
				}
				event = party_member_update_sub => {
					event?.msg_ts()
				}
			})
		} else {
			// The user is not in a party, only watch for party member change
			let party_member_update_sub =
				tail_anchor!([ctx, anchor] party::msg::member_update(current_user_id));

			util::macros::select_with_timeout!({
				event = party_member_update_sub => {
					event?.msg_ts()
				}
			})
		}
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	// Fetch party member again since this may have changed
	let party =
		if let Some(party_id) = utils::get_current_party(ctx.op_ctx(), current_user_id).await? {
			// Fetch associated party
			let res = fetch::party::profile(&ctx, current_user_id, party_id).await;
			match res {
				// Explicitly handle PARTY_PARTY_NOT_FOUND since indicates there was a race condition
				// between the party the user is in.
				Err(x) if x.is(formatted_error::code::PARTY_PARTY_NOT_FOUND) => None,
				x => Some(x?),
			}
		} else {
			None
		};

	Ok(models::GetPartySelfProfileResponse {
		party,
		watch: convert::watch_response(WatchResponse::new(update_ts + 1)),
	})
}

// MARK: POST /parties
pub async fn create_party(
	ctx: Ctx<Auth>,
	body: models::CreatePartyRequest,
) -> GlobalResult<models::CreatePartyResponse> {
	let (current_user_id, game_user) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	assert_with!(
		body.party_size >= 2 && body.party_size <= 16,
		PARTY_BAD_PARTY_SIZE
	);

	let party_id = Uuid::new_v4();

	struct CreatedInvite {
		alias: Option<String>,
		invite_id: Uuid,
		token: String,
	}
	let mut created_invites = Vec::new();

	if let Some(invites) = body.invites {
		assert_with!(invites.len() <= 16, PARTY_TOO_MANY_INVITES);

		// Create invites
		let mut create_invite_error = None;

		for invite in &invites {
			// Build party alias
			let alias = if let Some(alias) = &invite.alias {
				if let Some(game_user) = &game_user {
					Some(party::msg::invite_create::Alias {
						namespace_id: game_user.namespace_id,
						alias: alias.clone(),
					})
				} else {
					panic_with!(PARTY_UNEXPECTED_INVITE_ALIAS)
				}
			} else {
				None
			};

			let invite_id = Uuid::new_v4();
			let create_res = msg!([ctx] party::msg::invite_create(party_id, invite_id) -> Result<party::msg::invite_create_complete, party::msg::invite_create_fail> {
				party_id: Some(party_id.into()),
				invite_id: Some(invite_id.into()),
				alias: alias,
				preemptive_party: true,
				..Default::default()
			}).await?;
			match create_res
				.map_err(|msg| party::msg::invite_create_fail::ErrorCode::from_i32(msg.error_code))
			{
				Ok(res) => {
					created_invites.push(CreatedInvite {
						alias: invite.alias.clone(),
						invite_id,
						token: res.token.to_owned(),
					});
				}
				Err(Some(code)) => {
					use party::msg::invite_create_fail::ErrorCode::*;

					create_invite_error = Some(match code {
						Unknown => internal_panic!("unknown party invite create error code"),
						PartyDoesNotExist => err_code!(PARTY_PARTY_NOT_FOUND),
						AliasNotUnique => err_code!(PARTY_INVITE_ALIAS_NOT_UNIQUE),
					});
					break;
				}
				Err(None) => internal_panic!("failed to parse invite error code"),
			};
		}

		// Rollback created invites if error
		if let Some(create_invite_error) = create_invite_error {
			for &CreatedInvite { invite_id, .. } in &created_invites {
				msg!([ctx] party::msg::invite_destroy(invite_id) {
					invite_id: Some(invite_id.into()),
					skip_party_updated: true,
				})
				.await?;
			}

			return Err(create_invite_error);
		}
	}

	// Create party
	let initial_state = if let (Some(game_user), Some(matchmaker_current_player_token)) =
		(game_user, body.matchmaker_current_player_token)
	{
		// TODO: Race condition

		let token = rivet_claims::decode(&matchmaker_current_player_token)??;
		let player_ent = token.as_matchmaker_player()?;

		// Player should always exist since the token is valid
		let player_get_res = op!([ctx] mm_player_get {
			player_ids: vec![player_ent.player_id.into()],
		})
		.await?;
		let player = internal_unwrap_owned!(player_get_res.players.first());

		Some(party::msg::create::message::InitialState::MatchmakerLobby(
			party::msg::create::message::StateMatchmakerLobby {
				namespace_id: game_user.namespace_id,
				lobby_id: player.lobby_id,
				leader_player_id: player.player_id,
				leader_player_token: matchmaker_current_player_token.clone(),
			},
		))
	} else {
		None
	};

	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(current_user_id.into()),
		party_size: body.party_size.try_into()?,
		initial_state:  initial_state,
		publicity: body.publicity.map(|x| party::msg::create::message::Publicity {
			public: x.public.map(convert::party::publicity_level_to_proto),
			friends: x.mutual_followers.map(convert::party::publicity_level_to_proto),
			teams: x.groups.map(convert::party::publicity_level_to_proto),
		}),
		client: Some(ctx.client_info()),
	})
	.await?;

	Ok(models::CreatePartyResponse {
		party_id: party_id.to_string(),
		invites: created_invites
			.into_iter()
			.map(|invite| models::CreatedInvite {
				alias: invite.alias,
				token: invite.token,
			})
			.collect(),
	})
}

// MARK: POST /parties/self/invites
pub async fn create_party_invite(
	ctx: Ctx<Auth>,
	body: models::CreatePartyInviteRequest,
) -> GlobalResult<models::CreatePartyInviteResponse> {
	let (current_user_id, game_user) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	// Build party alias
	let alias = if let Some(alias) = &body.alias {
		assert_with!(
			!alias.is_empty() && alias.len() <= 64,
			PARTY_BAD_INVITE_ALIAS_LENGTH
		);

		if let Some(game_user) = &game_user {
			Some(party::msg::invite_create::Alias {
				namespace_id: game_user.namespace_id,
				alias: alias.clone(),
			})
		} else {
			panic_with!(PARTY_UNEXPECTED_INVITE_ALIAS)
		}
	} else {
		None
	};

	let party_id = unwrap_with_owned!(
		utils::get_current_party(ctx.op_ctx(), current_user_id).await?,
		PARTY_IDENTITY_NOT_IN_ANY_PARTY
	);

	utils::assert_party_leader(ctx.op_ctx(), party_id, current_user_id).await?;

	// Create invite
	let invite_id = Uuid::new_v4();
	let create_res = msg!([ctx] party::msg::invite_create(party_id, invite_id) -> Result<party::msg::invite_create_complete, party::msg::invite_create_fail> {
		party_id: Some(party_id.into()),
		invite_id: Some(invite_id.into()),
		alias: alias,
		..Default::default()
	}).await?;

	let token = match create_res
		.map_err(|msg| party::msg::invite_create_fail::ErrorCode::from_i32(msg.error_code))
	{
		Ok(res) => res.token.to_owned(),
		Err(Some(code)) => {
			use party::msg::invite_create_fail::ErrorCode::*;

			match code {
				Unknown => internal_panic!("unknown party invite create error code"),
				PartyDoesNotExist => panic_with!(PARTY_PARTY_NOT_FOUND),
				AliasNotUnique => panic_with!(PARTY_INVITE_ALIAS_NOT_UNIQUE),
			};
		}
		Err(None) => internal_panic!("failed to parse invite error code"),
	};

	Ok(models::CreatePartyInviteResponse {
		invite: models::CreatedInvite {
			alias: body.alias.clone(),
			token,
		},
	})
}

// MARK: DELETE /parties/self/invites/{}
pub async fn revoke_party_invite(
	ctx: Ctx<Auth>,
	invite_id: Uuid,
) -> GlobalResult<models::RevokePartyInviteResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	let party_id = unwrap_with_owned!(
		utils::get_current_party(ctx.op_ctx(), current_user_id).await?,
		PARTY_IDENTITY_NOT_IN_ANY_PARTY
	);

	utils::assert_party_leader(ctx.op_ctx(), party_id, current_user_id).await?;

	// Revoke invite
	msg!([ctx] party::msg::invite_destroy(invite_id) -> party::msg::update(party_id) {
		invite_id: Some(invite_id.into()),
		..Default::default()
	})
	.await?;

	Ok(models::RevokePartyInviteResponse {})
}

// MARK: GET /invites
#[derive(Debug, Deserialize)]
pub struct GetPartyFromInviteQuery {
	token: Option<String>,
	alias: Option<String>,
}

pub async fn get_party_from_invite(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: GetPartyFromInviteQuery,
) -> GlobalResult<models::GetPartyFromInviteResponse> {
	let (current_user_id, game_user) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	utils::touch_user_presence(ctx.op_ctx().base(), current_user_id);

	let invite_id = if let Some(token) = query.token {
		let invite = rivet_claims::decode(&token)??.as_party_invite()?;
		invite.invite_id
	} else if let Some(alias) = query.alias {
		let namespace_id = if let Some(game_user) = &game_user {
			game_user.namespace_id
		} else {
			panic_with!(PARTY_UNEXPECTED_INVITE_ALIAS)
		};

		let lookup_res = op!([ctx] party_invite_alias_lookup {
			namespace_id: namespace_id,
			alias: alias.clone(),
		})
		.await?;
		if let Some(invite_id) = &lookup_res.invite_id {
			invite_id.as_uuid()
		} else {
			panic_with!(PARTY_INVITE_ALIAS_NOT_FOUND)
		}
	} else {
		panic_with!(API_BAD_QUERY, error = "One of token, alias, must be set.",);
	};

	let invite_res = op!([ctx] party_invite_get {
		invite_ids: vec![invite_id.into()],
	})
	.await?;
	let invite = unwrap_with_owned!(invite_res.invites.first(), PARTY_INVITE_NOT_FOUND);
	let party_id = internal_unwrap!(invite.party_id).as_uuid();

	let mut parties = fetch::party::summaries(&ctx, current_user_id, vec![party_id]).await?;
	let party = internal_unwrap_owned!(parties.pop());

	Ok(models::GetPartyFromInviteResponse { party })
}

// MARK: POST /parties/join
pub async fn join_party(
	ctx: Ctx<Auth>,
	body: models::JoinPartyRequest,
) -> GlobalResult<models::JoinPartyResponse> {
	let (current_user_id, game_user) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	// Find the party ID to join
	let party_id = match &body.invite {
		models::JoinPartyInvite::PartyId(party_id) => {
			let party_id = util::uuid::parse(party_id)?;

			// Get the party
			let get_res = op!([ctx] party_get {
				party_ids: vec![party_id.into()],
			})
			.await?;
			assert_with!(!get_res.parties.is_empty(), PARTY_PARTY_NOT_FOUND);

			let publicity_res = op!([ctx] party_publicity_for_user {
				user_id: Some(current_user_id.into()),
				party_ids: vec![party_id.into()],
			})
			.await?;
			let party = internal_unwrap_owned!(publicity_res.parties.first());
			let publicity = internal_unwrap_owned!(
				backend::party::party::PublicityLevel::from_i32(party.publicity)
			);

			assert_eq_with!(
				publicity,
				backend::party::party::PublicityLevel::Join,
				PARTY_PARTY_NOT_JOINABLE
			);

			party_id
		}
		models::JoinPartyInvite::Token(_) | models::JoinPartyInvite::Alias(_) => {
			// Find the invite ID to join
			let invite_id = match &body.invite {
				models::JoinPartyInvite::Token(token) => {
					let invite = rivet_claims::decode(token)??.as_party_invite()?;
					invite.invite_id
				}
				models::JoinPartyInvite::Alias(alias) => {
					let namespace_id = if let Some(game_user) = &game_user {
						game_user.namespace_id
					} else {
						panic_with!(PARTY_UNEXPECTED_INVITE_ALIAS)
					};

					let lookup_res = op!([ctx] party_invite_alias_lookup {
						namespace_id: namespace_id,
						alias: alias.clone(),
					})
					.await?;
					if let Some(invite_id) = &lookup_res.invite_id {
						invite_id.as_uuid()
					} else {
						panic_with!(PARTY_INVITE_ALIAS_NOT_FOUND)
					}
				}
				_ => unreachable!(),
			};

			// Consume the invite alias
			let query_id = Uuid::new_v4();
			let consume_res = msg!([ctx] party::msg::invite_consume(query_id) -> Result<party::msg::invite_consume_complete, party::msg::invite_consume_fail> {
				query_id: Some(query_id.into()),
				invite_id: Some(invite_id.into()),
			}).await?;

			match consume_res
				.map_err(|msg| party::msg::invite_consume_fail::ErrorCode::from_i32(msg.error_code))
			{
				Ok(msg) => internal_unwrap!(msg.party_id).as_uuid(),
				Err(Some(code)) => {
					use party::msg::invite_consume_fail::ErrorCode::*;

					match code {
						Unknown => internal_panic!("unknown party invite create error code"),
						InviteNotFound => {
							if matches!(body.invite, models::JoinPartyInvite::Alias(_)) {
								// If we receive this error with an alias, then it means there was a race
								// condition with the party getting destroyed
								panic_with!(PARTY_INVITE_ALIAS_NOT_FOUND)
							} else {
								panic_with!(PARTY_INVITE_NOT_FOUND)
							}
						}
					};
				}
				Err(None) => internal_panic!("failed to parse consume error code"),
			}
		}
	};

	let initial_state =
		if let Some(matchmaker_current_player_token) = body.matchmaker_current_player_token {
			let token = rivet_claims::decode(&matchmaker_current_player_token)??;
			let player_ent = token.as_matchmaker_player()?;

			// Player should always exist since the token is valid
			let player_get_res = op!([ctx] mm_player_get {
				player_ids: vec![player_ent.player_id.into()],
			})
			.await?;
			let player = internal_unwrap_owned!(player_get_res.players.first());

			Some(
				party::msg::member_create::message::InitialState::MatchmakerLobby(
					party::msg::member_create::message::StateMatchmakerLobby {
						player_id: player.player_id,
						player_token: matchmaker_current_player_token,
					},
				),
			)
		} else {
			Some(
				party::msg::member_create::message::InitialState::MatchmakerReady(
					party::msg::member_create::message::StateMatchmakerReady {},
				),
			)
		};

	// Create the party member
	let member_create_res = msg!([ctx] party::msg::member_create(party_id, current_user_id) -> Result<party::msg::member_create_complete, party::msg::member_create_fail> {
		party_id: Some(party_id.into()),
		user_id: Some(current_user_id.into()),
		initial_state: initial_state,
		client: Some(ctx.client_info()),
	}).await?;
	match member_create_res
		.map_err(|msg| party::msg::member_create_fail::ErrorCode::from_i32(msg.error_code))
	{
		Ok(_) => {}
		Err(Some(code)) => {
			use party::msg::member_create_fail::ErrorCode::*;

			match code {
				Unknown => internal_panic!("unknown party invite create error code"),
				PartyDoesNotExist => panic_with!(PARTY_PARTY_NOT_FOUND),
				PartyFull => panic_with!(PARTY_PARTY_FULL),
				AlreadyInParty => panic_with!(PARTY_ALREADY_IN_PARTY),
			};
		}
		Err(None) => internal_panic!("failed to parse member create error code"),
	};

	Ok(models::JoinPartyResponse {
		party_id: party_id.to_string(),
	})
}

// MARK: POST /parties/self/leave
pub async fn leave_party(
	ctx: Ctx<Auth>,
	_body: models::LeavePartyRequest,
) -> GlobalResult<models::LeavePartyResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	let party_id = unwrap_with_owned!(
		utils::get_current_party(ctx.op_ctx(), current_user_id).await?,
		PARTY_IDENTITY_NOT_IN_ANY_PARTY
	);

	msg!([ctx] party::msg::member_remove(party_id, current_user_id) -> party::msg::member_remove_complete {
		party_id: Some(party_id.into()),
		user_id: Some(current_user_id.into()),
		..Default::default()
	})
	.await?;

	Ok(models::LeavePartyResponse {})
}

// MARK: PUT /parties/self/publicity
pub async fn set_party_publicity(
	ctx: Ctx<Auth>,
	body: models::SetPartyPublicityRequest,
) -> GlobalResult<models::SetPartyPublicityResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	let party_id = unwrap_with_owned!(
		utils::get_current_party(ctx.op_ctx(), current_user_id).await?,
		PARTY_IDENTITY_NOT_IN_ANY_PARTY
	);

	utils::assert_party_leader(ctx.op_ctx(), party_id, current_user_id).await?;

	msg!([ctx] party::msg::publicity_set(party_id) {
		party_id: Some(party_id.into()),
		public: body.public.map(convert::party::publicity_level_to_proto),
		friends: body.mutual_followers.map(convert::party::publicity_level_to_proto),
		teams: body.groups.map(convert::party::publicity_level_to_proto),
	})
	.await?;

	Ok(models::SetPartyPublicityResponse {})
}

// MARK: POST /parties/self/members/{}/transfer-ownership
pub async fn transfer(
	ctx: Ctx<Auth>,
	new_leader_user_id: Uuid,
	_body: models::TransferPartyOwnershipRequest,
) -> GlobalResult<models::TransferPartyOwnershipResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	assert_ne_with!(
		current_user_id,
		new_leader_user_id,
		PARTY_CANNOT_SET_LEADER_TO_SELF
	);

	// Get party members
	let current_user_id_proto = Into::<common::Uuid>::into(current_user_id);
	let party_member_res = op!([ctx] party_member_get {
		user_ids: vec![current_user_id_proto, new_leader_user_id.into()],
	})
	.await?;

	// Get the party ID from the current member
	let current_party_member = unwrap_with_owned!(
		party_member_res
			.party_members
			.iter()
			.find(|x| x.user_id.as_ref() == Some(&current_user_id_proto)),
		PARTY_IDENTITY_NOT_IN_ANY_PARTY
	);
	let party_id = internal_unwrap!(current_party_member.party_id).as_uuid();

	utils::assert_party_leader(ctx.op_ctx(), party_id, current_user_id).await?;

	// Validate the new leader is in the same party
	let new_leader_user_id_proto = Into::<common::Uuid>::into(new_leader_user_id);
	let leader_party_member = unwrap_with_owned!(
		party_member_res
			.party_members
			.iter()
			.find(|x| x.user_id.as_ref() == Some(&new_leader_user_id_proto)),
		PARTY_LEADER_NOT_IN_SAME_PARTY
	);
	assert_eq_with!(
		party_id,
		internal_unwrap!(leader_party_member.party_id).as_uuid(),
		PARTY_LEADER_NOT_IN_SAME_PARTY
	);

	msg!([ctx] party::msg::leader_set(party_id) {
		party_id: Some(party_id.into()),
		leader_user_id: Some(new_leader_user_id.into()),
	})
	.await?;

	Ok(models::TransferPartyOwnershipResponse {})
}

// MARK: POST /parties/self/members/{}/kick
pub async fn kick(
	ctx: Ctx<Auth>,
	kick_user_id: Uuid,
	_body: models::KickMemberRequest,
) -> GlobalResult<models::KickMemberResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	assert_ne_with!(current_user_id, kick_user_id, PARTY_CANNOT_KICK_SELF);

	// Get party members
	let current_user_id_proto = Into::<common::Uuid>::into(current_user_id);
	let party_member_res = op!([ctx] party_member_get {
		user_ids: vec![current_user_id_proto, kick_user_id.into()],
	})
	.await?;

	// Get the party ID from the current member
	let current_party_member = unwrap_with_owned!(
		party_member_res
			.party_members
			.iter()
			.find(|x| x.user_id.as_ref() == Some(&current_user_id_proto)),
		PARTY_IDENTITY_NOT_IN_ANY_PARTY
	);
	let party_id = internal_unwrap!(current_party_member.party_id).as_uuid();

	utils::assert_party_leader(ctx.op_ctx(), party_id, current_user_id).await?;

	// Validate the new leader is in the same party
	let kick_user_id_proto = Into::<common::Uuid>::into(kick_user_id);
	let leader_party_member = unwrap_with_owned!(
		party_member_res
			.party_members
			.iter()
			.find(|x| x.user_id.as_ref() == Some(&kick_user_id_proto)),
		PARTY_LEADER_NOT_IN_SAME_PARTY
	);
	assert_eq_with!(
		party_id,
		internal_unwrap!(leader_party_member.party_id).as_uuid(),
		PARTY_LEADER_NOT_IN_SAME_PARTY
	);

	msg!([ctx] party::msg::member_kick(party_id, kick_user_id) {
		party_id: Some(party_id.into()),
		kick_user_id: Some(kick_user_id.into()),
	})
	.await?;

	Ok(models::KickMemberResponse {})
}

// MARK: POST /parties/{}/join-request/send
pub async fn send_join_request(
	ctx: Ctx<Auth>,
	party_id: Uuid,
	_body: models::SendJoinRequestRequest,
) -> GlobalResult<models::SendJoinRequestResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	// Get the party
	let party_get_res = op!([ctx] party_get {
		party_ids: vec![party_id.into()],
	})
	.await?;
	assert_with!(!party_get_res.parties.is_empty(), PARTY_PARTY_NOT_FOUND);

	let publicity_res = op!([ctx] party_publicity_for_user {
		user_id: Some(current_user_id.into()),
		party_ids: vec![party_id.into()],
	})
	.await?;
	let party = internal_unwrap_owned!(publicity_res.parties.first());
	let publicity = internal_unwrap_owned!(backend::party::party::PublicityLevel::from_i32(
		party.publicity
	));

	// User can't see party so it doesn't exist
	assert_ne_with!(
		publicity,
		backend::party::party::PublicityLevel::None,
		PARTY_PARTY_NOT_FOUND
	);

	// Send party invite message
	let chat_message_id = Uuid::new_v4();
	op!([ctx] chat_message_create_with_topic {
		chat_message_id: Some(chat_message_id.into()),
		topic: Some(backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Party(
				backend::chat::topic::Party {
					party_id: Some(party_id.into()),
				},
			)),
		}),
		send_ts: util::timestamp::now(),
		body: Some(backend::chat::MessageBody {
			kind: Some(backend::chat::message_body::Kind::PartyJoinRequest(
				backend::chat::message_body::PartyJoinRequest {
					sender_user_id: Some(current_user_id.into()),
				}
			)),
		}),
	})
	.await?;

	Ok(models::SendJoinRequestResponse {})
}
