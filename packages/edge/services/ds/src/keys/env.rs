use std::result::Result::Ok;

use anyhow::*;
use chirp_workflow::prelude::*;
use fdb_util::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct ServerKey {
	environment_id: Uuid,
	create_ts: i64,
	pub server_id: Uuid,
}

impl ServerKey {
	pub fn new(environment_id: Uuid, create_ts: i64, server_id: Uuid) -> Self {
		ServerKey {
			environment_id,
			create_ts,
			server_id,
		}
	}

	pub fn subspace(environment_id: Uuid) -> ServerSubspaceKey {
		ServerSubspaceKey::new(environment_id)
	}
}

impl FormalKey for ServerKey {
	type Value = ServerKeyData;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		serde_json::from_slice(raw).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		serde_json::to_vec(&value).map_err(Into::into)
	}
}

impl TuplePack for ServerKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			"env",
			self.environment_id,
			"server",
			self.create_ts,
			self.server_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ServerKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, environment_id, _, create_ts, server_id)) =
			<(Cow<str>, Uuid, Cow<str>, i64, Uuid)>::unpack(input, tuple_depth)?;
		let v = ServerKey {
			environment_id,
			create_ts,
			server_id,
		};

		Ok((input, v))
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerKeyData {
	pub is_destroyed: bool,
	pub tags: Vec<(String, String)>,
}

pub struct ServerSubspaceKey {
	environment_id: Uuid,
}

impl ServerSubspaceKey {
	pub fn new(environment_id: Uuid) -> Self {
		ServerSubspaceKey { environment_id }
	}
}

impl TuplePack for ServerSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("env", self.environment_id, "server");
		t.pack(w, tuple_depth)
	}
}
