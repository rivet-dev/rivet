use anyhow::Result;
use rivet_keys::keys::{
	VolumeKey,
	codec::{GroupId, Placement},
};
use rivet_util::Id;

// An actor's state volume, under its namespace's volume group.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [ACTOR, actor_id], name = ACTOR_STATE, placement = actor_placement(*actor_id))]
pub struct ActorVolume {
	#[group]
	pub group: GroupId,
	pub actor_id: Id,
}

/// Legacy: an actor's state volume at its unprefixed key. Volumes created before namespaces had
/// groups live at unprefixed keys; new behavior puts every volume under its namespace's group
/// ([`ActorVolume`]). Taken only to read a volume of a namespace whose backfill is not done.
/// Removed in 3.1 with the backfill.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [ACTOR, actor_id], name = ACTOR_STATE, placement = actor_placement(*actor_id))]
pub struct LegacyActorVolume {
	pub actor_id: Id,
}

impl ActorVolume {
	/// The state volume of `actor_id` in a namespace whose volume key group is `group` (see
	/// `NamespaceStanding::group_id`): the grouped volume, or the legacy unprefixed one for a
	/// namespace without a group. Every caller picks the shape here, never by hand.
	pub fn in_namespace(group: Option<GroupId>, actor_id: Id) -> VolumeKey {
		match group {
			Some(group) => Self::new(group, actor_id).volume_key(),
			None => LegacyActorVolume::new(actor_id).volume_key(),
		}
	}

	/// Reads a state volume key of either shape: its group, `None` for the legacy shape, and its
	/// actor id.
	pub fn parse(key: &VolumeKey) -> Result<(Option<GroupId>, Id)> {
		match Self::from_volume_key(key) {
			Ok(volume) => Ok((Some(volume.group), volume.actor_id)),
			Err(_) => {
				let volume = LegacyActorVolume::from_volume_key(key)?;
				Ok((None, volume.actor_id))
			}
		}
	}
}

// One incarnation's database. A recreated keyed actor gets a fresh volume, so a late destroy of
// the old incarnation can only delete its own, and a zombie of it can only open its own.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(
	key = [ACTOR, actor_id, create_unix_ms],
	name = ACTOR_DATABASE,
	placement = actor_placement(*actor_id)
)]
pub struct ActorDatabaseVolume {
	#[group]
	pub group: GroupId,
	pub actor_id: Id,
	pub create_unix_ms: i64,
}

/// Legacy: an incarnation's database volume at its unprefixed key. Volumes created before
/// namespaces had groups live at unprefixed keys; new behavior puts every volume under its
/// namespace's group ([`ActorDatabaseVolume`]). Taken only to read a volume of a namespace whose
/// backfill is not done. Removed in 3.1 with the backfill.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(
	key = [ACTOR, actor_id, create_unix_ms],
	name = ACTOR_DATABASE,
	placement = actor_placement(*actor_id)
)]
pub struct LegacyActorDatabaseVolume {
	pub actor_id: Id,
	pub create_unix_ms: i64,
}

impl ActorDatabaseVolume {
	/// The database volume of one incarnation in a namespace whose volume key group is `group`;
	/// see [`ActorVolume::in_namespace`].
	pub fn in_namespace(group: Option<GroupId>, actor_id: Id, create_unix_ms: i64) -> VolumeKey {
		match group {
			Some(group) => Self::new(group, actor_id, create_unix_ms).volume_key(),
			None => LegacyActorDatabaseVolume::new(actor_id, create_unix_ms).volume_key(),
		}
	}
}

/// Where an actor's volumes go: the 128 bits of its id. A random id spreads actors uniformly,
/// and a keyed id is its key's placement.
pub fn actor_placement(actor_id: Id) -> Placement {
	let Id::V1(data) = actor_id;
	let mut placement = [0; 16];
	placement.copy_from_slice(&data[..16]);
	placement
}

// One index bucket of a name's actors, under the namespace's volume group. It carries no
// namespace string, so a fork's bucket key differs from its parent's only in the group prefix and
// the parent's bucket is found by rewriting that prefix alone.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [ACTOR_INDEX, name, bucket])]
pub struct ActorIndexVolume {
	#[group]
	pub group: GroupId,
	pub name: String,
	pub bucket: u64,
}

/// Legacy: an index bucket at its unprefixed key, which names its namespace. Buckets created
/// before namespaces had groups live at unprefixed keys; new behavior puts every bucket under its
/// namespace's group with no namespace string ([`ActorIndexVolume`]). Taken only to list or read
/// the buckets of a namespace whose backfill is not done. Removed in 3.1 with the backfill.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [ACTOR_INDEX, namespace, name, bucket])]
pub struct LegacyActorIndexVolume {
	pub namespace: String,
	pub name: String,
	pub bucket: u64,
}

impl ActorIndexVolume {
	/// The index bucket `bucket` of `name` in a namespace whose volume key group is `group`; see
	/// [`ActorVolume::in_namespace`]. `namespace` is the namespace's name, which only the legacy
	/// shape carries.
	pub fn in_namespace(
		group: Option<GroupId>,
		namespace: String,
		name: String,
		bucket: u64,
	) -> VolumeKey {
		match group {
			Some(group) => Self::new(group, name, bucket).volume_key(),
			None => LegacyActorIndexVolume::new(namespace, name, bucket).volume_key(),
		}
	}

