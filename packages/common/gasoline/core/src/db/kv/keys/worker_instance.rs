use std::result::Result::Ok;

use anyhow::*;
use rivet_util::Id;
use udb_util::prelude::*;

#[derive(Debug)]
pub struct LastPingTsKey {
	worker_instance_id: Id,
}

impl LastPingTsKey {
	pub fn new(worker_instance_id: Id) -> Self {
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
		let t = (WORKER_INSTANCE, DATA, self.worker_instance_id, LAST_PING_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastPingTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, worker_instance_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
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
		let t = (WORKER_INSTANCE, METRICS_LOCK);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for MetricsLockKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _)) = <(usize, usize)>::unpack(input, tuple_depth)?;
		let v = MetricsLockKey {};

		Ok((input, v))
	}
}
