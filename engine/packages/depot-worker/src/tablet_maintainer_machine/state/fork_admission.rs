//! Exact reservation evidence precedes the first lifecycle action in a grant.
use super::MaintainerState;
use depot_fs_core::{FsVolumeRecord, forks::LifecycleLookup};
use depot_types::VolumeKey;

/// None means a durable preparation owns this name. Expiry or a cached replica
/// observation never makes that reservation disappear. Background discovery or
/// an identical fork retry will continue it; ordinary lifecycle callers retry.
pub(crate) fn lookup(
	state: &mut MaintainerState,
	key: &VolumeKey,
	result: Result<LifecycleLookup, String>,
) -> Option<Result<Option<FsVolumeRecord>, String>> {
	let lookup = match result {
		Ok(lookup) => lookup,
		Err(error) => return Some(Err(error)),
	};
	if lookup
		.row
		.as_ref()
		.is_some_and(|row| row.volume_key() != *key)
		|| lookup.pending.as_ref().is_some_and(|pending| {
			pending.spec.destination_volume_key != *key
				|| !depot_fs_core::forks::valid_spec(&pending.spec, state.config.local_region)
		}) {
		return Some(Err("lifecycle lookup address mismatch".into()));
	}
	if lookup.pending.is_some() {
		state.fork_checked.remove(key);
		return None;
	}
	// This is an absence cache, not an authority registry. Eviction only costs
	// another bounded point lookup; keeping names after unload cannot grow it.
	if state.fork_checked.len() >= state.config.max_loaded_volumes as usize {
		state.fork_checked.clear();
	}
	state.fork_checked.insert(key.clone());
	Some(Ok(lookup.row))
}
