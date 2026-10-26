use anyhow::Result;
use universaldb::prelude::*;

#[derive(Debug)]
pub struct EngineVersionKey {}

impl EngineVersionKey {
	pub fn new() -> Self {
		EngineVersionKey {}
	}
}

impl FormalKey for EngineVersionKey {
	type Value = semver::Version;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		semver::Version::parse(str::from_utf8(raw)?).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_string().into_bytes())
	}
}

impl TuplePack for EngineVersionKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RIVET, VERSION, ENGINE);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for EngineVersionKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, data)) = <(usize, usize, usize)>::unpack(input, tuple_depth)?;
		if data != ENGINE {
			return Err(PackError::Message("expected ENGINE data".into()));
		}

		let v = EngineVersionKey {};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ProtocolVersionKey {
	pub protocol_kind: rivet_config::RuntimeProtocolKind,
	pub version: u16,
}

impl ProtocolVersionKey {
	pub fn new(protocol_kind: rivet_config::RuntimeProtocolKind, version: u16) -> Self {
		ProtocolVersionKey {
			protocol_kind,
			version,
		}
	}

	pub fn subspace(
		protocol_kind: rivet_config::RuntimeProtocolKind,
	) -> ProtocolVersionSubspaceKey {
		ProtocolVersionSubspaceKey::new(protocol_kind)
	}
}

impl FormalKey for ProtocolVersionKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_le_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_le_bytes().to_vec())
	}
}

impl TuplePack for ProtocolVersionKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			RIVET,
			VERSION,
			PROTOCOL,
			self.protocol_kind as usize,
			self.version,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ProtocolVersionKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, data, protocol_kind, version)) =
			<(usize, usize, usize, usize, u16)>::unpack(input, tuple_depth)?;
		if data != PROTOCOL {
			return Err(PackError::Message("expected PROTOCOL data".into()));
		}
		let protocol_kind = rivet_config::RuntimeProtocolKind::from_repr(protocol_kind)
			.ok_or_else(|| {
				PackError::Message(format!("invalid protocol kind `{protocol_kind}` in key").into())
			})?;

		let v = ProtocolVersionKey {
			protocol_kind,
			version,
		};

		Ok((input, v))
	}
}

pub struct ProtocolVersionSubspaceKey {
	protocol_kind: rivet_config::RuntimeProtocolKind,
}

impl ProtocolVersionSubspaceKey {
	pub fn new(protocol_kind: rivet_config::RuntimeProtocolKind) -> Self {
		ProtocolVersionSubspaceKey { protocol_kind }
	}
}

impl TuplePack for ProtocolVersionSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RIVET, VERSION, PROTOCOL, self.protocol_kind as usize);
		t.pack(w, tuple_depth)
	}
}


