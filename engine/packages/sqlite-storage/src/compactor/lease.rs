use anyhow::{Context, Result, bail};
use rivet_pools::NodeId;
use serde::{Deserialize, Serialize};
use universaldb::utils::IsolationLevel::Serializable;
use vbare::OwnedVersionedData;

use crate::pump::keys;

pub const SQLITE_COMPACTOR_LEASE_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactorLease {
	pub holder_id: NodeId,
	pub expires_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeOutcome {
	Acquired,
	Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenewOutcome {
	Renewed,
	Stolen,
	Expired,
}

enum VersionedCompactorLease {
	V1(CompactorLease),
}

impl OwnedVersionedData for VersionedCompactorLease {
	type Latest = CompactorLease;

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
			_ => bail!("invalid sqlite compactor lease version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub fn encode_lease(lease: CompactorLease) -> Result<Vec<u8>> {
	VersionedCompactorLease::wrap_latest(lease)
		.serialize_with_embedded_version(SQLITE_COMPACTOR_LEASE_VERSION)
		.context("encode sqlite compactor lease")
}

pub fn decode_lease(payload: &[u8]) -> Result<CompactorLease> {
	VersionedCompactorLease::deserialize_with_embedded_version(payload)
		.context("decode sqlite compactor lease")
}

pub async fn take(
	tx: &universaldb::Transaction,
	actor_id: &str,
	holder_id: NodeId,
	ttl_ms: u64,
	now_ms: i64,
) -> Result<TakeOutcome> {
	let key = keys::meta_compactor_lease_key(actor_id);
	let current = tx.informal().get(&key, Serializable).await?;

	if let Some(current) = current {
		let lease = decode_lease(&current)?;
		if lease.holder_id != holder_id && lease.expires_at_ms > now_ms {
			return Ok(TakeOutcome::Skip);
		}
	}

	let lease = CompactorLease {
		holder_id,
		expires_at_ms: expires_at_ms(now_ms, ttl_ms)?,
	};
	tx.informal().set(&key, &encode_lease(lease)?);

	Ok(TakeOutcome::Acquired)
}

pub async fn renew(
	tx: &universaldb::Transaction,
	actor_id: &str,
	holder_id: NodeId,
	ttl_ms: u64,
	now_ms: i64,
) -> Result<RenewOutcome> {
	let key = keys::meta_compactor_lease_key(actor_id);
	let Some(current) = tx.informal().get(&key, Serializable).await? else {
		return Ok(RenewOutcome::Expired);
	};
	let lease = decode_lease(&current)?;

	if lease.holder_id != holder_id {
		return Ok(RenewOutcome::Stolen);
	}

	if lease.expires_at_ms <= now_ms {
		return Ok(RenewOutcome::Expired);
	}

	let lease = CompactorLease {
		holder_id,
		expires_at_ms: expires_at_ms(now_ms, ttl_ms)?,
	};
	tx.informal().set(&key, &encode_lease(lease)?);

	Ok(RenewOutcome::Renewed)
}

pub async fn release(
	tx: &universaldb::Transaction,
	actor_id: &str,
	_holder_id: NodeId,
) -> Result<()> {
	tx.informal().clear(&keys::meta_compactor_lease_key(actor_id));
	Ok(())
}

fn expires_at_ms(now_ms: i64, ttl_ms: u64) -> Result<i64> {
	let ttl_ms = i64::try_from(ttl_ms).context("sqlite compactor lease ttl overflowed i64")?;
	now_ms
		.checked_add(ttl_ms)
		.context("sqlite compactor lease expiration overflowed i64")
}
