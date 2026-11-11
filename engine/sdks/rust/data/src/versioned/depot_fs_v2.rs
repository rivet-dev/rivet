//! Converters between versions 1 and 2 of the filesystem schemas: `shared/depot.fs`,
//! `depot.fs_entry`, `depot.fs_rows` and `shared/depot.fs_read`.
//!
//! Version 2 replaces every position that spans a volume with a cut over its claim logs and
//! names the claim log that wrote each stored row. A version 1 value is claim log 0 throughout:
//! an upgrade wraps each position in a one-log cut and stamps claim log 0, and a downgrade
//! refuses a value that names any other claim log, which a cluster with a version 1 reader
//! never writes. The types that did not change convert through the `From` impls `build.rs`
//! generates; only the types that changed, and the types that contain them, are mapped here.
//!
//! Version 2 also adds volume groups. A volume record carries the fork numbers a grouped volume
//! is stamped with and where a lazily forked volume came from, a base link can name a fork pin,
//! a fork pin row exists, a fork acquire can select the parent's state as of a group fork, a
//! volume can hold range whiteouts, and a child can read one layer of its base's rows. A version
//! 1 value has none of these: an upgrade leaves every new field absent, and a downgrade refuses a
//! value that sets one, which a cluster with a version 1 reader never writes, because group
//! creation is gated on the fleet's protocol version. Version 2 also refuses a fork for now as
//! `TailUnproven`, which a downgrade reads as `Busy`.

use crate::{
	ROOT_CLAIM_LOG,
	generated::{
		depot_fs_entry_v1 as entry1, depot_fs_entry_v2 as entry2, depot_fs_rows_v1 as rows1,
		depot_fs_rows_v2 as rows2, shared_depot_fs_read_v1 as read1,
		shared_depot_fs_read_v2 as read2, shared_depot_fs_v1 as fs1, shared_depot_fs_v2 as fs2,
		shared_depot_types_v1::VolumeLogPosition,
	},
};
use anyhow::{Result, ensure};

/// A value at the older of two schema versions, from the newer one. Fails for a value the
/// older version cannot carry.
pub trait Downgrade {
	type Older;

	fn downgrade(self) -> Result<Self::Older>;
}

/// A void type is the same `()` at every version.
impl Downgrade for () {
	type Older = ();

	fn downgrade(self) -> Result<()> {
		Ok(())
	}
}

/// Implements [`Downgrade`] through the generated `From` for types that did not change.
macro_rules! downgrade_into {
	($($newer:ty => $older:ty),* $(,)?) => {
		$(impl Downgrade for $newer {
			type Older = $older;

			fn downgrade(self) -> Result<$older> {
				Ok(self.into())
			}
		})*
	};
}

pub(crate) use downgrade_into;

pub(crate) fn downgrade_all<T: Downgrade>(items: Vec<T>) -> Result<Vec<T::Older>> {
	items.into_iter().map(Downgrade::downgrade).collect()
}

pub(crate) fn downgrade_opt<T: Downgrade>(item: Option<T>) -> Result<Option<T::Older>> {
	item.map(Downgrade::downgrade).transpose()
}

/// Refuses a stored row stamped by a claim log version 1 cannot name.
fn root_claim_log(claim_log: u16) -> Result<()> {
	ensure!(
		claim_log == ROOT_CLAIM_LOG,
		"the row was written by claim log {claim_log}, which version 1 cannot name"
	);
	Ok(())
}

// Cuts.

impl Downgrade for fs2::FsCut {
	type Older = VolumeLogPosition;

	fn downgrade(self) -> Result<VolumeLogPosition> {
		self.into_single()
	}
}

/// Drops a checkpoint or hold row's cut, which its key already carries for claim log 0.
fn drop_cut(cut: Option<fs2::FsCut>) -> Result<()> {
	if let Some(cut) = cut {
		cut.into_single()?;
	}
	Ok(())
}

// shared/depot.fs

impl From<fs1::FsVolumeCheckpoint> for fs2::FsVolumeCheckpoint {
	fn from(value: fs1::FsVolumeCheckpoint) -> Self {
		Self {
			named: value.named,
			retention: value.retention,
			created_at: value.created_at,
			client_epoch: value.client_epoch,
			cut: None,
		}
	}
}

impl Downgrade for fs2::FsVolumeCheckpoint {
	type Older = fs1::FsVolumeCheckpoint;

	fn downgrade(self) -> Result<Self::Older> {
		drop_cut(self.cut)?;
		Ok(fs1::FsVolumeCheckpoint {
			named: self.named,
			retention: self.retention,
			created_at: self.created_at,
			client_epoch: self.client_epoch,
		})
	}
}

impl From<fs1::FsForkHold> for fs2::FsForkHold {
	fn from(value: fs1::FsForkHold) -> Self {
		Self {
			fork_volume_key: value.fork_volume_key,
			fork_region: value.fork_region,
			created_at: value.created_at,
			cut: None,
		}
	}
}

impl Downgrade for fs2::FsForkHold {
	type Older = fs1::FsForkHold;

	fn downgrade(self) -> Result<Self::Older> {
		drop_cut(self.cut)?;
		Ok(fs1::FsForkHold {
			fork_volume_key: self.fork_volume_key,
			fork_region: self.fork_region,
			created_at: self.created_at,
		})
	}
}

impl From<fs1::FsBaseLink> for fs2::FsBaseLink {
	fn from(value: fs1::FsBaseLink) -> Self {
		Self {
			volume_key: value.volume_key,
			incarnation: value.incarnation,
			checkpoint_name: value.checkpoint_name,
			position: fs2::FsCut::single(value.position),
			region: value.region,
			fork_pin: None,
		}
	}
}

impl Downgrade for fs2::FsBaseLink {
	type Older = fs1::FsBaseLink;

	fn downgrade(self) -> Result<Self::Older> {
		ensure!(self.fork_pin.is_none(), "version 1 has no fork pins");
		Ok(fs1::FsBaseLink {
			volume_key: self.volume_key,
			incarnation: self.incarnation,
			checkpoint_name: self.checkpoint_name,
			position: self.position.downgrade()?,
			region: self.region,
		})
	}
}

