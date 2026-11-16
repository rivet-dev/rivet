//! Rows in the `kv` inode of an actor's database volume.
use bytes::Bytes;

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [KV, key])]
pub struct ActorKvKey {
	pub key: Bytes,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [KV])]
pub struct ActorKvPrefix {}
