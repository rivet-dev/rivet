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
		let t = (RIVET, VERSION);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for EngineVersionKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, data)) = <(usize, usize)>::unpack(input, tuple_depth)?;
		if data != VERSION {
			return Err(PackError::Message("expected VERSION data".into()));
		}

		let v = EngineVersionKey {};

		Ok((input, v))
	}
}
