//! Converters between versions 1 and 2 of `depot.client_protocol` and `depot.maintainer_log`.
//!
//! Version 2 carries the version 2 filesystem types, turns a checkpoint position and a session
//! pin into cuts, and lets a maintainer log op target several claim logs. A version 1 value is
//! claim log 0 throughout, so an upgrade wraps each position in a one-log cut and lists a
//! single target; a downgrade refuses anything that names another claim log or more than one
//! target.
//!
//! Version 2 also lets an open ask for a group fork, lets a fork acquire answer with the
//! parent's metadata, refuses an open in an archived group, and adds the fork pin op. A version 1
//! value has none of these: an upgrade leaves them absent and a downgrade refuses a value that
//! sets one, which a cluster with a version 1 reader never writes. Version 2 also refuses a
//! checkpoint for now as `TailUnproven`, which a downgrade reads as `Lagging`. The types that did
//! not change convert through the generated `From` impls.

use super::depot_fs_v2::{Downgrade, downgrade_all, downgrade_into, downgrade_opt};
use crate::generated::{
	depot_client_protocol_v1 as client1, depot_client_protocol_v2 as client2,
	depot_maintainer_log_v1 as log1, depot_maintainer_log_v2 as log2, shared_depot_fs_v2::FsCut,
};
use anyhow::{Result, ensure};

// depot.client_protocol

impl From<client1::VolumeCreateResponse> for client2::VolumeCreateResponse {
	fn from(value: client1::VolumeCreateResponse) -> Self {
		Self {
			record: value.record.into(),
			open_claim: value.open_claim,
			states: value.states.into_iter().map(Into::into).collect(),
		}
	}
}

impl Downgrade for client2::VolumeCreateResponse {
	type Older = client1::VolumeCreateResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::VolumeCreateResponse {
			record: self.record.downgrade()?,
			open_claim: self.open_claim,
			states: self.states.into_iter().map(Into::into).collect(),
		})
	}
}

impl From<client1::VolumeOpenRequest> for client2::VolumeOpenRequest {
	fn from(value: client1::VolumeOpenRequest) -> Self {
		Self {
			volume_key: value.volume_key,
			client_incarnation: value.client_incarnation,
			holder: value.holder.into(),
			create_if_absent: value.create_if_absent.map(Into::into),
			lazy_fork: false,
		}
	}
}

impl Downgrade for client2::VolumeOpenRequest {
	type Older = client1::VolumeOpenRequest;

	fn downgrade(self) -> Result<Self::Older> {
		ensure!(!self.lazy_fork, "version 1 has no group forks");
		Ok(client1::VolumeOpenRequest {
			volume_key: self.volume_key,
			client_incarnation: self.client_incarnation,
			holder: self.holder.into(),
			create_if_absent: self.create_if_absent.map(Into::into),
		})
	}
}

impl From<client1::VolumeOpenResponse> for client2::VolumeOpenResponse {
	fn from(value: client1::VolumeOpenResponse) -> Self {
		Self {
			record: value.record.into(),
			open_claim: value.open_claim,
			states: value.states.into_iter().map(Into::into).collect(),
			created: value.created,
			read: value.read,
			applied: value.applied.map(Into::into),
		}
	}
}

impl Downgrade for client2::VolumeOpenResponse {
	type Older = client1::VolumeOpenResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::VolumeOpenResponse {
			record: self.record.downgrade()?,
			open_claim: self.open_claim,
			states: self.states.into_iter().map(Into::into).collect(),
			created: self.created,
			read: self.read,
			applied: self.applied.map(Into::into),
		})
	}
}

impl From<client1::CheckpointPosition> for client2::CheckpointPosition {
	fn from(value: client1::CheckpointPosition) -> Self {
		Self {
			volume_incarnation: value.volume_incarnation,
			cut: FsCut::single(value.position),
		}
	}
}

impl Downgrade for client2::CheckpointPosition {
	type Older = client1::CheckpointPosition;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::CheckpointPosition {
			volume_incarnation: self.volume_incarnation,
			position: self.cut.downgrade()?,
		})
	}
}

impl From<client1::CheckpointCreate> for client2::CheckpointCreate {
	fn from(value: client1::CheckpointCreate) -> Self {
		Self {
			name: value.name,
			minimum: value.minimum.map(Into::into),
		}
	}
}

impl Downgrade for client2::CheckpointCreate {
	type Older = client1::CheckpointCreate;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::CheckpointCreate {
			name: self.name,
			minimum: downgrade_opt(self.minimum)?,
		})
	}
}

impl From<client1::CheckpointDelete> for client2::CheckpointDelete {
	fn from(value: client1::CheckpointDelete) -> Self {
		Self {
			volume_incarnation: value.volume_incarnation,
			name: value.name,
			minimum: value.minimum.map(Into::into),
		}
	}
}

impl Downgrade for client2::CheckpointDelete {
	type Older = client1::CheckpointDelete;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::CheckpointDelete {
			volume_incarnation: self.volume_incarnation,
			name: self.name,
			minimum: downgrade_opt(self.minimum)?,
		})
	}
}

impl From<client1::CheckpointSetRetention> for client2::CheckpointSetRetention {
	fn from(value: client1::CheckpointSetRetention) -> Self {
		Self {
			retention: value.retention.map(Into::into),
			minimum: value.minimum.map(Into::into),
		}
	}
}

impl Downgrade for client2::CheckpointSetRetention {
	type Older = client1::CheckpointSetRetention;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::CheckpointSetRetention {
			retention: self.retention.map(Into::into),
			minimum: downgrade_opt(self.minimum)?,
		})
	}
}

impl From<client1::CheckpointOperation> for client2::CheckpointOperation {
	fn from(value: client1::CheckpointOperation) -> Self {
		match value {
			client1::CheckpointOperation::CheckpointCreate(v) => Self::CheckpointCreate(v.into()),
			client1::CheckpointOperation::CheckpointInspect(v) => Self::CheckpointInspect(v.into()),
			client1::CheckpointOperation::CheckpointList(v) => Self::CheckpointList(v.into()),
			client1::CheckpointOperation::CheckpointDelete(v) => Self::CheckpointDelete(v.into()),
			client1::CheckpointOperation::CheckpointSetRetention(v) => {
				Self::CheckpointSetRetention(v.into())
			}
		}
	}
}

impl Downgrade for client2::CheckpointOperation {
	type Older = client1::CheckpointOperation;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::CheckpointCreate(v) => {
				client1::CheckpointOperation::CheckpointCreate(v.downgrade()?)
			}
			Self::CheckpointInspect(v) => client1::CheckpointOperation::CheckpointInspect(v.into()),
			Self::CheckpointList(v) => client1::CheckpointOperation::CheckpointList(v.into()),
			Self::CheckpointDelete(v) => {
				client1::CheckpointOperation::CheckpointDelete(v.downgrade()?)
			}
			Self::CheckpointSetRetention(v) => {
				client1::CheckpointOperation::CheckpointSetRetention(v.downgrade()?)
			}
		})
	}
}

