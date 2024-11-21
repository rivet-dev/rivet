use deno_core::JsBuffer;
use foundationdb::tuple::{
	Bytes, PackError, PackResult, TupleDepth, TuplePack, TupleUnpack, VersionstampOffset,
};
use serde::Deserialize;

// TODO: Custom deser impl that uses arrays instead of objects?
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Key {
	/// Contains references to v8-owned buffers. Requires no copies.
	JsInKey(Vec<JsBuffer>),
	/// Cant use `ToJsBuffer` because of its API, so it gets converted to ToJsBuffer in the KV ext.
	JsOutKey(Vec<Vec<u8>>),
}

impl std::fmt::Debug for Key {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Key({})", self.len())
	}
}

impl PartialEq for Key {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Key::JsInKey(a), Key::JsInKey(b)) => a
				.iter()
				.map(|x| x.as_ref())
				.eq(b.iter().map(|x| x.as_ref())),
			(Key::JsOutKey(a), Key::JsOutKey(b)) => a == b,
			(Key::JsInKey(a), Key::JsOutKey(b)) => a.iter().map(|x| x.as_ref()).eq(b.iter()),
			(Key::JsOutKey(a), Key::JsInKey(b)) => a.iter().eq(b.iter().map(|x| x.as_ref())),
		}
	}
}

impl Eq for Key {}

impl std::hash::Hash for Key {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		match self {
			Key::JsInKey(js_in_key) => {
				for buffer in js_in_key {
					state.write(buffer.as_ref());
				}
			}
			Key::JsOutKey(out_key) => {
				for buffer in out_key {
					state.write(buffer);
				}
			}
		}
	}
}

impl Key {
	pub fn len(&self) -> usize {
		match self {
			Key::JsInKey(js_in_key) => {
				// Arbitrary 4 accounting for nesting overhead
				js_in_key.iter().fold(0, |acc, x| acc + x.len()) + 4 * js_in_key.len()
			}
			Key::JsOutKey(out_key) => {
				// Arbitrary 4 accounting for nesting overhead
				out_key.iter().fold(0, |acc, x| acc + x.len()) + 4 * out_key.len()
			}
		}
	}
}

impl TuplePack for Key {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		match self {
			Key::JsInKey(tuple) => {
				let mut offset = VersionstampOffset::None { size: 0 };

				w.write_all(&[NESTED])?;
				offset += 1;

				for v in tuple.iter() {
					offset += v.as_ref().pack(w, tuple_depth.increment())?;
				}

				w.write_all(&[NIL])?;
				offset += 1;

				Ok(offset)
			}
			Key::JsOutKey(_) => unreachable!("should not be packing out keys"),
		}
	}
}

impl<'de> TupleUnpack<'de> for Key {
	fn unpack(mut input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		input = parse_code(input, NESTED)?;

		let mut vec = Vec::new();
		while !is_end_of_tuple(input, true) {
			let (rem, v) = Bytes::unpack(input, tuple_depth.increment())?;
			input = rem;
			vec.push(v.into_owned());
		}

		input = parse_code(input, NIL)?;

		Ok((input, Key::JsOutKey(vec)))
	}
}

/// Same as Key::JsInKey except when packing, it leaves off the NIL byte to allow for an open range.
#[derive(Deserialize)]
pub struct ListKey(Vec<JsBuffer>);

impl TuplePack for ListKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		w.write_all(&[NESTED])?;
		offset += 1;

		for v in self.0.iter() {
			offset += v.as_ref().pack(w, tuple_depth.increment())?;
		}

		// No ending NIL byte compared to `Key::pack`

		Ok(offset)
	}
}

impl ListKey {
	pub fn len(&self) -> usize {
		// Arbitrary 4 accounting for nesting overhead
		self.0.iter().fold(0, |acc, x| acc + x.len()) + 4 * self.0.len()
	}
}

// === Copied from foundationdbrs ===
const NIL: u8 = 0x00;
const NESTED: u8 = 0x05;
const ESCAPE: u8 = 0xff;

#[inline]
fn parse_byte(input: &[u8]) -> PackResult<(&[u8], u8)> {
	if input.is_empty() {
		Err(PackError::MissingBytes)
	} else {
		Ok((&input[1..], input[0]))
	}
}

fn parse_code(input: &[u8], expected: u8) -> PackResult<&[u8]> {
	let (input, found) = parse_byte(input)?;
	if found == expected {
		Ok(input)
	} else {
		Err(PackError::BadCode {
			found,
			expected: Some(expected),
		})
	}
}

fn is_end_of_tuple(input: &[u8], nested: bool) -> bool {
	match input.first() {
		None => true,
		_ if !nested => false,
		Some(&NIL) => Some(&ESCAPE) != input.get(1),
		_ => false,
	}
}
