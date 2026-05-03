#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DepotFaultPoint {
	Commit(CommitFaultPoint),
	Read(ReadFaultPoint),
	HotCompaction(HotCompactionFaultPoint),
	ColdCompaction(ColdCompactionFaultPoint),
	Reclaim(ReclaimFaultPoint),
	ColdTier(ColdTierFaultPoint),
	ShardCacheFill(ShardCacheFillFaultPoint),
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
	ColdRefSelected,
	ColdObjectMissing,
	BeforeReturnPages,
	ShardCacheFillEnqueue,
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
pub enum ColdCompactionFaultPoint {
	UploadBeforeInputRead,
	UploadAfterInputRead,
	UploadBeforePutObject,
	UploadAfterPutObject,
	PublishBeforeInputRead,
	PublishAfterInputRead,
	PublishBeforeColdRefWrite,
	PublishAfterColdRefWriteBeforeRootUpdate,
	PublishAfterRootUpdate,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ReclaimFaultPoint {
	PlanBeforeSnapshot,
	PlanAfterSnapshot,
	BeforeHotDelete,
	AfterHotDelete,
	BeforeColdRetire,
	AfterColdRetire,
	BeforeColdDelete,
	AfterColdDelete,
	BeforeCleanupRows,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ColdTierFaultPoint {
	PutObject,
	GetObject,
	DeleteObjects,
	ListPrefix,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ShardCacheFillFaultPoint {
	BeforeEnqueue,
	BeforeGetObject,
	AfterGetObject,
	BeforeShardWrite,
	AfterShardWrite,
	Skipped,
}

impl DepotFaultPoint {
	pub fn boundary(&self) -> FaultBoundary {
		match self {
			DepotFaultPoint::Commit(point) => point.boundary(),
			DepotFaultPoint::Read(point) => point.boundary(),
			DepotFaultPoint::HotCompaction(point) => point.boundary(),
			DepotFaultPoint::ColdCompaction(point) => point.boundary(),
			DepotFaultPoint::Reclaim(point) => point.boundary(),
			DepotFaultPoint::ColdTier(point) => point.boundary(),
			DepotFaultPoint::ShardCacheFill(point) => point.boundary(),
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
			| ReadFaultPoint::ColdRefSelected
			| ReadFaultPoint::ColdObjectMissing
			| ReadFaultPoint::BeforeReturnPages
			| ReadFaultPoint::ShardCacheFillEnqueue => FaultBoundary::ReadOnly,
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

impl ColdCompactionFaultPoint {
	pub fn boundary(&self) -> FaultBoundary {
		match self {
			ColdCompactionFaultPoint::UploadBeforeInputRead
			| ColdCompactionFaultPoint::UploadAfterInputRead
			| ColdCompactionFaultPoint::UploadBeforePutObject
			| ColdCompactionFaultPoint::UploadAfterPutObject
			| ColdCompactionFaultPoint::PublishBeforeInputRead
			| ColdCompactionFaultPoint::PublishAfterInputRead
			| ColdCompactionFaultPoint::PublishBeforeColdRefWrite
			| ColdCompactionFaultPoint::PublishAfterColdRefWriteBeforeRootUpdate
			| ColdCompactionFaultPoint::PublishAfterRootUpdate => FaultBoundary::WorkflowOnly,
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
			| ReclaimFaultPoint::BeforeColdRetire
			| ReclaimFaultPoint::AfterColdRetire
			| ReclaimFaultPoint::BeforeColdDelete
			| ReclaimFaultPoint::AfterColdDelete
			| ReclaimFaultPoint::BeforeCleanupRows => FaultBoundary::WorkflowOnly,
		}
	}
}

impl ColdTierFaultPoint {
	pub fn boundary(&self) -> FaultBoundary {
		match self {
			ColdTierFaultPoint::GetObject | ColdTierFaultPoint::ListPrefix => {
				FaultBoundary::ReadOnly
			}
			ColdTierFaultPoint::PutObject | ColdTierFaultPoint::DeleteObjects => {
				FaultBoundary::WorkflowOnly
			}
		}
	}
}

impl ShardCacheFillFaultPoint {
	pub fn boundary(&self) -> FaultBoundary {
		match self {
			ShardCacheFillFaultPoint::BeforeEnqueue
			| ShardCacheFillFaultPoint::BeforeGetObject
			| ShardCacheFillFaultPoint::AfterGetObject
			| ShardCacheFillFaultPoint::BeforeShardWrite
			| ShardCacheFillFaultPoint::AfterShardWrite
			| ShardCacheFillFaultPoint::Skipped => FaultBoundary::ReadOnly,
		}
	}
}
