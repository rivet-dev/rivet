//! Object-store addresses outside the tablet's sorted key/value rows.

use uuid::Uuid;
use vbare::Versioned;

// The coordinator's conditional-write row at rivet/depot/manifest.
rivet_keys::s3_key! {
	pub ManifestKey() => [RIVET, DEPOT, MANIFEST];
	value: vbare(rivet_data::versioned::DepotAlarmTabletManifestData, rivet_data::versioned::DepotAlarmTabletManifestData::LATEST);
}

// Present while the single-node provisioning service of one storage worker is creating and
// granting its tablets, so a restart can tell its own partial layout from an operator's.
rivet_keys::s3_key! {
	pub AutoProvisionIntentKey(pub worker_id: Uuid) => [RIVET, DEPOT, AUTO_PROVISION, {worker_id}];
	value: unit;
}

// SlateDB owns all objects below rivet/depot/tablets/{tablet_id}/volumes/.
rivet_keys::s3_key! {
	pub TabletVolumePrefix(pub tablet_id: Uuid) => [RIVET, DEPOT, TABLET, {tablet_id}, VOLUME];
	value: unit;
}
