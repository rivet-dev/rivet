use std::result::Result::Ok;

use anyhow::*;
use chirp_workflow::prelude::*;
use fdb_util::prelude::*;

use crate::types::GameGuardProtocol;

#[derive(Debug)]
pub struct IngressKey {
	protocol: GameGuardProtocol,
	pub port: u16,
	pub server_id: Uuid,
}

impl IngressKey {
	pub fn new(protocol: GameGuardProtocol, port: u16, server_id: Uuid) -> Self {
		IngressKey {
			protocol,
			port,
			server_id,
		}
	}

	pub fn subspace(protocol: GameGuardProtocol, port: u16) -> IngressSubspaceKey {
		IngressSubspaceKey::new(protocol, port)
	}
}

impl FormalKey for IngressKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for IngressKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			"port",
			"ingress",
			self.protocol as usize,
			self.port,
			self.server_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for IngressKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, protocol_variant, port, server_id)) =
			<(Cow<str>, Cow<str>, usize, u16, Uuid)>::unpack(input, tuple_depth)?;
		let protocol = GameGuardProtocol::from_repr(protocol_variant).ok_or_else(|| {
			PackError::Message(
				format!("invalid game guard protocol variant `{protocol_variant}` in key").into(),
			)
		})?;
		let v = IngressKey {
			protocol,
			port,
			server_id,
		};

		Ok((input, v))
	}
}

pub struct IngressSubspaceKey {
	protocol: GameGuardProtocol,
	port: u16,
}

impl IngressSubspaceKey {
	pub fn new(protocol: GameGuardProtocol, port: u16) -> Self {
		IngressSubspaceKey { protocol, port }
	}
}

impl TuplePack for IngressSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("port", "ingress", self.protocol as usize, self.port);
		t.pack(w, tuple_depth)
	}
}
