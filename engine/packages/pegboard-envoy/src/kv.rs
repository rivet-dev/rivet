//! Actor KV storage. Rows live in a `kv` inode of the actor's database volume, beside its
//! SQLite file, and use the same exclusive filesystem owner.
use crate::keys::kv::{ActorKvKey, ActorKvPrefix};
use anyhow::{Context, Result, ensure};
use bytes::Bytes;
use depot_client::filesystem::{self, Filesystem, Transaction};
use pegboard::wire;
use rivet_envoy_protocol as p;
use rivet_error::*;
use rivet_keys::keys::{KeyAddress, SubspaceRange};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound;
use std::sync::Arc;
use vbare::Versioned;

const MAX_KEY_BYTES: usize = 2048;
/// The most keys one get or put may name, matching the limit Envoy clients already follow.
const MAX_KEYS: usize = 128;
const MAX_PUT_BYTES: usize = 976 * 1024;
/// The most bytes of keys, values and versions one get may return, held by this layer whatever
/// the filesystem's configuration. The filesystem's default per-request read budget (8 MiB)
/// is smaller and usually refuses first; its refusal is reported as the same error.
const MAX_GET_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

#[derive(RivetError, Debug, Serialize, Deserialize)]
#[error(
	"actor",
	"kv_too_many_keys",
	"The KV request names too many keys.",
	"KV get names {count} keys; at most {max} are allowed"
)]
struct KvTooManyKeys {
	count: usize,
	max: usize,
}

#[derive(RivetError, Debug, Serialize, Deserialize)]
#[error(
	"actor",
	"kv_key_too_large",
	"A KV key is too large.",
	"KV key is {bytes} bytes; at most {max} are allowed"
)]
struct KvKeyTooLarge {
	bytes: usize,
	max: usize,
}

#[derive(RivetError, Debug, Serialize, Deserialize)]
#[error(
	"actor",
	"kv_response_too_large",
	"The KV response is too large. Read fewer keys per request.",
	"KV get response passes {max_bytes} bytes; read fewer keys per request"
)]
struct KvResponseTooLarge {
	max_bytes: usize,
}

/// Builds the reply for a KV request that failed.
pub(crate) fn error_response(request_id: u32, message: String) -> p::ToEnvoy {
	p::ToEnvoy::ToEnvoyKvResponse(p::ToEnvoyKvResponse {
		request_id,
		data: p::KvResponseData::KvErrorResponse(p::KvErrorResponse {
			message: message.into(),
		}),
	})
}

/// A refused request's error, carried as `<group>.<code>: <message>` because the KV error
/// response has only a message.
fn refusal(error: anyhow::Error) -> p::KvResponseData {
	let error = RivetError::extract(&error);
	p::KvResponseData::KvErrorResponse(p::KvErrorResponse {
		message: format!("{}.{}: {}", error.group(), error.code(), error.message()).into(),
	})
}

fn get_response_too_large() -> anyhow::Error {
	KvResponseTooLarge {
		max_bytes: MAX_GET_RESPONSE_BYTES,
	}
	.build()
}

/// Refuses a get before any read when it names too many keys or a key that is too large.
/// Duplicate keys count toward the limit, as they are sent.
fn admit_get(keys: &[p::KvKey]) -> Result<()> {
	if keys.len() > MAX_KEYS {
		return Err(KvTooManyKeys {
			count: keys.len(),
			max: MAX_KEYS,
		}
		.build());
	}
	if let Some(key) = keys.iter().find(|key| key.len() > MAX_KEY_BYTES) {
		return Err(KvKeyTooLarge {
			bytes: key.len(),
			max: MAX_KEY_BYTES,
		}
		.build());
	}
	Ok(())
}

