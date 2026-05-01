use anyhow::{Context, Result, bail};
use rivet_pools::NodeId;
use serde::{Deserialize, Serialize};
use universaldb::utils::IsolationLevel::Serializable;
use vbare::OwnedVersionedData;

use crate::conveyer::{keys, types::DatabaseBranchId};

pub const SQLITE_COLD_COMPACTOR_LEASE_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColdCompactorLease {
	pub holder_id: NodeId,
	pub expires_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColdTakeOutcome {
	Acquired,
	Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColdRenewOutcome {
	Renewed,
	Stolen,
	Expired,
}

enum VersionedColdCompactorLease {
	V1(ColdCompactorLease),
}

impl OwnedVersionedData for VersionedColdCompactorLease {
	type Latest = ColdCompactorLease;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid sqlite cold compactor lease version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub fn encode_cold_lease(lease: ColdCompactorLease) -> Result<Vec<u8>> {
	VersionedColdCompactorLease::wrap_latest(lease)
		.serialize_with_embedded_version(SQLITE_COLD_COMPACTOR_LEASE_VERSION)
		.context("encode sqlite cold compactor lease")
}

pub fn decode_cold_lease(payload: &[u8]) -> Result<ColdCompactorLease> {
	VersionedColdCompactorLease::deserialize_with_embedded_version(payload)
		.context("decode sqlite cold compactor lease")
}

pub async fn take(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	holder_id: NodeId,
	ttl_ms: u64,
	now_ms: i64,
) -> Result<ColdTakeOutcome> {
	let key = keys::branch_meta_cold_lease_key(branch_id);
	let current = tx.informal().get(&key, Serializable).await?;

	if let Some(current) = current {
		let lease = decode_cold_lease(&current)?;
		if lease.holder_id != holder_id && lease.expires_at_ms > now_ms {
			return Ok(ColdTakeOutcome::Skip);
		}
	}

	let lease = ColdCompactorLease {
		holder_id,
		expires_at_ms: expires_at_ms(now_ms, ttl_ms)?,
	};
	tx.informal().set(&key, &encode_cold_lease(lease)?);

	Ok(ColdTakeOutcome::Acquired)
}

pub async fn renew(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	holder_id: NodeId,
	ttl_ms: u64,
	now_ms: i64,
) -> Result<ColdRenewOutcome> {
	let key = keys::branch_meta_cold_lease_key(branch_id);
	let Some(current) = tx.informal().get(&key, Serializable).await? else {
		return Ok(ColdRenewOutcome::Expired);
	};
	let lease = decode_cold_lease(&current)?;

	if lease.holder_id != holder_id {
		return Ok(ColdRenewOutcome::Stolen);
	}

	if lease.expires_at_ms <= now_ms {
		return Ok(ColdRenewOutcome::Expired);
	}

	let lease = ColdCompactorLease {
		holder_id,
		expires_at_ms: expires_at_ms(now_ms, ttl_ms)?,
	};
	tx.informal().set(&key, &encode_cold_lease(lease)?);

	Ok(ColdRenewOutcome::Renewed)
}

pub async fn release(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	_holder_id: NodeId,
) -> Result<()> {
	tx.informal().clear(&keys::branch_meta_cold_lease_key(branch_id));
	Ok(())
}

fn expires_at_ms(now_ms: i64, ttl_ms: u64) -> Result<i64> {
	let ttl_ms = i64::try_from(ttl_ms).context("sqlite cold compactor lease ttl overflowed i64")?;
	now_ms
		.checked_add(ttl_ms)
		.context("sqlite cold compactor lease expiration overflowed i64")
}
