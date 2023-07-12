use chirp_worker::prelude::*;
use futures_util::{StreamExt, TryStreamExt};
use proto::backend::{self, pkg::*};
use redis_util::{escape_search_query, RedisResult};

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/state_mm_lobby_find_complete.lua"));
}

struct JoiningPartyMember {
	user_id: Uuid,
	player_id: Uuid,
	player_token: String,
}

#[worker(name = "party-state-mm-lobby-find-complete")]
async fn worker(ctx: &OperationContext<mm::msg::lobby_find_complete::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let mut redis = ctx.redis_party().await?;

	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();
	let query_id = internal_unwrap!(ctx.query_id).as_uuid();
	let lobby_id = internal_unwrap!(ctx.lobby_id).as_uuid();
	let player_ids = ctx
		.player_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Fetch party ID
	let search_query = format!(
		"@mm_query_id:{{{query_id}}}",
		query_id = escape_search_query(query_id),
	);
	let party_id = redis::cmd("FT.SEARCH")
		.arg("party-idx")
		.arg(search_query)
		.arg("RETURN")
		.arg(1)
		.arg("$.party_id")
		.query_async::<_, redis_util::SearchResult>(&mut redis)
		.await?
		.entries
		.first()
		.map(|entry| {
			let data = internal_unwrap_owned!(entry.data.first());
			let party_id = util::uuid::parse(&data.value)?;
			GlobalResult::Ok(party_id)
		})
		.transpose()?;
	let party_id = if let Some(party_id) = party_id {
		party_id
	} else {
		tracing::info!("no matching party");
		return Ok(());
	};

	// Set associated lobby
	let redis_result = REDIS_SCRIPT
		.arg(util::timestamp::now())
		.arg(party_id.to_string())
		.arg(query_id.to_string())
		.arg(serde_json::to_string(
			&util_party::key::party_config::State::MatchmakerLobby {
				namespace_id,
				lobby_id,
			},
		)?)
		.key(util_party::key::party_config(party_id))
		.invoke_async::<_, RedisResult<Vec<Vec<String>>>>(&mut redis)
		.await?;
	let joining_party_members = match redis_result.as_ref().map_err(String::as_str) {
		Ok(joining_party_members) => {
			tracing::info!(joining_member_count = ?joining_party_members.len(), "joining party members");
			joining_party_members
				.iter()
				.map(|row| -> GlobalResult<JoiningPartyMember> {
					internal_assert_eq!(3, row.len());
					let mut iter = row.iter();

					let user_id = internal_unwrap_owned!(iter.next());
					let player_id = internal_unwrap_owned!(iter.next());
					let player_token = internal_unwrap_owned!(iter.next());
					Ok(JoiningPartyMember {
						user_id: util::uuid::parse(user_id)?,
						player_id: util::uuid::parse(player_id)?,
						player_token: player_token.clone(),
					})
				})
				.collect::<GlobalResult<Vec<_>>>()?
		}
		Err("PARTY_DOES_NOT_EXIST") => {
			tracing::info!("party does not exist, likely removed in race condition");
			return Ok(());
		}
		Err("PARTY_IN_DIFFERENT_STATE") => {
			tracing::info!("party in different state, likely race condition");
			return Ok(());
		}
		Err("PARTY_HAS_DIFFERENT_QUERY") => {
			// TODO: Invalidate players, etc.
			tracing::warn!("party has a different query id, removing players");

			// Fetch all party members
			let party_res = op!([ctx] party_member_list {
				party_ids: vec![party_id.into()],
			})
			.await?;
			let party = internal_unwrap_owned!(party_res.parties.first());
			let party_members_res = op!([ctx] party_member_get {
				user_ids: party.user_ids.clone(),
			})
			.await?;

			// Remove all created players
			futures_util::stream::iter(
				party_members_res
					.party_members
					.iter()
					.cloned()
					.filter_map(|party_member| match party_member.state {
						Some(backend::party::party_member::State::MatchmakerFindingLobby(
							backend::party::party_member::StateMatchmakerFindingLobby {
								player_id: Some(player_id),
								..
							},
						)) => Some(player_id),
						_ => None,
					})
					.map(|player_id| async move {
						let res = msg!([ctx] mm::msg::player_remove(player_id) ->
							Result<mm::msg::player_remove_complete, mm::msg::player_remove_fail>
						{
							player_id: Some(player_id),
							lobby_id: Some(lobby_id.into()),
							..Default::default()
						})
						.await?;

						match res {
							Ok(_) => {}
							Err(msg) => {
								use mm::msg::player_remove_fail::ErrorCode;

								match ErrorCode::from_i32(msg.error_code) {
									Some(ErrorCode::DeprecatedPlayerNotFound) => {
										tracing::warn!("player not found");
									}
									Some(ErrorCode::PlayerInDifferentLobby) => {
										tracing::warn!("player in different lobby");
									}
									Some(ErrorCode::Unknown) | None => {
										tracing::error!("unknown player remove error {:?}", msg);
									}
								}
							}
						};

						GlobalResult::Ok(())
					}),
			)
			.buffer_unordered(32)
			.try_collect::<Vec<_>>()
			.await?;

			return Ok(());
		}
		Err(_) => internal_panic!("unknown redis error"),
	};

	// Validate player IDs are in find query
	for party_member in &joining_party_members {
		// TODO: Should any cleanup be done if this assertion fails?
		internal_assert!(
			player_ids.contains(&party_member.player_id),
			"find query does not contain party member's player id"
		);
	}

	// Publish join messages
	for party_member in joining_party_members {
		msg!([ctx] user::msg::mm_lobby_join(party_member.user_id, lobby_id) {
			user_id: Some(party_member.user_id.into()),
			namespace_id: ctx.namespace_id,
			query_id: ctx.query_id,
			lobby_id: ctx.lobby_id,
			player_id: Some(party_member.player_id.into()),
			player_token: party_member.player_token,
		})
		.await?;
	}

	msg!([ctx] party::msg::update(party_id) {
		party_id: Some(party_id.into()),
	})
	.await?;

	// Send party activity change message
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
			kind: Some(backend::chat::message_body::Kind::PartyActivityChange(
				backend::chat::message_body::PartyActivityChange {
					state: Some(backend::chat::message_body::party_activity_change::State::MatchmakerLobby(
						backend::party::party::StateMatchmakerLobby {
							namespace_id: Some(namespace_id.into()),
							lobby_id: Some(lobby_id.into()),
						},
					)),
				}
			)),
		}),
	})
	.await?;

	Ok(())
}
