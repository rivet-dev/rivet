use std::result::Result::Ok;

use anyhow::*;
use chirp_workflow::prelude::*;
use fdb_util::prelude::*;

#[derive(Debug)]
pub struct RemainingMemoryKey {
	client_id: Uuid,
}

impl RemainingMemoryKey {
	pub fn new(client_id: Uuid) -> Self {
		RemainingMemoryKey { client_id }
	}
}

impl FormalKey for RemainingMemoryKey {
	/// MiB.
	type Value = u64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for RemainingMemoryKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (CLIENT, DATA, self.client_id, REMAINING_MEMORY);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RemainingMemoryKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, client_id, _)) =
			<(usize, usize, Uuid, usize)>::unpack(input, tuple_depth)?;
		let v = RemainingMemoryKey { client_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct RemainingCpuKey {
	client_id: Uuid,
}

impl RemainingCpuKey {
	pub fn new(client_id: Uuid) -> Self {
		RemainingCpuKey { client_id }
	}
}

impl FormalKey for RemainingCpuKey {
	/// Millicores.
	type Value = u64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for RemainingCpuKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (CLIENT, DATA, self.client_id, REMAINING_CPU);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RemainingCpuKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, client_id, _)) =
			<(usize, usize, Uuid, usize)>::unpack(input, tuple_depth)?;
		let v = RemainingCpuKey { client_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct LastPingTsKey {
	client_id: Uuid,
}

impl LastPingTsKey {
	pub fn new(client_id: Uuid) -> Self {
		LastPingTsKey { client_id }
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
		let t = (CLIENT, DATA, self.client_id, LAST_PING_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastPingTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, client_id, _)) =
			<(usize, usize, Uuid, usize)>::unpack(input, tuple_depth)?;
		let v = LastPingTsKey { client_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct TotalMemoryKey {
	client_id: Uuid,
}

impl TotalMemoryKey {
	pub fn new(client_id: Uuid) -> Self {
		TotalMemoryKey { client_id }
	}
}

impl FormalKey for TotalMemoryKey {
	/// MiB.
	type Value = u64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for TotalMemoryKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (CLIENT, DATA, self.client_id, TOTAL_MEMORY);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for TotalMemoryKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, client_id, _)) =
			<(usize, usize, Uuid, usize)>::unpack(input, tuple_depth)?;
		let v = TotalMemoryKey { client_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct TotalCpuKey {
	client_id: Uuid,
}

impl TotalCpuKey {
	pub fn new(client_id: Uuid) -> Self {
		TotalCpuKey { client_id }
	}
}

impl FormalKey for TotalCpuKey {
	/// Millicores.
	type Value = u64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for TotalCpuKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (CLIENT, DATA, self.client_id, TOTAL_CPU);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for TotalCpuKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, client_id, _)) =
			<(usize, usize, Uuid, usize)>::unpack(input, tuple_depth)?;
		let v = TotalCpuKey { client_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ActorKey {
	client_id: Uuid,
	pub actor_id: Uuid,
}

impl ActorKey {
	pub fn new(client_id: Uuid, actor_id: Uuid) -> Self {
		ActorKey {
			client_id,
			actor_id,
		}
	}

	pub fn subspace(client_id: Uuid) -> ActorSubspaceKey {
		ActorSubspaceKey::new(client_id)
	}
}

impl FormalKey for ActorKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for ActorKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (CLIENT, ACTOR, self.client_id, self.actor_id);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActorKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, client_id, actor_id)) =
			<(usize, usize, Uuid, Uuid)>::unpack(input, tuple_depth)?;
		let v = ActorKey {
			client_id,
			actor_id,
		};

		Ok((input, v))
	}
}

pub struct ActorSubspaceKey {
	client_id: Uuid,
}

impl ActorSubspaceKey {
	fn new(client_id: Uuid) -> Self {
		ActorSubspaceKey { client_id }
	}
}

impl TuplePack for ActorSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (CLIENT, ACTOR, self.client_id);
		t.pack(w, tuple_depth)
	}
}

#[derive(Debug)]
pub struct WorkflowIdKey {
	client_id: Uuid,
}

impl WorkflowIdKey {
	pub fn new(client_id: Uuid) -> Self {
		WorkflowIdKey { client_id }
	}
}

impl FormalKey for WorkflowIdKey {
	type Value = Uuid;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Uuid::from_slice(raw)?)
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
		let t = (CLIENT, DATA, self.client_id, WORKFLOW_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WorkflowIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, client_id, _)) =
			<(usize, usize, Uuid, usize)>::unpack(input, tuple_depth)?;

		let v = WorkflowIdKey { client_id };

		Ok((input, v))
	}
}
