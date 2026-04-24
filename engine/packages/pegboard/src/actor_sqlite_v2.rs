//! Per-actor SQLite VFS v2 storage helpers.
//!
//! SQLite v2 data lives outside the actor KV subspace at
//! `pegboard::keys::subspace() / "sqlite-storage" / [0x02, actor_id_bytes, ...]`.
//! These helpers expose enough of that layout for export/import flows to scan
//! and rewrite a single actor's storage without depending on `pegboard-envoy`.

use std::sync::atomic::AtomicUsize;

use anyhow::Result;
use gas::prelude::Id;
use sqlite_storage::{
	keys::actor_prefix,
	udb::{WriteOp, apply_write_ops, scan_prefix_values},
};
use universaldb::utils::Subspace;

/// Subspace that holds every actor's SQLite v2 storage.
pub fn sqlite_subspace() -> Subspace {
	crate::keys::subspace().subspace(&("sqlite-storage",))
}

/// Returns all `(key_suffix, value)` pairs for the given actor's SQLite v2
/// storage. `key_suffix` is the bytes after the actor-scoped prefix, i.e.
/// `/META`, `/SHARD/<u32>`, `/DELTA/<u64>/<u32>`, `/PIDX/delta/<u32>`.
#[tracing::instrument(skip_all, fields(%actor_id))]
pub async fn export_actor(
	db: &universaldb::Database,
	actor_id: Id,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let subspace = sqlite_subspace();
	let actor_id_str = actor_id.to_string();
	let prefix = actor_prefix(&actor_id_str);
	let prefix_len = prefix.len();

	let op_counter = AtomicUsize::new(0);
	let entries = scan_prefix_values(db, &subspace, &op_counter, prefix).await?;

	Ok(entries
		.into_iter()
		.map(|(full_key, value)| (full_key[prefix_len..].to_vec(), value))
		.collect())
}

/// Writes the given `(key_suffix, value)` pairs under the target actor's
/// SQLite v2 prefix, overwriting any existing values at those keys.
#[tracing::instrument(skip_all, fields(%actor_id, entry_count = entries.len()))]
pub async fn import_actor(
	db: &universaldb::Database,
	actor_id: Id,
	entries: Vec<(Vec<u8>, Vec<u8>)>,
) -> Result<()> {
	if entries.is_empty() {
		return Ok(());
	}

	let subspace = sqlite_subspace();
	let actor_id_str = actor_id.to_string();
	let prefix = actor_prefix(&actor_id_str);

	let ops: Vec<WriteOp> = entries
		.into_iter()
		.map(|(suffix, value)| {
			let mut full_key = prefix.clone();
			full_key.extend_from_slice(&suffix);
			WriteOp::put(full_key, value)
		})
		.collect();

	let op_counter = AtomicUsize::new(0);
	apply_write_ops(db, &subspace, &op_counter, ops).await
}