/// Runs one KV request on the actor's `kv` inode. Requests of one actor run in order on its
/// lane, and the lane's exclusive filesystem owner fences a stale engine.
pub(crate) async fn execute(
	env: &dyn rivet_env::Env,
	fs: &Arc<Filesystem>,
	ino: u64,
	data: p::KvRequestData,
) -> Result<p::KvResponseData> {
	match data {
		p::KvRequestData::KvGetRequest(get) => {
			if let Err(error) = admit_get(&get.keys) {
				return Ok(refusal(error));
			}

			let mut tx = fs.begin(ino).await?;

			// Each distinct key is read and returned once. Clients match values to the keys
			// they asked for, so a repeated key still gets its value.
			let mut rows = Rows::default();
			for key in get.keys.into_iter().collect::<BTreeSet<_>>() {
				let packed = ActorKvKey::new(key.clone()).pack();
				let stored = match tx.get(&packed).await {
					Ok(stored) => stored,
					Err(filesystem::Error::TooLarge) => {
						return Ok(refusal(get_response_too_large()));
					}
					Err(error) => return Err(error.into()),
				};
				if let Some(value) = stored {
					rows.push(key, value)?;
					if rows.bytes > MAX_GET_RESPONSE_BYTES {
						return Ok(refusal(get_response_too_large()));
					}
				}
			}

			Ok(p::KvResponseData::KvGetResponse(rows.into_get()))
		}
		p::KvRequestData::KvListRequest(list) => {
			let mut tx = fs.begin(ino).await?;

			let reverse = list.reverse.unwrap_or(false);
			let limit = list
				.limit
				.map(|limit| usize::try_from(limit).unwrap_or(usize::MAX));
			let mut rows = Rows::default();
			if let Some(range) = list_range(&list.query) {
				for (key, value) in scan(&mut tx, range, limit, reverse).await? {
					rows.push(key.into(), value)?;
				}
			}

			Ok(p::KvResponseData::KvListResponse(rows.into_list()))
		}
		p::KvRequestData::KvPutRequest(put) => {
			ensure!(
				put.keys.len() == put.values.len() && put.keys.len() <= MAX_KEYS,
				"invalid KV batch length"
			);
			ensure!(
				put.keys.iter().map(|key| key.len()).sum::<usize>()
					+ put.values.iter().map(|value| value.len()).sum::<usize>()
					<= MAX_PUT_BYTES,
				"KV batch too large"
			);
			ensure!(
				put.keys.iter().all(|key| key.len() <= MAX_KEY_BYTES),
				"KV key too large"
			);

			let mut tx = fs.begin(ino).await?;

			let now = env.clock().observe()?.unix_ms;
			let entries: BTreeMap<_, _> = put.keys.into_iter().zip(put.values).collect();
			for (key, value) in entries {
				let packed = ActorKvKey::new(Bytes::from(key)).pack();
				let value = wire::KvValue {
					value: value.into(),
					version: env
						.rng("actor.kv.version")
						.uuid_v4()
						.as_bytes()
						.to_vec()
						.into(),
					updated_unix_ms: now,
				};
				tx.put(
					&packed,
					&rivet_data::versioned::KvValue::serialize_with_embedded_version(
						&value,
						rivet_data::generated::PEGBOARD_ACTOR_VERSION,
					)?,
				)?;
			}
			tx.commit().await?;

			Ok(p::KvResponseData::KvPutResponse)
		}
		p::KvRequestData::KvDeleteRequest(delete) => {
			let mut tx = fs.begin(ino).await?;

			for key in delete.keys.into_iter().collect::<BTreeSet<_>>() {
				tx.delete(&ActorKvKey::new(Bytes::from(key)).pack())?;
			}
			tx.commit().await?;

			Ok(p::KvResponseData::KvDeleteResponse)
		}
		p::KvRequestData::KvDeleteRangeRequest(range) => {
			let start = ActorKvKey::new(range.start).pack();
			let end = ActorKvKey::new(range.end).pack();
			if start < end {
				let mut tx = fs.begin(ino).await?;
				tx.delete_range((
					Bound::Included(start.as_ref()),
					Bound::Excluded(end.as_ref()),
				))?;
				tx.commit().await?;
			}
			Ok(p::KvResponseData::KvDeleteResponse)
		}
		p::KvRequestData::KvDropRequest => {
			let range = ActorKvPrefix::new().range();
			let mut tx = fs.begin(ino).await?;
			tx.delete_range((
				Bound::Included(range.start.as_slice()),
				Bound::Excluded(range.end.as_slice()),
			))?;
			tx.commit().await?;
			Ok(p::KvResponseData::KvDropResponse)
		}
	}
}

