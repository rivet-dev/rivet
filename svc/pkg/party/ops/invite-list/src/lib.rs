use proto::backend::pkg::*;

use redis_util::escape_search_query;
use rivet_operation::prelude::*;

#[operation(name = "party-invite-list")]
async fn handle(
	ctx: OperationContext<party::invite_list::Request>,
) -> GlobalResult<party::invite_list::Response> {
	// TODO:
	return Ok(party::invite_list::Response {
		invite_ids: Vec::new(),
	});

	let party_id = internal_unwrap!(ctx.party_id).as_uuid();

	let invite_ids = redis::cmd("FT.SEARCH")
		.arg("party-invite-idx")
		.arg(format!(
			"@party_id:{{{party_id}}}",
			party_id = escape_search_query(party_id)
		))
		.arg("RETURN")
		.arg(1)
		.arg("$.invite_id")
		.query_async::<_, redis_util::SearchResult>(&mut ctx.redis_party().await?)
		.await?
		.entries
		.into_iter()
		.map(|entry| {
			let data = internal_unwrap_owned!(entry.data.first());
			let invite_id = util::uuid::parse(&data.value)?;
			GlobalResult::Ok(common::Uuid::from(invite_id))
		})
		.collect::<GlobalResult<Vec<common::Uuid>>>()?;

	Ok(party::invite_list::Response { invite_ids })
}