impl From<fs1::FsVolumeRecord> for fs2::FsVolumeRecord {
	fn from(value: fs1::FsVolumeRecord) -> Self {
		let forked = !value.base_chain.is_empty();
		Self {
			partition_key: value.partition_key,
			name: value.name,
			volume_incarnation: value.volume_incarnation,
			status: value.status.into(),
			cold: value.cold,
			base_chain: value.base_chain.into_iter().map(Into::into).collect(),
			retention: value.retention.map(Into::into),
			recent_writes: value.recent_writes.into_iter().map(Into::into).collect(),
			incarnation_epoch: value.incarnation_epoch,
			fork_pinned_through: None,
			created_at_fork: None,
			deleted_at_fork: None,
			lazy_fork: None,
			// Every version 1 fork copied its parent's metadata before it published.
			metadata_local: forked,
		}
	}
}

impl Downgrade for fs2::FsVolumeRecord {
	type Older = fs1::FsVolumeRecord;

	fn downgrade(self) -> Result<Self::Older> {
		ensure!(
			self.fork_pinned_through.is_none()
				&& self.created_at_fork.is_none()
				&& self.deleted_at_fork.is_none()
				&& self.lazy_fork.is_none(),
			"version 1 has no volume groups"
		);
		ensure!(
			self.metadata_local || self.base_chain.is_empty(),
			"version 1 has no fork that reads its metadata through its base"
		);
		Ok(fs1::FsVolumeRecord {
			partition_key: self.partition_key,
			name: self.name,
			volume_incarnation: self.volume_incarnation,
			status: self.status.into(),
			cold: self.cold,
			base_chain: downgrade_all(self.base_chain)?,
			retention: self.retention.map(Into::into),
			recent_writes: self.recent_writes.into_iter().map(Into::into).collect(),
			incarnation_epoch: self.incarnation_epoch,
		})
	}
}

impl From<fs1::FsCheckpointInfo> for fs2::FsCheckpointInfo {
	fn from(value: fs1::FsCheckpointInfo) -> Self {
		Self {
			name: value.name,
			volume_incarnation: value.volume_incarnation,
			position: fs2::FsCut::single(value.position),
			checkpoint: value.checkpoint.into(),
		}
	}
}

impl Downgrade for fs2::FsCheckpointInfo {
	type Older = fs1::FsCheckpointInfo;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsCheckpointInfo {
			name: self.name,
			volume_incarnation: self.volume_incarnation,
			position: self.position.downgrade()?,
			checkpoint: self.checkpoint.downgrade()?,
		})
	}
}

impl From<fs1::FsForkMinimum> for fs2::FsForkMinimum {
	fn from(value: fs1::FsForkMinimum) -> Self {
		Self {
			incarnation: value.incarnation,
			position: fs2::FsCut::single(value.position),
		}
	}
}

impl Downgrade for fs2::FsForkMinimum {
	type Older = fs1::FsForkMinimum;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsForkMinimum {
			incarnation: self.incarnation,
			position: self.position.downgrade()?,
		})
	}
}

impl From<fs1::FsForkSelector> for fs2::FsForkSelector {
	fn from(value: fs1::FsForkSelector) -> Self {
		match value {
			fs1::FsForkSelector::FsForkLatest => Self::FsForkLatest,
			fs1::FsForkSelector::FsForkNamed(v) => Self::FsForkNamed(v.into()),
			fs1::FsForkSelector::FsForkAtTime(v) => Self::FsForkAtTime(v.into()),
		}
	}
}

impl Downgrade for fs2::FsForkSelector {
	type Older = fs1::FsForkSelector;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsForkLatest => fs1::FsForkSelector::FsForkLatest,
			Self::FsForkNamed(v) => fs1::FsForkSelector::FsForkNamed(v.into()),
			Self::FsForkAtTime(v) => fs1::FsForkSelector::FsForkAtTime(v.into()),
			Self::FsForkGroup(_) => anyhow::bail!("version 1 has no group fork selector"),
		})
	}
}

impl From<fs1::FsForkSpec> for fs2::FsForkSpec {
	fn from(value: fs1::FsForkSpec) -> Self {
		Self {
			operation_id: value.operation_id,
			parent_volume_key: value.parent_volume_key,
			parent_region: value.parent_region,
			destination_volume_key: value.destination_volume_key,
			destination_region: value.destination_region,
			selector: value.selector.into(),
			minimum: value.minimum.map(Into::into),
			restore: value.restore,
		}
	}
}

impl Downgrade for fs2::FsForkSpec {
	type Older = fs1::FsForkSpec;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsForkSpec {
			operation_id: self.operation_id,
			parent_volume_key: self.parent_volume_key,
			parent_region: self.parent_region,
			destination_volume_key: self.destination_volume_key,
			destination_region: self.destination_region,
			selector: self.selector.downgrade()?,
			minimum: downgrade_opt(self.minimum)?,
			restore: self.restore,
		})
	}
}

impl From<fs1::FsForkHoldRelease> for fs2::FsForkHoldRelease {
	fn from(value: fs1::FsForkHoldRelease) -> Self {
		Self {
			spec: value.spec.into(),
		}
	}
}

impl Downgrade for fs2::FsForkHoldRelease {
	type Older = fs1::FsForkHoldRelease;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsForkHoldRelease {
			spec: self.spec.downgrade()?,
		})
	}
}

impl From<fs1::FsForkCheckDone> for fs2::FsForkCheckDone {
	fn from(value: fs1::FsForkCheckDone) -> Self {
		Self {
			spec: value.spec.into(),
		}
	}
}

impl Downgrade for fs2::FsForkCheckDone {
	type Older = fs1::FsForkCheckDone;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsForkCheckDone {
			spec: self.spec.downgrade()?,
		})
	}
}

