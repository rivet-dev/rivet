use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../redis-scripts/main.lua"));
}

#[operation(name = "user-thread-typing-status-set")]
async fn handle(
	ctx: OperationContext<user::thread_typing_status_set::Request>,
) -> GlobalResult<user::thread_typing_status_set::Response> {
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();
	let thread_id = internal_unwrap!(ctx.thread_id).as_uuid();
	let typing_status = internal_unwrap!(ctx.status);

	let mut typing_status_buf = Vec::with_capacity(typing_status.encoded_len());
	typing_status.encode(&mut typing_status_buf)?;

	let user_id_str = user_id.to_string();

	let do_broadcast =
		if let backend::chat::typing_status::Kind::Idle(_) = internal_unwrap!(typing_status.kind) {
			// Check if key exists before deleting so we don't send unnecessary thread update messages
			let exists = redis::pipe()
				.atomic()
				.hexists(
					util_chat::key::typing_statuses(thread_id),
					user_id_str.clone(),
				)
				.query_async::<_, Vec<bool>>(&mut ctx.cache_handle().redis())
				.await?
				.first()
				.cloned()
				.unwrap_or_default();

			if exists {
				tokio::try_join!(
					async {
						redis::pipe()
							.atomic()
							.hdel(
								util_chat::key::typing_statuses(thread_id),
								user_id_str.clone(),
							)
							.zrem(
								util_chat::key::typing_statuses_update_ts(thread_id),
								user_id_str,
							)
							.query_async::<_, ()>(&mut ctx.cache_handle().redis())
							.await
							.map_err(Into::<GlobalError>::into)
					},
					clear_expired_typing_statuses(&ctx, thread_id),
				)?;
			}

			exists
		} else {
			let topic_key = util_chat::key::typing_statuses(thread_id);
			let topic_update_ts_key = util_chat::key::typing_statuses_update_ts(thread_id);

			// Save status in cache, set update timestamp, and clear expired keys
			tokio::try_join!(
				async {
					redis::pipe()
						.atomic()
						.hset(topic_key.clone(), user_id_str.clone(), typing_status_buf)
						.zadd(
							topic_update_ts_key.clone(),
							user_id_str,
							util::timestamp::now() / 1000,
						)
						.expire(
							topic_key,
							util_chat::key::TYPING_STATUS_EXPIRE_DURATION as usize,
						)
						.expire(
							topic_update_ts_key,
							util_chat::key::TYPING_STATUS_EXPIRE_DURATION as usize,
						)
						.query_async::<_, ()>(&mut ctx.cache_handle().redis())
						.await
						.map_err(Into::<GlobalError>::into)
				},
				clear_expired_typing_statuses(&ctx, thread_id),
			)?;

			true
		};

	if do_broadcast && !ctx.no_broadcast {
		msg!([ctx] chat_thread::msg::update(thread_id) {
			kind: Some(chat_thread::msg::update::message::Kind::TypingStatus(
				typing_status.clone()
			)),
		})
		.await?;
	}

	Ok(user::thread_typing_status_set::Response {})
}

// Remove all statuses that are older than expiration time
async fn clear_expired_typing_statuses(
	ctx: &OperationContext<user::thread_typing_status_set::Request>,
	thread_id: Uuid,
) -> GlobalResult<()> {
	// Initiate redis script
	let mut script = REDIS_SCRIPT.prepare_invoke();
	script
		.key(util_chat::key::typing_statuses(thread_id))
		.key(util_chat::key::typing_statuses_update_ts(thread_id))
		.arg(util_chat::key::TYPING_STATUS_EXPIRE_DURATION)
		.arg(util::timestamp::now() / 1000);

	script
		.invoke_async::<_, ()>(&mut ctx.cache_handle().redis())
		.await?;

	Ok(())
}
