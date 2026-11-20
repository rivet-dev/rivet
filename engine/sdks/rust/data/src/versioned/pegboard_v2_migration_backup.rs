//! Version envelope for the manifest of a V2 actor's export backup.

use crate::generated::pegboard_v2_migration_backup_v1 as wire;

pub enum BackupManifest {
	V1(wire::BackupManifest),
}

impl_versioned_v1!(
	BackupManifest,
	PEGBOARD_V2_MIGRATION_BACKUP,
	wire::BackupManifest
);
