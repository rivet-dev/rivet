use std::{
	collections::{hash_map, HashMap},
	result::Result::{Err, Ok},
};

use anyhow::*;
use deno_core::{JsBuffer, ToJsBuffer};
use foundationdb::{self as fdb, directory::Directory};
use futures_util::{StreamExt, TryStreamExt};
use metadata::Metadata;
use prost::Message;
use serde::{Deserialize, Serialize};
use utils::TransactionExt;
use uuid::Uuid;

mod metadata;
mod utils;

const MAX_KEY_SIZE: usize = 2 * 1024;
const MAX_VALUE_SIZE: usize = 128 * 1024;
const MAX_KEYS: usize = 128;
const MAX_PUT_PAYLOAD_SIZE: usize = 976 * 1024;
const MAX_STORAGE_SIZE: usize = 1024 * 1024 * 1024; // 1 GiB
const VALUE_CHUNK_SIZE: usize = 1000; // 1 KB, not KiB

pub struct ActorKv {
	version: &'static str,
	db: fdb::Database,
	actor_id: Uuid,
	subspace: Option<fdb::tuple::Subspace>,
}

impl ActorKv {
	pub fn new(db: fdb::Database, actor_id: Uuid) -> Self {
		Self {
			version: env!("CARGO_PKG_VERSION"),
			db,
			actor_id,
			subspace: None,
		}
	}

	pub async fn init(&mut self) -> Result<()> {
		let root = fdb::directory::DirectoryLayer::default();

		let tx = self.db.create_trx()?;
		let actor_dir = root
			.create_or_open(
				&tx,
				&["pegboard".into(), self.actor_id.into()],
				None,
				Some(b"partition"),
			)
			.await
			.map_err(|err| anyhow!("{err:?}"))?;
		let kv_dir = actor_dir
			.create_or_open(&tx, &["kv".into()], None, None)
			.await
			.map_err(|err| anyhow!("{err:?}"))?;
		tx.commit().await.map_err(|err| anyhow!("{err:?}"))?;

		self.subspace = Some(kv_dir.subspace(&()).map_err(|err| anyhow!("{err:?}"))?);

		Ok(())
	}

	/// Returns estimated size of the given subspace.
	pub async fn get_subspace_size(&self, subspace: &fdb::tuple::Subspace) -> Result<i64> {
		let (start, end) = subspace.range();

		// This txn does not have to be committed because we are not modifying any data
		let tx = self.db.create_trx()?;
		tx.get_estimated_range_size_bytes(&start, &end)
			.await
			.map_err(Into::into)
	}

