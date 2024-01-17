use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use chirp_client::{TailAllConfig, TailAllResponse, TailAnchor};
use futures_util::{StreamExt, TryStreamExt};
use proto::backend::pkg::*;
use rivet_api::models;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::{Duration, Instant};

use crate::{auth::Auth, utils};

const GET_BATCH_MAX: usize = 64;
const WATCH_BATCH_MAX: usize = 16;
const PUT_AND_DELETE_BATCH_MAX: usize = 64;

#[derive(Deserialize, Serialize)]
pub struct BatchQuery {
	keys: Vec<String>,
	namespace_id: Option<Uuid>,
}

struct NewEntry {
	key: String,
	value: Vec<u8>,
}

// MARK: GET /entries/batch
pub async fn get_batch(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
	query: BatchQuery,
) -> GlobalResult<models::KvGetBatchResponse> {
	let namespace_id = ctx
		.auth()
		.namespace(ctx.op_ctx(), query.namespace_id, true)
		.await?;

	ensure_with!(!query.keys.is_empty(), KV_KEYS_MISSING);
	if watch_index.has_watch_index() {
		ensure_with!(query.keys.len() <= WATCH_BATCH_MAX, KV_BATCH_TOO_LARGE);
	} else {
		ensure_with!(query.keys.len() <= GET_BATCH_MAX, KV_BATCH_TOO_LARGE);
	}

	let keys = query.keys;
	utils::validate_keys(&keys, false)?;

	// Watch
	if let Some(anchor) = watch_index.to_consumer()? {
		let (entries, update_ts) = watch_batch(&ctx, anchor, namespace_id, keys.clone()).await?;
		let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

		return Ok(models::KvGetBatchResponse {
			entries,
			watch: Box::new(utils::watch_response(WatchResponse::new(update_ts + 1))),
		});
	};

	// Fetch keys
	let keys = keys
		.into_iter()
		.map(|key| kv::get::request::Key {
			namespace_id: Some(namespace_id.into()),
			key,
		})
		.collect::<Vec<_>>();

	let res = op!([ctx] kv_get {
		keys: keys,
	})
	.await?;

	// Convert to response
	let entries = res
		.values
		.iter()
		.map(|value| {
			GlobalResult::Ok(models::KvEntry {
				key: value.key.clone(),
				value: Some(serde_json::from_slice(&value.value)?),
				deleted: None,
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::KvGetBatchResponse {
		entries,
		watch: Box::new(utils::watch_response(WatchResponse::new(
			util::timestamp::now() + 1,
		))),
	})
}

// MARK: PUT /entries/batch
pub async fn put_batch(
	ctx: Ctx<Auth>,
	body: models::KvPutBatchRequest,
) -> GlobalResult<serde_json::Value> {
	let namespace_id = ctx
		.auth()
		.namespace(ctx.op_ctx(), body.namespace_id, false)
		.await?;

	ensure_with!(!body.entries.is_empty(), KV_ENTRIES_MISSING);
	ensure_with!(
		body.entries.len() <= PUT_AND_DELETE_BATCH_MAX,
		KV_BATCH_TOO_LARGE
	);

	// Validate all entry keys
	let new_entries = body
		.entries
		.into_iter()
		.map(|entry| {
			utils::validate_keys(&[entry.key.clone()], false)?;

			let value = entry.value.unwrap_or_else(|| json!(null));
			let value_buf = serde_json::to_vec(&value)?;
			ensure_with!(value_buf.len() <= util_kv::MAX_VALUE_LEN, KV_VALUE_TOO_LONG);

			Ok(NewEntry {
				key: entry.key,
				value: value_buf,
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	// Perform all writes simultaneously
	futures_util::stream::iter(new_entries.into_iter().map(|entry| {
		let ctx = ctx.chirp();
		async move {
			msg!([ctx] kv::msg::write(namespace_id, &entry.key) {
				namespace_id: Some(namespace_id.into()),
				key: entry.key,
				value: Some(entry.value),
			})
			.await
			.map_err(Into::<GlobalError>::into)
		}
	}))
	.buffer_unordered(32)
	.try_collect::<Vec<_>>()
	.await?;

	Ok(json!({}))
}

// MARK: DELETE /entries/batch
pub async fn delete_batch(ctx: Ctx<Auth>, query: BatchQuery) -> GlobalResult<serde_json::Value> {
	let namespace_id = ctx
		.auth()
		.namespace(ctx.op_ctx(), query.namespace_id, false)
		.await?;

	ensure_with!(!query.keys.is_empty(), KV_KEYS_MISSING);
	ensure_with!(
		query.keys.len() <= PUT_AND_DELETE_BATCH_MAX,
		KV_BATCH_TOO_LARGE
	);
	utils::validate_keys(&query.keys, false)?;

	// Perform all writes simultaneously
	futures_util::stream::iter(query.keys.into_iter().map(|key| {
		let ctx = ctx.chirp();
		async move {
			msg!([ctx] kv::msg::write(&namespace_id, &key) {
				namespace_id: Some(namespace_id.into()),
				key: key,
				value: None,
			})
			.await
			.map_err(Into::<GlobalError>::into)
		}
	}))
	.buffer_unordered(32)
	.try_collect::<Vec<_>>()
	.await?;

	Ok(json!({}))
}

/// Watches multiple KV entries in batch.
///
/// This has two phases:
/// 1. Fetch initial messages from the tails. If messages exist, then return the
///    results immediately after the grace period.
/// 2. If no messages exist, then merge all of the subscriptions for KV updates.
async fn watch_batch(
	ctx: &Ctx<Auth>,
	anchor: TailAnchor,
	namespace_id: Uuid,
	keys: Vec<String>,
) -> GlobalResult<(Vec<models::KvEntry>, Option<i64>)> {
	// First phase, receiving messages from logs only
	match watch_batch_immediate(ctx, anchor, namespace_id, keys).await? {
		ImmediateResult::Complete((entries, update_ts)) => Ok((entries, Some(update_ts))),
		// Second phase, merge all subscriptions
		ImmediateResult::Continue(tail_all_res) => watch_batch_merge_subs(tail_all_res).await,
	}
}

enum ImmediateResult {
	Complete((Vec<models::KvEntry>, i64)),
	Continue(Vec<TailAllResponse<kv::msg::update::Message>>),
}

/// Collects messages from message log and returns immediately. If none are
/// found, returns subscriptions for polling.
async fn watch_batch_immediate(
	ctx: &Ctx<Auth>,
	anchor: TailAnchor,
	namespace_id: Uuid,
	keys: Vec<String>,
) -> GlobalResult<ImmediateResult> {
	// TODO: This does not replicate the same grace period that you get in
	// tail_all to act as a rate limiter for frequent events

	// Create tail alls
	let tail_all_res = futures_util::stream::iter(keys)
		.map(|key| {
			let chirp = ctx.chirp().clone();
			let anchor = anchor.clone();
			async move {
				tail_all!(
					[chirp, anchor, TailAllConfig::return_after_logs()]
					kv::msg::update(namespace_id, key)
				)
				.await
			}
		})
		.buffer_unordered(32)
		.try_collect::<Vec<_>>()
		.await?;

	// If any message was found, return immediately
	let has_message = tail_all_res.iter().any(|res| res.messages.is_empty());
	if has_message {
		let mut entries = tail_all_res
			.into_iter()
			.flat_map(|res| {
				res.messages
					.into_iter()
					.map(|msg| rivet_convert::kv::DestructuredKvEntry {
						key: msg.key.clone(),
						value: msg.value.clone(),
						msg_ts: msg.msg_ts(),
					})
					.collect::<Vec<_>>()
			})
			.collect::<Vec<_>>();
		entries.sort_by_key(|x| x.msg_ts);
		let latest_update_ts = unwrap!(entries.last()).msg_ts;

		Ok(ImmediateResult::Complete((
			entries
				.into_iter()
				.map(ApiTryInto::<models::KvEntry>::try_into)
				.collect::<GlobalResult<Vec<_>>>()?,
			latest_update_ts,
		)))
	} else {
		Ok(ImmediateResult::Continue(tail_all_res))
	}
}

/// Merge multiple `TailAll`'s subscriptions together.
///
/// This is not the same as polling multiple `TailAll`'s simultaneously because
/// the grace period begins and continues with all subscriptions together which
/// allows messages from any subscription to be collected, not just one.
async fn watch_batch_merge_subs(
	entry_tail_all_res: Vec<TailAllResponse<kv::msg::update::Message>>,
) -> GlobalResult<(Vec<models::KvEntry>, Option<i64>)> {
	// Merge all subscriptions together
	let streams = entry_tail_all_res
		.into_iter()
		.flat_map(|x| x.subs)
		.map(|subs| subs.into_stream().boxed());
	let mut all_subs = futures_util::stream::select_all(streams);

	// Loop all subs to consume as many updates as we can within 100ms
	let mut entries = Vec::new();
	let mut collect_expire = None;
	loop {
		let now = Instant::now();
		if collect_expire.map_or(false, |x| now > x) || entries.len() > 128 {
			break;
		}

		let timeout = collect_expire
			.map(|collect_expire| (collect_expire - now).as_millis() as u64)
			.unwrap_or_else(|| chirp_client::TAIL_ALL_DEFAULT_EMPTY_GRACE.as_millis() as u64);

		util::macros::select_with_timeout!([timeout MS] {
			msg = all_subs.next() => {
				if let Some(msg) = msg {
					let msg = msg?;

					entries.push(rivet_convert::kv::DestructuredKvEntry {
						key: msg.key.clone(),
						value: msg.value.clone(),
						msg_ts: msg.msg_ts(),
					});

					if collect_expire.is_none() {
						collect_expire = Some(Instant::now() + Duration::from_millis(100));
					}
				}
			}
		});

		if collect_expire.is_none() {
			break;
		}
	}

	let latest_update_ts = entries.last().map(|entry| entry.msg_ts);

	Ok((
		entries
			.into_iter()
			.map(ApiTryInto::try_into)
			.collect::<GlobalResult<Vec<_>>>()?,
		latest_update_ts,
	))
}
