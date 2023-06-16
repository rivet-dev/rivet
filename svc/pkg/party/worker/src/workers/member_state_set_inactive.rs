use chirp_worker::prelude::*;
use proto::backend::pkg::*;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/member_state_set_inactive.lua"));
}

#[worker(name = "party-member-state-set-inactive")]
async fn worker(
	ctx: OperationContext<party::msg::member_state_set_inactive::Message>,
) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let mut redis = ctx.redis_party().await?;

	let user_id = internal_unwrap!(ctx.user_id).as_uuid();
	let party_id = internal_unwrap!(ctx.party_id).as_uuid();

	let updated = REDIS_SCRIPT
		.arg(util::timestamp::now())
		.arg(party_id.to_string())
		.arg(serde_json::to_string(
			&util_party::key::party_member_config::State::Inactive {},
		)?)
		.key(util_party::key::party_member_config(user_id))
		.invoke_async::<_, bool>(&mut redis)
		.await?;
	if updated {
		msg!([ctx] party::msg::update(party_id) {
			party_id: Some(party_id.into()),
		})
		.await?;
	}

	Ok(())
}
