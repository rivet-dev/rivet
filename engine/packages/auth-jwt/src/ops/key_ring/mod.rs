pub mod get_latest;

use anyhow::{Context, Result, bail};
use epoxy_protocol::protocol::CommittedValue;
use universaldb::prelude::FormalKey;

use crate::{SigningKeyRing, storage_keys::SigningKeyRingKey};

pub mod compare_and_set;

#[derive(Clone)]
pub struct Snapshot {
	pub encoded: Vec<u8>,
	pub ring: SigningKeyRing,
}

impl std::fmt::Debug for Snapshot {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Snapshot")
			.field("encoded", &"<redacted>")
			.field("ring", &self.ring)
			.finish()
	}
}

#[derive(Debug, Clone)]
pub enum ReadOutput {
	Absent,
	Present(Box<Snapshot>),
	Corrupt { reason: String },
}

impl ReadOutput {
	pub fn optional(self) -> Result<Option<Snapshot>> {
		match self {
			Self::Absent => Ok(None),
			Self::Present(snapshot) => Ok(Some(*snapshot)),
			Self::Corrupt { reason } => bail!("corrupt authoritative JWT key ring: {reason}"),
		}
	}

	pub fn require(self) -> Result<Snapshot> {
		self.optional()?.context("JWT key ring is not initialized")
	}
}

pub(crate) fn decode_committed(
	logical_key: &SigningKeyRingKey,
	committed: CommittedValue,
) -> Result<Snapshot> {
	let encoded = committed
		.value
		.context("JWT key-ring record is a tombstone")?;
	if !committed.mutable {
		bail!("JWT key-ring record must be mutable");
	}
	let ring = logical_key
		.deserialize(&encoded)
		.context("invalid authoritative JWT signing key ring")?;
	Ok(Snapshot { encoded, ring })
}
