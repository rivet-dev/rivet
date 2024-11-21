use std::{future::Future, sync::Arc};

use anyhow::Context;
use deno_core::{error::AnyError, op2, OpState};
use foundationdb as fdb;
use futures_util::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};

deno_core::extension!(
  rivet_kv,
  ops = [
	op_rivet_kv_get,
	op_rivet_kv_get_batch,
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

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
enum OutputFormat {
	Json,
	#[default]
	Text,
	ArrayBuffer,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GetOptions {
	format: OutputFormat,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Output {
	Text(String),
	Json(serde_json::Value),
	Buffer(deno_core::ToJsBuffer),
}

#[op2(async)]
#[serde]
pub fn op_rivet_kv_get(
	state: &mut OpState,
	#[string] key: String,
	#[serde] options: Option<GetOptions>,
) -> impl Future<Output = Result<Option<Output>, AnyError>> {
	let db = state.borrow::<Arc<fdb::Database>>().clone();
	let options = options.unwrap_or_default();

	async move {
		let bkey = key.as_bytes();

		let data = db
			.run(|tx, _maybe_committed| async move { Ok(tx.get(bkey, false).await?) })
			.await?;

		let Some(data) = data else {
			return Ok(None);
		};

		let output = match options.format {
			OutputFormat::Text => Output::Text(
				std::str::from_utf8(&data)
					.with_context(|| {
						format!("failed to deserialize value as text for key {key:?}")
					})?
					.to_string(),
			),
			OutputFormat::Json => Output::Json(
				serde_json::from_slice::<serde_json::Value>(&data).with_context(|| {
					format!("failed to deserialize value as JSON for key {key:?}")
				})?,
			),
			OutputFormat::ArrayBuffer => Output::Buffer(data.to_vec().into()),
		};

		Ok(Some(output))
	}
}

#[op2(async)]
#[serde]
pub fn op_rivet_kv_get_batch(
	state: &mut OpState,
	#[serde] keys: Vec<String>,
	#[serde] options: Option<GetOptions>,
) -> Result<impl Future<Output = Result<Vec<Output>, AnyError>>, AnyError> {
	let db = state.borrow::<Arc<fdb::Database>>().clone();
	let options = options.unwrap_or_default();

	anyhow::ensure!(
		keys.len() <= 128,
		"a maximum of 128 keys is allowed for `Rivet.getBatch`"
	);

	Ok(async move {
		let data = db
			.run(|tx, _maybe_committed| {
				let keys = keys.clone();

				async move {
					futures_util::stream::iter(keys)
						.map(|key| {
							let tx = tx.clone();
							async move {
								Ok(tx.get(key.as_bytes(), false).await?.map(|data| (key, data)))
							}
						})
						.buffer_unordered(16)
						.try_filter_map(|x| std::future::ready(Ok(x)))
						.try_collect::<Vec<_>>()
						.await
				}
			})
			.await?;

		data.into_iter()
			.map(|(key, data)| {
				let output = match options.format {
					OutputFormat::Text => Output::Text(
						std::str::from_utf8(&data)
							.with_context(|| {
								format!("failed to deserialize value as text for key {key:?}")
							})?
							.to_string(),
					),
					OutputFormat::Json => Output::Json(
						serde_json::from_slice::<serde_json::Value>(&data).with_context(|| {
							format!("failed to deserialize value as JSON for key {key:?}")
						})?,
					),
					OutputFormat::ArrayBuffer => Output::Buffer(data.to_vec().into()),
				};

				Ok(output)
			})
			.collect()
	})
}
