// Each Epoxy key has its own record volume on each replica.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [EPOXY_KEY, replica_id, key])]
pub struct EpoxyKey {
	pub replica_id: u64,
	pub key: bytes::Bytes,
}

// A key whose caller placed it, under the caller's volume group: its volume is placed where the
// key's own bytes say, next to the data the key guards. Depot tells volumes apart by placement
// and name, so the name keeps it apart from the caller's own volumes at that placement.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [PLACED, placement, EPOXY_KEY, replica_id, key], name = EPOXY_KEY)]
pub struct PlacedEpoxyKey {
	#[group]
	pub group: rivet_keys::keys::codec::GroupId,
	pub placement: [u8; 16],
	pub replica_id: u64,
	pub key: bytes::Bytes,
}

/// Legacy: a placed key's volume at its unprefixed key. Keys written before namespaces had groups
/// live at unprefixed keys; new behavior keeps every placed key in its caller's volume group
/// ([`PlacedEpoxyKey`]). Taken for a logical key that carries no group, which only legacy
/// reservations do. Removed in 3.1 with the backfill.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [PLACED, placement, EPOXY_KEY, replica_id, key], name = EPOXY_KEY)]
pub struct LegacyPlacedEpoxyKey {
	pub placement: [u8; 16],
	pub replica_id: u64,
	pub key: bytes::Bytes,
}

/// The volume that holds `key`'s record on `replica_id`. Every reader and writer of a key's
/// record finds its volume here. A logical key that starts with a placement, as a placed formal
/// key does, puts its volume at that placement, so a caller co-locates the key with the data it
/// guards; a logical key that also starts with a group keeps its volume in that group, so a
/// fork's reservations land with the fork's data and never with the parent's. Any other key is
/// placed by the hash of its volume address.
pub fn key_volume(replica_id: u64, key: &[u8]) -> rivet_keys::keys::VolumeKey {
	let key_bytes = bytes::Bytes::copy_from_slice(key);
	let prefixes = rivet_keys::keys::codec::prefixes(key);
	match (prefixes.group, prefixes.placement) {
		(Some(group), Some(placement)) => {
			PlacedEpoxyKey::new(group, placement, replica_id, key_bytes).volume_key()
		}
		(None, Some(placement)) => {
			LegacyPlacedEpoxyKey::new(placement, replica_id, key_bytes).volume_key()
		}
		(_, None) => EpoxyKey::new(replica_id, key_bytes).volume_key(),
	}
}
