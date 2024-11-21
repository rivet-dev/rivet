use std::{collections::HashMap, future::Future, sync::Arc};

use deno_core::{error::AnyError, op2, JsBuffer, OpState, ToJsBuffer};
use foundationdb as fdb;
use futures_util::{StreamExt, TryStreamExt};

deno_core::extension!(
  rivet_kv,
  ops = [
	op_rivet_kv_get,
	op_rivet_kv_get_batch,
	op_rivet_kv_put,
	op_rivet_kv_put_batch,
	op_rivet_kv_delete,
	// op_rivet_kv_delete_batch,
  ],
  esm = [
	dir "js",
	"40_rivet_kv.js",
  ],
  options = {
	db: fdb::Database,
  },
  state = |state, options| {
	state.put::<Arc<fdb::Database>>(Arc::new(options.db));
  },
);

#[op2(async)]
#[buffer]
pub fn op_rivet_kv_get(
	state: &mut OpState,
	#[string] key: String,
) -> Result<impl Future<Output = Result<Option<Vec<u8>>, AnyError>>, AnyError> {
	validate_key(&key)?;

	let db = state.borrow::<Arc<fdb::Database>>().clone();

	Ok(async move {
		let bkey = key.as_bytes();

		let data = db
			.run(|tx, _maybe_committed| async move { Ok(tx.get(bkey, false).await?) })
			.await?;

		let Some(data) = data else {
			return Ok(None);
		};

		Ok(Some(data.to_vec()))
	})
}

#[op2(async)]
#[serde]
pub fn op_rivet_kv_get_batch(
	state: &mut OpState,
	#[serde] keys: Vec<String>,
) -> Result<impl Future<Output = Result<HashMap<String, ToJsBuffer>, AnyError>>, AnyError> {
	anyhow::ensure!(
		keys.len() <= 128,
		"a maximum of 128 keys is allowed for `Rivet.getBatch`"
	);

	for key in &keys {
		validate_key(key)?;
	}

	let db = state.borrow::<Arc<fdb::Database>>().clone();

	Ok(async move {
		let data = db
			.run(|tx, _maybe_committed| {
				let keys = keys.clone();

				async move {
					futures_util::stream::iter(keys)
						.map(|key| {
							let tx = tx.clone();
							async move {
								Ok(tx
									.get(key.as_bytes(), false)
									.await?
									.map(|data| (key, data.to_vec().into())))
							}
						})
						.buffer_unordered(16)
						.try_filter_map(|x| std::future::ready(Ok(x)))
						.try_collect::<HashMap<_, _>>()
						.await
				}
			})
			.await?;

		Ok(data)
	})
}

#[op2(async)]
pub fn op_rivet_kv_put(
	state: &mut OpState,
	#[string] key: String,
	#[buffer] value: JsBuffer,
) -> Result<impl Future<Output = Result<(), AnyError>>, AnyError> {
	validate_key(&key)?;
	validate_value(&key, &value)?;

	let db = state.borrow::<Arc<fdb::Database>>().clone();

	Ok(async move {
		let bkey = key.as_bytes();

		db.run(|tx, _maybe_committed| {
			let value = value.clone(); // Creates a new ref, does not clone data

			async move {
				tx.set(bkey, &value);
				Ok(())
			}
		})
		.await?;

		Ok(())
	})
}

#[op2(async)]
pub fn op_rivet_kv_put_batch(
	state: &mut OpState,
	#[serde] obj: HashMap<String, JsBuffer>,
) -> Result<impl Future<Output = Result<(), AnyError>>, AnyError> {
	for (key, value) in &obj {
		validate_key(&key)?;
		validate_value(&key, &value)?;
	}

	let db = state.borrow::<Arc<fdb::Database>>().clone();

	Ok(async move {
		db.run(|tx, _maybe_committed| {
			let obj = obj.clone();

			async move {
				for (key, value) in obj {
					tx.set(key.as_bytes(), &value);
				}

				Ok(())
			}
		})
		.await?;

		Ok(())
	})
}

#[op2(async)]
pub fn op_rivet_kv_delete(
	state: &mut OpState,
	#[string] key: String,
) -> Result<impl Future<Output = Result<bool, AnyError>>, AnyError> {
	validate_key(&key)?;

	let db = state.borrow::<Arc<fdb::Database>>().clone();

	Ok(async move {
		let bkey = key.as_bytes();

		let existed = db
			.run(|tx, _maybe_committed| async move {
				let existed = tx.get(bkey, false).await?.is_some();

				tx.clear(bkey);
				Ok(existed)
			})
			.await?;

		Ok(existed)
	})
}

fn validate_key(key: &str) -> Result<(), AnyError> {
	// 2048 bytes
	anyhow::ensure!(key.len() <= 2048, "key is too long (max 2048 bytes)");

	Ok(())
}

fn validate_value(key: &str, value: &[u8]) -> Result<(), AnyError> {
	// 2048 bytes
	anyhow::ensure!(
		value.len() <= 128 * 1024,
		"value for key {key:?} is too large (max 128 KiB)"
	);

	Ok(())
}
