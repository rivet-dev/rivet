use std::sync::Arc;

use anyhow::{Context, Result};
use gas::prelude::Id;
use parking_lot::Mutex;
use rivet_pools::NodeId;
use tokio::time::Instant;
use universaldb::Database;

use crate::{compactor::Ups, page_index::DeltaPageIndex};

use super::{
	branch,
	constants::MAX_FORK_DEPTH,
	error::SqliteStorageError,
	types::{ActorBranchId, NamespaceId},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BranchAncestry {
	pub root_branch_id: ActorBranchId,
	pub ancestors: Vec<BranchAncestor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BranchAncestor {
	pub branch_id: ActorBranchId,
	pub parent_versionstamp_cap: Option<[u8; 16]>,
}

impl BranchAncestry {
	pub(super) fn root(branch_id: ActorBranchId) -> Self {
		Self {
			root_branch_id: branch_id,
			ancestors: vec![BranchAncestor {
				branch_id,
				parent_versionstamp_cap: None,
			}],
		}
	}
}

#[allow(dead_code)]
pub struct ActorDb {
	pub(super) udb: Arc<Database>,
	pub(super) ups: Ups,
	pub(super) namespace_id: Id,
	pub(super) actor_id: String,
	pub(super) node_id: NodeId,
	/// Cached actor branch id. This is a perf cache; FDB remains the source of truth.
	pub(super) branch_id: Mutex<Option<ActorBranchId>>,
	/// Cached immutable branch ancestry. This is a perf cache for read planning.
	pub(super) ancestors: Mutex<Option<BranchAncestry>>,
	pub(super) cache: Mutex<DeltaPageIndex>,
	/// Cached `/META/quota`. Loaded once on the first UDB tx.
	pub(super) storage_used: Mutex<Option<i64>>,
	/// Bytes written across commits since the last metering rollup.
	pub(super) commit_bytes_since_rollup: Mutex<u64>,
	/// Bytes read across `get_pages` calls since the last metering rollup.
	pub(super) read_bytes_since_rollup: Mutex<u64>,
	/// Last time this actor published a compaction trigger.
	pub(super) last_trigger_at: Mutex<Option<Instant>>,
}

impl ActorDb {
	pub fn new(
		udb: Arc<Database>,
		ups: Ups,
		namespace_id: Id,
		actor_id: String,
		node_id: NodeId,
	) -> Self {
		#[cfg(debug_assertions)]
		crate::takeover::reconcile_blocking(udb.clone(), actor_id.clone(), node_id);

		Self {
			udb,
			ups,
			namespace_id,
			actor_id,
			node_id,
			branch_id: Mutex::new(None),
			ancestors: Mutex::new(None),
			cache: Mutex::new(DeltaPageIndex::new()),
			storage_used: Mutex::new(None),
			commit_bytes_since_rollup: Mutex::new(0),
			read_bytes_since_rollup: Mutex::new(0),
			last_trigger_at: Mutex::new(None),
		}
	}

	pub(super) fn sqlite_namespace_id(&self) -> NamespaceId {
		NamespaceId::from_gas_id(self.namespace_id)
	}

	pub fn take_metering_snapshot(&self) -> (u64, u64) {
		let mut commit_bytes = self.commit_bytes_since_rollup.lock();
		let mut read_bytes = self.read_bytes_since_rollup.lock();
		let snapshot = (*commit_bytes, *read_bytes);

		*commit_bytes = 0;
		*read_bytes = 0;

		snapshot
	}
}

pub(super) async fn load_branch_ancestry(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
) -> Result<BranchAncestry> {
	let mut ancestors = vec![BranchAncestor {
		branch_id,
		parent_versionstamp_cap: None,
	}];
	let mut current_branch_id = branch_id;

	for depth in 0..=MAX_FORK_DEPTH {
		let record = branch::read_actor_branch_record(tx, current_branch_id).await?;
		let Some(parent_branch_id) = record.parent else {
			return Ok(BranchAncestry {
				root_branch_id: branch_id,
				ancestors,
			});
		};
		if depth == MAX_FORK_DEPTH {
			return Err(SqliteStorageError::ForkChainTooDeep.into());
		}

		let parent_versionstamp = record
			.parent_versionstamp
			.context("sqlite actor branch parent versionstamp is missing")?;
		ancestors.push(BranchAncestor {
			branch_id: parent_branch_id,
			parent_versionstamp_cap: Some(parent_versionstamp),
		});
		current_branch_id = parent_branch_id;
	}

	Err(SqliteStorageError::ForkChainTooDeep.into())
}
