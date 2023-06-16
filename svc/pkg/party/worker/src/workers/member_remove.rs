use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/member_remove.lua"));
}

#[worker(name = "party-member-remove")]
async fn worker(ctx: OperationContext<party::msg::member_remove::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let user_id = internal_unwrap!(ctx.user_id).as_uuid();
	let party_id = internal_unwrap!(ctx.party_id).as_uuid();

	let redis_res = {
		use util_party::key;

		REDIS_SCRIPT
			.arg(party_id.to_string())
			.arg(user_id.to_string())
			.arg(ctx.skip_delete)
			.arg(ctx.skip_party_cleanup)
			.key(key::party_config(party_id))
			.key(key::party_member_config(user_id))
			.invoke_async::<_, redis_util::RedisResult<(bool, bool, bool)>>(
				&mut ctx.redis_party().await?,
			)
			.await?
	};
	tracing::info!(?redis_res, "remove member res");

	match redis_res.as_ref().map_err(String::as_str) {
		Ok((member_updated, remove_party, set_party_leader)) => {
			tokio::try_join!(
				async {
					if !ctx.skip_party_updated {
						msg!([ctx] party::msg::update(party_id) {
							party_id: Some(party_id.into()),
						})
						.await?;
					}

					Ok(())
				},
				async {
					if *member_updated {
						msg!([ctx] party::msg::member_update(user_id) {
							user_id: Some(user_id.into()),
						})
						.await?;
					}
					Ok(())
				},
				async {
					if *remove_party {
						msg!([ctx] party::msg::destroy(party_id) {
							party_id: Some(party_id.into()),
						})
						.await?;
					}
					Ok(())
				},
				async {
					if *set_party_leader {
						msg!([ctx] party::msg::leader_set(party_id) {
							party_id: Some(party_id.into()),
							leader_user_id: None,
						})
						.await?;
					}

					Ok(())
				},
				async {
					// Send party member leave message
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
							kind: Some(backend::chat::message_body::Kind::PartyLeave(backend::chat::message_body::PartyLeave {
								user_id: Some(user_id.into()),
							})),
						}),
					})
					.await
					.map_err(Into::<GlobalError>::into)
				}
			)?;

			msg!([ctx] party::msg::member_remove_complete(party_id, user_id) {
				party_id: Some(party_id.into()),
				user_id: Some(user_id.into()),
			})
			.await?;
		}
		Err("PARTY_MEMBER_DOES_NOT_EXIST") => {
			tracing::info!("party member does not exist, likely removed in race condition");
			return Ok(());
		}
		Err(_) => {
			internal_panic!("unknown redis error")
		}
	}

	Ok(())
}
