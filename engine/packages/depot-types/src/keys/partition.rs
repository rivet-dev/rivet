//! Formal volume keys of Depot's own volumes that groups add, and the rule that tells which
//! group a volume's forks and pins belong to.

use rivet_keys::keys::{
	codec::{GroupId, embedded_group, family_tag},
	tags::partition as tags,
};

// The group key of a namespace: the identity of the group a namespace's volumes belong to.
// The group id is this key's own partition key hash, so it is never drawn at random and a
// namespace always maps to one group. Another kind of owner would get its own first tag. It is
// a key and not a volume: the group's volume is [`GroupVolume`], keyed by the id, because a
// maintainer that reads a volume key learns the id alone and a hash cannot be inverted back to
// the namespace. Its bytes equal pegboard's legacy catalog row key of the same name
// (`LegacyNamespaceKey`), which is harmless: this key is only hashed into a group id and stored
// in the group record, while that one is a row inside the catalog volume, so the two are never
// compared or looked up in one space.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [NAMESPACE, namespace])]
pub struct NamespaceGroupKey {
	pub namespace: String,
}

impl NamespaceGroupKey {
	/// The id of the group this key names: its own partition key hash.
	pub fn group_id(&self) -> GroupId {
		crate::partition_key_hash(&self.volume_key())
	}
}

// The group volume of a group: the ungrouped KV volume holding its record, its lineage and its
// forks, written only by the group commands and read by maintainers and the namespace API.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [GROUP_VOLUME, group])]
pub struct GroupVolume {
	pub group: GroupId,
}

impl GroupVolume {
	pub fn group(&self) -> GroupId {
		self.group
	}
}

// An event shard: one of a fixed number of unprefixed KV volumes per region that carry the
// events every storage worker polls. Any event may be posted to any shard.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [EVENT, shard])]
pub struct EventShardVolume {
	pub shard: u64,
}

/// The partition key the same volume has in `group`: the bytes after the group prefix are
/// identical along a lineage, so a child finds its ancestor's volume by rewriting the prefix
/// and nothing else.
pub fn in_group(partition_key: &[u8], group: GroupId) -> Vec<u8> {
	let rest = match embedded_group(partition_key) {
		Some((_, rest)) => rest,
		None => partition_key,
	};
	let mut key = Vec::with_capacity(rest.len() + 20);
	rivet_keys::keys::codec::pack_group(&group, &mut key);
	key.extend_from_slice(rest);
	key
}

/// The group whose forks pin and lazily fork the volume with this partition key, or `None` for a
/// volume no group fork touches.
///
/// Only a namespace's data takes part: actor volumes, placed under `PLACED`, and their index
/// buckets, and only with a `GROUPED` prefix, which names the group. An unprefixed volume
/// predates groups and belongs to none, so no fork pins or reads through it. An Epoxy key
/// reservation carries a group for placement only, so that
/// an actor's three volumes share a tablet, and is re-reserved in a child rather than read
/// through; every other family is runtime or system state that is never forked.
pub fn forkable_group(partition_key: &[u8]) -> Option<GroupId> {
	let family = family_tag(partition_key)?;
	let forkable = [tags::ACTOR, tags::ACTOR_INDEX]
		.iter()
		.any(|tag| *tag as u64 == family);
	if !forkable {
		return None;
	}
	embedded_group(partition_key).map(|(group, _)| group)
}
