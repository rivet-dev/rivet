use proto::backend::pkg::*;
use redis_util::escape_search_query;
use rivet_operation::prelude::*;

#[operation(name = "party-invite-alias-lookup")]
async fn handle(
	ctx: OperationContext<party::invite_alias_lookup::Request>,
) -> GlobalResult<party::invite_alias_lookup::Response> {
	// TODO:
	return Ok(party::invite_alias_lookup::Response { invite_id: None });

	let mut redis = ctx.redis_party().await?;

	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();

	let invite_id = redis::cmd("FT.SEARCH")
		.arg("party-invite-idx")
		.arg(format!(
			"(@alias_namespace_id:{{{namespace_id}}}) (@alias:{{{alias}}})",
			namespace_id = escape_search_query(namespace_id),
			alias = escape_search_query(&ctx.alias),
		))
		.arg("RETURN")
		.arg(1)
		.arg("$.invite_id")
		.query_async::<_, redis_util::SearchResult>(&mut redis)
		.await?
		.entries
		.into_iter()
		.next()
		.map(|entry| {
			let data = internal_unwrap_owned!(entry.data.first());
			let invite_id = util::uuid::parse(&data.value)?;
			GlobalResult::Ok(invite_id)
		})
		.transpose()?;

	Ok(party::invite_alias_lookup::Response {
		invite_id: invite_id.map(Into::into),
	})
}
