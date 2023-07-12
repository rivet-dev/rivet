use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "party-invite-destroy")]
async fn worker(ctx: &OperationContext<party::msg::invite_destroy::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let invite_id = internal_unwrap!(ctx.invite_id).as_uuid();

	// Delete Redis data
	let invite_config_key = util_party::key::party_invite_config(invite_id);
	let (party_id,) = redis::pipe()
		.cmd("JSON.RESP")
		.arg(&invite_config_key)
		.arg("party_id")
		.unlink(&invite_config_key)
		.ignore()
		.query_async::<_, (Option<String>,)>(&mut ctx.redis_party().await?)
		.await?;

	if let Some(party_id) = party_id {
		let party_id = util::uuid::parse(&party_id)?;

		if !ctx.skip_party_updated {
			msg!([ctx] party::msg::update(party_id) {
				party_id: Some(party_id.into()),
			})
			.await?;
		}
	} else {
		tracing::info!("no party id found");
	}

	Ok(())
}
