// Each Epoxy key has its own record volume on each replica.
rivet_keys::volume_key! {
	pub EpoxyKey(replica_id: u64, key: bytes::Bytes) => [EPOXY_KEY, {replica_id}, {key}];
}

// A key whose caller placed it: its volume is placed where the key's own bytes say, next to
// the data the key guards. Depot tells volumes apart by placement and name, so the name keeps
// it apart from the caller's own volumes at that placement.
rivet_keys::volume_key! {
	pub PlacedEpoxyKey(placement: [u8; 16], replica_id: u64, key: bytes::Bytes) => [PLACED, {placement}, EPOXY_KEY, {replica_id}, {key}] name [EPOXY_KEY];
}

/// The volume that holds `key`'s record on `replica_id`. Every reader and writer of a key's
/// record finds its volume here. A logical key that starts with a placement, as a placed formal
/// key does, puts its volume at that placement, so a caller co-locates the key with the data it
/// guards. Any other key is placed by the hash of its volume address.
pub fn key_volume(replica_id: u64, key: &[u8]) -> rivet_keys::keys::VolumeKey {
	let key_bytes = bytes::Bytes::copy_from_slice(key);
	match rivet_keys::keys::codec::embedded_placement(key) {
		Some(placement) => PlacedEpoxyKey::new(placement, replica_id, key_bytes).volume_key(),
		None => EpoxyKey::new(replica_id, key_bytes).volume_key(),
	}
}
