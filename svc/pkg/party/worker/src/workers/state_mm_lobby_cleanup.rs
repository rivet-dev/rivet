use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use redis_util::escape_search_query;

#[worker(name = "party-state-mm-lobby-cleanup")]
async fn worker(ctx: &OperationContext<mm::msg::lobby_cleanup::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let mut redis = ctx.redis_party().await?;

	let lobby_id = internal_unwrap!(ctx.lobby_id).as_uuid();

	let party_ids = redis::cmd("FT.SEARCH")
		.arg("party-idx")
		.arg(format!(
			"@mm_lobby_id:{{{lobby_id}}}",
			lobby_id = escape_search_query(lobby_id)
		))
		.arg("RETURN")
		.arg(1)
		.arg("$.party_id")
		.query_async::<_, redis_util::SearchResult>(&mut redis)
		.await?
		.entries
		.into_iter()
		.map(|entry| {
			let data = internal_unwrap_owned!(entry.data.first());
			let party_id = util::uuid::parse(&data.value)?;
			GlobalResult::Ok(party_id)
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	for party_id in party_ids {
		msg!([ctx] party::msg::state_set_idle(party_id) {
			party_id: Some(party_id.into()),
		})
		.await?;
	}

	Ok(())
}
