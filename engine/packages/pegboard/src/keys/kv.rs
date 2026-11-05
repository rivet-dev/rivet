//! Rows inside pegboard volumes. Each row key starts with one singular tag naming the row kind.
use anyhow::Result;
use depot_worker::command::KvHandle;
use rivet_data::families::VersionFamily;
use rivet_keys::keys::codec::{AtVersion, DescendingU64};
use rivet_util::Id;

/// Pairs a row value with the version it is written at: the highest version of
/// `pegboard.actor` that every live storage worker reads.
pub(crate) fn at_write_version<T>(db: &KvHandle, value: T) -> Result<AtVersion<T>> {
	let version = rivet_data::pegboard_actor::record_write_version(
		db.write_version(VersionFamily::PEGBOARD_ACTOR)?,
	);
	Ok(AtVersion::new(value, version))
}

rivet_keys::volume_kv_key! {
	pub ActorDataKey() => [DATA];
	value: vbare_selected(rivet_data::versioned::ActorData);
}

rivet_keys::volume_kv_key! {
	pub EnvoyDataKey() => [DATA];
	value: vbare_selected(rivet_data::versioned::EnvoyData);
}

// Row keys keep one shape in every bucket volume; the volume names the namespace and actor name.
rivet_keys::volume_kv_key! { pub ActorIndexKey(created_at: i64, actor_id: Id) => [ACTOR, {created_at}, {actor_id}]; }
rivet_keys::volume_kv_key! { pub ActorIndexPrefix() => [ACTOR]; }

// A destroyed actor's entry moves here when its live entry is removed, so a listing of live
// actors never scans destroyed ones. The value is when the entry moved.
rivet_keys::volume_kv_key! {
	pub DestroyedActorIndexKey(created_at: i64, actor_id: Id) => [DESTROYED_ACTOR, {created_at}, {actor_id}];
	value: vbare_selected(rivet_data::versioned::DestroyedActorIndexEntry);
}
rivet_keys::volume_kv_key! { pub DestroyedActorIndexPrefix() => [DESTROYED_ACTOR]; }

rivet_keys::volume_kv_key! { pub EnvoyIndexKey(created_at: i64, envoy_key: String) => [ENVOY, {created_at}, {envoy_key}]; }
rivet_keys::volume_kv_key! { pub EnvoyIndexPrefix() => [ENVOY]; }

// The actor name leads the key so a listing for one name scans only the envoys serving it. The
// envoy version follows, descending, so the newest version's entries come first and a listing
// settles on one version before it reads a page, and the heartbeat follows that so the page
// scans only entries inside the eligibility window. The value is the envoy's route, so an
// allocation needs no second read.
rivet_keys::volume_kv_key! {
	pub ActiveEnvoyIndexKey(name: String, version: DescendingU64, heartbeat_unix_ms: i64, envoy_key: String) => [ENVOY, {name}, {version}, {heartbeat_unix_ms}, {envoy_key}];
	value: vbare_selected(rivet_data::versioned::ActiveEnvoyIndexEntry);
}
rivet_keys::volume_kv_key! { pub(crate) ActiveEnvoyIndexPrefix() => [ENVOY]; }
rivet_keys::volume_kv_key! { pub ActiveEnvoyIndexNamePrefix(name: String) => [ENVOY, {name}]; }
rivet_keys::volume_kv_key! { pub ActiveEnvoyIndexVersionPrefix(name: String, version: DescendingU64) => [ENVOY, {name}, {version}]; }

// Pre-version allocation rows remain readable during rolling upgrades. Their value uses the
// same versioned codec; only the key lacks the version component.
rivet_keys::volume_kv_key! {
	pub(crate) LegacyActiveEnvoyIndexKey(name: String, heartbeat_unix_ms: i64, envoy_key: String) => [ENVOY, {name}, {heartbeat_unix_ms}, {envoy_key}];
	value: vbare_selected(rivet_data::versioned::ActiveEnvoyIndexEntry);
}

