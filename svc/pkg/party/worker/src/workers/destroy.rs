use chirp_worker::prelude::*;
use futures_util::{
	stream::{StreamExt, TryStreamExt},
	FutureExt,
};
use proto::backend::pkg::*;
use redis::AsyncCommands;
use redis_util::{escape_search_query, SearchResult};

#[worker(name = "party-destroy")]
async fn worker(ctx: OperationContext<party::msg::destroy::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let mut redis = ctx.redis_party().await?;

	let party_id = internal_unwrap!(ctx.party_id).as_uuid();

	let mut pipe = redis::pipe();
	pipe.cmd("FT.SEARCH")
		.arg("party-member-idx")
		.arg(format!(
			"@party_id:{{{party_id}}}",
			party_id = escape_search_query(party_id)
		))
		.arg("RETURN")
		.arg(1)
		.arg("$.user_id");
	pipe.cmd("FT.SEARCH")
		.arg("party-invite-idx")
		.arg(format!(
			"@party_id:{{{party_id}}}",
			party_id = escape_search_query(party_id)
		))
		.arg("RETURN")
		.arg(1)
		.arg("$.invite_id");
	pipe.unlink(util_party::key::party_config(party_id))
		.ignore();
	let (party_members, party_invites) = pipe
		.query_async::<_, (SearchResult, SearchResult)>(&mut redis)
		.await?;
	tracing::info!(?party_members, ?party_invites, "removed party");

	msg!([ctx] party::msg::update(party_id) {
		party_id: Some(party_id.into()),
	})
	.await?;

	let events =
		// Clean up party members
		(
			party_members
				.entries
				.iter()
				.map(|entry| {
					let user_id = util::uuid::parse(&internal_unwrap_owned!(entry.data.first()).value)?;

					Ok(msg!([ctx] @wait party::msg::member_remove(party_id, user_id) {
						party_id: Some(party_id.into()),
						user_id: Some(user_id.into()),
						skip_party_cleanup: true,
						skip_party_updated: true,
						..Default::default()
					})
					.boxed())
				})
		)
		// Clean up invites
		.chain(party_invites.entries.iter().map(|entry| {
			let invite_id = util::uuid::parse(&internal_unwrap_owned!(entry.data.first()).value)?;

			Ok(msg!([ctx] @wait party::msg::invite_destroy(invite_id) {
				invite_id: Some(invite_id.into()),
				skip_party_updated: true,
			})
			.boxed())
		}))
		.collect::<GlobalResult<Vec<_>>>()?;

	// Dispatch events
	futures_util::stream::iter(events)
		.buffer_unordered(32)
		.try_collect::<Vec<_>>()
		.await?;

	msg!([ctx] party::msg::destroy_complete(party_id) {
		party_id: Some(party_id.into()),
	})
	.await?;

	Ok(())
}
