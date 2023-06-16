use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

use crate::redis::{from_redis_value, FromRedisValue, RedisResult, Value};

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../redis-scripts/main.lua"));
}

// Copied from redis/types.rs
macro_rules! invalid_type_error_inner {
	($v:expr, $det:expr) => {
		redis::RedisError::from((
			redis::ErrorKind::TypeError,
			"Response was of incompatible type",
			format!("{:?} (response was {:?})", $det, $v),
		))
	};
}

/// Response type for recent threads fetched from Redis.
#[derive(Debug)]
struct TailResponse {
	thread_id: String,
	message_buf: Option<Vec<u8>>,
}

impl FromRedisValue for TailResponse {
	fn from_redis_value(v: &Value) -> RedisResult<TailResponse> {
		match *v {
			Value::Bulk(ref items) => {
				let mut iter = items.iter();

				Ok(TailResponse {
					thread_id: from_redis_value(iter.next().ok_or_else(|| {
						invalid_type_error_inner!(v, "Bulk response of wrong dimension")
					})?)?,
					message_buf: iter.next().map(from_redis_value).transpose()?,
				})
			}
			_ => Err(invalid_type_error_inner!(
				v,
				"Response type not vector compatible."
			)),
		}
	}
}

// /// Helper type for Redis responses.
// #[derive(Debug)]
// struct NoNilVec<T> {
// 	vec: Vec<T>,
// }

// impl<T> NoNilVec<T> {
// 	fn inner(self) -> Vec<T> {
// 		self.vec
// 	}
// }

// impl<T: FromRedisValue> FromRedisValue for NoNilVec<T> {
// 	fn from_redis_value(v: &Value) -> RedisResult<NoNilVec<T>> {
// 		match *v {
// 			// This hack allows us to specialize Vec<u8> to work with
// 			// binary data whereas all others will fail with an error.
// 			Value::Data(ref bytes) => match FromRedisValue::from_byte_vec(bytes) {
// 				Some(x) => Ok(NoNilVec { vec: x }),
// 				None => Err(invalid_type_error_inner!(
// 					v,
// 					"Response type not vector compatible."
// 				)),
// 			},
// 			Value::Bulk(ref items) => Ok(NoNilVec {
// 				vec: items
// 					.iter()
// 					.map(FromRedisValue::from_redis_value)
// 					.collect::<RedisResult<Vec<_>>>()?,
// 			}),
// 			_ => Err(invalid_type_error_inner!(
// 				v,
// 				"Response type not vector compatible."
// 			)),
// 		}
// 	}
// }

struct ThreadWithTail {
	thread_id: Uuid,
	tail_message: Option<backend::chat::Message>,
}

