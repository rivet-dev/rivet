use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/member_create.lua"));
}

async fn fail(
	client: &chirp_client::Client,
	party_id: Uuid,
	user_id: Uuid,
	error_code: party::msg::member_create_fail::ErrorCode,
) -> GlobalResult<()> {
	msg!([client] party::msg::member_create_fail(party_id, user_id) {
		party_id: Some(party_id.into()),
		user_id: Some(user_id.into()),
		error_code: error_code as i32,
	})
	.await?;
	Ok(())
}

#[worker(name = "party-member-create")]
async fn worker(ctx: OperationContext<party::msg::member_create::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let mut redis = ctx.redis_party().await?;

	let party_id = internal_unwrap!(ctx.party_id).as_uuid();
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	let redis_res = {
		use util_party::key;

		let state = match &ctx.initial_state {
			None => key::party_member_config::State::Inactive {},
			Some(party::msg::member_create::message::InitialState::MatchmakerReady(_)) => {
				key::party_member_config::State::MatchmakerReady {}
			}
			Some(party::msg::member_create::message::InitialState::MatchmakerLobby(state)) => {
				key::party_member_config::State::MatchmakerLobby {
					player_id: internal_unwrap!(state.player_id).as_uuid(),
					player_token: state.player_token.clone(),
				}
			}
		};

		let config = key::party_member_config::Config {
			party_id,
			user_id,
			create_ts: ctx.ts(),
			state_change_ts: util::timestamp::now(),
			state,
			client_info: ctx
				.client
				.as_ref()
				.map(|client| key::party_member_config::ClientInfo {
					user_agent: client.user_agent.clone(),
					remote_address: client.remote_address.clone(),
				}),
		};

		let exists = redis
			.exists::<_, bool>(key::party_member_config(user_id))
			.await?;
		tracing::info!(exists = ?exists, "does party member exist");

		REDIS_SCRIPT
			.arg(ctx.ts())
			.arg(party_id.to_string())
			.arg(user_id.to_string())
			.arg(serde_json::to_string(&config)?)
			.key(key::party_config(party_id))
			.key(key::party_member_config(user_id))
			.invoke_async::<_, redis_util::RedisResult<(String,)>>(&mut redis)
			.await?
	};
	tracing::info!(?redis_res, "create party member res");

	match redis_res.as_ref().map_err(String::as_str) {
		Ok((old_party_id,)) => {
			tokio::try_join!(
				async {
					// Publish remove member message from old party
					if !old_party_id.is_empty() {
						let old_party_id = util::uuid::parse(old_party_id)?;
						msg!([ctx] party::msg::member_remove(old_party_id, user_id) {
							party_id: Some(old_party_id.into()),
							user_id: Some(user_id.into()),
							skip_delete: true,
							..Default::default()
						})
						.await?;
					}

					Ok(())
				},
				async {
					Ok(
						msg!([ctx] party::msg::member_state_resolve(party_id, user_id) {
							party_id: Some(party_id.into()),
							user_id: Some(user_id.into()),
						})
						.await?,
					)
				},
				async {
					Ok(msg!([ctx] party::msg::member_update(user_id) {
						user_id: Some(user_id.into()),
					})
					.await?)
				},
				async {
					// Send new party member message
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
							kind: Some(backend::chat::message_body::Kind::PartyJoin(backend::chat::message_body::PartyJoin {
								user_id: Some(user_id.into()),
							})),
						}),
					})
					.await
					.map_err(Into::<GlobalError>::into)
				}
			)?;

			msg!([ctx] party::msg::member_create_complete(party_id, user_id) {
				party_id: Some(party_id.into()),
				user_id: Some(user_id.into()),
			})
			.await?;

			msg!([ctx] party::msg::update(party_id) {
				party_id: Some(party_id.into()),
			})
			.await?;
		}
		Err("PARTY_DOES_NOT_EXIST") => {
			return fail(
				ctx.chirp(),
				party_id,
				user_id,
				party::msg::member_create_fail::ErrorCode::PartyDoesNotExist,
			)
			.await;
		}
		Err("PARTY_FULL") => {
			return fail(
				ctx.chirp(),
				party_id,
				user_id,
				party::msg::member_create_fail::ErrorCode::PartyFull,
			)
			.await;
		}
		Err("ALREADY_IN_PARTY") => {
			return fail(
				ctx.chirp(),
				party_id,
				user_id,
				party::msg::member_create_fail::ErrorCode::AlreadyInParty,
			)
			.await;
		}
		Err(_) => {
			internal_panic!("unknown redis error")
		}
	}

	Ok(())
}
