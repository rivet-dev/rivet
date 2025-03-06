use std::result::Result::Ok;

use anyhow::*;
use chirp_workflow::prelude::*;
use fdb_util::prelude::*;

use crate::protocol;

#[derive(Debug)]
pub struct ClientsByRemainingMemKey {
	pub flavor: protocol::ClientFlavor,
	/// MiB.
	pub remaining_mem: u64,
	pub last_ping_ts: i64,
	pub client_id: Uuid,
}

impl ClientsByRemainingMemKey {
	pub fn new(
		flavor: protocol::ClientFlavor,
		remaining_mem: u64,
		last_ping_ts: i64,
		client_id: Uuid,
	) -> Self {
		ClientsByRemainingMemKey {
			flavor,
			last_ping_ts,
			remaining_mem,
			client_id,
		}
	}

	pub fn subspace(flavor: protocol::ClientFlavor) -> ClientsByRemainingMemSubspaceKey {
		ClientsByRemainingMemSubspaceKey::new(flavor)
	}

	pub fn subspace_with_mem(
		flavor: protocol::ClientFlavor,
		remaining_mem: u64,
	) -> ClientsByRemainingMemSubspaceKey {
		ClientsByRemainingMemSubspaceKey::new_with_mem(flavor, remaining_mem)
	}

	pub fn entire_subspace() -> ClientsByRemainingMemSubspaceKey {
		ClientsByRemainingMemSubspaceKey::entire()
	}
}

impl FormalKey for ClientsByRemainingMemKey {
	/// Client workflow id.
	type Value = Uuid;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Uuid::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for ClientsByRemainingMemKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			"datacenter",
			"clients_by_remaining_mem",
			self.flavor as usize,
			self.remaining_mem,
			self.last_ping_ts,
			self.client_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ClientsByRemainingMemKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, flavor, remaining_mem, last_ping_ts, client_id)) =
			<(Cow<str>, Cow<str>, usize, u64, i64, Uuid)>::unpack(input, tuple_depth)?;
		let flavor = protocol::ClientFlavor::from_repr(flavor).ok_or_else(|| {
			PackError::Message(format!("invalid flavor `{flavor}` in key").into())
		})?;

		let v = ClientsByRemainingMemKey {
			flavor,
			remaining_mem,
			last_ping_ts,
			client_id,
		};

		Ok((input, v))
	}
}

pub struct ClientsByRemainingMemSubspaceKey {
	pub flavor: Option<protocol::ClientFlavor>,
	pub remaining_mem: Option<u64>,
}

impl ClientsByRemainingMemSubspaceKey {
	pub fn new(flavor: protocol::ClientFlavor) -> Self {
		ClientsByRemainingMemSubspaceKey {
			flavor: Some(flavor),
			remaining_mem: None,
		}
	}

	pub fn new_with_mem(flavor: protocol::ClientFlavor, remaining_mem: u64) -> Self {
		ClientsByRemainingMemSubspaceKey {
			flavor: Some(flavor),
			remaining_mem: Some(remaining_mem),
		}
	}

	pub fn entire() -> Self {
		ClientsByRemainingMemSubspaceKey {
			flavor: None,
			remaining_mem: None,
		}
	}
}

impl TuplePack for ClientsByRemainingMemSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = ("datacenter", "clients_by_remaining_mem");
		offset += t.pack(w, tuple_depth)?;

		if let Some(flavor) = &self.flavor {
			offset += (*flavor as usize).pack(w, tuple_depth)?;

			if let Some(remaining_mem) = &self.remaining_mem {
				offset += remaining_mem.pack(w, tuple_depth)?;
			}
		}

		Ok(offset)
	}
}
