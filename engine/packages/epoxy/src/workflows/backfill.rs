use anyhow::Result;
use epoxy_protocol::protocol;
use futures_util::{FutureExt, TryStreamExt};
use gas::prelude::*;
use serde::{Deserialize, Serialize};
use universaldb::{
	KeySelector, RangeOption,
	options::StreamingMode,
	utils::{
		FormalKey, IsolationLevel::Serializable,
		keys::{COMMITTED_VALUE, KV, VALUE},
	},
};

use crate::keys::{self, CommittedValue, KvValueKey, LegacyCommittedValueKey};

const DEFAULT_CHUNK_SIZE: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	#[serde(default)]
	pub chunk_size: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct State {
	after_key: Option<Vec<u8>>,
	migrated_keys: u64,
}

#[workflow]
pub async fn epoxy_backfill_v2(ctx: &mut WorkflowCtx, input: &Input) -> Result<u64> {
	let chunk_size = input.chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE).max(1);

	// This intentionally rebuilds v2 local committed values and changelog from legacy committed
	// data in the background after the workflow cutover. Until this completes, old committed values
	// are still readable locally through dual-read fallback, but they are not yet available to new
	// learners through the v2 changelog.
	let migrated_keys = ctx
		.loope(State::default(), |ctx, state| {
			async move {
				let output = ctx
					.activity(BackfillChunkInput {
						after_key: state.after_key.clone(),
						chunk_size,
					})
					.await?;

				state.migrated_keys += output.migrated_keys;

				if output.complete {
					return Ok(Loop::Break(state.migrated_keys));
				}

				state.after_key = output.next_after_key;
				Ok(Loop::Continue)
			}
			.boxed()
		})
		.await?;

	Ok(migrated_keys)
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct BackfillChunkInput {
	pub after_key: Option<Vec<u8>>,
	pub chunk_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillChunkOutput {
	pub next_after_key: Option<Vec<u8>>,
	pub migrated_keys: u64,
	pub complete: bool,
}

#[activity(BackfillChunk)]
pub async fn backfill_chunk(ctx: &ActivityCtx, input: &BackfillChunkInput) -> Result<BackfillChunkOutput> {
	let replica_id = ctx.config().epoxy_replica_id();

	ctx.udb()?
		.run(|tx| {
			let input = input.clone();
			async move {
				let legacy_subspace = keys::legacy_subspace(replica_id);
				let kv_subspace = legacy_subspace.subspace(&(KV,));
				let mut range: RangeOption<'static> = (&kv_subspace).into();
				range.mode = StreamingMode::WantAll;

				if let Some(after_key) = &input.after_key {
					let key_subspace = legacy_subspace.subspace(&(KV, after_key.clone()));
					let mut after_all_entries = key_subspace.pack(&());
					after_all_entries.push(0xFF);
					range.begin = KeySelector::first_greater_or_equal(after_all_entries);
				}

				let mut current_key: Option<Vec<u8>> = None;
				let mut current_value: Option<Vec<u8>> = None;
				let mut current_committed_value: Option<Vec<u8>> = None;
				let mut last_processed_key: Option<Vec<u8>> = None;
				let mut migrated_keys = 0_u64;
				let mut processed_keys = 0_usize;
				let mut complete = true;
				let mut stream = tx.get_ranges_keyvalues(range, Serializable);

				while let Some(entry) = stream.try_next().await? {
					let (root, key, leaf) =
						legacy_subspace.unpack::<(usize, Vec<u8>, usize)>(entry.key())?;
					if root != KV || (leaf != VALUE && leaf != COMMITTED_VALUE) {
						continue;
					}

					if let Some(existing_key) = &current_key {
						if existing_key != &key {
							migrated_keys += u64::from(
								migrate_legacy_key(
									&tx,
									replica_id,
									existing_key.clone(),
									current_value.take(),
									current_committed_value.take(),
								)
								.await?,
							);
							processed_keys += 1;
							last_processed_key = Some(existing_key.clone());

							if processed_keys >= input.chunk_size {
								complete = false;
								break;
							}
						}
					}

					if current_key.as_ref() != Some(&key) {
						current_key = Some(key);
						current_value = None;
						current_committed_value = None;
					}

					match leaf {
						VALUE => current_value = Some(entry.value().to_vec()),
						COMMITTED_VALUE => current_committed_value = Some(entry.value().to_vec()),
						_ => {}
					}
				}

				if complete {
					if let Some(current_key) = current_key {
						migrated_keys += u64::from(
							migrate_legacy_key(
								&tx,
								replica_id,
								current_key.clone(),
								current_value,
								current_committed_value,
							)
							.await?,
						);
						last_processed_key = Some(current_key);
					}
				}

				Ok(BackfillChunkOutput {
					next_after_key: last_processed_key,
					migrated_keys,
					complete,
				})
			}
		})
		.custom_instrument(tracing::info_span!("epoxy_backfill_chunk_tx", %replica_id))
		.await
}

async fn migrate_legacy_key(
	tx: &universaldb::Transaction,
	replica_id: protocol::ReplicaId,
	key: Vec<u8>,
	legacy_value: Option<Vec<u8>>,
	legacy_committed_value: Option<Vec<u8>>,
) -> Result<bool> {
	let Some(committed_value) = build_legacy_committed_value(
		key.clone(),
		legacy_value,
		legacy_committed_value,
	)? else {
		return Ok(false);
	};

	let v2_tx = tx.with_subspace(keys::subspace(replica_id));
	if v2_tx
		.read_opt(&KvValueKey::new(key.clone()), Serializable)
		.await?
		.is_some()
	{
		return Ok(false);
	}

	v2_tx.write(&KvValueKey::new(key.clone()), committed_value.clone())?;
	crate::replica::changelog::append(
		replica_id,
		tx,
		key,
		committed_value.value,
		committed_value.version,
		committed_value.mutable,
	)?;

	Ok(true)
}

fn build_legacy_committed_value(
	key: Vec<u8>,
	legacy_value: Option<Vec<u8>>,
	legacy_committed_value: Option<Vec<u8>>,
) -> Result<Option<CommittedValue>> {
	if let Some(raw) = legacy_committed_value {
		return Ok(Some(CommittedValue {
			value: LegacyCommittedValueKey::new(key).deserialize(&raw)?,
			version: 0,
			mutable: false,
		}));
	}

	if let Some(raw) = legacy_value {
		return Ok(Some(KvValueKey::new(key).deserialize(&raw)?));
	}

	Ok(None)
}