	/// Gets keys from the KV store.
	pub async fn get(&self, keys: Vec<String>) -> Result<HashMap<String, Entry>> {
		let subspace = self
			.subspace
			.as_ref()
			.context("must call `ActorKv::init` before using KV operations")?;

		validate_keys(&keys)?;

		self.db
			.run(|tx, _mc| {
				let keys = keys.clone();
				async move {
					futures_util::stream::iter(keys)
						.map(|key| {
							let tx = tx.clone();
							let key_subspace = subspace.subspace(&key);

							async move {
								// Get all sub keys in the key subspace
								let stream = tx.get_ranges_keyvalues_owned(
									fdb::RangeOption {
										mode: fdb::options::StreamingMode::WantAll,
										..key_subspace.range().into()
									},
									false,
								);

								stream.map(move |res| {
									match res {
										Ok(value) => {
											// Parse key as string
											if let Ok(sub_key) =
												key_subspace.unpack::<String>(value.key())
											{
												if sub_key != "metadata" {
													bail!("unexpected sub key: {sub_key:?}");
												}

												Ok(SubKey {
													key: key.clone(),
													data: SubKeyData::Metadata(value),
												})
											} else {
												// Parse sub key as idx
												let (_, idx) = key_subspace
													.unpack::<(String, usize)>(value.key())?;

												Ok(SubKey {
													key: key.clone(),
													data: SubKeyData::Chunk(idx, value),
												})
											}
										}
										Err(err) => Err(err.into()),
									}
								})
							}
						})
						// Should remain in order
						.buffered(32)
						.flatten()
						.try_fold(HashMap::new(), |mut acc, sub_key| async {
							acc.entry(sub_key.key.clone())
								.or_insert_with(EntryBuilder::default)
								.add_sub_key(sub_key)?;

							Ok(acc)
						})
						.await
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))
				}
			})
			.await
			.map_err(Into::<anyhow::Error>::into)?
			.into_iter()
			.map(|(key, builder)| {
				let entry = builder.build(&key)?;

				Ok((key, entry))
			})
			.collect()
	}

	/// Gets keys from the KV store.
	pub async fn list(
		&self,
		query: ListQuery,
		reverse: bool,
		limit: Option<usize>,
	) -> Result<HashMap<String, Entry>> {
		let subspace = self
			.subspace
			.as_ref()
			.context("must call `ActorKv::init` before using KV operations")?;

		query.validate()?;

		let list_range = query.range(&subspace);

		let res = self
			.db
			.run(|tx, _mc| {
				let list_range = list_range.clone();

				async move {
					// Get all sub keys in the key subspace
					let stream = tx.get_ranges_keyvalues_owned(
						fdb::RangeOption {
							mode: if limit.is_some() {
								fdb::options::StreamingMode::Iterator
							} else {
								fdb::options::StreamingMode::WantAll
							},
							reverse,
							..list_range.into()
						},
						false,
					);

					let stream = stream.map(move |res| {
						match res {
							Ok(value) => {
								// Parse key as string
								if let Ok((key, sub_key)) =
									subspace.unpack::<(String, String)>(value.key())
								{
									if sub_key != "metadata" {
										bail!("unexpected sub key: {sub_key:?}");
									}

									Ok(SubKey {
										key,
										data: SubKeyData::Metadata(value),
									})
								} else {
									// Parse sub key as idx
									let (key, _, idx) =
										subspace.unpack::<(String, String, usize)>(value.key())?;

									Ok(SubKey {
										key,
										data: SubKeyData::Chunk(idx, value),
									})
								}
							}
							Err(err) => Err(err.into()),
						}
					});

					if let Some(limit) = limit {
						stream
							.try_fold(HashMap::new(), |mut acc, sub_key| async {
								let size = acc.len();
								let entry = acc.entry(sub_key.key.clone());

								// Short circuit when limit is reached. This relies on data from the stream being
								// in order.
								if size == limit && matches!(entry, hash_map::Entry::Vacant(_)) {
									return Err(ListLimitReached(acc).into());
								}

								entry
									.or_insert_with(EntryBuilder::default)
									.add_sub_key(sub_key)?;

								Ok(acc)
							})
							.await
							// The downcast further down doesn't work without this downcast, I have no idea why
							.map_err(|x| match x.downcast::<ListLimitReached>() {
								Ok(err) => fdb::FdbBindingError::CustomError(err.into()),
								Err(err) => fdb::FdbBindingError::CustomError(err.into()),
							})
					} else {
						stream
							.try_fold(HashMap::new(), |mut acc, sub_key| async {
								acc.entry(sub_key.key.clone())
									.or_insert_with(EntryBuilder::default)
									.add_sub_key(sub_key)?;

								Ok(acc)
							})
							.await
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))
					}
				}
			})
			.await;

		let values = match res {
			Ok(values) => values,
			Err(fdb::FdbBindingError::CustomError(err)) => {
				let ListLimitReached(values) = *err
					.downcast::<ListLimitReached>()
					.map_err(fdb::FdbBindingError::CustomError)?;

				values
			}
			Err(err) => return Err(err.into()),
		};

		values
			.into_iter()
			.map(|(key, builder)| {
				let entry = builder.build(&key)?;

				Ok((key, entry))
			})
			.collect()
	}

	/// Puts keys into the KV store.
	pub async fn put(&self, entries: HashMap<String, JsBuffer>) -> Result<()> {
		let subspace = self
			.subspace
			.as_ref()
			.context("must call `ActorKv::init` before using KV operations")?;
		let total_size = self.get_subspace_size(subspace).await? as usize;

		validate_entries(&entries, total_size)?;

		self.db
			.run(|tx, _mc| {
				// TODO: Potentially costly clone
				let entries = entries.clone();
				let subspace = subspace.clone();

				async move {
					futures_util::stream::iter(entries)
						.map(|(key, value)| {
							let tx = tx.clone();
							let key_subspace = subspace.subspace(&key);

							async move {
								// Clear previous before setting
								tx.clear_subspace_range(&key_subspace);

								let metadata = Metadata {
									kv_version: self.version.to_string(),
									create_ts: utils::now(),
								};
								let mut buf = Vec::new();
								metadata
									.encode(&mut buf)
									.map_err(|err| fdb::FdbBindingError::CustomError(err.into()))?;

								// Set metadata
								tx.set(&key_subspace.pack(&"metadata"), &buf);

								// Set data
								for start in (0..value.len()).step_by(VALUE_CHUNK_SIZE) {
									let idx = start / VALUE_CHUNK_SIZE;
									let end = (start + VALUE_CHUNK_SIZE).min(value.len());

									tx.set(
										&key_subspace.pack(&("data", idx)),
										&value.get(start..end).context("bad slice").map_err(
											|err| fdb::FdbBindingError::CustomError(err.into()),
										)?,
									);
								}

								Ok(())
							}
						})
						.buffer_unordered(32)
						.try_collect()
						.await
				}
			})
			.await
			.map_err(Into::into)
	}

	/// Deletes keys from the KV store. Returns true for keys that existed before deletion.
	pub async fn delete(&self, keys: Vec<String>) -> Result<HashMap<String, bool>> {
		let subspace = self
			.subspace
			.as_ref()
			.context("must call `ActorKv::init` before using KV operations")?;

		validate_keys(&keys)?;

		self.db
			.run(|tx, _mc| {
				let keys = keys.clone();

				async move {
					futures_util::stream::iter(keys)
						.map(|key| async {
							let key_subspace = subspace.subspace(&key);

							let existed = tx
								.get(&key_subspace.pack(&"metadata"), false)
								.await?
								.is_some();
							tx.clear_subspace_range(&key_subspace);

							Ok((key, existed))
						})
						.buffer_unordered(32)
						.try_collect()
						.await
				}
			})
			.await
			.map_err(Into::into)
	}

	/// **Destroys entire actor's KV. Cannot be undone.**
	pub async fn destroy(self) -> Result<()> {
		let root = fdb::directory::DirectoryLayer::default();

		let tx = self.db.create_trx()?;
		root.remove_if_exists(&tx, &["pegboard".into(), self.actor_id.into()])
			.await
			.map_err(|err| anyhow!("{err:?}"))?;
		tx.commit().await.map_err(|err| anyhow!("{err:?}"))?;

		Ok(())
	}
}

