use anyhow::{Context, Result};
use universaldb::utils::IsolationLevel::Snapshot;

use crate::conveyer::{
	branch,
	db::{BranchAncestry, load_branch_ancestry},
	error::SqliteStorageError,
	keys::{self, SHARD_SIZE},
	types::{BucketId, DBHead, DatabaseBranchId, decode_db_head},
};

#[derive(Debug, Clone)]
pub(super) enum StorageScope {
	Branch(BranchReadPlan),
}

impl StorageScope {
	pub(super) fn branch_id(&self) -> DatabaseBranchId {
		match self {
			Self::Branch(plan) => plan.branch_id,
		}
	}

	pub(super) fn branch_ancestry(&self) -> BranchAncestry {
		match self {
			Self::Branch(plan) => plan.ancestry.clone(),
		}
	}

	pub(super) fn cold_layer_candidates(&self, pgno: u32) -> Vec<super::cold::ColdLayerCandidate> {
		match self {
			Self::Branch(plan) => plan
				.sources
				.iter()
				.map(|source| match source {
					ReadSource::Branch(source) => super::cold::ColdLayerCandidate {
						branch_id: source.branch_id,
						owner_txid: source.max_txid,
						shard_id: pgno / SHARD_SIZE,
					},
				})
				.collect(),
		}
	}
}

#[derive(Debug, Clone)]
pub(super) struct BranchReadPlan {
	pub(super) branch_id: DatabaseBranchId,
	pub(super) head: DBHead,
	pub(super) ancestry: BranchAncestry,
	pub(super) sources: Vec<ReadSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReadSource {
	Branch(BranchSource),
}

impl ReadSource {
	pub(super) fn pidx_prefix(self, database_id: &str) -> Vec<u8> {
		let _ = database_id;
		match self {
			Self::Branch(source) => keys::branch_pidx_prefix(source.branch_id),
		}
	}

	pub(super) fn decode_pidx_pgno(self, database_id: &str, key: &[u8]) -> Result<u32> {
		let _ = database_id;
		match self {
			Self::Branch(source) => super::pidx::decode_branch_pidx_pgno(source.branch_id, key),
		}
	}

	pub(super) fn delta_chunk_prefix(self, database_id: &str, txid: u64) -> Vec<u8> {
		let _ = database_id;
		match self {
			Self::Branch(source) => keys::branch_delta_chunk_prefix(source.branch_id, txid),
		}
	}

	pub(super) fn delta_prefix(self, database_id: &str) -> Vec<u8> {
		let _ = database_id;
		match self {
			Self::Branch(source) => keys::branch_delta_prefix(source.branch_id),
		}
	}

	pub(super) fn decode_delta_chunk_txid(self, database_id: &str, key: &[u8]) -> Result<u64> {
		let _ = database_id;
		match self {
			Self::Branch(source) => keys::decode_branch_delta_chunk_txid(source.branch_id, key),
		}
	}

	pub(super) fn decode_delta_chunk_idx(
		self,
		database_id: &str,
		txid: u64,
		key: &[u8],
	) -> Result<u32> {
		let _ = database_id;
		match self {
			Self::Branch(source) => {
				keys::decode_branch_delta_chunk_idx(source.branch_id, txid, key)
			}
		}
	}

	pub(super) fn max_txid(self) -> u64 {
		match self {
			Self::Branch(source) => source.max_txid,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct BranchSource {
	pub(super) branch_id: DatabaseBranchId,
	pub(super) max_txid: u64,
}

pub(super) async fn resolve_storage_scope(
	tx: &universaldb::Transaction,
	bucket_id: BucketId,
	database_id: &str,
	cached_ancestry: Option<&BranchAncestry>,
) -> Result<StorageScope> {
	Ok(
		match branch::resolve_database_branch(tx, bucket_id, database_id, Snapshot).await? {
			Some(branch_id) => {
				StorageScope::Branch(load_branch_read_plan(tx, branch_id, cached_ancestry).await?)
			}
			None => return Err(SqliteStorageError::DatabaseNotFound.into()),
		},
	)
}

async fn load_branch_read_plan(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	cached_ancestry: Option<&BranchAncestry>,
) -> Result<BranchReadPlan> {
	let head_bytes = super::tx::tx_get_value(tx, &keys::branch_meta_head_key(branch_id)).await?;
	let head = if let Some(head_bytes) = head_bytes {
		decode_db_head(&head_bytes)?
	} else {
		let head_at_fork_bytes =
			super::tx::tx_get_value(tx, &keys::branch_meta_head_at_fork_key(branch_id))
				.await?
				.ok_or(SqliteStorageError::MetaMissing {
					operation: "get_pages",
				})?;
		decode_db_head(&head_at_fork_bytes)?
	};

	let ancestry = if let Some(cached_ancestry) =
		cached_ancestry.filter(|ancestry| ancestry.root_branch_id == branch_id)
	{
		cached_ancestry.clone()
	} else {
		load_branch_ancestry(tx, branch_id).await?
	};

	let mut sources = Vec::new();
	for ancestor in &ancestry.ancestors {
		let max_txid = match ancestor.parent_versionstamp_cap {
			Some(parent_versionstamp) => {
				lookup_txid_for_read(tx, ancestor.branch_id, parent_versionstamp).await?
			}
			None => head.head_txid,
		};
		sources.push(ReadSource::Branch(BranchSource {
			branch_id: ancestor.branch_id,
			max_txid,
		}));
	}

	Ok(BranchReadPlan {
		branch_id,
		head,
		ancestry,
		sources,
	})
}

async fn lookup_txid_for_read(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	versionstamp: [u8; 16],
) -> Result<u64> {
	let bytes = super::tx::tx_get_value(tx, &keys::branch_vtx_key(branch_id, versionstamp))
		.await?
		.ok_or(SqliteStorageError::RestoreTargetExpired)?;
	let bytes: [u8; std::mem::size_of::<u64>()] = bytes
		.as_slice()
		.try_into()
		.context("sqlite VTX entry should be exactly 8 bytes")?;

	Ok(u64::from_be_bytes(bytes))
}
