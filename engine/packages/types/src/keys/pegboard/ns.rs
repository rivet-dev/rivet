use std::result::Result::Ok;

use anyhow::*;
use gas::prelude::*;
use universaldb::prelude::*;

#[derive(Debug)]
pub struct ServerlessDesiredSlotsKey {
	pub namespace_id: Id,
	pub runner_name: String,
}

impl ServerlessDesiredSlotsKey {
	pub fn new(namespace_id: Id, runner_name: String) -> Self {
		ServerlessDesiredSlotsKey {
			namespace_id,
			runner_name,
		}
	}

	pub fn subspace(namespace_id: Id) -> ServerlessDesiredSlotsSubspaceKey {
		ServerlessDesiredSlotsSubspaceKey::new(namespace_id)
	}

	pub fn entire_subspace() -> ServerlessDesiredSlotsSubspaceKey {
		ServerlessDesiredSlotsSubspaceKey::entire()
	}
}

impl FormalKey for ServerlessDesiredSlotsKey {
	/// Count.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		// NOTE: Atomic ops use little endian
		Ok(i64::from_le_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		// NOTE: Atomic ops use little endian
		Ok(value.to_le_bytes().to_vec())
	}
}

impl TuplePack for ServerlessDesiredSlotsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			SERVERLESS,
			DESIRED_SLOTS,
			self.namespace_id,
			&self.runner_name,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ServerlessDesiredSlotsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, runner_name)) =
			<(usize, usize, usize, Id, String)>::unpack(input, tuple_depth)?;

		let v = ServerlessDesiredSlotsKey {
			namespace_id,
			runner_name,
		};

		Ok((input, v))
	}
}

pub struct ServerlessDesiredSlotsSubspaceKey {
	namespace_id: Option<Id>,
}

impl ServerlessDesiredSlotsSubspaceKey {
	pub fn new(namespace_id: Id) -> Self {
		ServerlessDesiredSlotsSubspaceKey {
			namespace_id: Some(namespace_id),
		}
	}

	pub fn entire() -> Self {
		ServerlessDesiredSlotsSubspaceKey { namespace_id: None }
	}
}

impl TuplePack for ServerlessDesiredSlotsSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (NAMESPACE, SERVERLESS, DESIRED_SLOTS);
		offset += t.pack(w, tuple_depth)?;

		if let Some(namespace_id) = self.namespace_id {
			offset += namespace_id.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}
