//! Rows in the `kv` inode of an actor's database volume.
use bytes::Bytes;

rivet_keys::volume_kv_key! { pub ActorKvKey(key: Bytes) => [KV, {key}]; }
rivet_keys::volume_kv_key! { pub ActorKvPrefix() => [KV]; }