impl From<client1::VolumeCheckpointRequest> for client2::VolumeCheckpointRequest {
	fn from(value: client1::VolumeCheckpointRequest) -> Self {
		Self {
			volume_key: value.volume_key,
			operation: value.operation.into(),
		}
	}
}

impl Downgrade for client2::VolumeCheckpointRequest {
	type Older = client1::VolumeCheckpointRequest;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::VolumeCheckpointRequest {
			volume_key: self.volume_key,
			operation: self.operation.downgrade()?,
		})
	}
}

impl From<client1::CheckpointInspection> for client2::CheckpointInspection {
	fn from(value: client1::CheckpointInspection) -> Self {
		Self {
			volume_incarnation: value.volume_incarnation,
			checkpoint: value.checkpoint.map(Into::into),
			named_count: value.named_count,
			max_named: value.max_named,
		}
	}
}

impl Downgrade for client2::CheckpointInspection {
	type Older = client1::CheckpointInspection;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::CheckpointInspection {
			volume_incarnation: self.volume_incarnation,
			checkpoint: downgrade_opt(self.checkpoint)?,
			named_count: self.named_count,
			max_named: self.max_named,
		})
	}
}

impl From<client1::CheckpointPage> for client2::CheckpointPage {
	fn from(value: client1::CheckpointPage) -> Self {
		Self {
			checkpoints: value.checkpoints.into_iter().map(Into::into).collect(),
			after: value.after,
		}
	}
}

impl Downgrade for client2::CheckpointPage {
	type Older = client1::CheckpointPage;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::CheckpointPage {
			checkpoints: downgrade_all(self.checkpoints)?,
			after: self.after,
		})
	}
}

impl From<client1::CheckpointError> for client2::CheckpointError {
	fn from(value: client1::CheckpointError) -> Self {
		match value {
			client1::CheckpointError::NotFound => Self::NotFound,
			client1::CheckpointError::NameInUse => Self::NameInUse,
			client1::CheckpointError::TooManyNamed => Self::TooManyNamed,
			client1::CheckpointError::Invalid => Self::Invalid,
			client1::CheckpointError::VolumeNotFound => Self::VolumeNotFound,
			client1::CheckpointError::IncarnationChanged => Self::IncarnationChanged,
			client1::CheckpointError::Lagging => Self::Lagging,
		}
	}
}

/// A version 1 reader has no `TailUnproven` and reads it as `Lagging`: both refuse a checkpoint
/// that changed nothing, and the reader sends it again. The `Lagging` answer's retry hint does
/// not help a tail that stays unproven, so such a reader may resend early for nothing; that is
/// bounded and accepted.
impl From<client2::CheckpointError> for client1::CheckpointError {
	fn from(value: client2::CheckpointError) -> Self {
		match value {
			client2::CheckpointError::NotFound => Self::NotFound,
			client2::CheckpointError::NameInUse => Self::NameInUse,
			client2::CheckpointError::TooManyNamed => Self::TooManyNamed,
			client2::CheckpointError::Invalid => Self::Invalid,
			client2::CheckpointError::VolumeNotFound => Self::VolumeNotFound,
			client2::CheckpointError::IncarnationChanged => Self::IncarnationChanged,
			client2::CheckpointError::Lagging | client2::CheckpointError::TailUnproven => {
				Self::Lagging
			}
		}
	}
}

impl From<client1::VolumeCheckpointResponse> for client2::VolumeCheckpointResponse {
	fn from(value: client1::VolumeCheckpointResponse) -> Self {
		match value {
			client1::VolumeCheckpointResponse::FsCheckpointInfo(v) => {
				Self::FsCheckpointInfo(v.into())
			}
			client1::VolumeCheckpointResponse::CheckpointInspection(v) => {
				Self::CheckpointInspection(v.into())
			}
			client1::VolumeCheckpointResponse::CheckpointPage(v) => Self::CheckpointPage(v.into()),
			client1::VolumeCheckpointResponse::CheckpointUpdated => Self::CheckpointUpdated,
			client1::VolumeCheckpointResponse::CheckpointError(v) => {
				Self::CheckpointError(v.into())
			}
		}
	}
}

impl Downgrade for client2::VolumeCheckpointResponse {
	type Older = client1::VolumeCheckpointResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsCheckpointInfo(v) => {
				client1::VolumeCheckpointResponse::FsCheckpointInfo(v.downgrade()?)
			}
			Self::CheckpointInspection(v) => {
				client1::VolumeCheckpointResponse::CheckpointInspection(v.downgrade()?)
			}
			Self::CheckpointPage(v) => {
				client1::VolumeCheckpointResponse::CheckpointPage(v.downgrade()?)
			}
			Self::CheckpointUpdated => client1::VolumeCheckpointResponse::CheckpointUpdated,
			Self::CheckpointError(v) => {
				client1::VolumeCheckpointResponse::CheckpointError(v.into())
			}
		})
	}
}

impl From<client1::FsForkRequest> for client2::FsForkRequest {
	fn from(value: client1::FsForkRequest) -> Self {
		Self {
			spec: value.spec.into(),
		}
	}
}

impl Downgrade for client2::FsForkRequest {
	type Older = client1::FsForkRequest;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::FsForkRequest {
			spec: self.spec.downgrade()?,
		})
	}
}

impl From<client1::FsForkResponse> for client2::FsForkResponse {
	fn from(value: client1::FsForkResponse) -> Self {
		match value {
			client1::FsForkResponse::FsVolumeRecord(v) => Self::FsVolumeRecord(v.into()),
			client1::FsForkResponse::FsForkError(v) => Self::FsForkError(v.into()),
		}
	}
}

impl Downgrade for client2::FsForkResponse {
	type Older = client1::FsForkResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsVolumeRecord(v) => client1::FsForkResponse::FsVolumeRecord(v.downgrade()?),
			Self::FsForkError(v) => client1::FsForkResponse::FsForkError(v.into()),
		})
	}
}

impl From<client1::FsForkAcquire> for client2::FsForkAcquire {
	fn from(value: client1::FsForkAcquire) -> Self {
		Self {
			spec: value.spec.into(),
		}
	}
}

impl Downgrade for client2::FsForkAcquire {
	type Older = client1::FsForkAcquire;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::FsForkAcquire {
			spec: self.spec.downgrade()?,
		})
	}
}

impl From<client1::FsForkRelease> for client2::FsForkRelease {
	fn from(value: client1::FsForkRelease) -> Self {
		Self {
			spec: value.spec.into(),
		}
	}
}

impl Downgrade for client2::FsForkRelease {
	type Older = client1::FsForkRelease;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::FsForkRelease {
			spec: self.spec.downgrade()?,
		})
	}
}

impl From<client1::FsForkCheck> for client2::FsForkCheck {
	fn from(value: client1::FsForkCheck) -> Self {
		Self {
			spec: value.spec.into(),
		}
	}
}

