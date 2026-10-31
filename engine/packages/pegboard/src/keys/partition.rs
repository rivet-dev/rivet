use rivet_util::Id;

rivet_keys::volume_key! { pub ActorVolume(actor_id: Id) placed_by(placement) => [ACTOR, {actor_id}] name [ACTOR_STATE]; }

// One incarnation's database. A recreated keyed actor gets a fresh volume, so a late destroy of
// the old incarnation can only delete its own, and a zombie of it can only open its own.
rivet_keys::volume_key! { pub ActorDatabaseVolume(actor_id: Id, create_unix_ms: i64) placed_by(database_placement) => [ACTOR, {actor_id}, {create_unix_ms}] name [ACTOR_DATABASE]; }

/// Where an actor's volumes go: the 128 bits of its id. A random id spreads actors uniformly,
/// and a keyed id is its key's placement.
pub fn actor_placement(actor_id: Id) -> rivet_keys::keys::codec::Placement {
	let Id::V1(data) = actor_id;
	let mut placement = [0; 16];
	placement.copy_from_slice(&data[..16]);
	placement
}

fn placement(volume: &ActorVolume) -> rivet_keys::keys::codec::Placement {
	actor_placement(volume.actor_id)
}

fn database_placement(volume: &ActorDatabaseVolume) -> rivet_keys::keys::codec::Placement {
	actor_placement(volume.actor_id)
}

rivet_keys::volume_key! { pub ActorIndexVolume(namespace: String, name: String, bucket: u64) => [ACTOR_INDEX, {namespace}, {name}, {bucket}]; }

impl ActorIndexVolume {
	pub fn for_actor(namespace: String, name: String, actor_id: Id, buckets: u64) -> Self {
		Self::new(namespace, name, actor_index_bucket(actor_id, buckets))
	}
}

/// The placement of a key: XXH3-128 (seed 0) over a domain tag and the key's length-prefixed
/// namespace, name and key. A keyed actor's id is these bytes, so the key's state volume, its
/// database volume and its Epoxy reservation all land here.
///
/// **Contract:** a keyed actor's state volume, its database volume and its Epoxy key
/// volume share this one placement, so they land on the same tablet, maintainer and log shards.
/// The hot paths (create, wake, gateway routing, and SQLite reads and writes) then keep every
/// operation local, with no extra network hop between those volumes. Nothing may place a volume
/// of the key elsewhere, and nothing else may share the placement: Depot tells volumes apart by
/// placement and name, so each volume of the key has its own name.
pub fn key_placement(namespace: &str, name: &str, key: &str) -> rivet_keys::keys::codec::Placement {
	let mut input = Vec::with_capacity(64 + name.len() + key.len());
	input.extend_from_slice(b"rivet.actor.key_placement.v1");
	input.extend_from_slice(&namespace.as_bytes());
	for part in [name.as_bytes(), key.as_bytes()] {
		input.extend_from_slice(&(part.len() as u64).to_le_bytes());
		input.extend_from_slice(part);
	}
	xxhash_rust::xxh3::xxh3_128(&input).to_be_bytes()
}

/// The index bucket of an actor: big-endian xxh3-128 (seed 0) of the actor id modulo `buckets`,
/// the same hash Depot uses for partition keys. `buckets` must be positive.
pub fn actor_index_bucket(actor_id: Id, buckets: u64) -> u64 {
	let hash = xxhash_rust::xxh3::xxh3_128(&actor_id.as_bytes()).to_be_bytes();
	let hash = u128::from_be_bytes(hash);
	(hash % u128::from(buckets.max(1))) as u64
}

rivet_keys::volume_key! { pub EnvoyVolume(namespace: String, pool_name: String, envoy_key: String) => [ENVOY, {namespace}, {pool_name}, {envoy_key}]; }

rivet_keys::volume_key! { pub EnvoyIndexVolume(namespace: String, pool_name: String) => [ENVOY_INDEX, {namespace}, {pool_name}]; }

rivet_keys::volume_key! { pub ActiveEnvoyIndexVolume(namespace: String, pool_name: String) => [ACTIVE_ENVOY_INDEX, {namespace}, {pool_name}]; }

rivet_keys::volume_key! { pub EnvoyPoolIndexVolume(namespace: String) => [ENVOY_POOL_INDEX, {namespace}]; }

// One catalog per cluster holds every namespace record, keyed by name.
rivet_keys::volume_key! { pub NamespaceCatalogVolume() => [NAMESPACE_CATALOG]; }


