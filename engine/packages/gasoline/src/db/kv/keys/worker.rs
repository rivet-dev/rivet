use std::result::Result::Ok;

use anyhow::*;
use rivet_util::Id;
use universaldb::prelude::*;

#[derive(Debug)]
pub struct LastPingTsKey {
	worker_id: Id,
}

impl LastPingTsKey {
	pub fn new(worker_id: Id) -> Self {
		LastPingTsKey { worker_id }
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
		let t = (WORKER, DATA, self.worker_id, LAST_PING_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastPingTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, worker_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = LastPingTsKey { worker_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ActiveWorkerIdxKey {
	last_ping_ts: i64,
	pub worker_id: Id,
}

impl ActiveWorkerIdxKey {
	pub fn new(last_ping_ts: i64, worker_id: Id) -> Self {
		ActiveWorkerIdxKey {
			last_ping_ts,
			worker_id,
		}
	}

	pub fn subspace(last_ping_ts: i64) -> ActiveWorkerIdxSubspaceKey {
		ActiveWorkerIdxSubspaceKey::new(last_ping_ts)
	}

	pub fn entire_subspace() -> ActiveWorkerIdxSubspaceKey {
		ActiveWorkerIdxSubspaceKey::entire()
	}
}

impl FormalKey for ActiveWorkerIdxKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for ActiveWorkerIdxKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKER, ACTIVE, self.last_ping_ts, self.worker_id);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActiveWorkerIdxKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, last_ping_ts, worker_id)) =
			<(usize, usize, i64, Id)>::unpack(input, tuple_depth)?;
		let v = ActiveWorkerIdxKey {
			last_ping_ts,
			worker_id,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ActiveWorkerIdxSubspaceKey {
	last_ping_ts: Option<i64>,
}

impl ActiveWorkerIdxSubspaceKey {
	pub fn new(last_ping_ts: i64) -> Self {
		ActiveWorkerIdxSubspaceKey {
			last_ping_ts: Some(last_ping_ts),
		}
	}

	pub fn entire() -> Self {
		ActiveWorkerIdxSubspaceKey { last_ping_ts: None }
	}
}

impl TuplePack for ActiveWorkerIdxSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (WORKER, ACTIVE);
		offset += t.pack(w, tuple_depth)?;

		if let Some(last_ping_ts) = self.last_ping_ts {
			offset += last_ping_ts.pack(w, tuple_depth)?;
		}

		Ok(offset)
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
		let t = (WORKER, METRICS_LOCK);
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