impl Downgrade for client2::FsForkCheck {
	type Older = client1::FsForkCheck;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::FsForkCheck {
			spec: self.spec.downgrade()?,
		})
	}
}

impl From<client1::FsForkReferenceRequest> for client2::FsForkReferenceRequest {
	fn from(value: client1::FsForkReferenceRequest) -> Self {
		match value {
			client1::FsForkReferenceRequest::FsForkAcquire(v) => Self::FsForkAcquire(v.into()),
			client1::FsForkReferenceRequest::FsForkRelease(v) => Self::FsForkRelease(v.into()),
			client1::FsForkReferenceRequest::FsForkCheck(v) => Self::FsForkCheck(v.into()),
		}
	}
}

impl Downgrade for client2::FsForkReferenceRequest {
	type Older = client1::FsForkReferenceRequest;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsForkAcquire(v) => {
				client1::FsForkReferenceRequest::FsForkAcquire(v.downgrade()?)
			}
			Self::FsForkRelease(v) => {
				client1::FsForkReferenceRequest::FsForkRelease(v.downgrade()?)
			}
			Self::FsForkCheck(v) => client1::FsForkReferenceRequest::FsForkCheck(v.downgrade()?),
		})
	}
}

impl From<client1::FsForkReferenceResponse> for client2::FsForkReferenceResponse {
	fn from(value: client1::FsForkReferenceResponse) -> Self {
		match value {
			client1::FsForkReferenceResponse::FsForkReceipt(v) => Self::FsForkReceipt(v.into()),
			client1::FsForkReferenceResponse::FsForkPresence(v) => Self::FsForkPresence(v.into()),
			client1::FsForkReferenceResponse::FsForkReleased => Self::FsForkReleased,
			client1::FsForkReferenceResponse::FsForkError(v) => Self::FsForkError(v.into()),
		}
	}
}

impl Downgrade for client2::FsForkReferenceResponse {
	type Older = client1::FsForkReferenceResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsForkReceipt(v) => {
				client1::FsForkReferenceResponse::FsForkReceipt(v.downgrade()?)
			}
			Self::FsForkPresence(v) => client1::FsForkReferenceResponse::FsForkPresence(v.into()),
			Self::FsForkReleased => client1::FsForkReferenceResponse::FsForkReleased,
			Self::FsForkError(v) => client1::FsForkReferenceResponse::FsForkError(v.into()),
			Self::FsForkGroupReceipt(_) | Self::FsForkGroupAbsent | Self::FsForkGroupContinue => {
				anyhow::bail!("version 1 has no group fork answers")
			}
		})
	}
}

impl From<client1::RepairIncarnationView> for client2::RepairIncarnationView {
	fn from(value: client1::RepairIncarnationView) -> Self {
		Self {
			volume_log_key: value.volume_log_key,
			stored: value.stored.map(Into::into),
			known: value.known.map(Into::into),
			evidence: value.evidence.map(Into::into),
			registered: value.registered,
			applied: value.applied,
			members: value.members.into_iter().map(Into::into).collect(),
			record_missing: value.record_missing,
			deleting: value.deleting,
			unloading: value.unloading,
			awaiting_replay: value.awaiting_replay,
			holder: value.holder,
			idle_ms: value.idle_ms,
		}
	}
}

impl Downgrade for client2::RepairIncarnationView {
	type Older = client1::RepairIncarnationView;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairIncarnationView {
			volume_log_key: self.volume_log_key,
			stored: downgrade_opt(self.stored)?,
			known: downgrade_opt(self.known)?,
			evidence: downgrade_opt(self.evidence)?,
			registered: self.registered,
			applied: self.applied,
			members: self.members.into_iter().map(Into::into).collect(),
			record_missing: self.record_missing,
			deleting: self.deleting,
			unloading: self.unloading,
			awaiting_replay: self.awaiting_replay,
			holder: self.holder,
			idle_ms: self.idle_ms,
		})
	}
}

impl From<client1::RepairInspection> for client2::RepairInspection {
	fn from(value: client1::RepairInspection) -> Self {
		Self {
			tablet_id: value.tablet_id,
			tablet_grant_epoch: value.tablet_grant_epoch,
			replica_set: value.replica_set,
			pointer: value.pointer.map(Into::into),
			pending_fork: value.pending_fork.map(Into::into),
			pending_open: value.pending_open,
			incarnations: value.incarnations.into_iter().map(Into::into).collect(),
		}
	}
}

impl Downgrade for client2::RepairInspection {
	type Older = client1::RepairInspection;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairInspection {
			tablet_id: self.tablet_id,
			tablet_grant_epoch: self.tablet_grant_epoch,
			replica_set: self.replica_set,
			pointer: downgrade_opt(self.pointer)?,
			pending_fork: downgrade_opt(self.pending_fork)?,
			pending_open: self.pending_open,
			incarnations: downgrade_all(self.incarnations)?,
		})
	}
}

impl From<client1::RepairAppend> for client2::RepairAppend {
	fn from(value: client1::RepairAppend) -> Self {
		Self {
			claim: value.claim,
			expected_appended: value.expected_appended,
			entry: value.entry.into(),
			preimages: value.preimages.into_iter().map(Into::into).collect(),
		}
	}
}

impl From<client1::RepairPreimage> for client2::RepairPreimage {
	fn from(value: client1::RepairPreimage) -> Self {
		Self {
			key: value.key.into(),
			value_digest: value.value_digest,
		}
	}
}

impl Downgrade for client2::RepairPreimage {
	type Older = client1::RepairPreimage;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairPreimage {
			key: self.key.downgrade()?,
			value_digest: self.value_digest,
		})
	}
}

impl From<client1::RepairRowChange> for client2::RepairRowChange {
	fn from(value: client1::RepairRowChange) -> Self {
		Self {
			key: value.key.into(),
			before: value.before,
		}
	}
}

impl Downgrade for client2::RepairRowChange {
	type Older = client1::RepairRowChange;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairRowChange {
			key: self.key.downgrade()?,
			before: self.before,
		})
	}
}

impl From<client1::RepairAppendPlan> for client2::RepairAppendPlan {
	fn from(value: client1::RepairAppendPlan) -> Self {
		Self {
			lands: value.lands,
			rows: value.rows.into_iter().map(Into::into).collect(),
		}
	}
}

impl Downgrade for client2::RepairAppendPlan {
	type Older = client1::RepairAppendPlan;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairAppendPlan {
			lands: self.lands,
			rows: downgrade_all(self.rows)?,
		})
	}
}

impl Downgrade for client2::RepairAppend {
	type Older = client1::RepairAppend;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairAppend {
			claim: self.claim,
			expected_appended: self.expected_appended,
			entry: self.entry.downgrade()?,
			preimages: downgrade_all(self.preimages)?,
		})
	}
}

