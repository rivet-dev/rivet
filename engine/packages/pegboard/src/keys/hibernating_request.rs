use anyhow::Result;
use universaldb::prelude::*;
use uuid::Uuid;

#[derive(Debug)]
pub struct LastPingTsKey {
	request_id: Uuid,
}

impl LastPingTsKey {
	pub fn new(request_id: Uuid) -> Self {
		LastPingTsKey { request_id }
	}
}

impl FormalKey for LastPingTsKey {
	/// Timestamp.
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
		let t = (HIBERNATING_REQUEST, DATA, self.request_id, LAST_PING_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastPingTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, request_id, _)) =
			<(usize, usize, Uuid, usize)>::unpack(input, tuple_depth)?;

		let v = LastPingTsKey { request_id };

		Ok((input, v))
	}
}
