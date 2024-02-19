use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use chirp_client::TailAnchorResponse;
use proto::backend::pkg::*;
use rivet_api::models;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{auth::Auth, utils};

#[derive(Deserialize, Serialize)]
pub struct SingleQuery {
	key: String,
	namespace_id: Option<Uuid>,
}

// MARK: GET /entries
pub async fn get(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
	query: SingleQuery,
) -> GlobalResult<models::KvGetResponse> {
	let namespace_id = ctx
		.auth()
		.namespace(ctx.op_ctx(), query.namespace_id, true)
		.await?;

	let key = urlencoding::decode(&query.key)?.to_string();
	utils::validate_keys(&[key.clone()], false)?;

	// Watch for value
	if let Some(anchor) = watch_index.to_consumer()? {
		let update_sub = tail_anchor!([ctx, anchor] kv::msg::update(namespace_id, &key));

		let msg = util::macros::select_with_timeout!({
			msg = update_sub => {
				Some(msg?)
			}
		});

		// Return value if exists
		if let Some(TailAnchorResponse::Message(msg)) = msg {
			let value = if let Some(value) = &msg.value {
				serde_json::from_slice(value)?
			} else {
				serde_json::Value::Null
			};

			return Ok(models::KvGetResponse {
				value: Some(value),
				deleted: Some(msg.value.is_none()),
				watch: Box::new(utils::watch_response(WatchResponse::new(msg.msg_ts() + 1))),
			});
		}
	}

	let update_ts = util::timestamp::now();

	// Get current value
	let res = op!([ctx] kv_get {
		keys: vec![
			kv::get::request::Key {
				namespace_id: Some(namespace_id.into()),
				key: key.clone(),
			},
		],
	})
	.await?;

	let value = res.values.first().map_or_else(
		|| Ok(serde_json::Value::Null),
		|v| serde_json::from_slice(&v.value),
	)?;

	Ok(models::KvGetResponse {
		value: Some(value),
		deleted: None,
		watch: Box::new(utils::watch_response(WatchResponse::new(update_ts + 1))),
	})
}

// MARK: PUT /entries
pub async fn put(ctx: Ctx<Auth>, body: models::KvPutRequest) -> GlobalResult<serde_json::Value> {
	let namespace_id = ctx
		.auth()
		.namespace(ctx.op_ctx(), body.namespace_id, false)
		.await?;

	let key = body.key;
	utils::validate_keys(&[&key], false)?;
	let value = serde_json::to_vec(&body.value)?;
	ensure_with!(value.len() <= util_kv::MAX_VALUE_LEN, KV_VALUE_TOO_LONG);

	msg!([ctx] kv::msg::write(&namespace_id, &key) {
		namespace_id: Some(namespace_id.into()),
		key: key.clone(),
		value: Some(value),
	})
	.await?;

	Ok(json!({}))
}

// MARK: DELETE /entries
pub async fn delete(ctx: Ctx<Auth>, query: SingleQuery) -> GlobalResult<serde_json::Value> {
	let namespace_id = ctx
		.auth()
		.namespace(ctx.op_ctx(), query.namespace_id, false)
		.await?;

	let key = urlencoding::decode(&query.key)?.to_string();
	utils::validate_keys(&[&key], false)?;

	// We do not wait for a return message because if the value we are deleting doesn't
	// exist, no message is sent back from `kv-write`
	msg!([ctx] kv::msg::write(&namespace_id, &key) {
		namespace_id: Some(namespace_id.into()),
		key: key.clone(),
		value: None,
	})
	.await?;

	Ok(json!({}))
}

// MARK: GET /entries/list
#[derive(Deserialize, Serialize)]
pub struct ListQuery {
	directory: String,
	namespace_id: Uuid,
}

pub async fn list(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::KvListResponse> {
	// Only allow from cloud
	let namespace_id = ctx
		.auth()
		.namespace_from_cloud(ctx.op_ctx(), query.namespace_id)
		.await?;

	let directory = urlencoding::decode(&query.directory)?.to_string();
	utils::validate_keys(&[&directory], true)?;

	let _update_ts = util::timestamp::now();

	// Get current value
	let res = op!([ctx] kv_list {
		namespace_id: Some(namespace_id.into()),
		directory: directory.clone(),
		with_values: true,
		limit: Some(32),
	})
	.await?;

	let entries = res
		.entries
		.iter()
		.map(|entry| {
			let value = entry.value.as_ref().map_or_else(
				|| Ok(serde_json::Value::Null),
				|v| serde_json::from_slice(v),
			)?;

			GlobalResult::Ok(models::KvEntry {
				key: entry.key.clone(),
				value: Some(value),
				deleted: None,
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::KvListResponse { entries })
}
