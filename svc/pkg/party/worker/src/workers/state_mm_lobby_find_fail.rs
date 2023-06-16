use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use redis_util::escape_search_query;

#[worker(name = "party-state-mm-lobby-find-fail")]
async fn worker(ctx: OperationContext<mm::msg::lobby_find_fail::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let mut redis = ctx.redis_party().await?;

	let query_id = internal_unwrap!(ctx.query_id).as_uuid();

	// Fetch party ID
	let party_id = redis::cmd("FT.SEARCH")
		.arg("party-idx")
		.arg(format!(
			"@mm_query_id:{{{query_id}}}",
			query_id = escape_search_query(query_id)
		))
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
		tracing::info!("no matching party, likely race condition");
		return Ok(());
	};

	// Remove the party state
	msg!([ctx] party::msg::state_set_idle(party_id) {
		party_id: Some(party_id.into()),
	})
	.await?;

	Ok(())
}