impl From<fs1::FsOp> for fs2::FsOp {
	fn from(value: fs1::FsOp) -> Self {
		match value {
			fs1::FsOp::FsRoot(v) => Self::FsRoot(v.into()),
			fs1::FsOp::FsCreate(v) => Self::FsCreate(v.into()),
			fs1::FsOp::FsMkdir(v) => Self::FsMkdir(v.into()),
			fs1::FsOp::FsSymlink(v) => Self::FsSymlink(v.into()),
			fs1::FsOp::FsMknod(v) => Self::FsMknod(v.into()),
			fs1::FsOp::FsLink(v) => Self::FsLink(v.into()),
			fs1::FsOp::FsUnlink(v) => Self::FsUnlink(v.into()),
			fs1::FsOp::FsRename(v) => Self::FsRename(v.into()),
			fs1::FsOp::FsSetAttr(v) => Self::FsSetAttr(v.into()),
			fs1::FsOp::FsWrite(v) => Self::FsWrite(v.into()),
			fs1::FsOp::FsSetXattr(v) => Self::FsSetXattr(v.into()),
			fs1::FsOp::FsRemoveXattr(v) => Self::FsRemoveXattr(v.into()),
			fs1::FsOp::FsReclaim(v) => Self::FsReclaim(v.into()),
			fs1::FsOp::FsKvPut(v) => Self::FsKvPut(v.into()),
			fs1::FsOp::FsKvDelete(v) => Self::FsKvDelete(v.into()),
			fs1::FsOp::FsKvDeleteRange(v) => Self::FsKvDeleteRange(v.into()),
			fs1::FsOp::FsCheckpoint(v) => Self::FsCheckpoint(v.into()),
			fs1::FsOp::FsCheckpointRelease(v) => Self::FsCheckpointRelease(v.into()),
			fs1::FsOp::FsRetentionSet(v) => Self::FsRetentionSet(v.into()),
			fs1::FsOp::FsForkHoldRelease(v) => Self::FsForkHoldRelease(v.into()),
			fs1::FsOp::FsForkCheckDone(v) => Self::FsForkCheckDone(v.into()),
		}
	}
}

impl Downgrade for fs2::FsOp {
	type Older = fs1::FsOp;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsRoot(v) => fs1::FsOp::FsRoot(v.downgrade()?),
			Self::FsCreate(v) => fs1::FsOp::FsCreate(v.into()),
			Self::FsMkdir(v) => fs1::FsOp::FsMkdir(v.into()),
			Self::FsSymlink(v) => fs1::FsOp::FsSymlink(v.into()),
			Self::FsMknod(v) => fs1::FsOp::FsMknod(v.into()),
			Self::FsLink(v) => fs1::FsOp::FsLink(v.into()),
			Self::FsUnlink(v) => fs1::FsOp::FsUnlink(v.into()),
			Self::FsRename(v) => fs1::FsOp::FsRename(v.into()),
			Self::FsSetAttr(v) => fs1::FsOp::FsSetAttr(v.into()),
			Self::FsWrite(v) => fs1::FsOp::FsWrite(v.into()),
			Self::FsSetXattr(v) => fs1::FsOp::FsSetXattr(v.into()),
			Self::FsRemoveXattr(v) => fs1::FsOp::FsRemoveXattr(v.into()),
			Self::FsReclaim(v) => fs1::FsOp::FsReclaim(v.into()),
			Self::FsKvPut(v) => fs1::FsOp::FsKvPut(v.into()),
			Self::FsKvDelete(v) => fs1::FsOp::FsKvDelete(v.into()),
			Self::FsKvDeleteRange(v) => fs1::FsOp::FsKvDeleteRange(v.into()),
			Self::FsCheckpoint(v) => fs1::FsOp::FsCheckpoint(v.into()),
			Self::FsCheckpointRelease(v) => fs1::FsOp::FsCheckpointRelease(v.into()),
			Self::FsRetentionSet(v) => fs1::FsOp::FsRetentionSet(v.into()),
			Self::FsForkHoldRelease(v) => fs1::FsOp::FsForkHoldRelease(v.downgrade()?),
			Self::FsForkCheckDone(v) => fs1::FsOp::FsForkCheckDone(v.downgrade()?),
		})
	}
}

impl From<fs1::FsForkImage> for fs2::FsForkImage {
	fn from(value: fs1::FsForkImage) -> Self {
		Self {
			link: value.link.into(),
			parent_base: value.parent_base.into_iter().map(Into::into).collect(),
		}
	}
}

impl Downgrade for fs2::FsForkImage {
	type Older = fs1::FsForkImage;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsForkImage {
			link: self.link.downgrade()?,
			parent_base: downgrade_all(self.parent_base)?,
		})
	}
}

impl From<fs1::FsForkReceipt> for fs2::FsForkReceipt {
	fn from(value: fs1::FsForkReceipt) -> Self {
		Self {
			spec: value.spec.into(),
			image: value.image.into(),
		}
	}
}

impl Downgrade for fs2::FsForkReceipt {
	type Older = fs1::FsForkReceipt;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsForkReceipt {
			spec: self.spec.downgrade()?,
			image: self.image.downgrade()?,
		})
	}
}

impl From<fs1::FsForkReferenceHeld> for fs2::FsForkReferenceHeld {
	fn from(value: fs1::FsForkReferenceHeld) -> Self {
		Self {
			receipt: value.receipt.into(),
			created_at: value.created_at,
		}
	}
}

impl Downgrade for fs2::FsForkReferenceHeld {
	type Older = fs1::FsForkReferenceHeld;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsForkReferenceHeld {
			receipt: self.receipt.downgrade()?,
			created_at: self.created_at,
		})
	}
}

impl From<fs1::FsForkReferenceReleased> for fs2::FsForkReferenceReleased {
	fn from(value: fs1::FsForkReferenceReleased) -> Self {
		Self {
			spec: value.spec.into(),
		}
	}
}

impl Downgrade for fs2::FsForkReferenceReleased {
	type Older = fs1::FsForkReferenceReleased;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsForkReferenceReleased {
			spec: self.spec.downgrade()?,
		})
	}
}

impl From<fs1::FsForkReference> for fs2::FsForkReference {
	fn from(value: fs1::FsForkReference) -> Self {
		match value {
			fs1::FsForkReference::FsForkReferenceHeld(v) => Self::FsForkReferenceHeld(v.into()),
			fs1::FsForkReference::FsForkReferenceReleased(v) => {
				Self::FsForkReferenceReleased(v.into())
			}
		}
	}
}