#[derive(Default)]
struct EntryBuilder {
	metadata: Option<Metadata>,
	value: Vec<u8>,
	next_idx: usize,
}

impl EntryBuilder {
	fn add_sub_key(&mut self, sub_key: SubKey) -> Result<()> {
		match sub_key.data {
			SubKeyData::Metadata(value) => {
				// We ignore setting the metadata again because it means the same key was given twice in the
				// input keys for `ActorKv::get`. We don't perform automatic deduplication.
				if self.metadata.is_none() {
					self.metadata = Some(Metadata::decode(value.value())?);
				}
			}
			SubKeyData::Chunk(idx, value) => {
				// We don't perform deduplication on the input keys for `ActorKv::get` so we might have
				// duplicate data chunks. This idx check ignores chunks that were already passed and ensures
				// contiguity.
				if idx == self.next_idx {
					self.value.extend(value.value());
					self.next_idx = idx + 1;
				}
			}
		}

		Ok(())
	}

	fn build(self, key: &str) -> Result<Entry> {
		ensure!(!self.value.is_empty(), "empty value at key {key:?}");

		Ok(Entry {
			metadata: self
				.metadata
				.with_context(|| format!("no metadata for key {key:?}"))?,
			value: self.value.into(),
		})
	}
}

/// Represents a Rivet KV value.
#[derive(Serialize)]
pub struct Entry {
	metadata: Metadata,
	value: ToJsBuffer,
}

