use std::result::Result::Ok;

use anyhow::*;
use gas::prelude::*;
use udb_util::prelude::*;

pub fn subspace() -> udb_util::Subspace {
	udb_util::Subspace::new(&(RIVET, NAMESPACE))
}

#[derive(Debug)]
pub struct NameKey {
	namespace_id: Id,
}

impl NameKey {
	pub fn new(namespace_id: Id) -> Self {
		NameKey { namespace_id }
	}

	pub fn namespace_id(&self) -> Id {
		self.namespace_id
	}
}

impl FormalKey for NameKey {
	type Value = String;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		String::from_utf8(raw.to_vec()).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.into_bytes())
	}
}

impl TuplePack for NameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (DATA, self.namespace_id, NAME);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for NameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _)) = <(usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = NameKey { namespace_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct DisplayNameKey {
	namespace_id: Id,
}

impl DisplayNameKey {
	pub fn new(namespace_id: Id) -> Self {
		DisplayNameKey { namespace_id }
	}
}

impl FormalKey for DisplayNameKey {
	type Value = String;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		String::from_utf8(raw.to_vec()).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.into_bytes())
	}
}

impl TuplePack for DisplayNameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (DATA, self.namespace_id, DISPLAY_NAME);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for DisplayNameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _)) = <(usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = DisplayNameKey { namespace_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct CreateTsKey {
	namespace_id: Id,
}

impl CreateTsKey {
	pub fn new(namespace_id: Id) -> Self {
		CreateTsKey { namespace_id }
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
		let t = (DATA, self.namespace_id, CREATE_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for CreateTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _)) = <(usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = CreateTsKey { namespace_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ByNameKey {
	name: String,
}

impl ByNameKey {
	pub fn new(name: String) -> Self {
		ByNameKey { name }
	}
}

impl FormalKey for ByNameKey {
	/// Namespace id.
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes())
	}
}

impl TuplePack for ByNameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (BY_NAME, &self.name);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ByNameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, name)) = <(usize, String)>::unpack(input, tuple_depth)?;

		let v = ByNameKey { name };

		Ok((input, v))
	}
}