// One entry per envoy, keyed by descending version and heartbeat. Once a version is known,
// a heartbeat can seek directly into its live window, whatever names the envoys serve.
rivet_keys::volume_kv_key! { pub ActiveEnvoyVersionKey(version: DescendingU64, heartbeat_unix_ms: i64, envoy_key: String) => [ENVOY_VERSION, {version}, {heartbeat_unix_ms}, {envoy_key}]; }
rivet_keys::volume_kv_key! { pub ActiveEnvoyVersionPrefix() => [ENVOY_VERSION]; }
rivet_keys::volume_kv_key! { pub(crate) ActiveEnvoyVersionNumberPrefix(version: DescendingU64) => [ENVOY_VERSION, {version}]; }

// Monotonic candidate catalogs: never move or delete these keys on heartbeat/disconnect.
// Discovering versions through timestamped rows walks deleted MVCC history, even with a
// one-row limit. Catalog candidates must always be checked against the live route window.
rivet_keys::volume_kv_key! { pub(crate) ActiveEnvoyNameVersionKey(name: String, version: DescendingU64) => [ENVOY_NAME_VERSION, {name}, {version}]; }
rivet_keys::volume_kv_key! { pub(crate) ActiveEnvoyNameVersionPrefix(name: String) => [ENVOY_NAME_VERSION, {name}]; }
rivet_keys::volume_kv_key! { pub(crate) ActiveEnvoyPoolVersionKey(version: DescendingU64) => [ENVOY_POOL_VERSION, {version}]; }
rivet_keys::volume_kv_key! { pub(crate) ActiveEnvoyPoolVersionPrefix() => [ENVOY_POOL_VERSION]; }

// One state per envoy fences its index entries: the connection that wrote them, the heartbeat
// they sit at and the names they sit under, so an upsert or delete from an older connection
// changes nothing and a newer one replaces the entries wholesale.
rivet_keys::volume_kv_key! {
	pub ActiveEnvoyStateKey(envoy_key: String) => [ENVOY_STATE, {envoy_key}];
	value: vbare_selected(rivet_data::versioned::ActiveEnvoyState);
}
rivet_keys::volume_kv_key! { pub ActiveEnvoyStatePrefix() => [ENVOY_STATE]; }

rivet_keys::volume_kv_key! {
 pub EnvoyPoolKey(pool_name: String) => [ENVOY_POOL, {pool_name}];
 value: vbare_selected(rivet_data::versioned::NodePoolConfig);
}
rivet_keys::volume_kv_key! {
 pub PoolConnectionKey(pool_name: String, connection_id: uuid::Uuid) => [POOL_CONNECTION, {pool_name}, {connection_id}];
 value: vbare_selected(rivet_data::versioned::PoolLease);
}
rivet_keys::volume_kv_key! { pub PoolConnectionPrefix(pool_name: String) => [POOL_CONNECTION, {pool_name}]; }
rivet_keys::volume_kv_key! { pub PoolHeartbeatKey(pool_name: String, heartbeat_unix_ms: i64, connection_id: uuid::Uuid) => [POOL_HEARTBEAT, {pool_name}, {heartbeat_unix_ms}, {connection_id}]; }
rivet_keys::volume_kv_key! { pub PoolHeartbeatPrefix(pool_name: String) => [POOL_HEARTBEAT, {pool_name}]; }

rivet_keys::volume_kv_key! { pub EnvoyPoolPrefix() => [ENVOY_POOL]; }

// A namespace's record in the cluster catalog. The name is the namespace's only identity: every
// volume and Epoxy key of the namespace carries it, so a name is never renamed or reused.
rivet_keys::volume_kv_key! {
	pub NamespaceKey(name: String) => [NAMESPACE, {name}];
	value: vbare_selected(rivet_data::versioned::NamespaceRecord);
}
rivet_keys::volume_kv_key! { pub NamespacePrefix() => [NAMESPACE]; }
