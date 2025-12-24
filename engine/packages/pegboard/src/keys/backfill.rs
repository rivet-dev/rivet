//! Keys for tracking backfill completion status.

use anyhow::Result;
use universaldb::prelude::*;

/// Key to mark a backfill as complete. The value is the completion timestamp.
#[derive(Debug)]
pub struct CompleteKey {
	pub name: String,
}

impl CompleteKey {
	pub fn new(name: impl Into<String>) -> Self {
		CompleteKey { name: name.into() }
	}
}

impl FormalKey for CompleteKey {
	/// Completion timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for CompleteKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (BACKFILL, COMPLETE, &self.name);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for CompleteKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, name)) = <(usize, usize, String)>::unpack(input, tuple_depth)?;
		let v = CompleteKey { name };

		Ok((input, v))
	}
}
