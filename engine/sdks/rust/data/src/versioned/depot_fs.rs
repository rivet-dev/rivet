//! Version envelopes for filesystem payloads and typed row values.

macro_rules! envelope {
	($name:ident, $ty:ty) => {
		pub enum $name {
			V1($ty),
		}

		impl_versioned_v1!($name, $ty);
	};
}

#[cfg(not(feature = "test-util"))]
envelope!(
	DepotFsEntryData,
	crate::generated::depot_fs_entry_v1::FsEntry
);






envelope!(
	DepotFsInodeRowData,
	crate::generated::depot_fs_rows_v1::InodeRow
);
envelope!(
	DepotFsDentryRowData,
	crate::generated::depot_fs_rows_v1::DentryRow
);
envelope!(
	DepotFsLinkRowData,
	crate::generated::depot_fs_rows_v1::LinkRow
);
envelope!(
	DepotFsXattrRowData,
	crate::generated::depot_fs_rows_v1::XattrRow
);
envelope!(DepotFsKvRowData, crate::generated::depot_fs_rows_v1::KvRow);
envelope!(
	DepotFsOrphanRowData,
	crate::generated::depot_fs_rows_v1::OrphanRow
);
envelope!(
	DepotFsChunkRowData,
	crate::generated::depot_fs_rows_v1::ChunkRow
);
envelope!(
	DepotFsVolumeRecordData,
	crate::generated::shared_depot_fs_v1::FsVolumeRecord
);
envelope!(
	DepotFsVolumeCheckpointData,
	crate::generated::shared_depot_fs_v1::FsVolumeCheckpoint
);
envelope!(
	DepotFsForkHoldData,
	crate::generated::shared_depot_fs_v1::FsForkHold
);
envelope!(
	DepotFsPendingForkCheckData,
	crate::generated::shared_depot_fs_v1::FsPendingForkCheck
);
envelope!(
	DepotFsPendingMaterializeData,
	crate::generated::shared_depot_fs_v1::FsPendingMaterialize
);
envelope!(
	DepotFsPendingSweepData,
	crate::generated::shared_depot_fs_v1::FsPendingSweep
);
envelope!(
	DepotFsDirectoryCursorData,
	crate::generated::shared_depot_fs_v1::FsDirectoryCursor
);

envelope!(
	DepotFsForkReferenceData,
	crate::generated::shared_depot_fs_v1::FsForkReference
);

envelope!(
	DepotFsPendingForkData,
	crate::generated::shared_depot_fs_v1::FsPendingFork
);

envelope!(
	DepotFsIncarnationRecordData,
	crate::generated::shared_depot_fs_v1::FsIncarnationRecord
);

envelope!(
	DepotFsPendingForkReleaseData,
	crate::generated::shared_depot_fs_v1::FsPendingForkRelease
);

envelope!(
	DepotFsForkFailureData,
	crate::generated::shared_depot_fs_v1::FsForkFailure
);

envelope!(
	DepotFsFormatRowData,
	crate::generated::depot_fs_rows_v1::FormatRow
);
