use std::{
	collections::HashMap,
	result::Result::{Err, Ok},
	sync::Arc,
};

use anyhow::*;
use deno_core::JsBuffer;
pub use entry::Entry;
use entry::{EntryBuilder, SubKey};
use foundationdb::{self as fdb, directory::Directory, tuple::Subspace};
use futures_util::{StreamExt, TryStreamExt};
use indexmap::IndexMap;
use key::Key;
use uuid::Uuid;
use list_query::ListLimitReached;
pub use list_query::ListQuery;
pub use metadata::Metadata;
use prost::Message;
use utils::{validate_entries, validate_keys, TransactionExt};

mod entry;
pub mod key;
mod list_query;
mod metadata;
mod utils;

const MAX_KEY_SIZE: usize = 2 * 1024;
const MAX_VALUE_SIZE: usize = 128 * 1024;
const MAX_KEYS: usize = 128;
const MAX_PUT_PAYLOAD_SIZE: usize = 976 * 1024;
const MAX_STORAGE_SIZE: usize = 1024 * 1024 * 1024; // 1 GiB
const VALUE_CHUNK_SIZE: usize = 10_000; // 10 KB, not KiB, see https://apple.github.io/foundationdb/blob.html

// Currently designed largely around the Deno runtime. More abstractions can be made later.
pub struct ActorKv {
	version: &'static str,
	db: Arc<fdb::Database>,
	actor_id: Uuid,
	subspace: Option<Subspace>,
}

impl ActorKv {
	pub fn new(db: Arc<fdb::Database>, actor_id: Uuid) -> Self {
		Self {
			version: env!("CARGO_PKG_VERSION"),
			db,
			actor_id,
			subspace: None,
		}
	}

	/// Initializes actor's KV.
	///
	/// If FDB is down, this will hang indefinitely until connected.
	pub async fn init(&mut self) -> Result<()> {
		tracing::info!("initializing actor KV");

		let root = fdb::directory::DirectoryLayer::default();

		let tx = self.db.create_trx()?;
		let actor_dir = root
			.create_or_open(
				&tx,
				// Should match `destroy` fn
				&[
					"pegboard".into(),
					"actor".into(),
					self.actor_id.to_string(),
				],
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

		self.subspace = Some(Subspace::from_bytes(kv_dir.bytes().map_err(|err| anyhow!("{err:?}"))?));

		tracing::info!("successfully initialized KV");

		Ok(())
	}

	/// Returns estimated size of the given subspace.
	pub async fn get_subspace_size(&self, subspace: &Subspace) -> Result<i64> {
		let (start, end) = subspace.range();

		// This txn does not have to be committed because we are not modifying any data
		let tx = self.db.create_trx()?;
		tx.get_estimated_range_size_bytes(&start, &end)
			.await
			.map_err(Into::into)
	}

	/// Gets keys from the KV store.
	pub async fn get(&self, keys: Vec<Key>) -> Result<HashMap<Key, Entry>> {
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

												Ok((key.clone(), SubKey::Metadata(value)))
											} else {
												// Parse sub key as idx
												let (_, idx) = key_subspace
													.unpack::<(String, usize)>(value.key())?;

												Ok((key.clone(), SubKey::Chunk(idx, value)))
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
						.try_fold(HashMap::new(), |mut acc, (key, sub_key)| async {
							acc.entry(key)
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
	) -> Result<IndexMap<Key, Entry>> {
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
									subspace.unpack::<(Key, String)>(value.key())
								{
									if sub_key != "metadata" {
										bail!("unexpected sub key: {sub_key:?}");
									}

									Ok((key, SubKey::Metadata(value)))
								} else {
									// Parse sub key as idx
									let (key, _, idx) =
										subspace.unpack::<(Key, String, usize)>(value.key())?;

									Ok((key, SubKey::Chunk(idx, value)))
								}
							}
							Err(err) => Err(err.into()),
						}
					});

					// With a limit, we short circuit out of the `try_fold` once the limit is reached
					if let Some(limit) = limit {
						stream
							.try_fold(IndexMap::new(), |mut acc, (key, sub_key)| async {
								let size = acc.len();
								let entry = acc.entry(key);

								// Short circuit when limit is reached. This relies on data from the stream
								// being in order.
								if size == limit && matches!(entry, indexmap::map::Entry::Vacant(_))
								{
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
							.try_fold(IndexMap::new(), |mut acc, (key, sub_key)| async {
								acc.entry(key)
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
	pub async fn put(&self, entries: HashMap<Key, JsBuffer>) -> Result<()> {
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
									kv_version: self.version.as_bytes().to_vec(),
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

	/// Deletes keys from the KV store.
	pub async fn delete(&self, keys: Vec<Key>) -> Result<()> {
		let subspace = self
			.subspace
			.as_ref()
			.context("must call `ActorKv::init` before using KV operations")?;

		validate_keys(&keys)?;

		self.db
			.run(|tx, _mc| {
				let keys = keys.clone();
				async move {
					for key in keys {
						let key_subspace = subspace.subspace(&key);

						tx.clear_subspace_range(&key_subspace);
					}

					Ok(())
				}
			})
			.await
			.map_err(Into::into)
	}

	/// Deletes all keys from the KV store.
	pub async fn delete_all(&self) -> Result<()> {
		let subspace = self
			.subspace
			.as_ref()
			.context("must call `ActorKv::init` before using KV operations")?;

		self.db
			.run(|tx, _mc| async move {
				tx.clear_subspace_range(&subspace);
				Ok(())
			})
			.await
			.map_err(Into::into)
	}

	/// **Destroys entire actor's KV. Cannot be undone.**
	pub async fn destroy(self) -> Result<()> {
		let root = fdb::directory::DirectoryLayer::default();

		let tx = self.db.create_trx()?;
		root.remove_if_exists(
			&tx,
			&[
				"pegboard".into(),
				"actor".into(),
				self.actor_id.to_string(),
			],
		)
		.await
		.map_err(|err| anyhow!("{err:?}"))?;
		tx.commit().await.map_err(|err| anyhow!("{err:?}"))?;

		Ok(())
	}
}
