use anyhow::Result;
use universaldb::prelude::*;

pub fn subspace() -> universaldb::utils::Subspace {
	universaldb::utils::Subspace::new(&(RIVET, DATACENTER))
}

#[derive(Debug)]
pub struct LastPingTsKey {
	dc_label: u16,
}

impl LastPingTsKey {
	pub fn new(dc_label: u16) -> Self {
		LastPingTsKey { dc_label }
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
		let t = (DATA, self.dc_label, LAST_PING_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastPingTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, dc_label, _)) = <(usize, u16, usize)>::unpack(input, tuple_depth)?;
		let v = LastPingTsKey { dc_label };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct LastRttKey {
	dc_label: u16,
}

impl LastRttKey {
	pub fn new(dc_label: u16) -> Self {
		LastRttKey { dc_label }
	}
}

impl FormalKey for LastRttKey {
	// Milliseconds.
	type Value = u32;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u32::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for LastRttKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (DATA, self.dc_label, LAST_RTT);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastRttKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, dc_label, _)) = <(usize, u16, usize)>::unpack(input, tuple_depth)?;
		let v = LastRttKey { dc_label };

		Ok((input, v))
	}
}