impl From<client1::RepairBody> for client2::RepairBody {
	fn from(value: client1::RepairBody) -> Self {
		match value {
			client1::RepairBody::RepairRebuildRecord(v) => Self::RepairRebuildRecord(v.into()),
			client1::RepairBody::RepairClaimAcquire(v) => Self::RepairClaimAcquire(v.into()),
			client1::RepairBody::RepairClaimRelease(v) => Self::RepairClaimRelease(v.into()),
			client1::RepairBody::RepairAppend(v) => Self::RepairAppend(v.into()),
			client1::RepairBody::RepairReplicaReload(v) => Self::RepairReplicaReload(v.into()),
			client1::RepairBody::RepairAbortUnload => Self::RepairAbortUnload,
			client1::RepairBody::RepairRedrive => Self::RepairRedrive,
			client1::RepairBody::RepairReplicaForget(v) => Self::RepairReplicaForget(v.into()),
		}
	}
}

impl Downgrade for client2::RepairBody {
	type Older = client1::RepairBody;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::RepairRebuildRecord(v) => client1::RepairBody::RepairRebuildRecord(v.into()),
			Self::RepairClaimAcquire(v) => client1::RepairBody::RepairClaimAcquire(v.into()),
			Self::RepairClaimRelease(v) => client1::RepairBody::RepairClaimRelease(v.into()),
			Self::RepairAppend(v) => client1::RepairBody::RepairAppend(v.downgrade()?),
			Self::RepairReplicaReload(v) => client1::RepairBody::RepairReplicaReload(v.into()),
			Self::RepairAbortUnload => client1::RepairBody::RepairAbortUnload,
			Self::RepairRedrive => client1::RepairBody::RepairRedrive,
			Self::RepairReplicaForget(v) => client1::RepairBody::RepairReplicaForget(v.into()),
		})
	}
}

impl From<client1::RepairRequest> for client2::RepairRequest {
	fn from(value: client1::RepairRequest) -> Self {
		Self {
			envelope: value.envelope.into(),
			body: value.body.into(),
		}
	}
}

impl Downgrade for client2::RepairRequest {
	type Older = client1::RepairRequest;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairRequest {
			envelope: self.envelope.into(),
			body: self.body.downgrade()?,
		})
	}
}

impl From<client1::RepairRebuildPlan> for client2::RepairRebuildPlan {
	fn from(value: client1::RepairRebuildPlan) -> Self {
		Self {
			record: value.record.into(),
			format_row_present: value.format_row_present,
			root_inode_present: value.root_inode_present,
		}
	}
}

impl Downgrade for client2::RepairRebuildPlan {
	type Older = client1::RepairRebuildPlan;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairRebuildPlan {
			record: self.record.downgrade()?,
			format_row_present: self.format_row_present,
			root_inode_present: self.root_inode_present,
		})
	}
}

impl From<client1::RepairPlan> for client2::RepairPlan {
	fn from(value: client1::RepairPlan) -> Self {
		match value {
			client1::RepairPlan::RepairRebuildPlan(v) => Self::RepairRebuildPlan(v.into()),
			client1::RepairPlan::RepairClaimPlan(v) => Self::RepairClaimPlan(v.into()),
			client1::RepairPlan::RepairReleasePlan(v) => Self::RepairReleasePlan(v.into()),
			client1::RepairPlan::RepairAppendPlan(v) => Self::RepairAppendPlan(v.into()),
			client1::RepairPlan::RepairReloadPlan(v) => Self::RepairReloadPlan(v.into()),
			client1::RepairPlan::RepairAbortUnloadPlan(v) => Self::RepairAbortUnloadPlan(v.into()),
			client1::RepairPlan::RepairRedrivePlan(v) => Self::RepairRedrivePlan(v.into()),
			client1::RepairPlan::RepairForgetPlan(v) => Self::RepairForgetPlan(v.into()),
		}
	}
}

impl Downgrade for client2::RepairPlan {
	type Older = client1::RepairPlan;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::RepairRebuildPlan(v) => client1::RepairPlan::RepairRebuildPlan(v.downgrade()?),
			Self::RepairClaimPlan(v) => client1::RepairPlan::RepairClaimPlan(v.into()),
			Self::RepairReleasePlan(v) => client1::RepairPlan::RepairReleasePlan(v.into()),
			Self::RepairAppendPlan(v) => client1::RepairPlan::RepairAppendPlan(v.downgrade()?),
			Self::RepairReloadPlan(v) => client1::RepairPlan::RepairReloadPlan(v.into()),
			Self::RepairAbortUnloadPlan(v) => client1::RepairPlan::RepairAbortUnloadPlan(v.into()),
			Self::RepairRedrivePlan(v) => client1::RepairPlan::RepairRedrivePlan(v.into()),
			Self::RepairForgetPlan(v) => client1::RepairPlan::RepairForgetPlan(v.into()),
		})
	}
}

impl From<client1::RepairRefused> for client2::RepairRefused {
	fn from(value: client1::RepairRefused) -> Self {
		Self {
			code: value.code.into(),
			retry_after_ms: value.retry_after_ms,
			message: value.message,
			current: value.current.map(Into::into),
		}
	}
}

impl Downgrade for client2::RepairRefused {
	type Older = client1::RepairRefused;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairRefused {
			code: self.code.into(),
			retry_after_ms: self.retry_after_ms,
			message: self.message,
			current: downgrade_opt(self.current)?,
		})
	}
}

impl From<client1::RepairPlanned> for client2::RepairPlanned {
	fn from(value: client1::RepairPlanned) -> Self {
		Self {
			plan: value.plan.into(),
			plan_digest: value.plan_digest,
		}
	}
}

impl Downgrade for client2::RepairPlanned {
	type Older = client1::RepairPlanned;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairPlanned {
			plan: self.plan.downgrade()?,
			plan_digest: self.plan_digest,
		})
	}
}

impl From<client1::RepairApplied> for client2::RepairApplied {
	fn from(value: client1::RepairApplied) -> Self {
		Self {
			plan: value.plan.into(),
			plan_digest: value.plan_digest,
			detail: value.detail.map(Into::into),
		}
	}
}

impl Downgrade for client2::RepairApplied {
	type Older = client1::RepairApplied;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairApplied {
			plan: self.plan.downgrade()?,
			plan_digest: self.plan_digest,
			detail: self.detail.map(Into::into),
		})
	}
}

impl From<client1::RepairOutcome> for client2::RepairOutcome {
	fn from(value: client1::RepairOutcome) -> Self {
		match value {
			client1::RepairOutcome::RepairApplied(v) => Self::RepairApplied(v.into()),
			client1::RepairOutcome::RepairRefused(v) => Self::RepairRefused(v.into()),
			client1::RepairOutcome::RepairUnknown(v) => Self::RepairUnknown(v.into()),
		}
	}
}

