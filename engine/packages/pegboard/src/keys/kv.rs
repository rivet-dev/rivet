//! Rows inside pegboard volumes. Each row key starts with one singular tag naming the row kind.
use rivet_util::Id;

rivet_keys::volume_kv_key! {
	pub ActorDataKey() => [DATA];
	value: vbare(rivet_data::versioned::ActorData, rivet_data::generated::PEGBOARD_ACTOR_VERSION);
}

rivet_keys::volume_kv_key! {
	pub EnvoyDataKey() => [DATA];
	value: vbare(rivet_data::versioned::EnvoyData, rivet_data::generated::PEGBOARD_ACTOR_VERSION);
}

// Row keys keep one shape in every bucket volume; the volume names the namespace and actor name.
rivet_keys::volume_kv_key! { pub ActorIndexKey(created_at: i64, actor_id: Id) => [ACTOR, {created_at}, {actor_id}]; }
rivet_keys::volume_kv_key! { pub ActorIndexPrefix() => [ACTOR]; }

// A destroyed actor's entry moves here when its live entry is removed, so a listing of live
// actors never scans destroyed ones. The value is when the entry moved.
rivet_keys::volume_kv_key! {
	pub DestroyedActorIndexKey(created_at: i64, actor_id: Id) => [DESTROYED_ACTOR, {created_at}, {actor_id}];
	value: vbare(rivet_data::versioned::DestroyedActorIndexEntry, rivet_data::generated::PEGBOARD_ACTOR_VERSION);
}
rivet_keys::volume_kv_key! { pub DestroyedActorIndexPrefix() => [DESTROYED_ACTOR]; }

rivet_keys::volume_kv_key! { pub EnvoyIndexKey(created_at: i64, envoy_key: String) => [ENVOY, {created_at}, {envoy_key}]; }
rivet_keys::volume_kv_key! { pub EnvoyIndexPrefix() => [ENVOY]; }

// The actor name leads the key so a listing for one name scans only the envoys serving it, and
// the heartbeat follows so it scans only those inside the eligibility window. The value is the
// envoy's route, so an allocation needs no second read.
rivet_keys::volume_kv_key! {
	pub ActiveEnvoyIndexKey(name: String, heartbeat_unix_ms: i64, envoy_key: String) => [ENVOY, {name}, {heartbeat_unix_ms}, {envoy_key}];
	value: vbare(rivet_data::versioned::ActiveEnvoyIndexEntry, rivet_data::generated::PEGBOARD_ACTOR_VERSION);
}
rivet_keys::volume_kv_key! { pub ActiveEnvoyIndexNamePrefix(name: String) => [ENVOY, {name}]; }

// One state per envoy fences its index entries: the connection that wrote them, the heartbeat
// they sit at and the names they sit under, so an upsert or delete from an older connection
// changes nothing and a newer one replaces the entries wholesale.
rivet_keys::volume_kv_key! {
	pub ActiveEnvoyStateKey(envoy_key: String) => [ENVOY_STATE, {envoy_key}];
	value: vbare(rivet_data::versioned::ActiveEnvoyState, rivet_data::generated::PEGBOARD_ACTOR_VERSION);
}
rivet_keys::volume_kv_key! { pub ActiveEnvoyStatePrefix() => [ENVOY_STATE]; }

rivet_keys::volume_kv_key! {
 pub EnvoyPoolKey(pool_name: String) => [ENVOY_POOL, {pool_name}];
 value: vbare(rivet_data::versioned::NodePoolConfig, rivet_data::generated::PEGBOARD_ACTOR_VERSION);
}
rivet_keys::volume_kv_key! {
 pub PoolConnectionKey(pool_name: String, connection_id: uuid::Uuid) => [POOL_CONNECTION, {pool_name}, {connection_id}];
 value: vbare(rivet_data::versioned::PoolLease, rivet_data::generated::PEGBOARD_ACTOR_VERSION);
}
rivet_keys::volume_kv_key! { pub PoolConnectionPrefix(pool_name: String) => [POOL_CONNECTION, {pool_name}]; }
rivet_keys::volume_kv_key! { pub PoolHeartbeatKey(pool_name: String, heartbeat_unix_ms: i64, connection_id: uuid::Uuid) => [POOL_HEARTBEAT, {pool_name}, {heartbeat_unix_ms}, {connection_id}]; }
rivet_keys::volume_kv_key! { pub PoolHeartbeatPrefix(pool_name: String) => [POOL_HEARTBEAT, {pool_name}]; }

rivet_keys::volume_kv_key! { pub EnvoyPoolPrefix() => [ENVOY_POOL]; }

// A namespace's record in the cluster catalog. The name is the namespace's only identity: every
// volume and Epoxy key of the namespace carries it, so a name is never renamed or reused.
rivet_keys::volume_kv_key! {
	pub NamespaceKey(name: String) => [NAMESPACE, {name}];
	value: vbare(rivet_data::versioned::NamespaceRecord, rivet_data::generated::PEGBOARD_ACTOR_VERSION);
}
rivet_keys::volume_kv_key! { pub NamespacePrefix() => [NAMESPACE]; }
