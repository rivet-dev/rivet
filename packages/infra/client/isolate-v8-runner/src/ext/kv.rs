use std::{collections::HashMap, future::Future, sync::Arc};

use deno_core::{error::AnyError, op2, JsBuffer, OpState};

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

#[op2(async)]
#[serde]
pub fn op_rivet_kv_get(
	state: &mut OpState,
	#[string] key: String,
) -> Result<impl Future<Output = Result<Option<actor_kv::Entry>, AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move { kv.get(vec![key]).await.map(|res| res.into_values().next()) })
}

#[op2(async)]
#[serde]
pub fn op_rivet_kv_get_batch(
	state: &mut OpState,
	#[serde] keys: Vec<String>,
) -> Result<impl Future<Output = Result<HashMap<String, actor_kv::Entry>, AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move { kv.get(keys).await })
}

#[op2(async)]
#[serde]
pub fn op_rivet_kv_list(
	state: &mut OpState,
	#[serde] query: actor_kv::ListQuery,
	reverse: bool,
	limit: Option<u32>,
) -> Result<impl Future<Output = Result<HashMap<String, actor_kv::Entry>, AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move { kv.list(query, reverse, limit.map(|x| x as usize)).await })
}

#[op2(async)]
pub fn op_rivet_kv_put(
	state: &mut OpState,
	#[string] key: String,
	#[buffer] value: JsBuffer,
) -> Result<impl Future<Output = Result<(), AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move { kv.put([(key, value)].into()).await })
}

#[op2(async)]
pub fn op_rivet_kv_put_batch(
	state: &mut OpState,
	#[serde] obj: HashMap<String, JsBuffer>,
) -> Result<impl Future<Output = Result<(), AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move { kv.put(obj).await })
}

#[op2(async)]
pub fn op_rivet_kv_delete(
	state: &mut OpState,
	#[string] key: String,
) -> Result<impl Future<Output = Result<bool, AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move {
		kv.delete(vec![key])
			.await
			.map(|res| res.into_values().next().unwrap_or_default())
	})
}

#[op2(async)]
#[serde]
pub fn op_rivet_kv_delete_batch(
	state: &mut OpState,
	#[serde] keys: Vec<String>,
) -> Result<impl Future<Output = Result<HashMap<String, bool>, AnyError>>, AnyError> {
	let kv = state.borrow::<Arc<actor_kv::ActorKv>>().clone();

	Ok(async move { kv.delete(keys).await })
}