impl Downgrade for client2::RepairOutcome {
	type Older = client1::RepairOutcome;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::RepairApplied(v) => client1::RepairOutcome::RepairApplied(v.downgrade()?),
			Self::RepairRefused(v) => client1::RepairOutcome::RepairRefused(v.downgrade()?),
			Self::RepairUnknown(v) => client1::RepairOutcome::RepairUnknown(v.into()),
		})
	}
}

impl From<client1::RepairResult> for client2::RepairResult {
	fn from(value: client1::RepairResult) -> Self {
		match value {
			client1::RepairResult::RepairPlanned(v) => Self::RepairPlanned(v.into()),
			client1::RepairResult::RepairApplied(v) => Self::RepairApplied(v.into()),
			client1::RepairResult::RepairRefused(v) => Self::RepairRefused(v.into()),
			client1::RepairResult::RepairUnknown(v) => Self::RepairUnknown(v.into()),
		}
	}
}

impl Downgrade for client2::RepairResult {
	type Older = client1::RepairResult;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::RepairPlanned(v) => client1::RepairResult::RepairPlanned(v.downgrade()?),
			Self::RepairApplied(v) => client1::RepairResult::RepairApplied(v.downgrade()?),
			Self::RepairRefused(v) => client1::RepairResult::RepairRefused(v.downgrade()?),
			Self::RepairUnknown(v) => client1::RepairResult::RepairUnknown(v.into()),
		})
	}
}

impl From<client1::RepairResponse> for client2::RepairResponse {
	fn from(value: client1::RepairResponse) -> Self {
		Self {
			operation_id: value.operation_id,
			already_applied: value.already_applied,
			result: value.result.into(),
		}
	}
}

impl Downgrade for client2::RepairResponse {
	type Older = client1::RepairResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairResponse {
			operation_id: self.operation_id,
			already_applied: self.already_applied,
			result: self.result.downgrade()?,
		})
	}
}

impl From<client1::RepairAuditRow> for client2::RepairAuditRow {
	fn from(value: client1::RepairAuditRow) -> Self {
		Self {
			request: value.request.into(),
			requested_at_unix_ms: value.requested_at_unix_ms,
			plan_digest: value.plan_digest,
			outcome: value.outcome.map(Into::into),
		}
	}
}

impl Downgrade for client2::RepairAuditRow {
	type Older = client1::RepairAuditRow;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairAuditRow {
			request: self.request.downgrade()?,
			requested_at_unix_ms: self.requested_at_unix_ms,
			plan_digest: self.plan_digest,
			outcome: downgrade_opt(self.outcome)?,
		})
	}
}

impl From<client1::RepairAuditPage> for client2::RepairAuditPage {
	fn from(value: client1::RepairAuditPage) -> Self {
		Self {
			audits: value.audits.into_iter().map(Into::into).collect(),
			after: value.after,
		}
	}
}

impl Downgrade for client2::RepairAuditPage {
	type Older = client1::RepairAuditPage;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairAuditPage {
			audits: downgrade_all(self.audits)?,
			after: self.after,
		})
	}
}

impl From<client1::RepairPin> for client2::RepairPin {
	fn from(value: client1::RepairPin) -> Self {
		Self {
			session_id: value.session_id,
			as_of: value.as_of,
			applied: FsCut::single(value.applied),
			expires_boot_ms: value.expires_boot_ms,
			base_chain: value.base_chain.into_iter().map(Into::into).collect(),
		}
	}
}

impl Downgrade for client2::RepairPin {
	type Older = client1::RepairPin;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairPin {
			session_id: self.session_id,
			as_of: self.as_of,
			applied: self.applied.downgrade()?,
			expires_boot_ms: self.expires_boot_ms,
			base_chain: downgrade_all(self.base_chain)?,
		})
	}
}

impl From<client1::RepairDiagnosticValue> for client2::RepairDiagnosticValue {
	fn from(value: client1::RepairDiagnosticValue) -> Self {
		match value {
			client1::RepairDiagnosticValue::FsForkHold(v) => Self::FsForkHold(v.into()),
			client1::RepairDiagnosticValue::FsPendingFork(v) => Self::FsPendingFork(v.into()),
			client1::RepairDiagnosticValue::FsPendingForkCheck(v) => {
				Self::FsPendingForkCheck(v.into())
			}
			client1::RepairDiagnosticValue::FsPendingForkRelease(v) => {
				Self::FsPendingForkRelease(v.into())
			}
			client1::RepairDiagnosticValue::FsPendingMaterialize(v) => {
				Self::FsPendingMaterialize(v.into())
			}
			client1::RepairDiagnosticValue::FsPendingSweep(v) => Self::FsPendingSweep(v.into()),
			client1::RepairDiagnosticValue::RepairPendingDelete(v) => {
				Self::RepairPendingDelete(v.into())
			}
			client1::RepairDiagnosticValue::RepairPin(v) => Self::RepairPin(v.into()),
			client1::RepairDiagnosticValue::AlarmCurrent(v) => Self::AlarmCurrent(v),
			client1::RepairDiagnosticValue::FsVolumeRecord(v) => Self::FsVolumeRecord(v.into()),
			client1::RepairDiagnosticValue::FsIncarnationRecord(v) => {
				Self::FsIncarnationRecord(v.into())
			}
			client1::RepairDiagnosticValue::RepairActivity(v) => Self::RepairActivity(v.into()),
		}
	}
}

impl Downgrade for client2::RepairDiagnosticValue {
	type Older = client1::RepairDiagnosticValue;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsForkHold(v) => client1::RepairDiagnosticValue::FsForkHold(v.downgrade()?),
			Self::FsPendingFork(v) => client1::RepairDiagnosticValue::FsPendingFork(v.downgrade()?),
			Self::FsPendingForkCheck(v) => {
				client1::RepairDiagnosticValue::FsPendingForkCheck(v.into())
			}
			Self::FsPendingForkRelease(v) => {
				client1::RepairDiagnosticValue::FsPendingForkRelease(v.downgrade()?)
			}
			Self::FsPendingMaterialize(v) => {
				client1::RepairDiagnosticValue::FsPendingMaterialize(v.into())
			}
			Self::FsPendingSweep(v) => client1::RepairDiagnosticValue::FsPendingSweep(v.into()),
			Self::RepairPendingDelete(v) => {
				client1::RepairDiagnosticValue::RepairPendingDelete(v.into())
			}
			Self::RepairPin(v) => client1::RepairDiagnosticValue::RepairPin(v.downgrade()?),
			Self::AlarmCurrent(v) => client1::RepairDiagnosticValue::AlarmCurrent(v),
			Self::FsVolumeRecord(v) => {
				client1::RepairDiagnosticValue::FsVolumeRecord(v.downgrade()?)
			}
			Self::FsIncarnationRecord(v) => {
				client1::RepairDiagnosticValue::FsIncarnationRecord(v.downgrade()?)
			}
			Self::RepairActivity(v) => client1::RepairDiagnosticValue::RepairActivity(v.into()),
		})
	}
}

