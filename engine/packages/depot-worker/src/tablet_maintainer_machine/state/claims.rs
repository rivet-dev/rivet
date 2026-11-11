//! The keys of a volume's claim logs. A volume keeps one row space, and claim log 0, its base
//! key, holds the implicit claim on the root directory. Every volume has that one log, so
//! [`volume_logs`] names the base key alone; stored cuts and logged targets still name logs by
//! claim log number, which [`log_key`] resolves.

use depot_types::VolumeLogKey;
use rivet_data::ROOT_CLAIM_LOG;

/// The base key and claim log of `key`: a claim log's base volume, or `key` itself as claim
/// log 0.
pub(crate) fn base_of(key: &VolumeLogKey) -> (VolumeLogKey, u16) {
	match depot_types::is_claim_log(&key.0.volume_key) {
		Some((base, claim_log)) => {
			let mut wire = key.0.clone();
			wire.volume_key = base;
			(wire.into(), claim_log)
		}
		None => (key.clone(), ROOT_CLAIM_LOG),
	}
}

/// The key of claim log `claim_log` of the volume whose base key is `base`.
pub(crate) fn log_key(base: &VolumeLogKey, claim_log: u16) -> Option<VolumeLogKey> {
	let mut wire = base.0.clone();
	wire.volume_key = depot_types::claim_log_key(&base.0.volume_key, claim_log).ok()?;
	Some(wire.into())
}

/// Every log of `key`'s volume: its claim log 0, the base key.
pub(crate) fn volume_logs(key: &VolumeLogKey) -> Vec<VolumeLogKey> {
	vec![base_of(key).0]
}
