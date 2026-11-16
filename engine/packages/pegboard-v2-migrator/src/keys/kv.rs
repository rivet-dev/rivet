//! Entries of the migration volume. Each value is written at the version the writing command's
//! handle selects for `pegboard.v2_migrator`, the highest every live storage worker reads, so a
//! storage worker of an older release that takes the volume over decodes every entry.
use rivet_util::Id;

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [V2_MIGRATOR_FENCE], value = vbare_selected(rivet_data::versioned::MigrateFromV2Fence))]
pub struct V2MigratorFenceKey {}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [V2_MIGRATOR_PASS], value = vbare_selected(rivet_data::versioned::MigrateFromV2Pass))]
pub struct V2MigratorPassKey {}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [V2_MIGRATOR_IN_FLIGHT, v2_actor_id], value = vbare_selected(rivet_data::versioned::MigrateFromV2InFlight))]
pub struct V2MigratorInFlightKey {
	pub v2_actor_id: Id,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [V2_MIGRATOR_IN_FLIGHT])]
pub struct V2MigratorInFlightPrefix {}
