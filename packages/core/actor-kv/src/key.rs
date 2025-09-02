use rivet_runner_protocol as rp;
use universaldb::tuple::{
	Bytes, PackResult, TupleDepth, TuplePack, TupleUnpack, VersionstampOffset,
};

#[derive(Clone, PartialEq)]
pub struct KeyWrapper(pub rp::KvKey);

impl KeyWrapper {
	pub fn tuple_len(key: &rp::KvKey) -> usize {
		key.len() + 2
	}
}

impl TuplePack for KeyWrapper {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		w.write_all(&[udb_util::codes::NESTED])?;
		offset += 1;

		offset += self.0.pack(w, tuple_depth.increment())?;

		w.write_all(&[udb_util::codes::NIL])?;
		offset += 1;

		Ok(offset)
	}
}

impl<'de> TupleUnpack<'de> for KeyWrapper {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let input = udb_util::parse_code(input, udb_util::codes::NESTED)?;

		let (input, inner) = Bytes::unpack(input, tuple_depth.increment())?;

		let input = udb_util::parse_code(input, udb_util::codes::NIL)?;

		Ok((input, KeyWrapper(inner.into_owned())))
	}
}

/// Same as Key: except when packing, it leaves off the NIL byte to allow for an open range.
pub struct ListKeyWrapper(pub rp::KvKey);

impl TuplePack for ListKeyWrapper {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		w.write_all(&[udb_util::codes::NESTED])?;
		offset += 1;

		offset += self.0.pack(w, tuple_depth.increment())?;

		// No ending NIL byte compared to `KeyWrapper::pack`

		Ok(offset)
	}
}
