//! Shared readings of `CommandStartActor.preloadedKv` that the engine and runners agree on.
//!
//! `requestedPrefixes` lists every prefix the sender scanned in full, and `entries` holds every
//! key it found under them. A scan of the empty prefix covers the whole actor keyspace.

use crate::PreloadedKv;

/// The preload of an actor whose KV keyspace is empty: the whole keyspace was scanned and held
/// no entries.
///
/// The engine sends it on an actor's first generation, before any runner has written to the
/// actor's storage. A runner may then skip its startup probes of existing storage. Older
/// runners that ignore the field, or read it as a plain preload, still see a true statement.
pub fn empty_keyspace() -> PreloadedKv {
	PreloadedKv {
		entries: Vec::new(),
		requested_get_keys: Vec::new(),
		requested_prefixes: vec![Vec::new().into()],
	}
}

/// Whether `preloaded` states that the actor's whole KV keyspace is empty.
pub fn is_empty_keyspace(preloaded: &PreloadedKv) -> bool {
	let whole_keyspace_scanned = preloaded
		.requested_prefixes
		.iter()
		.any(|prefix| prefix.is_empty());

	whole_keyspace_scanned && preloaded.entries.is_empty()
}
