//! Rows inside pegboard volumes. Each row key starts with one singular tag naming the row kind.
//! A value is encoded at the version its writer's handle selects for its family, the highest
//! every live storage worker reads, so no binary of a rolling deploy reads a version it does
//! not know. Every value is in `pegboard.actor` except the namespace index entry, which is in
//! `pegboard.namespace`.
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

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [DATA], value = vbare_selected(rivet_data::versioned::ActorData))]
pub struct ActorDataKey {}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [DATA], value = vbare_selected(rivet_data::versioned::EnvoyData))]
pub struct EnvoyDataKey {}

// Row keys keep one shape in every bucket volume; the volume names the namespace and actor name.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ACTOR, created_at, actor_id])]
pub struct ActorIndexKey {
	pub created_at: i64,
	pub actor_id: Id,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ACTOR])]
pub struct ActorIndexPrefix {}

// A destroyed actor's entry moves here when its live entry is removed, so a listing of live
// actors never scans destroyed ones. The value is when the entry moved.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [DESTROYED_ACTOR, created_at, actor_id], value = vbare_selected(rivet_data::versioned::DestroyedActorIndexEntry))]
pub struct DestroyedActorIndexKey {
	pub created_at: i64,
	pub actor_id: Id,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [DESTROYED_ACTOR])]
pub struct DestroyedActorIndexPrefix {}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY, created_at, envoy_key])]
pub struct EnvoyIndexKey {
	pub created_at: i64,
	pub envoy_key: String,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY])]
pub struct EnvoyIndexPrefix {}

// The actor name leads the key so a listing for one name scans only the envoys serving it. The
// envoy version follows, descending, so the newest version's entries come first and a listing
// settles on one version before it reads a page, and the heartbeat follows that so the page
// scans only entries inside the eligibility window. The value is the envoy's route, so an
// allocation needs no second read.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY, name, version, heartbeat_unix_ms, envoy_key], value = vbare_selected(rivet_data::versioned::ActiveEnvoyIndexEntry))]
pub struct ActiveEnvoyIndexKey {
	pub name: String,
	pub version: DescendingU64,
	pub heartbeat_unix_ms: i64,
	pub envoy_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY])]
pub struct ActiveEnvoyIndexPrefix {}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY, name])]
pub struct ActiveEnvoyIndexNamePrefix {
	pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY, name, version])]
pub struct ActiveEnvoyIndexVersionPrefix {
	pub name: String,
	pub version: DescendingU64,
}

// Pre-version allocation rows remain readable during rolling upgrades. Their value uses the
// same versioned codec; only the key lacks the version component.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY, name, heartbeat_unix_ms, envoy_key], value = vbare_selected(rivet_data::versioned::ActiveEnvoyIndexEntry))]
pub struct LegacyActiveEnvoyIndexKey {
	pub name: String,
	pub heartbeat_unix_ms: i64,
	pub envoy_key: String,
}

// One entry per envoy, keyed by descending version and heartbeat. Once a version is known,
// a heartbeat can seek directly into its live window, whatever names the envoys serve.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY_VERSION, version, heartbeat_unix_ms, envoy_key])]
pub struct ActiveEnvoyVersionKey {
	pub version: DescendingU64,
	pub heartbeat_unix_ms: i64,
	pub envoy_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY_VERSION])]
pub struct ActiveEnvoyVersionPrefix {}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY_VERSION, version])]
pub struct ActiveEnvoyVersionNumberPrefix {
	pub version: DescendingU64,
}

// Monotonic candidate catalogs: never move or delete these keys on heartbeat/disconnect.
// Discovering versions through timestamped rows walks deleted MVCC history, even with a
// one-row limit. Catalog candidates must always be checked against the live route window.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY_NAME_VERSION, name, version])]
pub struct ActiveEnvoyNameVersionKey {
	pub name: String,
	pub version: DescendingU64,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY_NAME_VERSION, name])]
pub struct ActiveEnvoyNameVersionPrefix {
	pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY_POOL_VERSION, version])]
pub struct ActiveEnvoyPoolVersionKey {
	pub version: DescendingU64,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY_POOL_VERSION])]
pub struct ActiveEnvoyPoolVersionPrefix {}

// One state per envoy fences its index entries: the connection that wrote them, the heartbeat
// they sit at and the names they sit under, so an upsert or delete from an older connection
// changes nothing and a newer one replaces the entries wholesale.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY_STATE, envoy_key], value = vbare_selected(rivet_data::versioned::ActiveEnvoyState))]
pub struct ActiveEnvoyStateKey {
	pub envoy_key: String,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY_STATE])]
pub struct ActiveEnvoyStatePrefix {}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY_POOL, pool_name], value = vbare_selected(rivet_data::versioned::NodePoolConfig))]
pub struct EnvoyPoolKey {
	pub pool_name: String,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [POOL_CONNECTION, pool_name, connection_id], value = vbare_selected(rivet_data::versioned::PoolLease))]
pub struct PoolConnectionKey {
	pub pool_name: String,
	pub connection_id: uuid::Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [POOL_CONNECTION, pool_name])]
pub struct PoolConnectionPrefix {
	pub pool_name: String,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [POOL_HEARTBEAT, pool_name, heartbeat_unix_ms, connection_id])]
pub struct PoolHeartbeatKey {
	pub pool_name: String,
	pub heartbeat_unix_ms: i64,
	pub connection_id: uuid::Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [POOL_HEARTBEAT, pool_name])]
pub struct PoolHeartbeatPrefix {
	pub pool_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [ENVOY_POOL])]
pub struct EnvoyPoolPrefix {}

// Legacy: a namespace's record row in the cluster catalog, written before each namespace had its
// own namespace volume, or by a new binary while the fleet cannot read namespace volumes. New
// namespaces live in `NamespaceVolume`; a legacy row is read only by `legacy_read_namespace`,
// listed until it is moved, and deleted by `legacy_move_namespace` when its namespace moves.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [NAMESPACE, name], value = vbare_selected(rivet_data::versioned::LegacyNamespaceRecord))]
pub struct LegacyNamespaceKey {
	pub name: String,
}
// Legacy: the range of every `LegacyNamespaceKey`, which listings merge until none are left.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [NAMESPACE])]
pub struct LegacyNamespacePrefix {}

// The catalog's listing index: one entry per namespace with a namespace volume, written by the
// namespace's machine after its create. A listing reads each namespace's state from its volume.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [NAMESPACE_INDEX, name], value = vbare_selected(rivet_data::versioned::NamespaceIndexEntry))]
pub struct NamespaceIndexKey {
	pub name: String,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [NAMESPACE_INDEX])]
pub struct NamespaceIndexPrefix {}

// Legacy: an actor name on a namespace volume that a legacy fallback read found an actor under,
// so the backfill walks that name's legacy buckets even when no envoy record names it. Written
// by `legacy_record_actor_name`, read by the backfill, and never deleted: the namespace volume
// keeps them until 3.1 removes the backfill.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [LEGACY_BACKFILL_NAME, name])]
pub struct LegacyBackfillNameKey {
	pub name: String,
}
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [LEGACY_BACKFILL_NAME])]
pub struct LegacyBackfillNamePrefix {}