impl From<client1::RepairDiagnosticRow> for client2::RepairDiagnosticRow {
	fn from(value: client1::RepairDiagnosticRow) -> Self {
		Self {
			key: value.key,
			raw: value.raw,
			decoded: value.decoded.map(Into::into),
			decode_error: value.decode_error,
		}
	}
}

impl Downgrade for client2::RepairDiagnosticRow {
	type Older = client1::RepairDiagnosticRow;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairDiagnosticRow {
			key: self.key,
			raw: self.raw,
			decoded: downgrade_opt(self.decoded)?,
			decode_error: self.decode_error,
		})
	}
}

impl From<client1::RepairDiagnosticsPage> for client2::RepairDiagnosticsPage {
	fn from(value: client1::RepairDiagnosticsPage) -> Self {
		Self {
			rows: value.rows.into_iter().map(Into::into).collect(),
			after: value.after,
		}
	}
}

impl Downgrade for client2::RepairDiagnosticsPage {
	type Older = client1::RepairDiagnosticsPage;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::RepairDiagnosticsPage {
			rows: downgrade_all(self.rows)?,
			after: self.after,
		})
	}
}

// Version 2 adds the refusal of an open in an archived group, which a version 1 reader never
// meets: a group is archived only once every reader of its volumes is at version 2.

impl From<client1::VolumeRejectResponse> for client2::VolumeRejectResponse {
	fn from(value: client1::VolumeRejectResponse) -> Self {
		match value {
			client1::VolumeRejectResponse::NotMaintainer(v) => Self::NotMaintainer(v.into()),
			client1::VolumeRejectResponse::TabletSplitting => Self::TabletSplitting,
			client1::VolumeRejectResponse::VolumeNotFound => Self::VolumeNotFound,
			client1::VolumeRejectResponse::VolumeExists => Self::VolumeExists,
			client1::VolumeRejectResponse::VolumeDeleted => Self::VolumeDeleted,
			client1::VolumeRejectResponse::Overloaded(v) => Self::Overloaded(v.into()),
			client1::VolumeRejectResponse::VolumeRecordMissing(v) => {
				Self::VolumeRecordMissing(v.into())
			}
			client1::VolumeRejectResponse::VolumeUnderRepair(v) => {
				Self::VolumeUnderRepair(v.into())
			}
		}
	}
}

impl Downgrade for client2::VolumeRejectResponse {
	type Older = client1::VolumeRejectResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::NotMaintainer(v) => client1::VolumeRejectResponse::NotMaintainer(v.into()),
			Self::TabletSplitting => client1::VolumeRejectResponse::TabletSplitting,
			Self::VolumeNotFound => client1::VolumeRejectResponse::VolumeNotFound,
			Self::VolumeExists => client1::VolumeRejectResponse::VolumeExists,
			Self::VolumeDeleted => client1::VolumeRejectResponse::VolumeDeleted,
			Self::Overloaded(v) => client1::VolumeRejectResponse::Overloaded(v.into()),
			Self::VolumeRecordMissing(v) => {
				client1::VolumeRejectResponse::VolumeRecordMissing(v.into())
			}
			Self::VolumeUnderRepair(v) => {
				client1::VolumeRejectResponse::VolumeUnderRepair(v.into())
			}
			Self::VolumeGroupArchived => {
				anyhow::bail!("version 1 has no archived group refusal")
			}
			// A version 1 reader opens again after the hint, as it does when the lazy fork is
			// still running; it cannot ask for the ancestor's tail to be proven.
			Self::VolumeTailUnproven(v) => {
				client1::VolumeRejectResponse::Overloaded(client1::Overloaded {
					retry_after_ms: v.retry_after_ms,
				})
			}
		})
	}
}

impl From<client1::OperatorTabletRejected> for client2::OperatorTabletRejected {
	fn from(value: client1::OperatorTabletRejected) -> Self {
		Self {
			tablet_id: value.tablet_id,
			reason: value.reason.into(),
		}
	}
}

impl Downgrade for client2::OperatorTabletRejected {
	type Older = client1::OperatorTabletRejected;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::OperatorTabletRejected {
			tablet_id: self.tablet_id,
			reason: self.reason.downgrade()?,
		})
	}
}

impl From<client1::OperatorTabletOutcome> for client2::OperatorTabletOutcome {
	fn from(value: client1::OperatorTabletOutcome) -> Self {
		match value {
			client1::OperatorTabletOutcome::OperatorTabletReport(v) => {
				Self::OperatorTabletReport(v.into())
			}
			client1::OperatorTabletOutcome::OperatorTabletRejected(v) => {
				Self::OperatorTabletRejected(v.into())
			}
		}
	}
}

impl Downgrade for client2::OperatorTabletOutcome {
	type Older = client1::OperatorTabletOutcome;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::OperatorTabletReport(v) => {
				client1::OperatorTabletOutcome::OperatorTabletReport(v.into())
			}
			Self::OperatorTabletRejected(v) => {
				client1::OperatorTabletOutcome::OperatorTabletRejected(v.downgrade()?)
			}
		})
	}
}

impl From<client1::OperatorResponse> for client2::OperatorResponse {
	fn from(value: client1::OperatorResponse) -> Self {
		Self {
			tablets: value.tablets.into_iter().map(Into::into).collect(),
			caches: value.caches.map(Into::into),
		}
	}
}

impl Downgrade for client2::OperatorResponse {
	type Older = client1::OperatorResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(client1::OperatorResponse {
			tablets: downgrade_all(self.tablets)?,
			caches: self.caches.map(Into::into),
		})
	}
}

impl From<client1::ClientRequest> for client2::ClientRequest {
	fn from(value: client1::ClientRequest) -> Self {
		match value {
			client1::ClientRequest::FsReadRequest(v) => Self::FsReadRequest(v.into()),
			client1::ClientRequest::VolumeCheckpointRequest(v) => {
				Self::VolumeCheckpointRequest(v.into())
			}
			client1::ClientRequest::FsForkReferenceRequest(v) => {
				Self::FsForkReferenceRequest(v.into())
			}
			client1::ClientRequest::FsForkRequest(v) => Self::FsForkRequest(v.into()),
			client1::ClientRequest::VolumeCreateRequest(v) => Self::VolumeCreateRequest(v.into()),
			client1::ClientRequest::VolumeDeleteRequest(v) => Self::VolumeDeleteRequest(v.into()),
			client1::ClientRequest::VolumeOpenRequest(v) => Self::VolumeOpenRequest(v.into()),
			client1::ClientRequest::VolumeCloseRequest(v) => Self::VolumeCloseRequest(v.into()),
			client1::ClientRequest::ReplicaStatusNoticeRequest(v) => {
				Self::ReplicaStatusNoticeRequest(v.into())
			}
			client1::ClientRequest::OperatorTierRequest(v) => Self::OperatorTierRequest(v.into()),
			client1::ClientRequest::OperatorDropCachesRequest(v) => {
				Self::OperatorDropCachesRequest(v.into())
			}
			client1::ClientRequest::RepairInspectRequest(v) => Self::RepairInspectRequest(v.into()),
			client1::ClientRequest::RepairRequest(v) => Self::RepairRequest(v.into()),
			client1::ClientRequest::RepairAuditListRequest(v) => {
				Self::RepairAuditListRequest(v.into())
			}
			client1::ClientRequest::RepairDiagnosticsRequest(v) => {
				Self::RepairDiagnosticsRequest(v.into())
			}
		}
	}
}

