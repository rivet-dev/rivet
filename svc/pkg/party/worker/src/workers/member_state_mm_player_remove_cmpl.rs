use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use redis_util::escape_search_query;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/member_state_mm_player_remove_cmpl.lua"));
}

#[worker(name = "party-member-state-mm-player-remove-cmpl")]
async fn worker(
	ctx: OperationContext<mm::msg::player_remove_complete::Message>,
) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let mut redis = ctx.redis_party().await?;

	let player_id = internal_unwrap!(ctx.player_id).as_uuid();

	// Only listen to events not including lobby destroy.
	//
	// The party member states will be set to idle in
	// party-state-mm-lobby-cleanup.
	if ctx.from_lobby_destroy {
		tracing::info!("coming from lobby destroy");
		return Ok(());
	}

	// Get the player
	let player_res = op!([ctx] mm_player_get {
		player_ids: vec![player_id.into()],
	})
	.await?;
	let player = internal_unwrap_owned!(player_res.players.first(), "player not found");
	let lobby_id = internal_unwrap!(player.lobby_id).as_uuid();

	// Find the party member associated with this player
	let search_query = format!(
		"@mm_player_id:{{{player_id}}}",
		player_id = escape_search_query(player_id)
	);
	let user_id = redis::cmd("FT.SEARCH")
		.arg("party-member-idx")
		.arg(search_query)
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
		tracing::info!(?player_id, "party member not found for player id");
		return Ok(());
	};

	let redis_res = REDIS_SCRIPT
		.arg(util::timestamp::now())
		.arg(lobby_id.to_string())
		.arg(player_id.to_string())
		.arg(serde_json::to_string(
			&util_party::key::party_member_config::State::MatchmakerReady {},
		)?)
		.key(util_party::key::party_member_config(user_id))
		.invoke_async::<_, redis_util::RedisResult<(Option<String>, Option<String>, Option<String>)>>(
			&mut redis,
		)
		.await?;
	match redis_res.as_ref().map_err(String::as_str) {
		Ok((party_id, pending_user_id, pending_user_party_id)) => {
			if let Some(party_id) = party_id {
				let party_id = util::uuid::parse(party_id)?;

				tracing::info!(?party_id, "player removed from party member in party");
				msg!([ctx] party::msg::update(party_id) {
					party_id: Some(party_id.into()),
				})
				.await?;
			} else {
				tracing::info!("could not find party member to remove from");
			}

			// Resolve the party member
			if let (Some(pending_user_id), Some(pending_user_party_id)) =
				(pending_user_id, pending_user_party_id)
			{
				let pending_user_id = util::uuid::parse(pending_user_id)?;
				let pending_user_party_id = util::uuid::parse(pending_user_party_id)?;

				tracing::info!(
					?pending_user_id,
					?pending_user_party_id,
					"found pending user"
				);
				msg!([ctx] party::msg::member_state_resolve(pending_user_party_id, pending_user_id) {
					party_id: Some(pending_user_party_id.into()),
					user_id: Some(pending_user_id.into()),
				})
				.await?;
			} else {
				tracing::info!("no pending user found");
			}
		}
		Err(_) => {
			internal_panic!("unknown redis error")
		}
	}

	Ok(())
}
