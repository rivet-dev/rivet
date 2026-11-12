//! Converts the `migrate_from_v2` config section into the settings the state machine decides on.
use crate::migrator_machine::types::Settings;

/// `target_group` is the volume group of the target namespace's standing, if it has one.
pub fn settings_from_config(
	root: &rivet_config::config::Root,
	v3_datacenter_label: u16,
	target_group: Option<[u8; 16]>,
) -> Settings {
	let Some(migration) = root.migrate_from_v2() else {
		return Settings::disabled();
	};

	let mut datacenter_labels = migration
		.api_urls
		.keys()
		.filter_map(|label| label.parse::<u16>().ok())
		.collect::<Vec<_>>();
	datacenter_labels.sort_unstable();

	Settings {
		enabled: !datacenter_labels.is_empty(),
		migrate_ratio_ppm: (migration.migrate_ratio.clamp(0.0, 1.0) * 1_000_000.0).round() as u32,
		migrate_seed: migration.migrate_seed,
		max_in_flight: migration.max_in_flight(),
		max_actors_per_sec: migration.max_actors_per_sec(),
		pass_interval_ms: migration.pass_interval_ms(),
		min_sleep_ms: migration.min_sleep_ms(),
		wake_horizon_ms: migration.wake_horizon_ms(),
		confirm_delay_ms: migration.confirm_delay_ms(),
		max_copy_bytes: migration.max_copy_bytes(),
		max_copy_bytes_per_sec: migration.max_copy_bytes_per_sec(),
		datacenter_labels,
		target_namespace: migration.target_namespace().to_owned(),
		v3_datacenter_label,
		target_group,
	}
}
