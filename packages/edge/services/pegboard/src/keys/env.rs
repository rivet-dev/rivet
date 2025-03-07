use std::result::Result::Ok;

use anyhow::*;
use chirp_workflow::prelude::*;
use fdb_util::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct ActorKey {
	environment_id: Uuid,
	pub create_ts: i64,
	pub actor_id: Uuid,
}

impl ActorKey {
	pub fn new(environment_id: Uuid, create_ts: i64, actor_id: Uuid) -> Self {
		ActorKey {
			environment_id,
			create_ts,
			actor_id,
		}
	}

	pub fn subspace(environment_id: Uuid) -> ActorSubspaceKey {
		ActorSubspaceKey::new(environment_id)
	}
}

impl FormalKey for ActorKey {
	type Value = ActorKeyData;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		serde_json::from_slice(raw).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		serde_json::to_vec(&value).map_err(Into::into)
	}
}

impl TuplePack for ActorKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			ENV,
			self.environment_id,
			ACTOR,
			self.create_ts,
			self.actor_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActorKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, environment_id, _, create_ts, actor_id)) =
			<(usize, Uuid, usize, i64, Uuid)>::unpack(input, tuple_depth)?;
		let v = ActorKey {
			environment_id,
			create_ts,
			actor_id,
		};

		Ok((input, v))
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActorKeyData {
	pub is_destroyed: bool,
	pub tags: Vec<(String, String)>,
}

pub struct ActorSubspaceKey {
	environment_id: Uuid,
}

impl ActorSubspaceKey {
	pub fn new(environment_id: Uuid) -> Self {
		ActorSubspaceKey { environment_id }
	}
}

impl TuplePack for ActorSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ENV, self.environment_id, ACTOR);
		t.pack(w, tuple_depth)
	}
}