impl Downgrade for fs2::FsForkReference {
	type Older = fs1::FsForkReference;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsForkReferenceHeld(v) => {
				fs1::FsForkReference::FsForkReferenceHeld(v.downgrade()?)
			}
			Self::FsForkReferenceReleased(v) => {
				fs1::FsForkReference::FsForkReferenceReleased(v.downgrade()?)
			}
		})
	}
}

impl From<fs1::FsForkCopying> for fs2::FsForkCopying {
	fn from(value: fs1::FsForkCopying) -> Self {
		Self {
			image: value.image.into(),
			cursor: value.cursor,
		}
	}
}

impl Downgrade for fs2::FsForkCopying {
	type Older = fs1::FsForkCopying;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsForkCopying {
			image: self.image.downgrade()?,
			cursor: self.cursor,
		})
	}
}

impl From<fs1::FsForkReady> for fs2::FsForkReady {
	fn from(value: fs1::FsForkReady) -> Self {
		Self {
			image: value.image.into(),
			metadata_local: true,
		}
	}
}

impl Downgrade for fs2::FsForkReady {
	type Older = fs1::FsForkReady;

	fn downgrade(self) -> Result<Self::Older> {
		ensure!(
			self.metadata_local,
			"version 1 has no fork that reads its metadata through its base"
		);
		Ok(fs1::FsForkReady {
			image: self.image.downgrade()?,
		})
	}
}

impl From<fs1::FsForkPreparation> for fs2::FsForkPreparation {
	fn from(value: fs1::FsForkPreparation) -> Self {
		match value {
			fs1::FsForkPreparation::FsForkAcquirePending => Self::FsForkAcquirePending,
			fs1::FsForkPreparation::FsForkCopying(v) => Self::FsForkCopying(v.into()),
			fs1::FsForkPreparation::FsForkReady(v) => Self::FsForkReady(v.into()),
		}
	}
}

impl Downgrade for fs2::FsForkPreparation {
	type Older = fs1::FsForkPreparation;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsForkAcquirePending => fs1::FsForkPreparation::FsForkAcquirePending,
			Self::FsForkCopying(v) => fs1::FsForkPreparation::FsForkCopying(v.downgrade()?),
			Self::FsForkReady(v) => fs1::FsForkPreparation::FsForkReady(v.downgrade()?),
		})
	}
}

impl From<fs1::FsPendingFork> for fs2::FsPendingFork {
	fn from(value: fs1::FsPendingFork) -> Self {
		Self {
			spec: value.spec.into(),
			preparation: value.preparation.into(),
		}
	}
}

impl Downgrade for fs2::FsPendingFork {
	type Older = fs1::FsPendingFork;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsPendingFork {
			spec: self.spec.downgrade()?,
			preparation: self.preparation.downgrade()?,
		})
	}
}

impl From<fs1::FsIncarnationRecord> for fs2::FsIncarnationRecord {
	fn from(value: fs1::FsIncarnationRecord) -> Self {
		Self {
			record: value.record.into(),
			creation: value.creation.map(Into::into),
		}
	}
}

impl Downgrade for fs2::FsIncarnationRecord {
	type Older = fs1::FsIncarnationRecord;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsIncarnationRecord {
			record: self.record.downgrade()?,
			creation: downgrade_opt(self.creation)?,
		})
	}
}

impl From<fs1::FsPendingForkRelease> for fs2::FsPendingForkRelease {
	fn from(value: fs1::FsPendingForkRelease) -> Self {
		Self {
			spec: value.spec.into(),
		}
	}
}

impl Downgrade for fs2::FsPendingForkRelease {
	type Older = fs1::FsPendingForkRelease;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsPendingForkRelease {
			spec: self.spec.downgrade()?,
		})
	}
}

impl From<fs1::FsForkError> for fs2::FsForkError {
	fn from(value: fs1::FsForkError) -> Self {
		match value {
			fs1::FsForkError::Invalid => Self::Invalid,
			fs1::FsForkError::Busy => Self::Busy,
			fs1::FsForkError::DestinationExists => Self::DestinationExists,
			fs1::FsForkError::OperationConflict => Self::OperationConflict,
			fs1::FsForkError::ForkDeleted => Self::ForkDeleted,
			fs1::FsForkError::VolumeNotFound => Self::VolumeNotFound,
			fs1::FsForkError::VolumeDeleted => Self::VolumeDeleted,
			fs1::FsForkError::IncarnationChanged => Self::IncarnationChanged,
			fs1::FsForkError::Lagging => Self::Lagging,
			fs1::FsForkError::CheckpointNotFound => Self::CheckpointNotFound,
			fs1::FsForkError::CheckpointNotReachable => Self::CheckpointNotReachable,
			fs1::FsForkError::CheckpointAmbiguous => Self::CheckpointAmbiguous,
			fs1::FsForkError::ForkOutOfRetention => Self::ForkOutOfRetention,
			fs1::FsForkError::ForkChainTooDeep => Self::ForkChainTooDeep,
			fs1::FsForkError::BaseUnavailable => Self::BaseUnavailable,
			fs1::FsForkError::RegionUnavailable => Self::RegionUnavailable,
		}
	}
}

/// A version 1 reader has no `TailUnproven` and reads it as `Busy`: both refuse a fork for now,
/// and neither is ever a terminal failure, so no persisted `FsForkFailure` carries one.
impl From<fs2::FsForkError> for fs1::FsForkError {
	fn from(value: fs2::FsForkError) -> Self {
		match value {
			fs2::FsForkError::Invalid => Self::Invalid,
			fs2::FsForkError::Busy | fs2::FsForkError::TailUnproven => Self::Busy,
			fs2::FsForkError::DestinationExists => Self::DestinationExists,
			fs2::FsForkError::OperationConflict => Self::OperationConflict,
			fs2::FsForkError::ForkDeleted => Self::ForkDeleted,
			fs2::FsForkError::VolumeNotFound => Self::VolumeNotFound,
			fs2::FsForkError::VolumeDeleted => Self::VolumeDeleted,
			fs2::FsForkError::IncarnationChanged => Self::IncarnationChanged,
			fs2::FsForkError::Lagging => Self::Lagging,
			fs2::FsForkError::CheckpointNotFound => Self::CheckpointNotFound,
			fs2::FsForkError::CheckpointNotReachable => Self::CheckpointNotReachable,
			fs2::FsForkError::CheckpointAmbiguous => Self::CheckpointAmbiguous,
			fs2::FsForkError::ForkOutOfRetention => Self::ForkOutOfRetention,
			fs2::FsForkError::ForkChainTooDeep => Self::ForkChainTooDeep,
			fs2::FsForkError::BaseUnavailable => Self::BaseUnavailable,
			fs2::FsForkError::RegionUnavailable => Self::RegionUnavailable,
		}
	}
}

