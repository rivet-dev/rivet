#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DepotFaultPoint {
	Commit(CommitFaultPoint),
	Read(ReadFaultPoint),
	HotCompaction(HotCompactionFaultPoint),
	Reclaim(ReclaimFaultPoint),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FaultBoundary {
	PreDurableCommit,
	AmbiguousAfterDurableCommit,
	PostDurableNonData,
	ReadOnly,
	WorkflowOnly,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CommitFaultPoint {
	BeforeTx,
	AfterBranchResolution,
	AfterHeadRead,
	AfterTruncateCleanup,
	AfterLtxEncode,
	BeforeDeltaWrites,
	BeforePidxWrites,
	BeforeHeadWrite,
	BeforeCommitRows,
	BeforeQuotaMutation,
	AfterUdbCommit,
	BeforeCompactionSignal,
	AfterCompactionSignal,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ReadFaultPoint {
	BeforeScopeResolve,
	AfterScopeResolve,
	AfterPidxScan,
	DeltaBlobMissing,
	AfterDeltaBlobLoad,
	AfterShardBlobLoad,
	BeforeReturnPages,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum HotCompactionFaultPoint {
	StageBeforeInputRead,
	StageAfterInputRead,
	StageAfterShardWrite,
	AfterStageBeforeFinishSignal,
	InstallBeforeStagedRead,
	InstallAfterStagedRead,
	InstallBeforeShardPublish,
	InstallAfterShardPublishBeforePidxClear,
	InstallBeforeRootUpdate,
	InstallAfterRootUpdate,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ReclaimFaultPoint {
	PlanBeforeSnapshot,
	PlanAfterSnapshot,
	BeforeHotDelete,
	AfterHotDelete,
	BeforeCleanupRows,
}

impl DepotFaultPoint {
	pub fn boundary(&self) -> FaultBoundary {
		match self {
			DepotFaultPoint::Commit(point) => point.boundary(),
			DepotFaultPoint::Read(point) => point.boundary(),
			DepotFaultPoint::HotCompaction(point) => point.boundary(),
			DepotFaultPoint::Reclaim(point) => point.boundary(),
		}
	}
}

impl CommitFaultPoint {
	pub fn boundary(&self) -> FaultBoundary {
		match self {
			CommitFaultPoint::BeforeTx
			| CommitFaultPoint::AfterBranchResolution
			| CommitFaultPoint::AfterHeadRead
			| CommitFaultPoint::AfterTruncateCleanup
			| CommitFaultPoint::AfterLtxEncode
			| CommitFaultPoint::BeforeDeltaWrites
			| CommitFaultPoint::BeforePidxWrites
			| CommitFaultPoint::BeforeHeadWrite
			| CommitFaultPoint::BeforeCommitRows
			| CommitFaultPoint::BeforeQuotaMutation => FaultBoundary::PreDurableCommit,
			CommitFaultPoint::AfterUdbCommit => FaultBoundary::AmbiguousAfterDurableCommit,
			CommitFaultPoint::BeforeCompactionSignal | CommitFaultPoint::AfterCompactionSignal => {
				FaultBoundary::PostDurableNonData
			}
		}
	}
}

impl ReadFaultPoint {
	pub fn boundary(&self) -> FaultBoundary {
		match self {
			ReadFaultPoint::BeforeScopeResolve
			| ReadFaultPoint::AfterScopeResolve
			| ReadFaultPoint::AfterPidxScan
			| ReadFaultPoint::DeltaBlobMissing
			| ReadFaultPoint::AfterDeltaBlobLoad
			| ReadFaultPoint::AfterShardBlobLoad
			| ReadFaultPoint::BeforeReturnPages => FaultBoundary::ReadOnly,
		}
	}
}

impl HotCompactionFaultPoint {
	pub fn boundary(&self) -> FaultBoundary {
		match self {
			HotCompactionFaultPoint::StageBeforeInputRead
			| HotCompactionFaultPoint::StageAfterInputRead
			| HotCompactionFaultPoint::StageAfterShardWrite
			| HotCompactionFaultPoint::AfterStageBeforeFinishSignal
			| HotCompactionFaultPoint::InstallBeforeStagedRead
			| HotCompactionFaultPoint::InstallAfterStagedRead
			| HotCompactionFaultPoint::InstallBeforeShardPublish
			| HotCompactionFaultPoint::InstallAfterShardPublishBeforePidxClear
			| HotCompactionFaultPoint::InstallBeforeRootUpdate
			| HotCompactionFaultPoint::InstallAfterRootUpdate => FaultBoundary::WorkflowOnly,
		}
	}
}

impl ReclaimFaultPoint {
	pub fn boundary(&self) -> FaultBoundary {
		match self {
			ReclaimFaultPoint::PlanBeforeSnapshot
			| ReclaimFaultPoint::PlanAfterSnapshot
			| ReclaimFaultPoint::BeforeHotDelete
			| ReclaimFaultPoint::AfterHotDelete
			| ReclaimFaultPoint::BeforeCleanupRows => FaultBoundary::WorkflowOnly,
		}
	}
}
