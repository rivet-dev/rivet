use chirp_worker::prelude::*;
use proto::backend::pkg::*;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/leader_set.lua"));
}

#[worker(name = "party-leader-set")]
async fn worker(ctx: &OperationContext<party::msg::leader_set::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let party_id = internal_unwrap!(ctx.party_id).as_uuid();
	let leader_user_id = ctx.leader_user_id.as_ref().map(|x| x.as_uuid());

	let redis_res = {
		use util_party::key;

		REDIS_SCRIPT
			.arg(party_id.to_string())
			.arg(
				leader_user_id
					.as_ref()
					.map(Uuid::to_string)
					.unwrap_or_default(),
			)
			.key(key::party_config(party_id))
			.invoke_async::<_, redis_util::RedisResult<redis::Value>>(&mut ctx.redis_party().await?)
			.await?
	};
	tracing::info!(?redis_res, "set leader res");

	match redis_res.as_ref().map_err(String::as_str) {
		Ok(_) => {
			msg!([ctx] party::msg::update(party_id) {
				party_id: Some(party_id.into()),
			})
			.await?;
		}
		Err("PARTY_DOES_NOT_EXIST") => {
			tracing::warn!("party does not exist, likely a race condition");
		}
		Err("COULD_NOT_DECIDE_LEADER") => {
			tracing::error!("could not decide leader");
		}
		Err("USER_NOT_PARTY_MEMBER") => {
			tracing::warn!("user not a party member, likely a race condition");
		}
		Err(_) => {
			internal_panic!("unknown redis error")
		}
	}

	Ok(())
}
