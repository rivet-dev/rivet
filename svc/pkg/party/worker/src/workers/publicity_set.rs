use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use util_party::key::party_config::PublicityLevel;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/publicity_set.lua"));
}

#[worker(name = "party-publicity-set")]
async fn worker(ctx: OperationContext<party::msg::publicity_set::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let party_id = internal_unwrap!(ctx.party_id).as_uuid();

	let redis_res = REDIS_SCRIPT
		.arg(party_id.to_string())
		.arg(
			ctx.public
				.map(|v| serde_json::to_string(&convert_publicity(v)))
				.transpose()?
				.unwrap_or_default(),
		)
		.arg(
			ctx.friends
				.map(|v| serde_json::to_string(&convert_publicity(v)))
				.transpose()?
				.unwrap_or_default(),
		)
		.arg(
			ctx.teams
				.map(|v| serde_json::to_string(&convert_publicity(v)))
				.transpose()?
				.unwrap_or_default(),
		)
		.key(util_party::key::party_config(party_id))
		.invoke_async::<_, redis_util::RedisResult<redis::Value>>(&mut ctx.redis_party().await?)
		.await?;

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
		Err(_) => {
			internal_panic!("unknown redis error")
		}
	}

	Ok(())
}

fn convert_publicity(level: i32) -> PublicityLevel {
	match backend::party::party::PublicityLevel::from_i32(level) {
		None | Some(backend::party::party::PublicityLevel::None) => PublicityLevel::None,
		Some(backend::party::party::PublicityLevel::View) => PublicityLevel::View,
		Some(backend::party::party::PublicityLevel::Join) => PublicityLevel::Join,
	}
}
