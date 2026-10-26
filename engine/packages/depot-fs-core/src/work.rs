//! Bounded discovery and reclamation shared by checkpoint and fork lifetimes.
/// Bounded discovery of persisted work, independent of volume data cardinality.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PendingWorkPage {
	pub materializations: Vec<crate::keys::slatedb::Scope>,
	pub cleanup: Vec<crate::forks::cleanup::Work>,
	/// Specs only: recovery re-inspects authoritative progress before acting.
	pub forks: Vec<crate::FsForkSpec>,
	pub scopes: Vec<crate::keys::slatedb::Scope>,
	/// Deleted volume records, each with its current row, for the deletion drive.
	pub deletes: Vec<(depot_types::VolumeKey, crate::FsVolumeRecord)>,
	pub after: Option<Vec<u8>>,
}

/// One volume key's pending rows, `[FS_PENDING, hash, name]`. Every pending row names the
/// volume key it belongs to, so a store write that touched a volume key's pending rows is
/// rediscovered by scanning this prefix alone.
#[derive(
	Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct PendingVolume {
	pub hash: depot_types::PartitionKeyHash,
	pub name: Option<Vec<u8>>,
}

impl PendingVolume {
	pub fn of(key: &depot_types::VolumeKey) -> Self {
		Self {
			hash: depot_types::partition_key_hash(key),
			name: key.name.clone().map(Into::into),
		}
	}

	pub fn of_scope(scope: &crate::keys::slatedb::Scope) -> Self {
		Self {
			hash: scope.hash,
			name: scope.name.clone(),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PendingSweepProgress {
	Held(SweepHold),
	More,
	Complete,
}

/// What keeps a retired incarnation from being swept. The kind decides who drives the sweep
/// again, because only one of them leaves a trace when it goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SweepHold {
	/// A read session of the incarnation has not expired. Sessions expire inside the store
	/// without an event, so the maintainer retries the sweep on a deadline.
	ReadSession,
	/// A named or retention checkpoint or a fork hold keeps the incarnation, possibly for
	/// ever. The store operation that releases the last one keeps the incarnation's pending
	/// sweep row and touches its volume's pending rows, and that volume's scan queues the
	/// sweep again, so nothing waits on it in memory.
	Checkpoint,
}
