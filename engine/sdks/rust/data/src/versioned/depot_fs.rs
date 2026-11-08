//! Version envelopes for filesystem payloads and typed row values.

macro_rules! envelope {
	($name:ident, $family:ident, $ty:ty) => {
		pub enum $name {
			V1($ty),
		}

		impl_versioned_v1!($name, $family, $ty);
	};
}

envelope!(
	DepotFsEntryData,
	DEPOT_FS_ENTRY,
	crate::generated::depot_fs_entry_v1::FsEntry
);

envelope!(
	DepotFsInodeRowData,
	DEPOT_FS_ROWS,
	crate::generated::depot_fs_rows_v1::InodeRow
);
envelope!(
	DepotFsDentryRowData,
	DEPOT_FS_ROWS,
	crate::generated::depot_fs_rows_v1::DentryRow
);
envelope!(
	DepotFsLinkRowData,
	DEPOT_FS_ROWS,
	crate::generated::depot_fs_rows_v1::LinkRow
);
envelope!(
	DepotFsXattrRowData,
	DEPOT_FS_ROWS,
	crate::generated::depot_fs_rows_v1::XattrRow
);
envelope!(
	DepotFsKvRowData,
	DEPOT_FS_ROWS,
	crate::generated::depot_fs_rows_v1::KvRow
);
envelope!(
	DepotFsOrphanRowData,
	DEPOT_FS_ROWS,
	crate::generated::depot_fs_rows_v1::OrphanRow
);
envelope!(
	DepotFsChunkRowData,
	DEPOT_FS_ROWS,
	crate::generated::depot_fs_rows_v1::ChunkRow
);
envelope!(
	DepotFsVolumeRecordData,
	DEPOT_FS_ROWS,
	crate::generated::shared_depot_fs_v1::FsVolumeRecord
);
envelope!(
	DepotFsVolumeCheckpointData,
	DEPOT_FS_ROWS,
	crate::generated::shared_depot_fs_v1::FsVolumeCheckpoint
);
envelope!(
	DepotFsForkHoldData,
	DEPOT_FS_ROWS,
	crate::generated::shared_depot_fs_v1::FsForkHold
);
envelope!(
	DepotFsPendingForkCheckData,
	DEPOT_FS_ROWS,
	crate::generated::shared_depot_fs_v1::FsPendingForkCheck
);
envelope!(
	DepotFsPendingMaterializeData,
	DEPOT_FS_ROWS,
	crate::generated::shared_depot_fs_v1::FsPendingMaterialize
);
envelope!(
	DepotFsPendingSweepData,
	DEPOT_FS_ROWS,
	crate::generated::shared_depot_fs_v1::FsPendingSweep
);
envelope!(
	DepotFsDirectoryCursorData,
	DEPOT_FS_ROWS,
	crate::generated::shared_depot_fs_v1::FsDirectoryCursor
);

envelope!(
	DepotFsForkReferenceData,
	DEPOT_FS_ROWS,
	crate::generated::shared_depot_fs_v1::FsForkReference
);

envelope!(
	DepotFsPendingForkData,
	DEPOT_FS_ROWS,
	crate::generated::shared_depot_fs_v1::FsPendingFork
);

envelope!(
	DepotFsIncarnationRecordData,
	DEPOT_FS_ROWS,
	crate::generated::shared_depot_fs_v1::FsIncarnationRecord
);

envelope!(
	DepotFsPendingForkReleaseData,
	DEPOT_FS_ROWS,
	crate::generated::shared_depot_fs_v1::FsPendingForkRelease
);

envelope!(
	DepotFsForkFailureData,
	DEPOT_FS_ROWS,
	crate::generated::shared_depot_fs_v1::FsForkFailure
);

envelope!(
	DepotFsFormatRowData,
	DEPOT_FS_ROWS,
	crate::generated::depot_fs_rows_v1::FormatRow
);