impl From<fs1::FsForkFailure> for fs2::FsForkFailure {
	fn from(value: fs1::FsForkFailure) -> Self {
		Self {
			spec: value.spec.into(),
			error: value.error.into(),
		}
	}
}

impl Downgrade for fs2::FsForkFailure {
	type Older = fs1::FsForkFailure;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(fs1::FsForkFailure {
			spec: self.spec.downgrade()?,
			error: self.error.into(),
		})
	}
}

downgrade_into! {
	fs2::FsFormat => fs1::FsFormat,
	fs2::FsRoot => fs1::FsRoot,
	fs2::FsPendingForkCheck => fs1::FsPendingForkCheck,
	fs2::FsPendingMaterialize => fs1::FsPendingMaterialize,
	fs2::FsPendingSweep => fs1::FsPendingSweep,
	fs2::FsDirectoryCursor => fs1::FsDirectoryCursor,
}

// depot.fs_entry

impl From<entry1::FsEntry> for entry2::FsEntry {
	fn from(value: entry1::FsEntry) -> Self {
		Self {
			ops: value.ops.into_iter().map(Into::into).collect(),
			txn: value.txn.map(Into::into),
		}
	}
}

impl Downgrade for entry2::FsEntry {
	type Older = entry1::FsEntry;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(entry1::FsEntry {
			ops: downgrade_all(self.ops)?,
			txn: self.txn.map(Into::into),
		})
	}
}

// depot.fs_rows: every row gains the claim log that wrote it.

/// Converts one row kind: a version 1 union becomes a version 2 envelope stamped claim log 0,
/// and a version 2 envelope stamped claim log 0 becomes the union.
macro_rules! row_envelope {
	($row:ident, $value:ident, $live:ident) => {
		impl From<rows1::$row> for rows2::$row {
			fn from(value: rows1::$row) -> Self {
				Self {
					claim_log: ROOT_CLAIM_LOG,
					row: match value {
						rows1::$row::$live(v) => rows2::$value::$live(v.into()),
						rows1::$row::FsDeleted => rows2::$value::FsDeleted,
					},
				}
			}
		}

		impl Downgrade for rows2::$row {
			type Older = rows1::$row;

			fn downgrade(self) -> Result<Self::Older> {
				root_claim_log(self.claim_log)?;
				Ok(match self.row {
					rows2::$value::$live(v) => rows1::$row::$live(v.into()),
					rows2::$value::FsDeleted => rows1::$row::FsDeleted,
				})
			}
		}
	};
	// A row whose live value is `void` carries no payload.
	($row:ident, $value:ident, $live:ident, void) => {
		impl From<rows1::$row> for rows2::$row {
			fn from(value: rows1::$row) -> Self {
				Self {
					claim_log: ROOT_CLAIM_LOG,
					row: match value {
						rows1::$row::$live => rows2::$value::$live,
						rows1::$row::FsDeleted => rows2::$value::FsDeleted,
					},
				}
			}
		}

		impl Downgrade for rows2::$row {
			type Older = rows1::$row;

			fn downgrade(self) -> Result<Self::Older> {
				root_claim_log(self.claim_log)?;
				Ok(match self.row {
					rows2::$value::$live => rows1::$row::$live,
					rows2::$value::FsDeleted => rows1::$row::FsDeleted,
				})
			}
		}
	};
}

impl From<rows1::FormatRow> for rows2::FormatRow {
	fn from(value: rows1::FormatRow) -> Self {
		Self {
			claim_log: ROOT_CLAIM_LOG,
			row: match value {
				rows1::FormatRow::FsFormat(v) => rows2::FormatRowValue::FsFormat(v.into()),
				rows1::FormatRow::FsDeleted => rows2::FormatRowValue::FsDeleted,
			},
		}
	}
}

impl Downgrade for rows2::FormatRow {
	type Older = rows1::FormatRow;

	fn downgrade(self) -> Result<Self::Older> {
		root_claim_log(self.claim_log)?;
		Ok(match self.row {
			rows2::FormatRowValue::FsFormat(v) => rows1::FormatRow::FsFormat(v.downgrade()?),
			rows2::FormatRowValue::FsDeleted => rows1::FormatRow::FsDeleted,
		})
	}
}

row_envelope!(InodeRow, InodeRowValue, FsInode);
row_envelope!(DentryRow, DentryRowValue, FsDentry);
row_envelope!(LinkRow, LinkRowValue, FsPresent, void);
row_envelope!(XattrRow, XattrRowValue, FsBytes);
row_envelope!(KvRow, KvRowValue, FsBytes);
row_envelope!(OrphanRow, OrphanRowValue, FsPresent, void);
row_envelope!(ChunkRow, ChunkRowValue, FsBytes);

// shared/depot.fs_read

// The row kinds: version 2 adds the range whiteout row, which a version 1 reader never meets
// because only a volume with a base holds one, and every reader of such a volume is at
// version 2.

impl From<read1::FsRowKey> for read2::FsRowKey {
	fn from(value: read1::FsRowKey) -> Self {
		match value {
			read1::FsRowKey::FsInodeKey(v) => Self::FsInodeKey(v.into()),
			read1::FsRowKey::FsDentryKey(v) => Self::FsDentryKey(v.into()),
			read1::FsRowKey::FsLinkKey(v) => Self::FsLinkKey(v.into()),
			read1::FsRowKey::FsXattrKey(v) => Self::FsXattrKey(v.into()),
			read1::FsRowKey::FsKvKey(v) => Self::FsKvKey(v.into()),
			read1::FsRowKey::FsOrphanKey(v) => Self::FsOrphanKey(v.into()),
			read1::FsRowKey::FsChunkKey(v) => Self::FsChunkKey(v.into()),
			read1::FsRowKey::FsFormatKey => Self::FsFormatKey,
		}
	}
}

