//! A log identity ordered and hashed by its canonical BARE bytes.

use rivet_data::generated::shared_depot_types_v1::VolumeLogKey as WireVolumeLogKey;
use serde::{Deserialize, Serialize};
use std::{
	cmp::Ordering,
	hash::{Hash, Hasher},
};

/// The generated wire identity with canonical ordering for shard indexes.
/// Serialization remains exactly the generated BARE representation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VolumeLogKey(pub WireVolumeLogKey);

/// Canonical BARE length prefix. The encoding is prefix-free, so comparing two prefixes
/// as slices orders them exactly as the concatenated canonical byte streams would.
fn bare_length(len: usize) -> ([u8; 10], usize) {
	let mut length = [0u8; 10];
	let mut count = 0;
	let mut remaining = len;
	loop {
		let byte = (remaining & 0x7f) as u8;
		remaining >>= 7;
		length[count] = byte | if remaining == 0 { 0 } else { 0x80 };
		count += 1;
		if remaining == 0 {
			break;
		}
	}
	(length, count)
}

/// Orders two length-prefixed byte strings as their canonical BARE encodings. Equal
/// prefixes imply equal lengths, so the payload slices then compare byte for byte. Keys of
/// one length, the common case for partition keys, skip encoding the prefixes.
fn cmp_bare(a: &[u8], b: &[u8]) -> Ordering {
	if a.len() == b.len() {
		return a.cmp(b);
	}
	let (a_length, a_count) = bare_length(a.len());
	let (b_length, b_count) = bare_length(b.len());
	a_length[..a_count]
		.cmp(&b_length[..b_count])
		.then_with(|| a.cmp(b))
}

/// Feeds one BARE `data` value: its uvarint length, then its bytes.
fn update_bare_data(hasher: &mut xxhash_rust::xxh3::Xxh3, bytes: &[u8]) {
	let (length, count) = bare_length(bytes.len());
	hasher.update(&length[..count]);
	hasher.update(bytes);
}

fn hash_bare<H: Hasher>(bytes: &[u8], state: &mut H) {
	let (length, count) = bare_length(bytes.len());
	state.write(&length[..count]);
	state.write(bytes);
}

/// Compares component slices in canonical byte order instead of iterating bytes, because
/// shard and maintainer indexes compare keys on every lookup.
impl Ord for VolumeLogKey {
	fn cmp(&self, other: &Self) -> Ordering {
		let a = &self.0.volume_key;
		let b = &other.0.volume_key;
		cmp_bare(&a.partition_key, &b.partition_key)
			.then_with(|| match (&a.name, &b.name) {
				(None, None) => Ordering::Equal,
				(None, Some(_)) => Ordering::Less,
				(Some(_), None) => Ordering::Greater,
				(Some(a_name), Some(b_name)) => cmp_bare(a_name, b_name),
			})
			.then_with(|| self.0.volume_incarnation.cmp(&other.0.volume_incarnation))
	}
}

impl PartialOrd for VolumeLogKey {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

/// Hashes the canonical BARE bytes, written as whole components.
impl Hash for VolumeLogKey {
	fn hash<H: Hasher>(&self, state: &mut H) {
		let key = &self.0.volume_key;
		hash_bare(&key.partition_key, state);
		state.write_u8(u8::from(key.name.is_some()));
		if let Some(name) = &key.name {
			hash_bare(name, state);
		}
		state.write(&self.0.volume_incarnation);
	}
}

impl VolumeLogKey {
	/// XXH3-128 with seed zero of the key's canonical BARE encoding, big-endian. Persisted
	/// where a shard keeps a compact fence instead of the key, so it is computed from the
	/// encoding written out here, independent of the `Hash` impl, and must never change.
	pub fn digest(&self) -> [u8; 16] {
		let key = &self.0.volume_key;
		let mut hasher = xxhash_rust::xxh3::Xxh3::new();
		update_bare_data(&mut hasher, &key.partition_key);
		match &key.name {
			None => hasher.update(&[0]),
			Some(name) => {
				hasher.update(&[1]);
				update_bare_data(&mut hasher, name);
			}
		}
		hasher.update(&self.0.volume_incarnation);
		hasher.digest128().to_be_bytes()
	}

	/// A copy that no longer shares the buffer this key was decoded from. Callers that keep a
	/// key past its request detach it, so a stored key does not keep a large frame alive.
	pub fn detach(&self) -> Self {
		Self(vbare::Message::detach(&self.0))
	}
}

impl From<WireVolumeLogKey> for VolumeLogKey {
	fn from(value: WireVolumeLogKey) -> Self {
		Self(value)
	}
}

impl From<VolumeLogKey> for WireVolumeLogKey {
	fn from(value: VolumeLogKey) -> Self {
		value.0
	}
}

/// Routes only the partition key. A placed key carries its placement hash and routes by it;
/// any other key routes by XXH3-128 with seed zero of its bytes. The name and incarnation are
/// excluded so related volumes and recreations stay in the same tablet. Range producers,
/// cold-store keys and clients must share this exact codec.
pub fn partition_key_hash(volume_key: &crate::VolumeKey) -> crate::PartitionKeyHash {
	match rivet_keys::keys::codec::embedded_placement(&volume_key.partition_key) {
		Some(placement) => placement,
		None => xxhash_rust::xxh3::xxh3_128(&volume_key.partition_key).to_be_bytes(),
	}
}

/// The order of a tablet replica listing: partition-key hash, then key. A tablet owns a hash
/// range, so this order lets a shard page one tablet's replicas without visiting others.
pub fn listing_order(key: &VolumeLogKey) -> (crate::PartitionKeyHash, &VolumeLogKey) {
	(partition_key_hash(&key.0.volume_key), key)
}
