use std::{collections::HashMap, future::Future, sync::Arc};

use deno_core::{error::AnyError, op2, JsBuffer, OpState, ToJsBuffer};
use serde::Serialize;

type FakeMap<T, U> = Box<[(T, U)]>;

deno_core::extension!(
  rivet_kv,
  ops = [
	op_rivet_kv_get,
	op_rivet_kv_get_batch,
	op_rivet_kv_list,
	op_rivet_kv_put,
	op_rivet_kv_put_batch,
	op_rivet_kv_delete,
	op_rivet_kv_delete_batch,
	op_rivet_kv_delete_all,
  ],
  esm = [
	dir "js",
	"40_rivet_kv.js",
  ],
  options = {
	kv: actor_kv::ActorKv,
  },
  state = |state, options| {
	state.put::<Arc<actor_kv::ActorKv>>(Arc::new(options.kv));
  },
);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum Key {
	InKey(Vec<JsBuffer>),
	OutKey(Vec<ToJsBuffer>),
}

impl From<actor_kv::key::Key> for Key {
	fn from(value: actor_kv::key::Key) -> Self {
		match value {
			actor_kv::key::Key::JsInKey(tuple) => Key::InKey(tuple),
			actor_kv::key::Key::JsOutKey(tuple) => {
				Key::OutKey(tuple.into_iter().map(Into::into).collect())
			}
		}
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
	metadata: Metadata,
	value: ToJsBuffer,
}

impl From<actor_kv::Entry> for Entry {
	fn from(value: actor_kv::Entry) -> Self {
		Entry {
			metadata: value.metadata.into(),
			value: value.value.into(),
		}
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
	pub kv_version: ToJsBuffer,
	pub create_ts: i64,
}

impl From<actor_kv::Metadata> for Metadata {
	fn from(value: actor_kv::Metadata) -> Self {
		Metadata {
			kv_version: value.kv_version.into(),
			create_ts: value.create_ts,
		}
	}
}

#[op2(async)]
#[serde]
pub fn op_rivet_kv_get(
	state: &mut OpState,
	#[serde] key: actor_kv::key::Key,
) -> Result<impl Future<Output = Result<Option<Entry>, AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move {
		let res = kv.get(vec![key.into()]).await?;

		Ok(res.into_values().next().map(Into::into))
	})
}

#[op2(async)]
#[serde]
pub fn op_rivet_kv_get_batch(
	state: &mut OpState,
	#[serde] keys: Vec<actor_kv::key::Key>,
) -> Result<impl Future<Output = Result<FakeMap<Key, Entry>, AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move {
		let res = kv
			.get(keys.into_iter().map(Into::into).collect())
			.await?
			.into_iter()
			.map(|(k, v)| (k.into(), v.into()))
			.collect();

		Ok(res)
	})
}

#[op2(async)]
#[serde]
pub fn op_rivet_kv_list(
	state: &mut OpState,
	#[serde] query: actor_kv::ListQuery,
	reverse: bool,
	limit: Option<u32>,
) -> Result<impl Future<Output = Result<FakeMap<Key, Entry>, AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move {
		let res = kv
			.list(query.into(), reverse, limit.map(|x| x as usize))
			.await?
			.into_iter()
			.map(|(k, v)| (k.into(), v.into()))
			.collect();

		Ok(res)
	})
}

#[op2(async)]
pub fn op_rivet_kv_put(
	state: &mut OpState,
	#[serde] key: actor_kv::key::Key,
	#[buffer] value: JsBuffer,
) -> Result<impl Future<Output = Result<(), AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move { kv.put([(key, value)].into()).await })
}

#[op2(async)]
pub fn op_rivet_kv_put_batch(
	state: &mut OpState,
	#[serde] obj: HashMap<actor_kv::key::Key, JsBuffer>,
) -> Result<impl Future<Output = Result<(), AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move { kv.put(obj).await })
}

#[op2(async)]
pub fn op_rivet_kv_delete(
	state: &mut OpState,
	#[serde] key: actor_kv::key::Key,
) -> Result<impl Future<Output = Result<(), AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move { kv.delete(vec![key]).await })
}

#[op2(async)]
#[serde]
pub fn op_rivet_kv_delete_batch(
	state: &mut OpState,
	#[serde] keys: Vec<actor_kv::key::Key>,
) -> Result<impl Future<Output = Result<(), AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move { kv.delete(keys).await })
}

#[op2(async)]
pub fn op_rivet_kv_delete_all(
	state: &mut OpState,
) -> Result<impl Future<Output = Result<(), AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move { kv.delete_all().await })
}
