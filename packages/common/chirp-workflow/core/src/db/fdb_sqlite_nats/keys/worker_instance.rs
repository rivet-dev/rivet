use std::{borrow::Cow, result::Result::Ok};

// TODO: Use concrete error types
use anyhow::*;
use foundationdb::tuple::{PackResult, TupleDepth, TuplePack, TupleUnpack, VersionstampOffset};
use uuid::Uuid;

use super::FormalKey;

#[derive(Debug)]
pub struct LastPingTsKey {
	worker_instance_id: Uuid,
}

impl LastPingTsKey {
	pub fn new(worker_instance_id: Uuid) -> Self {
		LastPingTsKey { worker_instance_id }
	}
}

impl FormalKey for LastPingTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for LastPingTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			"worker_instance",
			"data",
			self.worker_instance_id,
			"last_ping_ts",
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastPingTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, worker_instance_id, _)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		let v = LastPingTsKey { worker_instance_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct MetricsLockKey {}

impl MetricsLockKey {
	pub fn new() -> Self {
		MetricsLockKey {}
	}
}

impl FormalKey for MetricsLockKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for MetricsLockKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("worker_instance", "metrics_lock");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for MetricsLockKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _)) = <(Cow<str>, Cow<str>)>::unpack(input, tuple_depth)?;
		let v = MetricsLockKey {};

		Ok((input, v))
	}
}
