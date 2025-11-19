use anyhow::Result;
use universaldb::prelude::*;

use crate::tunnel::id::{GatewayId, RequestId};

#[derive(Debug)]
pub struct LastPingTsKey {
	gateway_id: GatewayId,
	request_id: RequestId,
}

impl LastPingTsKey {
	pub fn new(gateway_id: GatewayId, request_id: RequestId) -> Self {
		LastPingTsKey {
			gateway_id,
			request_id,
		}
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
		let t = (
			HIBERNATING_REQUEST,
			DATA,
			&self.gateway_id[..],
			&self.request_id[..],
			LAST_PING_TS,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastPingTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, gateway_id_bytes, request_id_bytes, _)) =
			<(usize, usize, Vec<u8>, Vec<u8>, usize)>::unpack(input, tuple_depth)?;

		let gateway_id: GatewayId = gateway_id_bytes
			.as_slice()
			.try_into()
			.expect("invalid gateway_id length");

		let request_id: RequestId = request_id_bytes
			.as_slice()
			.try_into()
			.expect("invalid request_id length");

		let v = LastPingTsKey {
			gateway_id,
			request_id,
		};

		Ok((input, v))
	}
}
