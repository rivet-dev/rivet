use anyhow::{Context, Result};

use crate::conveyer::types::{DatabaseBranchId, RestorePointId};

pub(super) struct RestorePointCreateResult {
	pub(super) restore_point: RestorePointId,
}

#[derive(Clone)]
pub(super) struct ResolvedRestorePointPin {
	pub(super) restore_point: RestorePointId,
	pub(super) database_branch_id: DatabaseBranchId,
	pub(super) versionstamp: [u8; 16],
	pub(super) created_at_ms: i64,
}

pub(super) fn decode_i64_counter(bytes: &[u8]) -> Result<i64> {
	let bytes: [u8; std::mem::size_of::<i64>()] = bytes
		.try_into()
		.context("sqlite counter should be exactly 8 bytes")?;

	Ok(i64::from_le_bytes(bytes))
}