	/// The index bucket that holds `actor_id`'s entry; see [`Self::in_namespace`].
	pub fn for_actor_in_namespace(
		group: Option<GroupId>,
		namespace: String,
		name: String,
		actor_id: Id,
		buckets: u64,
	) -> VolumeKey {
		Self::in_namespace(
			group,
			namespace,
			name,
			actor_index_bucket(actor_id, buckets),
		)
	}

	/// The bucket number of an index bucket volume key of either shape.
	pub fn parse_bucket(key: &VolumeKey) -> Result<u64> {
		match Self::from_volume_key(key) {
			Ok(volume) => Ok(volume.bucket),
			Err(_) => Ok(LegacyActorIndexVolume::from_volume_key(key)?.bucket),
		}
	}
}

/// The placement of a key: XXH3-128 (seed 0) over a domain tag and the length-prefixed name and
/// key. A keyed actor's id is these bytes, so the key's state volume, its database volume and its
/// Epoxy reservation all land here. No namespace is hashed: one `(name, key)` has one id in every
/// namespace, and the group prefix that every volume key and Epoxy key of the actor carries keeps
/// unrelated namespaces apart, since the partition hash and the record key both include it.
///
/// **Contract:** a keyed actor's state volume, its database volume and its Epoxy key
/// volume share this one placement, so they land on the same tablet, maintainer and log shards.
/// The hot paths (create, wake, gateway routing, and SQLite reads and writes) then keep every
/// operation local, with no extra network hop between those volumes. Nothing may place a volume
/// of the key elsewhere, and nothing else may share the placement: Depot tells volumes apart by
/// placement and name, so each volume of the key has its own name.
pub fn key_placement(name: &str, key: &str) -> Placement {
	let mut input = Vec::with_capacity(64 + name.len() + key.len());
	input.extend_from_slice(b"rivet.actor.key_placement.v2");
	push_length_prefixed(&mut input, name.as_bytes());
	push_length_prefixed(&mut input, key.as_bytes());
	xxhash_rust::xxh3::xxh3_128(&input).to_be_bytes()
}

/// Legacy: the placement, and so the id, a keyed actor had before namespaces had groups: the
/// hash of a domain tag, the namespace's name and the length-prefixed name and key. New behavior
/// hashes the name and key alone ([`key_placement`]), under another domain tag so the two never
/// meet. Taken only to find an actor not yet moved by its namespace's backfill: a keyed lookup in
/// a namespace whose backfill is not done, and a namespace without a group, which only exists
/// while older binaries run and keeps every key in the legacy shape. Removed in 3.1 with the
/// backfill.
pub fn legacy_key_placement(namespace: &str, name: &str, key: &str) -> Placement {
	let mut input = Vec::with_capacity(64 + namespace.len() + name.len() + key.len());
	input.extend_from_slice(b"rivet.actor.key_placement.v1");
	input.extend_from_slice(namespace.as_bytes());
	push_length_prefixed(&mut input, name.as_bytes());
	push_length_prefixed(&mut input, key.as_bytes());
	xxhash_rust::xxh3::xxh3_128(&input).to_be_bytes()
}

/// Appends `part` after its length as a little-endian `u64`, so adjacent parts cannot run into
/// each other.
fn push_length_prefixed(input: &mut Vec<u8>, part: &[u8]) {
	input.extend_from_slice(&(part.len() as u64).to_le_bytes());
	input.extend_from_slice(part);
}

/// The index bucket of an actor: big-endian xxh3-128 (seed 0) of the actor id modulo `buckets`,
/// the same hash Depot uses for partition keys. `buckets` must be positive.
pub fn actor_index_bucket(actor_id: Id, buckets: u64) -> u64 {
	let hash = xxhash_rust::xxh3::xxh3_128(&actor_id.as_bytes()).to_be_bytes();
	let hash = u128::from_be_bytes(hash);
	(hash % u128::from(buckets.max(1))) as u64
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [ENVOY, namespace, pool_name, envoy_key])]
pub struct EnvoyVolume {
	pub namespace: String,
	pub pool_name: String,
	pub envoy_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [ENVOY_INDEX, namespace, pool_name])]
pub struct EnvoyIndexVolume {
	pub namespace: String,
	pub pool_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [ACTIVE_ENVOY_INDEX, namespace, pool_name])]
pub struct ActiveEnvoyIndexVolume {
	pub namespace: String,
	pub pool_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [ENVOY_POOL_INDEX, namespace])]
pub struct EnvoyPoolIndexVolume {
	pub namespace: String,
}

// One catalog per cluster: the listing index of namespace names, and the legacy record rows not
// yet moved into namespace volumes. It is written once per namespace create and read by admin
// listings, so it is on no request path.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [NAMESPACE_CATALOG])]
pub struct NamespaceCatalogVolume {}

// Each namespace's own volume, holding its namespace state machine, whose state is the
// namespace's record. Two creates of one name land on this one volume, so names are unique with
// no shared writer. Hashed by its bytes and never grouped.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [NAMESPACE_VOLUME, name], name = NAMESPACE_STATE)]
pub struct NamespaceVolume {
	pub name: String,
}

// The V2 migrator's one volume. It lives on V3's own log quorum, so its state is as durable as any
// V3 write, and every write to it is fenced by the migrator's singleton tenure. Defined here
// because a namespace's backfill reads it; the migrator owns its contents.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKey)]
#[volume(key = [V2_MIGRATOR])]
pub struct V2MigratorVolume {}
