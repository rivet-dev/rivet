use chirp_worker::prelude::*;
use proto::backend::pkg::*;

async fn fail(
	client: &chirp_client::Client,
	query_id: Uuid,
	error_code: party::msg::invite_consume_fail::ErrorCode,
) -> GlobalResult<()> {
	msg!([client] party::msg::invite_consume_fail(query_id) {
		query_id: Some(query_id.into()),
		error_code: error_code as i32,
	})
	.await?;
	Ok(())
}

#[worker(name = "party-invite-consume")]
async fn worker(ctx: &OperationContext<party::msg::invite_consume::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let mut redis = ctx.redis_party().await?;

	let query_id = internal_unwrap!(ctx.query_id).as_uuid();
	let invite_id = internal_unwrap!(ctx.invite_id).as_uuid();

	// Look up the invite information
	let party_id = if let Some(party_id) = redis::cmd("JSON.RESP")
		.arg(util_party::key::party_invite_config(invite_id))
		.arg("party_id")
		.query_async::<_, Option<String>>(&mut redis)
		.await?
	{
		util::uuid::parse(&party_id)?
	} else {
		return fail(
			ctx.chirp(),
			query_id,
			party::msg::invite_consume_fail::ErrorCode::InviteNotFound,
		)
		.await;
	};

	msg!([ctx] party::msg::invite_consume_complete(query_id) {
		query_id: Some(query_id.into()),
		party_id: Some(party_id.into()),
	})
	.await?;

	Ok(())
}