impl Downgrade for client2::ClientRequest {
	type Older = client1::ClientRequest;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsReadRequest(v) => client1::ClientRequest::FsReadRequest(v.downgrade()?),
			Self::VolumeCheckpointRequest(v) => {
				client1::ClientRequest::VolumeCheckpointRequest(v.downgrade()?)
			}
			Self::FsForkReferenceRequest(v) => {
				client1::ClientRequest::FsForkReferenceRequest(v.downgrade()?)
			}
			Self::FsForkRequest(v) => client1::ClientRequest::FsForkRequest(v.downgrade()?),
			Self::VolumeCreateRequest(v) => client1::ClientRequest::VolumeCreateRequest(v.into()),
			Self::VolumeDeleteRequest(v) => client1::ClientRequest::VolumeDeleteRequest(v.into()),
			Self::VolumeOpenRequest(v) => client1::ClientRequest::VolumeOpenRequest(v.downgrade()?),
			Self::VolumeCloseRequest(v) => client1::ClientRequest::VolumeCloseRequest(v.into()),
			Self::ReplicaStatusNoticeRequest(v) => {
				client1::ClientRequest::ReplicaStatusNoticeRequest(v.into())
			}
			Self::OperatorTierRequest(v) => client1::ClientRequest::OperatorTierRequest(v.into()),
			Self::OperatorDropCachesRequest(v) => {
				client1::ClientRequest::OperatorDropCachesRequest(v.into())
			}
			Self::RepairInspectRequest(v) => client1::ClientRequest::RepairInspectRequest(v.into()),
			Self::RepairRequest(v) => client1::ClientRequest::RepairRequest(v.downgrade()?),
			Self::RepairAuditListRequest(v) => {
				client1::ClientRequest::RepairAuditListRequest(v.into())
			}
			Self::RepairDiagnosticsRequest(v) => {
				client1::ClientRequest::RepairDiagnosticsRequest(v.into())
			}
		})
	}
}

impl From<client1::ClientResponse> for client2::ClientResponse {
	fn from(value: client1::ClientResponse) -> Self {
		match value {
			client1::ClientResponse::FsReadResponse(v) => Self::FsReadResponse(v.into()),
			client1::ClientResponse::VolumeCheckpointResponse(v) => {
				Self::VolumeCheckpointResponse(v.into())
			}
			client1::ClientResponse::FsForkReferenceResponse(v) => {
				Self::FsForkReferenceResponse(v.into())
			}
			client1::ClientResponse::FsForkResponse(v) => Self::FsForkResponse(v.into()),
			client1::ClientResponse::VolumeCreateResponse(v) => {
				Self::VolumeCreateResponse(v.into())
			}
			client1::ClientResponse::VolumeDeleteResponse => Self::VolumeDeleteResponse,
			client1::ClientResponse::VolumeOpenResponse(v) => Self::VolumeOpenResponse(v.into()),
			client1::ClientResponse::VolumeCloseResponse => Self::VolumeCloseResponse,
			client1::ClientResponse::VolumeRejectResponse(v) => {
				Self::VolumeRejectResponse(v.into())
			}
			client1::ClientResponse::ReplicaStatusNoticeResponse(v) => {
				Self::ReplicaStatusNoticeResponse(v.into())
			}
			client1::ClientResponse::OperatorTabletReport(v) => {
				Self::OperatorTabletReport(v.into())
			}
			client1::ClientResponse::OperatorResponse(v) => Self::OperatorResponse(v.into()),
			client1::ClientResponse::RepairInspection(v) => Self::RepairInspection(v.into()),
			client1::ClientResponse::RepairResponse(v) => Self::RepairResponse(v.into()),
			client1::ClientResponse::RepairAuditPage(v) => Self::RepairAuditPage(v.into()),
			client1::ClientResponse::RepairDiagnosticsPage(v) => {
				Self::RepairDiagnosticsPage(v.into())
			}
		}
	}
}

impl Downgrade for client2::ClientResponse {
	type Older = client1::ClientResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::FsReadResponse(v) => client1::ClientResponse::FsReadResponse(v.downgrade()?),
			Self::VolumeCheckpointResponse(v) => {
				client1::ClientResponse::VolumeCheckpointResponse(v.downgrade()?)
			}
			Self::FsForkReferenceResponse(v) => {
				client1::ClientResponse::FsForkReferenceResponse(v.downgrade()?)
			}
			Self::FsForkResponse(v) => client1::ClientResponse::FsForkResponse(v.downgrade()?),
			Self::VolumeCreateResponse(v) => {
				client1::ClientResponse::VolumeCreateResponse(v.downgrade()?)
			}
			Self::VolumeDeleteResponse => client1::ClientResponse::VolumeDeleteResponse,
			Self::VolumeOpenResponse(v) => {
				client1::ClientResponse::VolumeOpenResponse(v.downgrade()?)
			}
			Self::VolumeCloseResponse => client1::ClientResponse::VolumeCloseResponse,
			Self::VolumeRejectResponse(v) => {
				client1::ClientResponse::VolumeRejectResponse(v.downgrade()?)
			}
			Self::ReplicaStatusNoticeResponse(v) => {
				client1::ClientResponse::ReplicaStatusNoticeResponse(v.into())
			}
			Self::OperatorTabletReport(v) => {
				client1::ClientResponse::OperatorTabletReport(v.into())
			}
			Self::OperatorResponse(v) => client1::ClientResponse::OperatorResponse(v.downgrade()?),
			Self::RepairInspection(v) => client1::ClientResponse::RepairInspection(v.downgrade()?),
			Self::RepairResponse(v) => client1::ClientResponse::RepairResponse(v.downgrade()?),
			Self::RepairAuditPage(v) => client1::ClientResponse::RepairAuditPage(v.downgrade()?),
			Self::RepairDiagnosticsPage(v) => {
				client1::ClientResponse::RepairDiagnosticsPage(v.downgrade()?)
			}
		})
	}
}

// depot.maintainer_log

/// A version 2 op's targets as the one optional target version 1 carries.
fn single_target(
	targets: Vec<log2::MaintainerOpTarget>,
) -> Result<Option<log1::MaintainerOpTarget>> {
	let mut targets = targets.into_iter();
	let (target, None) = (targets.next(), targets.next()) else {
		anyhow::bail!("the op targets more than one claim log, which version 1 cannot name");
	};
	Ok(target.map(Into::into))
}

