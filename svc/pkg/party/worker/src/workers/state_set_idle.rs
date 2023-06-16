use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/state_set_idle.lua"));
}

#[worker(name = "party-state-set-idle")]
async fn worker(ctx: OperationContext<party::msg::state_set_idle::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let party_id = internal_unwrap!(ctx.party_id).as_uuid();

	{
		use util_party::key;

		REDIS_SCRIPT
			.arg(util::timestamp::now())
			.arg(party_id.to_string())
			.arg(serde_json::to_string(
				&util_party::key::party_config::State::Idle {},
			)?)
			.arg(serde_json::to_string(
				&util_party::key::party_member_config::State::Inactive {},
			)?)
			.arg(serde_json::to_string(
				&util_party::key::party_member_config::State::MatchmakerReady {},
			)?)
			.key(key::party_config(party_id))
			.invoke_async(&mut ctx.redis_party().await?)
			.await?;
	}

	msg!([ctx] party::msg::update(party_id) {
		party_id: Some(party_id.into()),
	})
	.await?;

	// Send party activity change message
	let chat_message_id = Uuid::new_v4();
	op!([ctx] chat_message_create_with_topic {
		chat_message_id: Some(chat_message_id.into()),
		topic: Some(backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Party(
				backend::chat::topic::Party {
					party_id: Some(party_id.into()),
				},
			)),
		}),
		send_ts: util::timestamp::now(),
		body: Some(backend::chat::MessageBody {
			kind: Some(backend::chat::message_body::Kind::PartyActivityChange(
				backend::chat::message_body::PartyActivityChange {
					state: None,
				}
			)),
		}),
	})
	.await?;

	Ok(())
}
