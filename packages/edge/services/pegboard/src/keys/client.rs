use std::result::Result::Ok;

use anyhow::*;
use chirp_workflow::prelude::*;
use fdb_util::prelude::*;

#[derive(Debug)]
pub struct RemainingMemKey {
	client_id: Uuid,
}

impl RemainingMemKey {
	pub fn new(client_id: Uuid) -> Self {
		RemainingMemKey { client_id }
	}
}

impl FormalKey for RemainingMemKey {
	/// MiB.
	type Value = u64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for RemainingMemKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("client", "data", self.client_id, "remaining_mem");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RemainingMemKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, client_id, _)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		let v = RemainingMemKey { client_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct LastPingTsKey {
	client_id: Uuid,
}

impl LastPingTsKey {
	pub fn new(client_id: Uuid) -> Self {
		LastPingTsKey { client_id }
	}
}

impl FormalKey for LastPingTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for LastPingTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("client", "data", self.client_id, "last_ping_ts");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastPingTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, client_id, _)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		let v = LastPingTsKey { client_id };

		Ok((input, v))
	}
}
