//! Entries of the migration volume.
use rivet_util::Id;

rivet_keys::volume_kv_key! {
	pub V2MigratorFenceKey() => [V2_MIGRATOR_FENCE];
	value: vbare(rivet_data::versioned::MigrateFromV2Fence, rivet_data::generated::PEGBOARD_V2_MIGRATOR_VERSION);
}

rivet_keys::volume_kv_key! {
	pub V2MigratorPassKey() => [V2_MIGRATOR_PASS];
	value: vbare(rivet_data::versioned::MigrateFromV2Pass, rivet_data::generated::PEGBOARD_V2_MIGRATOR_VERSION);
}

rivet_keys::volume_kv_key! {
	pub V2MigratorInFlightKey(v2_actor_id: Id) => [V2_MIGRATOR_IN_FLIGHT, {v2_actor_id}];
	value: vbare(rivet_data::versioned::MigrateFromV2InFlight, rivet_data::generated::PEGBOARD_V2_MIGRATOR_VERSION);
}

rivet_keys::volume_kv_key! { pub V2MigratorInFlightPrefix() => [V2_MIGRATOR_IN_FLIGHT]; }