impl Downgrade for read2::FsRowKey {
	type Older = read1::FsRowKey;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsInodeKey(v) => read1::FsRowKey::FsInodeKey(v.into()),
			Self::FsDentryKey(v) => read1::FsRowKey::FsDentryKey(v.into()),
			Self::FsLinkKey(v) => read1::FsRowKey::FsLinkKey(v.into()),
			Self::FsXattrKey(v) => read1::FsRowKey::FsXattrKey(v.into()),
			Self::FsKvKey(v) => read1::FsRowKey::FsKvKey(v.into()),
			Self::FsOrphanKey(v) => read1::FsRowKey::FsOrphanKey(v.into()),
			Self::FsChunkKey(v) => read1::FsRowKey::FsChunkKey(v.into()),
			Self::FsFormatKey => read1::FsRowKey::FsFormatKey,
			Self::FsKvRangeWhiteoutKey(_) => anyhow::bail!("version 1 has no range whiteout rows"),
		})
	}
}

impl From<read1::FsRowValue> for read2::FsRowValue {
	fn from(value: read1::FsRowValue) -> Self {
		match value {
			read1::FsRowValue::FsInode(v) => Self::FsInode(v.into()),
			read1::FsRowValue::FsDentry(v) => Self::FsDentry(v.into()),
			read1::FsRowValue::FsBytes(v) => Self::FsBytes(v.into()),
			read1::FsRowValue::FsPresent => Self::FsPresent,
			read1::FsRowValue::FsFormat(v) => Self::FsFormat(v.into()),
		}
	}
}

impl Downgrade for read2::FsRowValue {
	type Older = read1::FsRowValue;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsInode(v) => read1::FsRowValue::FsInode(v.into()),
			Self::FsDentry(v) => read1::FsRowValue::FsDentry(v.into()),
			Self::FsBytes(v) => read1::FsRowValue::FsBytes(v.into()),
			Self::FsPresent => read1::FsRowValue::FsPresent,
			Self::FsFormat(v) => read1::FsRowValue::FsFormat(v.downgrade()?),
			Self::FsKvRangeWhiteout(_) => anyhow::bail!("version 1 has no range whiteout rows"),
		})
	}
}

impl From<read1::FsRowRange> for read2::FsRowRange {
	fn from(value: read1::FsRowRange) -> Self {
		match value {
			read1::FsRowRange::FsAllRows => Self::FsAllRows,
			read1::FsRowRange::FsAllMetadataRows => Self::FsAllMetadataRows,
			read1::FsRowRange::FsAllChunkRows => Self::FsAllChunkRows,
			read1::FsRowRange::FsMetadataRows(v) => Self::FsMetadataRows(v.into()),
			read1::FsRowRange::FsChunkRows(v) => Self::FsChunkRows(v.into()),
			read1::FsRowRange::FsDirectoryRows(v) => Self::FsDirectoryRows(v.into()),
			read1::FsRowRange::FsXattrRows(v) => Self::FsXattrRows(v.into()),
			read1::FsRowRange::FsKvRows(v) => Self::FsKvRows(v.into()),
			read1::FsRowRange::FsOrphanRows => Self::FsOrphanRows,
		}
	}
}

impl Downgrade for read2::FsRowRange {
	type Older = read1::FsRowRange;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsAllRows => read1::FsRowRange::FsAllRows,
			Self::FsAllMetadataRows => read1::FsRowRange::FsAllMetadataRows,
			Self::FsAllChunkRows => read1::FsRowRange::FsAllChunkRows,
			Self::FsMetadataRows(v) => read1::FsRowRange::FsMetadataRows(v.into()),
			Self::FsChunkRows(v) => read1::FsRowRange::FsChunkRows(v.into()),
			Self::FsDirectoryRows(v) => read1::FsRowRange::FsDirectoryRows(v.into()),
			Self::FsXattrRows(v) => read1::FsRowRange::FsXattrRows(v.into()),
			Self::FsKvRows(v) => read1::FsRowRange::FsKvRows(v.into()),
			Self::FsOrphanRows => read1::FsRowRange::FsOrphanRows,
			Self::FsKvRangeWhiteoutRows(_) | Self::FsKvRangeWhiteoutsAllRows => {
				anyhow::bail!("version 1 has no range whiteout rows")
			}
		})
	}
}

impl From<read1::FsReadGet> for read2::FsReadGet {
	fn from(value: read1::FsReadGet) -> Self {
		Self {
			at: value.at.into(),
			key: value.key.into(),
		}
	}
}

impl Downgrade for read2::FsReadGet {
	type Older = read1::FsReadGet;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsReadGet {
			at: self.at.into(),
			key: self.key.downgrade()?,
		})
	}
}

impl From<read1::FsReadScan> for read2::FsReadScan {
	fn from(value: read1::FsReadScan) -> Self {
		Self {
			at: value.at.into(),
			range: value.range.into(),
			after: value.after.map(Into::into),
			limit: value.limit,
		}
	}
}

impl Downgrade for read2::FsReadScan {
	type Older = read1::FsReadScan;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsReadScan {
			at: self.at.into(),
			range: self.range.downgrade()?,
			after: downgrade_opt(self.after)?,
			limit: self.limit,
		})
	}
}

impl From<read1::FsReadRow> for read2::FsReadRow {
	fn from(value: read1::FsReadRow) -> Self {
		Self {
			key: value.key.into(),
			value: value.value.into(),
			change_seq: value.change_seq,
			claim_log: ROOT_CLAIM_LOG,
		}
	}
}

impl Downgrade for read2::FsReadRow {
	type Older = read1::FsReadRow;

	fn downgrade(self) -> Result<Self::Older> {
		root_claim_log(self.claim_log)?;
		Ok(read1::FsReadRow {
			key: self.key.downgrade()?,
			value: self.value.downgrade()?,
			change_seq: self.change_seq,
		})
	}
}

impl From<read1::FsBaseChunksRequest> for read2::FsBaseChunksRequest {
	fn from(value: read1::FsBaseChunksRequest) -> Self {
		Self {
			link: value.link.into(),
			ino: value.ino,
			indices: value.indices,
		}
	}
}

