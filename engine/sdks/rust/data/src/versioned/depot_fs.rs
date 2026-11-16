//! Version envelopes for filesystem payloads and typed row values.
//!
//! Version 2 of every filesystem schema carries cuts over claim logs and names the claim log
//! that wrote each row; version 1 is claim log 0 throughout. Version 2 also adds volume groups:
//! the fields a volume made by a group fork carries, the fork pin row and the link to one, and
//! the group fork selector. The converters live in `depot_fs_v2`.

use crate::generated::{
	depot_fs_entry_v1 as entry1, depot_fs_entry_v2 as entry2, depot_fs_rows_v1 as rows1,
	depot_fs_rows_v2 as rows2, shared_depot_fs_v1 as fs1, shared_depot_fs_v2 as fs2,
};

macro_rules! envelope {
	($name:ident, $family:ident, $v1:ty, $v2:ty) => {
		pub enum $name {
			V1($v1),
			V2($v2),
		}

		impl_versioned_v2!($name, $family, $v1, $v2);
	};
}

envelope!(
	DepotFsEntryData,
	DEPOT_FS_ENTRY,
	entry1::FsEntry,
	entry2::FsEntry
);

envelope!(
	DepotFsInodeRowData,
	DEPOT_FS_ROWS,
	rows1::InodeRow,
	rows2::InodeRow
);
envelope!(
	DepotFsDentryRowData,
	DEPOT_FS_ROWS,
	rows1::DentryRow,
	rows2::DentryRow
);
envelope!(
	DepotFsLinkRowData,
	DEPOT_FS_ROWS,
	rows1::LinkRow,
	rows2::LinkRow
);
envelope!(
	DepotFsXattrRowData,
	DEPOT_FS_ROWS,
	rows1::XattrRow,
	rows2::XattrRow
);
envelope!(DepotFsKvRowData, DEPOT_FS_ROWS, rows1::KvRow, rows2::KvRow);
envelope!(
	DepotFsOrphanRowData,
	DEPOT_FS_ROWS,
	rows1::OrphanRow,
	rows2::OrphanRow
);
envelope!(
	DepotFsChunkRowData,
	DEPOT_FS_ROWS,
	rows1::ChunkRow,
	rows2::ChunkRow
);
envelope!(
	DepotFsFormatRowData,
	DEPOT_FS_ROWS,
	rows1::FormatRow,
	rows2::FormatRow
);

/// A range whiteout row exists only from version 2: a volume reads through its base links with
/// version 2 rows, and a writer that encodes rows below it has no volume with a lineage.
pub enum DepotFsKvRangeWhiteoutRowData {
	V2(rows2::KvRangeWhiteoutRow),
}

impl_versioned_v2_only!(
	DepotFsKvRangeWhiteoutRowData,
	DEPOT_FS_ROWS,
	rows2::KvRangeWhiteoutRow
);

/// A fork pin row exists only from version 2: nothing written before volume groups existed has
/// one, and a writer that still encodes rows at an older version cannot write one.
pub enum DepotFsForkPinData {
	V2(fs2::FsForkPin),
}

impl_versioned_family!(DepotFsForkPinData, DEPOT_FS_ROWS);

impl ::vbare::Versioned for DepotFsForkPinData {
	type Latest = fs2::FsForkPin;

	const LATEST: u16 = 2;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V2(latest)
	}

	fn unwrap_latest(self) -> ::anyhow::Result<Self::Latest> {
		let Self::V2(latest) = self;
		Ok(latest)
	}

	fn decode_version(
		version: u16,
		body: ::vbare::Bytes,
	) -> ::std::result::Result<Self, ::vbare::DecodeError> {
		match version {
			2 => Ok(Self::V2(<fs2::FsForkPin as ::vbare::Body>::from_body(
				body,
			)?)),
			_ => Err(super::unsupported_version(version, 2)),
		}
	}

	fn encode_version<S: ::vbare::Sink>(
		&self,
		version: u16,
		out: &mut S,
	) -> ::std::result::Result<(), ::vbare::EncodeError> {
		if version != 2 {
			return Err(::vbare::EncodeError::UnsupportedVersion(version));
		}
		let Self::V2(latest) = self;
		::vbare::Body::put_body(latest, out);
		Ok(())
	}
}

/// A fork body an application persists to resume its fork after a restart, as the actor state
/// machine keeps its prepared database fork. Only from version 2: no application kept one
/// before volume groups existed.
///
/// Each release may add at most one version of this body. Readers treat the version one past
/// their latest as newer and wait, and anything further as corrupt and fail the fork (see
/// `depot_client::forks::decode_spec`), so two versions in one release, or a fleet upgraded
/// across two releases that each add one, would fail forks that a newer binary can still finish.
/// The `fork_spec_version_moves_one_step_per_release` test pins the step.
pub enum DepotFsForkSpecData {
	V2(fs2::FsForkSpec),
}

impl_versioned_v2_only!(DepotFsForkSpecData, DEPOT_FS_ROWS, fs2::FsForkSpec);

// Rows of the shared filesystem schema are versioned with `depot.fs_rows`, whose version
// selects the shared schema version they are encoded at.
envelope!(
	DepotFsVolumeRecordData,
	DEPOT_FS_ROWS,
	fs1::FsVolumeRecord,
	fs2::FsVolumeRecord
);
envelope!(
	DepotFsVolumeCheckpointData,
	DEPOT_FS_ROWS,
	fs1::FsVolumeCheckpoint,
	fs2::FsVolumeCheckpoint
);
envelope!(
	DepotFsForkHoldData,
	DEPOT_FS_ROWS,
	fs1::FsForkHold,
	fs2::FsForkHold
);
envelope!(
	DepotFsPendingForkCheckData,
	DEPOT_FS_ROWS,
	fs1::FsPendingForkCheck,
	fs2::FsPendingForkCheck
);
envelope!(
	DepotFsPendingMaterializeData,
	DEPOT_FS_ROWS,
	fs1::FsPendingMaterialize,
	fs2::FsPendingMaterialize
);
envelope!(
	DepotFsPendingSweepData,
	DEPOT_FS_ROWS,
	fs1::FsPendingSweep,
	fs2::FsPendingSweep
);
envelope!(
	DepotFsDirectoryCursorData,
	DEPOT_FS_ROWS,
	fs1::FsDirectoryCursor,
	fs2::FsDirectoryCursor
);
envelope!(
	DepotFsForkReferenceData,
	DEPOT_FS_ROWS,
	fs1::FsForkReference,
	fs2::FsForkReference
);
envelope!(
	DepotFsPendingForkData,
	DEPOT_FS_ROWS,
	fs1::FsPendingFork,
	fs2::FsPendingFork
);
envelope!(
	DepotFsIncarnationRecordData,
	DEPOT_FS_ROWS,
	fs1::FsIncarnationRecord,
	fs2::FsIncarnationRecord
);
envelope!(
	DepotFsPendingForkReleaseData,
	DEPOT_FS_ROWS,
	fs1::FsPendingForkRelease,
	fs2::FsPendingForkRelease
);
envelope!(
	DepotFsForkFailureData,
	DEPOT_FS_ROWS,
	fs1::FsForkFailure,
	fs2::FsForkFailure
);