#[operation(name = "chat-thread-recent-for-user")]
async fn handle(
	ctx: OperationContext<chat_thread::recent_for_user::Request>,
) -> Result<chat_thread::recent_for_user::Response, GlobalError> {
	// Overview:
	// 1. Fetch recent thread IDs & tail messages from Redis. If doesn't exists,
	//       fetch from Cockroach.
	// 2. Fetch any missing tail messages from Cockroach.
	// 3. If missing tail messages found, repopulate Redis.
	// 4. Map to Protobuf.

	let mut redis = ctx.redis_cache().await?;
	let crdb = ctx.crdb("db-chat").await?;

	let user_id = internal_unwrap!(ctx.user_id).as_uuid();
	let after_ts = ctx.after_ts;

	// Get rows from Redis cache
	let (mut recent_threads, is_threads_cached) =
		fetch_recent_threads(&ctx, &mut redis, user_id, after_ts).await?;
	tracing::info!(len = ?recent_threads.len(), ?is_threads_cached, "fetched threads");

	// Get all thread metadata
	let thread_ids = recent_threads
		.iter()
		.map(|thread_with_tail| thread_with_tail.thread_id.into())
		.collect::<Vec<_>>();
	let threads_res = op!([ctx] chat_thread_get {
		thread_ids: thread_ids,
	})
	.await?;

	// Get messages that are absent in Redis from Cockroach
	let threads_without_tail = recent_threads
		.iter()
		.filter(|x| x.tail_message.is_none())
		.map(|x| x.thread_id)
		.collect::<Vec<_>>();
	let missing_tails =
		fetch_missing_tails(&crdb, &mut redis, user_id, &threads_without_tail).await?;

	// Fill in tail messages with those fetched from Cockroach
	for (row, msg_proto) in missing_tails {
		if let Some(x) = recent_threads
			.iter_mut()
			.find(|x| x.thread_id == row.thread_id)
		{
			x.tail_message = Some(msg_proto);
		}
	}

	let threads = recent_threads
		.into_iter()
		// Filter out threads without tail message
		.filter_map(|thread_with_tail| {
			if let Some(tail_message) = thread_with_tail.tail_message {
				Some((thread_with_tail.thread_id, tail_message))
			} else {
				tracing::warn!(thread_id = ?thread_with_tail.thread_id, "thread missing tail message");
				None
			}
		})
		// Filter with after_ts
		.filter(|(_, message)| after_ts.map_or(true, |after_ts| message.send_ts >= after_ts))
		// Convert to proto
		.map(|(thread_id, tail_message)| {
			let thread_id = Into::<common::Uuid>::into(thread_id);
			let thread = internal_unwrap_owned!(threads_res
				.threads
				.iter()
				.find(|t| t.thread_id.as_ref() == Some(&thread_id)));

			Ok(chat_thread::recent_for_user::response::ThreadWithTail {
				thread: Some(thread.clone()),
				tail_message: Some(tail_message),
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(chat_thread::recent_for_user::Response { threads })
}

#[tracing::instrument(skip_all)]
async fn fetch_recent_threads(
	ctx: &OperationContext<chat_thread::recent_for_user::Request>,
	redis: &mut RedisPool,
	user_id: Uuid,
	after_ts: Option<i64>,
) -> GlobalResult<(Vec<ThreadWithTail>, bool)> {
	if let Some(recent_threads) = REDIS_SCRIPT
		.arg(after_ts)
		.key(util_chat::key::user_thread_history_loaded(user_id))
		.key(util_chat::key::user_thread_history(user_id))
		.invoke_async::<_, Option<Vec<TailResponse>>>(redis)
		.await?
	{
		let threads = recent_threads
			.into_iter()
			.map(|res| {
				let thread_id = util::uuid::parse(&res.thread_id)?;
				let tail_message = res
					.message_buf
					.map(|buf| backend::chat::Message::decode(buf.as_slice()))
					.transpose()?;

				Ok(ThreadWithTail {
					thread_id,
					tail_message,
				})
			})
			.collect::<GlobalResult<Vec<_>>>()?;

		Ok((threads, true))
	} else {
		tracing::info!("no recent thread cache, getting threads from cockroach");

		// If Redis cache is empty, fetch threads from
		// `chat_thread_list_for_participant`
		//
		// We don't factor in `after_ts` here since we need to populate the
		// cache with the entire recent threads history.
		let threads_res = op!([ctx] chat_thread_list_for_participant {
			user_id: Some(user_id.into()),
		})
		.await?;

		let threads = threads_res
			.threads
			.iter()
			.map(|thread| {
				Ok(ThreadWithTail {
					thread_id: internal_unwrap!(thread.thread_id).as_uuid(),
					tail_message: None,
				})
			})
			.collect::<GlobalResult<Vec<_>>>()?;

		Ok((threads, false))
	}
}

#[derive(sqlx::FromRow)]
struct TailMessageRow {
	thread_id: Uuid,
	message_id: Uuid,
	send_ts: i64,
	body: Vec<u8>,
}

async fn fetch_missing_tails(
	crdb: &CrdbPool,
	redis: &mut RedisPool,
	user_id: Uuid,
	threads_without_tail: &[Uuid],
) -> GlobalResult<Vec<(TailMessageRow, backend::chat::Message)>> {
	if threads_without_tail.is_empty() {
		return Ok(Vec::new());
	}

	let thread_with_tails = sqlx::query_as::<_, TailMessageRow>(indoc!(
		"
		SELECT messages.*
		FROM threads
		LEFT JOIN LATERAL (
			SELECT messages.thread_id, message_id, send_ts, body
			FROM messages
			WHERE messages.thread_id = threads.thread_id
			ORDER BY send_ts DESC, message_id DESC
			LIMIT 1
		) AS messages ON true
		WHERE threads.thread_id = ANY($1)
		"
	))
	.bind(threads_without_tail)
	.fetch_all(crdb)
	.await?
	.into_iter()
	.map(|row| {
		let body = backend::chat::MessageBody::decode(row.body.as_slice())?;
		let message = backend::chat::Message {
			chat_message_id: Some(row.message_id.into()),
			thread_id: Some(row.thread_id.into()),
			send_ts: row.send_ts,
			body: Some(body),
		};

		GlobalResult::Ok((row, message))
	})
	.collect::<GlobalResult<Vec<_>>>()?;

	// Repopulate Redis. This will make sure all threads & messages are up to
	// date in the cache so we don't have to fetch from the database next time
	// ths is called.
	//
	// Do this regardless of if no tails are returned so we can flag that the
	// cache is loaded.
	repopulate_redis(redis, user_id, &thread_with_tails).await?;

	Ok(thread_with_tails)
}

async fn repopulate_redis(
	redis: &mut RedisPool,
	user_id: Uuid,
	threads: &[(TailMessageRow, backend::chat::Message)],
) -> GlobalResult<()> {
	let key_user_thread_history_loaded = util_chat::key::user_thread_history_loaded(user_id);
	let key_user_thread_history = util_chat::key::user_thread_history(user_id);

	let mut pipe = redis::pipe();
	pipe.atomic();

	// Write messages to cache
	for (row, message) in threads {
		let key_thread_tail_message = util_chat::key::thread_tail_message(row.thread_id);

		// Encode message
		let mut message_buf = Vec::with_capacity(message.encoded_len());
		message.encode(&mut message_buf)?;

		// Save to user's thread history
		pipe.zadd(
			&key_user_thread_history,
			row.thread_id.to_string(),
			message.send_ts,
		)
		.ignore();

		// Set tail if hasn't been set in race condition
		pipe.hset_nx(
			&key_thread_tail_message,
			util_chat::key::thread_tail_message::MESSAGE_ID,
			row.message_id.to_string(),
		)
		.ignore();
		pipe.hset_nx(
			&key_thread_tail_message,
			util_chat::key::thread_tail_message::SEND_TS,
			row.send_ts,
		)
		.ignore();
		pipe.hset_nx(
			&key_thread_tail_message,
			util_chat::key::thread_tail_message::MESSAGE_BUF,
			message_buf,
		)
		.ignore();

		// Expire tail message
		pipe.expire(
			&key_thread_tail_message,
			util_chat::key::THREAD_TAIL_MESSAGE_EXPIRE_DURATION as usize,
		);
	}

	// Flag as loaded
	pipe.set(&key_user_thread_history_loaded, 1);

	// Expire cache. Do this after writing since the keys may not exist
	// beforehand.
	pipe.expire(
		&key_user_thread_history_loaded,
		util_chat::key::USER_THREAD_HISTORY_EXPIRE_DURATION as usize,
	);
	pipe.expire(
		&key_user_thread_history,
		util_chat::key::USER_THREAD_HISTORY_EXPIRE_DURATION as usize,
	);

	pipe.query_async(redis).await?;

	Ok(())
}