impl Downgrade for read2::FsBaseChunksRequest {
	type Older = read1::FsBaseChunksRequest;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsBaseChunksRequest {
			link: self.link.downgrade()?,
			ino: self.ino,
			indices: self.indices,
		})
	}
}

impl From<read1::FsForkMetadataRequest> for read2::FsForkMetadataRequest {
	fn from(value: read1::FsForkMetadataRequest) -> Self {
		Self {
			link: value.link.into(),
			after: value.after,
			limit: value.limit,
		}
	}
}

impl Downgrade for read2::FsForkMetadataRequest {
	type Older = read1::FsForkMetadataRequest;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsForkMetadataRequest {
			link: self.link.downgrade()?,
			after: self.after,
			limit: self.limit,
		})
	}
}

impl From<read1::FsForkMetadataPage> for read2::FsForkMetadataPage {
	fn from(value: read1::FsForkMetadataPage) -> Self {
		Self {
			rows: value.rows.into_iter().map(Into::into).collect(),
			complete: value.complete,
		}
	}
}

impl Downgrade for read2::FsForkMetadataPage {
	type Older = read1::FsForkMetadataPage;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsForkMetadataPage {
			rows: downgrade_all(self.rows)?,
			complete: self.complete,
		})
	}
}

impl From<read1::FsBaseKeysRequest> for read2::FsBaseKeysRequest {
	fn from(value: read1::FsBaseKeysRequest) -> Self {
		Self {
			link: value.link.into(),
			after: value.after.map(Into::into),
			limit: value.limit,
		}
	}
}

impl Downgrade for read2::FsBaseKeysRequest {
	type Older = read1::FsBaseKeysRequest;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsBaseKeysRequest {
			link: self.link.downgrade()?,
			after: self.after.map(Into::into),
			limit: self.limit,
		})
	}
}

impl From<read1::FsReadOpen> for read2::FsReadOpen {
	fn from(value: read1::FsReadOpen) -> Self {
		Self {
			cut: value.position.map(fs2::FsCut::single),
			prefetch: value.prefetch.map(Into::into),
		}
	}
}

impl Downgrade for read2::FsReadOpen {
	type Older = read1::FsReadOpen;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsReadOpen {
			position: downgrade_opt(self.cut)?,
			prefetch: self.prefetch.map(Into::into),
		})
	}
}

impl From<read1::FsReadKeys> for read2::FsReadKeys {
	fn from(value: read1::FsReadKeys) -> Self {
		Self {
			cut: value.position.map(fs2::FsCut::single),
			file: value.file,
			keys: value.keys,
		}
	}
}

impl Downgrade for read2::FsReadKeys {
	type Older = read1::FsReadKeys;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsReadKeys {
			position: downgrade_opt(self.cut)?,
			file: self.file,
			keys: self.keys,
		})
	}
}

impl From<read1::FsReadRaw> for read2::FsReadRaw {
	fn from(value: read1::FsReadRaw) -> Self {
		Self {
			cut: value.position.map(fs2::FsCut::single),
			key: value.key.into(),
			versions: value.versions,
		}
	}
}

impl Downgrade for read2::FsReadRaw {
	type Older = read1::FsReadRaw;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsReadRaw {
			position: downgrade_opt(self.cut)?,
			key: self.key.downgrade()?,
			versions: self.versions,
		})
	}
}

impl From<read1::FsReadOperation> for read2::FsReadOperation {
	fn from(value: read1::FsReadOperation) -> Self {
		match value {
			read1::FsReadOperation::FsReadOpen(v) => Self::FsReadOpen(v.into()),
			read1::FsReadOperation::FsReadGet(v) => Self::FsReadGet(v.into()),
			read1::FsReadOperation::FsReadScan(v) => Self::FsReadScan(v.into()),
			read1::FsReadOperation::FsReadChunks(v) => Self::FsReadChunks(v.into()),
			read1::FsReadOperation::FsReadClose(v) => Self::FsReadClose(v.into()),
			read1::FsReadOperation::FsBaseChunksRequest(v) => Self::FsBaseChunksRequest(v.into()),
			read1::FsReadOperation::FsForkMetadataRequest(v) => {
				Self::FsForkMetadataRequest(v.into())
			}
			read1::FsReadOperation::FsBaseKeysRequest(v) => Self::FsBaseKeysRequest(v.into()),
			read1::FsReadOperation::FsReadKeys(v) => Self::FsReadKeys(v.into()),
			read1::FsReadOperation::FsReadRaw(v) => Self::FsReadRaw(v.into()),
		}
	}
}

impl Downgrade for read2::FsReadOperation {
	type Older = read1::FsReadOperation;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsReadOpen(v) => read1::FsReadOperation::FsReadOpen(v.downgrade()?),
			Self::FsReadGet(v) => read1::FsReadOperation::FsReadGet(v.downgrade()?),
			Self::FsReadScan(v) => read1::FsReadOperation::FsReadScan(v.downgrade()?),
			Self::FsReadChunks(v) => read1::FsReadOperation::FsReadChunks(v.into()),
			Self::FsReadClose(v) => read1::FsReadOperation::FsReadClose(v.into()),
			Self::FsBaseChunksRequest(v) => {
				read1::FsReadOperation::FsBaseChunksRequest(v.downgrade()?)
			}
			Self::FsForkMetadataRequest(v) => {
				read1::FsReadOperation::FsForkMetadataRequest(v.downgrade()?)
			}
			Self::FsBaseKeysRequest(v) => read1::FsReadOperation::FsBaseKeysRequest(v.downgrade()?),
			Self::FsReadKeys(v) => read1::FsReadOperation::FsReadKeys(v.downgrade()?),
			Self::FsReadRaw(v) => read1::FsReadOperation::FsReadRaw(v.downgrade()?),
			Self::FsBaseRowsRequest(_) => anyhow::bail!("version 1 has no base rows read"),
		})
	}
}

impl From<read1::FsReadRequest> for read2::FsReadRequest {
	fn from(value: read1::FsReadRequest) -> Self {
		Self {
			target: value.target.into(),
			client_epoch: value.client_epoch,
			operation: value.operation.into(),
		}
	}
}