/// Represents FDB keys within a Rivet KV key.
struct SubKey {
	key: String,
	data: SubKeyData,
}

enum SubKeyData {
	Metadata(fdb::future::FdbValue),
	Chunk(usize, fdb::future::FdbValue),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListQuery {
	All,
	RangeInclusive(String, String),
	RangeExclusive(String, String),
	Prefix(String),
}

impl ListQuery {
	fn range(&self, subspace: &fdb::tuple::Subspace) -> (Vec<u8>, Vec<u8>) {
		match self {
			ListQuery::All => subspace.range(),
			ListQuery::RangeInclusive(start, end) => (
				subspace.subspace(&start).range().0,
				subspace.subspace(&end).range().1,
			),
			ListQuery::RangeExclusive(start, end) => (
				subspace.subspace(&start).range().0,
				subspace.subspace(&end).range().1,
			),
			ListQuery::Prefix(prefix) => subspace.subspace(&prefix).range(),
		}
	}

	fn validate(&self) -> Result<()> {
		match self {
			ListQuery::All => {}
			ListQuery::RangeInclusive(start, end) => {
				ensure!(
					start.len() <= MAX_KEY_SIZE,
					"start key is too long (max 2048 bytes)"
				);
				ensure!(
					end.len() <= MAX_KEY_SIZE,
					"end key is too long (max 2048 bytes)"
				);
			}
			ListQuery::RangeExclusive(start, end) => {
				ensure!(
					start.len() <= MAX_KEY_SIZE,
					"startAfter key is too long (max 2048 bytes)"
				);
				ensure!(
					end.len() <= MAX_KEY_SIZE,
					"end key is too long (max 2048 bytes)"
				);
			}
			ListQuery::Prefix(prefix) => {
				ensure!(
					prefix.len() <= MAX_KEY_SIZE,
					"prefix key is too long (max 2048 bytes)"
				);
			}
		}

		Ok(())
	}
}

// Used to short circuit after the
struct ListLimitReached(HashMap<String, EntryBuilder>);

impl std::fmt::Debug for ListLimitReached {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ListLimitReached")
	}
}

impl std::fmt::Display for ListLimitReached {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "List limit reached")
	}
}

impl std::error::Error for ListLimitReached {}

fn validate_keys(keys: &[String]) -> Result<()> {
	ensure!(keys.len() <= MAX_KEYS, "a maximum of 128 keys is allowed");

	for key in keys {
		ensure!(
			key.len() <= MAX_KEY_SIZE,
			"key is too long (max 2048 bytes)"
		);
	}

	Ok(())
}

fn validate_entries(entries: &HashMap<String, JsBuffer>, total_size: usize) -> Result<()> {
	ensure!(
		entries.len() <= MAX_KEYS,
		"A maximum of 128 key-value entries is allowed"
	);
	let payload_size = entries
		.iter()
		.fold(0, |acc, (k, v)| acc + k.len() + v.len());
	ensure!(
		payload_size <= MAX_PUT_PAYLOAD_SIZE,
		"total payload is too large (max 976 KiB)"
	);

	let storage_remaining = MAX_STORAGE_SIZE.saturating_sub(total_size);
	ensure!(
		payload_size <= storage_remaining,
		"not enough space left in storage ({storage_remaining} bytes remaining, current payload is {payload_size} bytes)"
	);

	for (key, value) in entries {
		ensure!(
			key.len() <= MAX_KEY_SIZE,
			"key is too long (max 2048 bytes)"
		);
		ensure!(
			value.len() <= MAX_VALUE_SIZE,
			"value for key {key:?} is too large (max 128 KiB)"
		);
	}

	Ok(())
}
