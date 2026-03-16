use anyhow::Result;
use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use rivet_types::actors::Actor;
use std::collections::HashMap;
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;

use crate::{errors, keys};

pub const MAX_KEY_SIZE: usize = 128;
pub const MAX_VALUE_SIZE: usize = 4096;
pub const MAX_TOTAL_SIZE: usize = 16 * 1024;
pub const MAX_LIST_KEYS: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct PatchEntry {
	pub key: String,
	pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Projection {
	None,
	Selected(Vec<String>),
	Full,
}

impl Projection {
	pub fn requested(&self) -> bool {
		!matches!(self, Projection::None)
	}
}

impl Default for Projection {
	fn default() -> Self {
		Self::None
	}
}

pub fn validate_projection_keys(keys: &[String]) -> Result<()> {
	if keys.len() > MAX_LIST_KEYS {
		return Err(errors::Actor::MetadataTooManyKeys {
			max: MAX_LIST_KEYS,
			count: keys.len(),
		}
		.build());
	}

	for key in keys {
		validate_key(key)?;
	}

	Ok(())
}

pub fn validate_patch(patch: &[PatchEntry]) -> Result<()> {
	if patch.is_empty() {
		return Err(errors::Actor::MetadataPatchEmpty.build());
	}

	for entry in patch {
		validate_key(&entry.key)?;

		if let Some(value) = &entry.value
			&& value.as_bytes().len() > MAX_VALUE_SIZE
		{
			return Err(errors::Actor::MetadataValueTooLarge {
				max_size: MAX_VALUE_SIZE,
				key_preview: util::safe_slice(&entry.key, 0, MAX_KEY_SIZE).to_string(),
			}
			.build());
		}
	}

	Ok(())
}

pub async fn get(
	db: &universaldb::Database,
	actor_ids: &[Id],
	projection: &Projection,
) -> Result<HashMap<Id, HashMap<String, String>>> {
	if actor_ids.is_empty() || !projection.requested() {
		return Ok(HashMap::new());
	}

	match projection {
		Projection::None => Ok(HashMap::new()),
		Projection::Selected(keys) => get_selected(db, actor_ids, keys).await,
		Projection::Full => get_full(db, actor_ids).await,
	}
}

pub async fn attach_to_actors(
	db: &universaldb::Database,
	actors: &mut [Actor],
	projection: &Projection,
) -> Result<()> {
	if actors.is_empty() || !projection.requested() {
		return Ok(());
	}

	let actor_ids = actors
		.iter()
		.map(|actor| actor.actor_id)
		.collect::<Vec<_>>();
	let metadata = get(db, &actor_ids, projection).await?;

	for actor in actors {
		actor.metadata = Some(metadata.get(&actor.actor_id).cloned().unwrap_or_default());
	}

	Ok(())
}

pub async fn apply_patch(
	db: &universaldb::Database,
	actor_id: Id,
	patch: &[PatchEntry],
) -> Result<HashMap<String, String>> {
	validate_patch(patch)?;

	db.run(|tx| {
		let patch = patch.to_vec();
		async move {
			let tx = tx.with_subspace(keys::subspace());
			let subspace = keys::actor_metadata::subspace(actor_id);
			let mut current = read_all_for_actor(&tx, actor_id).await?;

			for entry in &patch {
				match &entry.value {
					Some(value) => {
						current.insert(entry.key.clone(), value.clone());
					}
					None => {
						current.remove(&entry.key);
					}
				}
			}

			validate_total_size(&current)?;

			for entry in patch {
				let entry_key = keys::actor_metadata::EntryKey::new(actor_id, entry.key);

				match entry.value {
					Some(value) => {
						tx.write(&entry_key, value)?;
					}
					None => {
						tx.delete(&entry_key);
					}
				}
			}

			// Clear the entire subspace if the patch deleted every key.
			if current.is_empty() {
				tx.clear_subspace_range(&subspace);
			}

			Ok(current)
		}
	})
	.await
}

async fn get_selected(
	db: &universaldb::Database,
	actor_ids: &[Id],
	selected_keys: &[String],
) -> Result<HashMap<Id, HashMap<String, String>>> {
	db.run(|tx| {
		let actor_ids = actor_ids.to_vec();
		let selected_keys = selected_keys.to_vec();
		async move {
			let tx = tx.with_subspace(keys::subspace());

			let pairs = futures_util::stream::iter(actor_ids.into_iter().flat_map(|actor_id| {
				selected_keys
					.iter()
					.cloned()
					.map(move |key| (actor_id, key))
					.collect::<Vec<_>>()
			}))
			.map(|(actor_id, key)| {
				let tx = tx.clone();
				async move {
					let entry_key = keys::actor_metadata::EntryKey::new(actor_id, key.clone());
					let value = tx.read_opt(&entry_key, Serializable).await?;
					Ok::<_, anyhow::Error>(value.map(|value| (actor_id, key, value)))
				}
			})
			.buffer_unordered(256)
			.try_filter_map(|entry| std::future::ready(Ok(entry)))
			.try_collect::<Vec<_>>()
			.await?;

			let mut metadata = HashMap::<Id, HashMap<String, String>>::new();
			for (actor_id, key, value) in pairs {
				metadata.entry(actor_id).or_default().insert(key, value);
			}

			Ok(metadata)
		}
	})
	.await
}

async fn get_full(
	db: &universaldb::Database,
	actor_ids: &[Id],
) -> Result<HashMap<Id, HashMap<String, String>>> {
	db.run(|tx| {
		let actor_ids = actor_ids.to_vec();
		async move {
			let tx = tx.with_subspace(keys::subspace());

			futures_util::stream::iter(actor_ids)
				.map(|actor_id| {
					let tx = tx.clone();
					async move {
						Ok::<_, anyhow::Error>((actor_id, read_all_for_actor(&tx, actor_id).await?))
					}
				})
				.buffer_unordered(128)
				.try_collect::<Vec<_>>()
				.await
				.map(|entries| entries.into_iter().collect())
		}
	})
	.await
}

async fn read_all_for_actor(
	tx: &universaldb::Transaction,
	actor_id: Id,
) -> Result<HashMap<String, String>> {
	let subspace = keys::actor_metadata::subspace(actor_id);
	let mut stream = tx.get_ranges_keyvalues(
		universaldb::RangeOption {
			mode: StreamingMode::Iterator,
			..subspace.range().into()
		},
		Serializable,
	);
	let mut metadata = HashMap::new();

	while let Some(entry) = stream.try_next().await? {
		let key = tx
			.unpack::<keys::actor_metadata::EntryKey>(&entry.key())?
			.key;
		let value = String::from_utf8(entry.value().to_vec())?;
		metadata.insert(key, value);
	}

	Ok(metadata)
}

fn validate_key(key: &str) -> Result<()> {
	if key.is_empty() {
		return Err(errors::Actor::MetadataKeyInvalid {
			key_preview: String::new(),
		}
		.build());
	}

	if key.as_bytes().len() > MAX_KEY_SIZE {
		return Err(errors::Actor::MetadataKeyTooLarge {
			max_size: MAX_KEY_SIZE,
			key_preview: util::safe_slice(key, 0, MAX_KEY_SIZE).to_string(),
		}
		.build());
	}

	if !key
		.as_bytes()
		.iter()
		.all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-'))
	{
		return Err(errors::Actor::MetadataKeyInvalid {
			key_preview: util::safe_slice(key, 0, MAX_KEY_SIZE).to_string(),
		}
		.build());
	}

	Ok(())
}

fn validate_total_size(metadata: &HashMap<String, String>) -> Result<()> {
	let total_size = metadata.iter().fold(0usize, |sum, (key, value)| {
		sum + key.as_bytes().len() + value.as_bytes().len()
	});

	if total_size > MAX_TOTAL_SIZE {
		return Err(errors::Actor::MetadataTooLarge {
			max_size: MAX_TOTAL_SIZE,
		}
		.build());
	}

	Ok(())
}
