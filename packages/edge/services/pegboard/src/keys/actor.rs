use std::result::Result::Ok;

use anyhow::*;
use chirp_workflow::prelude::*;
use fdb_util::prelude::*;

use crate::types::GameGuardProtocol;

#[derive(Debug)]
pub struct CreateTsKey {
	actor_id: Uuid,
}

impl CreateTsKey {
	pub fn new(actor_id: Uuid) -> Self {
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
		let (input, (_, _, actor_id, _)) =
			<(usize, usize, Uuid, usize)>::unpack(input, tuple_depth)?;
		let v = CreateTsKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct WorkflowIdKey {
	actor_id: Uuid,
}

impl WorkflowIdKey {
	pub fn new(actor_id: Uuid) -> Self {
		WorkflowIdKey { actor_id }
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
		let t = (ACTOR, DATA, self.actor_id, WORKFLOW_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WorkflowIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) =
			<(usize, usize, Uuid, usize)>::unpack(input, tuple_depth)?;

		let v = WorkflowIdKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ProxiedPortsKey {
	pub actor_id: Uuid,
}

impl ProxiedPortsKey {
	pub fn new(actor_id: Uuid) -> Self {
		ProxiedPortsKey { actor_id }
	}

	pub fn subspace() -> ProxiedPortsSubspaceKey {
		ProxiedPortsSubspaceKey::new()
	}
}

impl FormalKey for ProxiedPortsKey {
	type Value = Vec<ProxiedPort>;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		serde_json::from_slice(raw).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		serde_json::to_vec(&value).map_err(Into::into)
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxiedPort {
	pub port_name: String,
	pub create_ts: i64,
	pub lan_hostname: String,
	pub source: u16,
	pub ingress_port_number: u16,
	pub protocol: GameGuardProtocol,
}

impl TuplePack for ProxiedPortsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, PORT, PROXIED, self.actor_id);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ProxiedPortsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, _, actor_id)) =
			<(usize, usize, usize, usize, Uuid)>::unpack(input, tuple_depth)?;
		let v = ProxiedPortsKey { actor_id };

		Ok((input, v))
	}
}

pub struct ProxiedPortsSubspaceKey {}

impl ProxiedPortsSubspaceKey {
	pub fn new() -> Self {
		ProxiedPortsSubspaceKey {}
	}
}

impl TuplePack for ProxiedPortsSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, PORT, PROXIED);
		t.pack(w, tuple_depth)
	}
}