impl Downgrade for read2::FsReadRequest {
	type Older = read1::FsReadRequest;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsReadRequest {
			target: self.target.into(),
			client_epoch: self.client_epoch,
			operation: self.operation.downgrade()?,
		})
	}
}

impl From<read1::FsReadOpened> for read2::FsReadOpened {
	fn from(value: read1::FsReadOpened) -> Self {
		Self {
			volume_log_key: value.volume_log_key,
			session: value.session.map(Into::into),
			applied: fs2::FsCut::single(value.applied),
			as_of: value.as_of,
			rows: value
				.rows
				.map(|rows| rows.into_iter().map(Into::into).collect()),
		}
	}
}

impl Downgrade for read2::FsReadOpened {
	type Older = read1::FsReadOpened;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsReadOpened {
			volume_log_key: self.volume_log_key,
			session: self.session.map(Into::into),
			applied: self.applied.downgrade()?,
			as_of: self.as_of,
			rows: self.rows.map(downgrade_all).transpose()?,
		})
	}
}

impl From<read1::FsReadRows> for read2::FsReadRows {
	fn from(value: read1::FsReadRows) -> Self {
		Self {
			session: value.session,
			applied: fs2::FsCut::single(value.applied),
			as_of: value.as_of,
			rows: value.rows.into_iter().map(Into::into).collect(),
		}
	}
}

impl Downgrade for read2::FsReadRows {
	type Older = read1::FsReadRows;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsReadRows {
			session: self.session,
			applied: self.applied.downgrade()?,
			as_of: self.as_of,
			rows: downgrade_all(self.rows)?,
		})
	}
}

impl From<read1::FsReadValues> for read2::FsReadValues {
	/// A version 1 maintainer does not report the commit position it knew, so the answer reads
	/// as lagging an unknown commit: a reader that caches only answers at the commit position,
	/// such as the group record cache, serves it without caching it.
	fn from(value: read1::FsReadValues) -> Self {
		Self {
			volume_log_key: value.volume_log_key,
			applied: fs2::FsCut::single(value.applied),
			as_of: value.as_of,
			committed: u64::MAX,
			values: value.values,
		}
	}
}

impl Downgrade for read2::FsReadValues {
	type Older = read1::FsReadValues;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsReadValues {
			volume_log_key: self.volume_log_key,
			applied: self.applied.downgrade()?,
			as_of: self.as_of,
			values: self.values,
		})
	}
}

impl From<read1::FsRawVersion> for read2::FsRawVersion {
	fn from(value: read1::FsRawVersion) -> Self {
		Self {
			seq: value.seq,
			claim_log: ROOT_CLAIM_LOG,
			raw: value.raw,
			deleted: value.deleted,
			decoded: value.decoded.map(Into::into),
			decode_error: value.decode_error,
		}
	}
}

impl Downgrade for read2::FsRawVersion {
	type Older = read1::FsRawVersion;

	fn downgrade(self) -> Result<Self::Older> {
		root_claim_log(self.claim_log)?;
		Ok(read1::FsRawVersion {
			seq: self.seq,
			raw: self.raw,
			deleted: self.deleted,
			decoded: downgrade_opt(self.decoded)?,
			decode_error: self.decode_error,
		})
	}
}

impl From<read1::FsReadRawVersions> for read2::FsReadRawVersions {
	fn from(value: read1::FsReadRawVersions) -> Self {
		Self {
			volume_log_key: value.volume_log_key,
			applied: fs2::FsCut::single(value.applied),
			as_of: value.as_of,
			versions: value.versions.into_iter().map(Into::into).collect(),
		}
	}
}

impl Downgrade for read2::FsReadRawVersions {
	type Older = read1::FsReadRawVersions;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(read1::FsReadRawVersions {
			volume_log_key: self.volume_log_key,
			applied: self.applied.downgrade()?,
			as_of: self.as_of,
			versions: downgrade_all(self.versions)?,
		})
	}
}

impl From<read1::FsReadResponse> for read2::FsReadResponse {
	fn from(value: read1::FsReadResponse) -> Self {
		match value {
			read1::FsReadResponse::FsReadOpened(v) => Self::FsReadOpened(v.into()),
			read1::FsReadResponse::FsReadRows(v) => Self::FsReadRows(v.into()),
			read1::FsReadResponse::FsReadClosed => Self::FsReadClosed,
			read1::FsReadResponse::FsReadError(v) => Self::FsReadError(v.into()),
			read1::FsReadResponse::FsBaseChunksResponse(v) => Self::FsBaseChunksResponse(v.into()),
			read1::FsReadResponse::FsForkMetadataPage(v) => Self::FsForkMetadataPage(v.into()),
			read1::FsReadResponse::FsBaseKeysPage(v) => Self::FsBaseKeysPage(v.into()),
			read1::FsReadResponse::FsReadValues(v) => Self::FsReadValues(v.into()),
			read1::FsReadResponse::FsReadRawVersions(v) => Self::FsReadRawVersions(v.into()),
		}
	}
}

impl Downgrade for read2::FsReadResponse {
	type Older = read1::FsReadResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsReadOpened(v) => read1::FsReadResponse::FsReadOpened(v.downgrade()?),
			Self::FsReadRows(v) => read1::FsReadResponse::FsReadRows(v.downgrade()?),
			Self::FsReadClosed => read1::FsReadResponse::FsReadClosed,
			Self::FsReadError(v) => read1::FsReadResponse::FsReadError(v.into()),
			Self::FsBaseChunksResponse(v) => read1::FsReadResponse::FsBaseChunksResponse(v.into()),
			Self::FsForkMetadataPage(v) => {
				read1::FsReadResponse::FsForkMetadataPage(v.downgrade()?)
			}
			Self::FsBaseKeysPage(v) => read1::FsReadResponse::FsBaseKeysPage(v.into()),
			Self::FsReadValues(v) => read1::FsReadResponse::FsReadValues(v.downgrade()?),
			Self::FsReadRawVersions(v) => read1::FsReadResponse::FsReadRawVersions(v.downgrade()?),
			Self::FsBaseRowsPage(_) => anyhow::bail!("version 1 has no base rows read"),
		})
	}
}