/// Packed key bounds of a list query, or `None` when the query selects nothing. User keys
/// pack to tuple byte strings, which sort in the same order as the keys themselves, so a
/// user key range maps to one packed range and a prefix maps to the keys sharing its packed
/// form without the terminator.
fn list_range(query: &p::KvListQuery) -> Option<(Bound<Vec<u8>>, Bound<Vec<u8>>)> {
	match query {
		p::KvListQuery::KvListAllQuery => {
			let range = ActorKvPrefix::new().range();
			Some((Bound::Included(range.start), Bound::Excluded(range.end)))
		}
		p::KvListQuery::KvListPrefixQuery(query) => {
			let mut start = ActorKvKey::new(query.key.clone()).pack().to_vec();
			// The packed key ends in the tuple terminator; without it, it is a prefix of
			// every packed key whose user key starts with the query.
			start.pop();
			let end = next_prefix(&start)?;
			Some((Bound::Included(start), Bound::Excluded(end)))
		}
		p::KvListQuery::KvListRangeQuery(query) => {
			if query.start >= query.end {
				return None;
			}
			let start = ActorKvKey::new(query.start.clone()).pack().to_vec();
			let end = ActorKvKey::new(query.end.clone()).pack().to_vec();
			let start = if query.exclusive {
				Bound::Excluded(start)
			} else {
				Bound::Included(start)
			};
			Some((start, Bound::Excluded(end)))
		}
	}
}

/// The smallest byte string greater than every string starting with `prefix`.
fn next_prefix(prefix: &[u8]) -> Option<Vec<u8>> {
	let mut end = prefix.to_vec();
	while let Some(last) = end.pop() {
		if last < u8::MAX {
			end.push(last + 1);
			return Some(end);
		}
	}
	None
}

/// Reads up to `limit` rows of `range` in user key order. Only rows inside the range are
/// read, and the filesystem bounds the bytes one scan may hold.
async fn scan(
	tx: &mut Transaction,
	range: (Bound<Vec<u8>>, Bound<Vec<u8>>),
	limit: Option<usize>,
	reverse: bool,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let (start, end) = range;
	let bounds = (
		start.as_ref().map(|key| key.as_slice()),
		end.as_ref().map(|key| key.as_slice()),
	);
	let rows = tx.scan(bounds, limit, reverse).await?;

	rows.into_iter()
		.map(|(packed, value)| {
			let key = ActorKvKey::unpack(&packed).context("invalid actor KV key")?;
			Ok((key.key.to_vec(), value))
		})
		.collect()
}

#[derive(Default)]
struct Rows {
	keys: Vec<Bytes>,
	values: Vec<Bytes>,
	metadata: Vec<p::KvMetadata>,
	/// Bytes of keys, values and versions held so far.
	bytes: usize,
}

impl Rows {
	/// Adds one stored row. The value and its version are slices of `stored`, so the row's
	/// bytes reach the response without another copy.
	fn push(&mut self, key: Bytes, stored: Vec<u8>) -> Result<()> {
		let value = rivet_data::versioned::KvValue::decode(Bytes::from(stored))?;
		self.bytes = self
			.bytes
			.saturating_add(key.len())
			.saturating_add(value.value.len())
			.saturating_add(value.version.len());
		self.keys.push(key);
		self.values.push(value.value);
		self.metadata.push(p::KvMetadata {
			version: value.version,
			update_ts: value.updated_unix_ms,
		});
		Ok(())
	}

	fn into_get(self) -> p::KvGetResponse {
		p::KvGetResponse {
			keys: self.keys,
			values: self.values,
			metadata: self.metadata,
		}
	}

	fn into_list(self) -> p::KvListResponse {
		p::KvListResponse {
			keys: self.keys,
			values: self.values,
			metadata: self.metadata,
		}
	}
}


