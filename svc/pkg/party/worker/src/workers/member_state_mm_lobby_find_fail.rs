use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use redis::AsyncCommands;
use redis_util::{escape_search_query, RedisResult};

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/member_state_mm_lobby_find_fail.lua"));
}

#[worker(name = "party-member-state-mm-lobby-find-fail")]
async fn worker(ctx: &OperationContext<mm::msg::lobby_find_fail::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let mut redis = ctx.redis_party().await?;

	let query_id = internal_unwrap!(ctx.query_id).as_uuid();

	// Fetch user ID
	let user_id = redis::cmd("FT.SEARCH")
		.arg("party-member-idx")
		.arg(format!(
			"@mm_direct_query_id:{{{query_id}}}",
			query_id = escape_search_query(query_id)
		))
		.arg("RETURN")
		.arg(1)
		.arg("$.user_id")
		.query_async::<_, redis_util::SearchResult>(&mut redis)
		.await?
		.entries
		.first()
		.map(|entry| {
			let data = internal_unwrap_owned!(entry.data.first());
			let user_id = util::uuid::parse(&data.value)?;
			GlobalResult::Ok(user_id)
		})
		.transpose()?;
	let user_id = if let Some(user_id) = user_id {
		user_id
	} else {
		tracing::info!("no matching party member");
		return Ok(());
	};

	// Set associated lobby
	let redis_result = REDIS_SCRIPT
		.arg(util::timestamp::now())
		.arg(query_id.to_string())
		.arg(serde_json::to_string(
			&util_party::key::party_member_config::State::MatchmakerReady {},
		)?)
		.key(util_party::key::party_member_config(user_id))
		.invoke_async::<_, RedisResult<String>>(&mut redis)
		.await?;
	let party_id = match redis_result.as_ref().map_err(String::as_str) {
		Ok(party_id) => util::uuid::parse(party_id)?,
		Err("PARTY_MEMBER_DOES_NOT_EXIST") => {
			tracing::info!("party member does not exist, likely removed in race condition");
			return Ok(());
		}
		Err("PARTY_MEMBER_IN_DIFFERENT_STATE") => {
			tracing::warn!("party member is not in finding lobby state");
			return Ok(());
		}
		Err("PARTY_MEMBER_HAS_DIFFERENT_QUERY") => {
			tracing::warn!("party member has a different query id");
			return Ok(());
		}
		Err(_) => internal_panic!("unknown redis error"),
	};

	msg!([ctx] party::msg::update(party_id) {
		party_id: Some(party_id.into()),
	})
	.await?;

	// The player's state will be re-resolved if a player leaves or party
	// finishes finding a new lobby.

	Ok(())
}
