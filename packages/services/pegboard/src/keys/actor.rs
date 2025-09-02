use std::result::Result::Ok;

use anyhow::*;
use gas::prelude::*;
use udb_util::prelude::*;

#[derive(Debug)]
pub struct CreateTsKey {
	actor_id: Id,
}

impl CreateTsKey {
	pub fn new(actor_id: Id) -> Self {
		CreateTsKey { actor_id }
	}
}

impl FormalKey for CreateTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for CreateTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, CREATE_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for CreateTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = CreateTsKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct WorkflowIdKey {
	actor_id: Id,
}

impl WorkflowIdKey {
	pub fn new(actor_id: Id) -> Self {
		WorkflowIdKey { actor_id }
	}
}

impl FormalKey for WorkflowIdKey {
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for WorkflowIdKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, WORKFLOW_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WorkflowIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = WorkflowIdKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct RunnerIdKey {
	actor_id: Id,
}

impl RunnerIdKey {
	pub fn new(actor_id: Id) -> Self {
		RunnerIdKey { actor_id }
	}
}

impl FormalKey for RunnerIdKey {
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes())
	}
}

impl TuplePack for RunnerIdKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, RUNNER_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RunnerIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = RunnerIdKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ConnectableKey {
	actor_id: Id,
}

impl ConnectableKey {
	pub fn new(actor_id: Id) -> Self {
		ConnectableKey { actor_id }
	}
}

impl FormalKey for ConnectableKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for ConnectableKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, CONNECTABLE);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ConnectableKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = ConnectableKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct SleepTsKey {
	actor_id: Id,
}

impl SleepTsKey {
	pub fn new(actor_id: Id) -> Self {
		SleepTsKey { actor_id }
	}
}

impl FormalKey for SleepTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for SleepTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, SLEEP_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for SleepTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = SleepTsKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct DestroyTsKey {
	actor_id: Id,
}

impl DestroyTsKey {
	pub fn new(actor_id: Id) -> Self {
		DestroyTsKey { actor_id }
	}
}

impl FormalKey for DestroyTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for DestroyTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, DESTROY_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for DestroyTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = DestroyTsKey { actor_id };

		Ok((input, v))
	}
}
