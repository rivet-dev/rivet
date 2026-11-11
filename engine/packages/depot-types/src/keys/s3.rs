//! Object-store addresses outside the tablet's sorted key/value rows.

use uuid::Uuid;
use vbare::Versioned;

// The coordinator's conditional-write row at rivet/depot/manifest.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::S3Key)]
#[s3(key = [RIVET, DEPOT, MANIFEST], value = vbare(rivet_data::versioned::DepotAlarmTabletManifestData, rivet_data::versioned::DepotAlarmTabletManifestData::LATEST))]
pub struct ManifestKey {}

// Present while the single-node provisioning service of one storage worker is creating and
// granting its tablets, so a restart can tell its own partial layout from an operator's.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::S3Key)]
#[s3(key = [RIVET, DEPOT, AUTO_PROVISION, worker_id], value = unit)]
pub struct AutoProvisionIntentKey {
	pub worker_id: Uuid,
}

// SlateDB owns all objects below rivet/depot/tablets/{tablet_id}/volumes/.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::S3Key)]
#[s3(key = [RIVET, DEPOT, TABLET, tablet_id, VOLUME], value = unit)]
pub struct TabletVolumePrefix {
	pub tablet_id: Uuid,
}
