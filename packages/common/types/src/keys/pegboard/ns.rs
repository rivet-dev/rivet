use std::result::Result::Ok;

use anyhow::*;
use gas::prelude::*;
use udb_util::prelude::*;

#[derive(Debug)]
pub struct OutboundDesiredSlotsKey {
	pub namespace_id: Id,
	pub runner_name_selector: String,
}

impl OutboundDesiredSlotsKey {
	pub fn new(namespace_id: Id, runner_name_selector: String) -> Self {
		OutboundDesiredSlotsKey {
			namespace_id,
			runner_name_selector,
		}
	}

	pub fn subspace(namespace_id: Id) -> OutboundDesiredSlotsSubspaceKey {
		OutboundDesiredSlotsSubspaceKey::new(namespace_id)
	}

	pub fn entire_subspace() -> OutboundDesiredSlotsSubspaceKey {
		OutboundDesiredSlotsSubspaceKey::entire()
	}
}

impl FormalKey for OutboundDesiredSlotsKey {
	/// Count.
	type Value = u32;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		// NOTE: Atomic ops use little endian
		Ok(u32::from_le_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		// NOTE: Atomic ops use little endian
		Ok(value.to_le_bytes().to_vec())
	}
}

impl TuplePack for OutboundDesiredSlotsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			OUTBOUND,
			DESIRED_SLOTS,
			self.namespace_id,
			&self.runner_name_selector,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for OutboundDesiredSlotsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, runner_name_selector)) =
			<(usize, usize, usize, Id, String)>::unpack(input, tuple_depth)?;

		let v = OutboundDesiredSlotsKey {
			namespace_id,
			runner_name_selector,
		};

		Ok((input, v))
	}
}

pub struct OutboundDesiredSlotsSubspaceKey {
	namespace_id: Option<Id>,
}

impl OutboundDesiredSlotsSubspaceKey {
	pub fn new(namespace_id: Id) -> Self {
		OutboundDesiredSlotsSubspaceKey {
			namespace_id: Some(namespace_id),
		}
	}

	pub fn entire() -> Self {
		OutboundDesiredSlotsSubspaceKey { namespace_id: None }
	}
}

impl TuplePack for OutboundDesiredSlotsSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (NAMESPACE, OUTBOUND, DESIRED_SLOTS);
		offset += t.pack(w, tuple_depth)?;

		if let Some(namespace_id) = self.namespace_id {
			offset += namespace_id.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}
