use proto::backend::pkg::*;
use redis_util::escape_search_query;
use rivet_operation::prelude::*;

#[operation(name = "party-member-list")]
async fn handle(
	ctx: OperationContext<party::member_list::Request>,
) -> GlobalResult<party::member_list::Response> {
	// TODO:
	return Ok(party::member_list::Response {
		parties: Vec::new(),
	});

	let party_ids = ctx
		.party_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Query all parties
	let mut pipe = redis::pipe();
	for &party_id in &party_ids {
		pipe.cmd("FT.SEARCH")
			.arg("party-member-idx")
			.arg(format!(
				"@party_id:{{{party_id}}}",
				party_id = escape_search_query(party_id)
			))
			.arg("RETURN")
			.arg(1)
			.arg("$.user_id");
	}
	let res = pipe
		.query_async::<_, Vec<redis_util::SearchResult>>(&mut ctx.redis_party().await?)
		.await?;

	// Build response
	let parties = res
		.iter()
		.zip(party_ids.iter())
		.map(|(res, &party_id)| {
			let user_ids = res
				.entries
				.iter()
				.map(|entry| {
					let data = internal_unwrap_owned!(entry.data.first());
					let user_id = util::uuid::parse(&data.value)?;
					GlobalResult::Ok(common::Uuid::from(user_id))
				})
				.collect::<GlobalResult<Vec<common::Uuid>>>()?;
			GlobalResult::Ok(party::member_list::response::Party {
				party_id: Some(party_id.into()),
				user_ids,
			})
		})
		.collect::<GlobalResult<Vec<party::member_list::response::Party>>>()?;

	Ok(party::member_list::Response { parties })
}
