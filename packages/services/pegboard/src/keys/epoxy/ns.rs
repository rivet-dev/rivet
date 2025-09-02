use std::result::Result::Ok;

use anyhow::*;
use gas::prelude::*;
use udb_util::prelude::*;

#[derive(Debug)]
pub struct ReservationByKeyKey {
	namespace_id: Id,
	name: String,
	key: String,
}

impl ReservationByKeyKey {
	pub fn new(namespace_id: Id, name: String, key: String) -> Self {
		ReservationByKeyKey {
			namespace_id,
			name,
			key,
		}
	}
}

impl FormalKey for ReservationByKeyKey {
	// Reservation ID
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Id::from_slice(raw).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes())
	}
}

impl TuplePack for ReservationByKeyKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			EPOXY,
			NAMESPACE,
			self.namespace_id,
			RESERVATION,
			BY_NAME_AND_KEY,
			&self.name,
			&self.key,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ReservationByKeyKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _, _, name, key, _)) =
			<(usize, Id, usize, usize, String, String, usize)>::unpack(input, tuple_depth)?;
		let v = ReservationByKeyKey {
			namespace_id,
			name,
			key,
		};

		Ok((input, v))
	}
}
