use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use redis_util::{escape_search_query, RedisResult};

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/member_state_mm_lobby_find_complete.lua"));
}

#[worker(name = "party-member-state-mm-lobby-find-complete")]
async fn worker(ctx: OperationContext<mm::msg::lobby_find_complete::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let mut redis = ctx.redis_party().await?;

	let query_id = internal_unwrap!(ctx.query_id).as_uuid();
	let lobby_id = internal_unwrap!(ctx.lobby_id).as_uuid();
	let player_ids = ctx
		.player_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Fetch party member data
	let search_query = format!(
		"@mm_direct_query_id:{{{query_id}}}",
		query_id = escape_search_query(query_id)
	);
	let party_member_search = redis::cmd("FT.SEARCH")
		.arg("party-member-idx")
		.arg(search_query)
		.arg("RETURN")
		.arg(4)
		.arg("$.user_id")
		.arg("$.party_id")
		.arg("$.state.matchmaker_finding_lobby_direct.player_id")
		.arg("$.state.matchmaker_finding_lobby_direct.player_token")
		.query_async::<_, redis_util::SearchResult>(&mut redis)
		.await?;
	let (user_id, party_id, new_party_member_state) = if let Some(entry) =
		party_member_search.entries.first()
	{
		let mut data = entry.data.iter();

		let user_id = util::uuid::parse(&internal_unwrap!(data.next()).value)?;
		let party_id = util::uuid::parse(&internal_unwrap!(data.next()).value)?;
		let player_id = util::uuid::parse(&internal_unwrap!(data.next()).value)?;
		let player_token = internal_unwrap!(data.next()).value.clone();

		let new_party_member_state = util_party::key::party_member_config::State::MatchmakerLobby {
			player_id,
			player_token,
		};

		(user_id, party_id, new_party_member_state)
	} else {
		tracing::info!("no matching party member with direct query");
		return Ok(());
	};

	// Set associated lobby
	let redis_result = REDIS_SCRIPT
		.arg(util::timestamp::now())
		.arg(party_id.to_string())
		.arg(query_id.to_string())
		.arg(lobby_id.to_string())
		.arg(serde_json::to_string(&new_party_member_state)?)
		.key(util_party::key::party_config(party_id))
		.key(util_party::key::party_member_config(user_id))
		.invoke_async::<_, RedisResult<(String, String, String)>>(&mut redis)
		.await?;
	let (party_id, player_id, player_token) = match redis_result.as_ref().map_err(String::as_str) {
		Ok((party_id, player_id, player_token)) => (
			util::uuid::parse(party_id)?,
			util::uuid::parse(player_id)?,
			player_token.clone(),
		),
		Err("PARTY_MEMBER_DOES_NOT_EXIST") => {
			tracing::info!("party member does not exist, likely removed in race condition");
			return Ok(());
		}
		Err("PARTY_MEMBER_NOT_IN_PARTY") => {
			tracing::info!("party member not in party, likely changed in race condition");
			return Ok(());
		}
		Err("PARTY_MEMBER_NOT_FINDING_LOBBY_DIRECT") => {
			tracing::info!("party member in different state, likely changed in race condition");
			return Ok(());
		}
		Err("PARTY_MEMBER_HAS_DIFFERENT_QUERY") => {
			tracing::warn!("party member has a different query id");
			return Ok(());
		}
		Err("PARTY_NOT_IN_LOBBY") => {
			tracing::warn!("party is not in a lobby");

			// Re-resolve state
			msg!([ctx] party::msg::member_state_set_mm_pending(party_id, user_id) {
				party_id: Some(party_id.into()),
				user_id: Some(user_id.into()),
			})
			.await?;

			return Ok(());
		}
		Err("PARTY_IN_DIFFERENT_LOBBY") => {
			tracing::warn!("party in different lobby, the party member's state should not still be finding lobby direct");

			// Re-resolve state
			msg!([ctx] party::msg::member_state_set_mm_pending(party_id, user_id) {
				party_id: Some(party_id.into()),
				user_id: Some(user_id.into()),
			})
			.await?;

			return Ok(());
		}
		Err(_) => internal_panic!("unknown redis error"),
	};
	internal_assert!(
		player_ids.contains(&player_id),
		"find query does not contain party member's player id"
	);

	msg!([ctx] user::msg::mm_lobby_join(user_id, lobby_id) {
		user_id: Some(user_id.into()),
		namespace_id: ctx.namespace_id,
		query_id: ctx.query_id,
		lobby_id: ctx.lobby_id,
		player_id: Some(player_id.into()),
		player_token: player_token,
	})
	.await?;

	msg!([ctx] party::msg::update(party_id) {
		party_id: Some(party_id.into()),
	})
	.await?;

	Ok(())
}
