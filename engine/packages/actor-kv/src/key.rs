use rivet_runner_protocol::mk2 as rp;
use universaldb::tuple::{
	Bytes, PackResult, TupleDepth, TuplePack, TupleUnpack, VersionstampOffset,
};

/// Wraps a key with a trailing NIL byte for exact key matching.
///
/// Encodes as: `[NESTED, ...bytes..., NIL]`
///
/// Use this for:
/// - Storing keys
/// - Getting/deleting specific keys
/// - Range query end points (to create closed boundaries)
#[derive(Debug, Clone, PartialEq)]
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

		w.write_all(&[universaldb::utils::codes::NESTED])?;
		offset += 1;

		offset += self.0.pack(w, tuple_depth.increment())?;

		w.write_all(&[universaldb::utils::codes::NIL])?;
		offset += 1;

		Ok(offset)
	}
}

impl<'de> TupleUnpack<'de> for KeyWrapper {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let input = universaldb::utils::parse_code(input, universaldb::utils::codes::NESTED)?;

		let (input, inner) = Bytes::unpack(input, tuple_depth.increment())?;

		let input = universaldb::utils::parse_code(input, universaldb::utils::codes::NIL)?;

		Ok((input, KeyWrapper(inner.into_owned())))
	}
}

/// Wraps a key without a trailing NIL byte for prefix/range matching.
///
/// Encodes as: `[NESTED, ...bytes...]` (no trailing NIL)
///
/// Use this for:
/// - Range query start points (to create open boundaries)
/// - Prefix queries (to match all keys starting with these bytes)
pub struct ListKeyWrapper(pub rp::KvKey);

impl TuplePack for ListKeyWrapper {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		w.write_all(&[universaldb::utils::codes::NESTED])?;
		offset += 1;

		offset += self.0.pack(w, tuple_depth.increment())?;

		// No ending NIL byte compared to `KeyWrapper::pack`

		Ok(offset)
	}
}