impl From<log1::CheckpointOp> for log2::CheckpointOp {
	fn from(value: log1::CheckpointOp) -> Self {
		Self {
			request: value.request.into(),
			created_at_unix_ms: value.created_at_unix_ms,
			targets: value.target.into_iter().map(Into::into).collect(),
		}
	}
}

impl Downgrade for log2::CheckpointOp {
	type Older = log1::CheckpointOp;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(log1::CheckpointOp {
			request: self.request.downgrade()?,
			created_at_unix_ms: self.created_at_unix_ms,
			target: single_target(self.targets)?,
		})
	}
}

impl From<log1::RetentionCheckpointOp> for log2::RetentionCheckpointOp {
	fn from(value: log1::RetentionCheckpointOp) -> Self {
		Self {
			targets: vec![value.target.into()],
			created_at_unix_ms: value.created_at_unix_ms,
		}
	}
}

impl Downgrade for log2::RetentionCheckpointOp {
	type Older = log1::RetentionCheckpointOp;

	fn downgrade(self) -> Result<Self::Older> {
		let Some(target) = single_target(self.targets)? else {
			anyhow::bail!("a retention checkpoint op targets one claim log");
		};
		Ok(log1::RetentionCheckpointOp {
			target,
			created_at_unix_ms: self.created_at_unix_ms,
		})
	}
}

impl From<log1::ForkReferenceOp> for log2::ForkReferenceOp {
	fn from(value: log1::ForkReferenceOp) -> Self {
		Self {
			request: value.request.into(),
			created_at_unix_ms: value.created_at_unix_ms,
			targets: value.target.into_iter().map(Into::into).collect(),
		}
	}
}

impl Downgrade for log2::ForkReferenceOp {
	type Older = log1::ForkReferenceOp;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(log1::ForkReferenceOp {
			request: self.request.downgrade()?,
			created_at_unix_ms: self.created_at_unix_ms,
			target: single_target(self.targets)?,
		})
	}
}

impl From<log1::ForkPublishOp> for log2::ForkPublishOp {
	fn from(value: log1::ForkPublishOp) -> Self {
		Self {
			spec: value.spec.into(),
			image: value.image.into(),
			targets: value.target.into_iter().map(Into::into).collect(),
			incarnation_epoch: value.incarnation_epoch,
			created_at_fork: None,
		}
	}
}

impl Downgrade for log2::ForkPublishOp {
	type Older = log1::ForkPublishOp;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(log1::ForkPublishOp {
			spec: self.spec.downgrade()?,
			image: self.image.downgrade()?,
			target: single_target(self.targets)?,
			incarnation_epoch: self.incarnation_epoch,
		})
	}
}

impl From<log1::ForkFailOp> for log2::ForkFailOp {
	fn from(value: log1::ForkFailOp) -> Self {
		Self {
			spec: value.spec.into(),
			error: value.error.into(),
		}
	}
}

impl Downgrade for log2::ForkFailOp {
	type Older = log1::ForkFailOp;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(log1::ForkFailOp {
			spec: self.spec.downgrade()?,
			error: self.error.into(),
		})
	}
}

impl From<log1::RepairAuditOp> for log2::RepairAuditOp {
	fn from(value: log1::RepairAuditOp) -> Self {
		Self {
			row: value.row.into(),
		}
	}
}

impl Downgrade for log2::RepairAuditOp {
	type Older = log1::RepairAuditOp;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(log1::RepairAuditOp {
			row: self.row.downgrade()?,
		})
	}
}

impl From<log1::MaintainerOp> for log2::MaintainerOp {
	fn from(value: log1::MaintainerOp) -> Self {
		match value {
			log1::MaintainerOp::CheckpointOp(v) => Self::CheckpointOp(v.into()),
			log1::MaintainerOp::RetentionCheckpointOp(v) => Self::RetentionCheckpointOp(v.into()),
			log1::MaintainerOp::ForkReferenceOp(v) => Self::ForkReferenceOp(v.into()),
			log1::MaintainerOp::ForkPublishOp(v) => Self::ForkPublishOp(v.into()),
			log1::MaintainerOp::ForkFailOp(v) => Self::ForkFailOp(v.into()),
			log1::MaintainerOp::RepairAuditOp(v) => Self::RepairAuditOp(v.into()),
		}
	}
}

impl Downgrade for log2::MaintainerOp {
	type Older = log1::MaintainerOp;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::CheckpointOp(v) => log1::MaintainerOp::CheckpointOp(v.downgrade()?),
			Self::RetentionCheckpointOp(v) => {
				log1::MaintainerOp::RetentionCheckpointOp(v.downgrade()?)
			}
			Self::ForkReferenceOp(v) => log1::MaintainerOp::ForkReferenceOp(v.downgrade()?),
			Self::ForkPublishOp(v) => log1::MaintainerOp::ForkPublishOp(v.downgrade()?),
			Self::ForkFailOp(v) => log1::MaintainerOp::ForkFailOp(v.downgrade()?),
			Self::RepairAuditOp(v) => log1::MaintainerOp::RepairAuditOp(v.downgrade()?),
			Self::ForkPinOp(_) => anyhow::bail!("version 1 has no fork pin op"),
		})
	}
}

impl From<log1::MaintainerEntry> for log2::MaintainerEntry {
	fn from(value: log1::MaintainerEntry) -> Self {
		Self {
			ops: value.ops.into_iter().map(Into::into).collect(),
		}
	}
}

impl Downgrade for log2::MaintainerEntry {
	type Older = log1::MaintainerEntry;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(log1::MaintainerEntry {
			ops: downgrade_all(self.ops)?,
		})
	}
}

impl From<log1::VolumeCreation> for log2::VolumeCreation {
	fn from(value: log1::VolumeCreation) -> Self {
		Self {
			incarnation_epoch: value.incarnation_epoch,
			base_chain: value.base_chain.into_iter().map(Into::into).collect(),
			retention: value.retention.map(Into::into),
			created_at_fork: None,
			lazy_fork: None,
			metadata_local: false,
		}
	}
}

impl Downgrade for log2::VolumeCreation {
	type Older = log1::VolumeCreation;

	fn downgrade(self) -> Result<Self::Older> {
		// A grant that selected version 1 refuses grouped creates and group forks, so evidence
		// that carries their fields never needs version 1; dropping them would rebuild the
		// volume without its place in its group's forks. `metadataLocal` has no version 1 field
		// and is dropped, so version 1 evidence decodes it unset.
		ensure!(
			self.created_at_fork.is_none() && self.lazy_fork.is_none(),
			"version 1 has no volume groups"
		);
		Ok(log1::VolumeCreation {
			incarnation_epoch: self.incarnation_epoch,
			base_chain: downgrade_all(self.base_chain)?,
			retention: self.retention.map(Into::into),
		})
	}
}

downgrade_into! {
	log2::MaintainerLogRecord => log1::MaintainerLogRecord,
}
