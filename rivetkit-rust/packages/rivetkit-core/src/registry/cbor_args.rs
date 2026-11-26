//! Reads action args out of cbor client requests as raw CBOR.
//!
//! cbor clients encode args with cbor-x, which writes `Date`, `Map`, `Set` and
//! typed arrays as tags and a `bigint` as an 8-byte integer. Decoding them into
//! a Rust value loses tags and integer widths, so core finds the bytes of the
//! `args` field and hands them to the actor unchanged, as bare requests already
//! do. The actor runtime decodes them like any other args.
//!
//! The read checks that every item is well formed: headers, lengths, UTF-8 text,
//! breaks and nesting up to `CBOR_MAX_NESTING`. Work is linear in the message
//! length, and memory is a fixed scratch buffer, whatever lengths the message
//! declares.
//!
//! It also rejects indefinite-length byte and text strings, which cbor-x
//! refuses to decode.

use std::ops::Range;

use anyhow::{Result, anyhow, bail};
use ciborium_ll::{Decoder, Header};

/// The deepest nesting of arrays, maps and tags a message may use. It matches
/// ciborium's default recursion limit, which bounded these requests when core
/// decoded them into JSON values.
const CBOR_MAX_NESTING: usize = 256;

const CBOR_EMPTY_ARRAY: u8 = 0x80;
const CBOR_ARRAY_OF_ONE: u8 = 0x81;
const CBOR_NULL: u8 = 0xf6;
const CBOR_UNDEFINED: u8 = 0xf7;

/// Bytes and text are read through one buffer of this size per message, so
/// memory does not depend on the lengths a message declares.
const SCRATCH_LEN: usize = 4096;

/// The longest map key `cbor_map_fields` can match. Longer keys are read and
/// skipped.
const MAX_FIELD_KEY_LEN: usize = 32;

type MessageDecoder<'a> = Decoder<&'a [u8]>;
type Scratch = [u8; SCRATCH_LEN];

/// Returns the raw bytes stored under each text key of `keys` in the CBOR map
/// at the start of `item`, with `None` for a key the map does not have. Tags in
/// front of the map are skipped, as serde does. Every entry of the map is read,
/// so a malformed entry or a repeated key of `keys` is an error.
pub(super) fn cbor_map_fields<'a, const N: usize>(
	item: &'a [u8],
	keys: [&str; N],
) -> Result<[Option<&'a [u8]>; N]> {
	let mut decoder = Decoder::from(item);
	let mut scratch = [0u8; SCRATCH_LEN];

	let mut header = pull(&mut decoder)?;
	let mut depth = 1;
	while let Header::Tag(_) = header {
		depth += 1;
		header = pull(&mut decoder)?;
	}
	let Header::Map(len) = header else {
		bail!("cbor message is not a map");
	};
	if depth > CBOR_MAX_NESTING {
		bail!("cbor message nests deeper than {CBOR_MAX_NESTING} levels");
	}

	let mut found: [Option<Range<usize>>; N] = std::array::from_fn(|_| None);
	let mut remaining = len;
	loop {
		if remaining == Some(0) {
			break;
		}
		let key_header = pull(&mut decoder)?;
		if remaining.is_none() && matches!(key_header, Header::Break) {
			break;
		}

		let key_index = match key_header {
			Header::Text(Some(len)) => {
				let key = read_short_text(&mut decoder, &mut scratch, len)?;
				key.and_then(|key| keys.iter().position(|wanted| wanted.as_bytes() == key))
			}
			header => {
				skip_item(&mut decoder, &mut scratch, header, depth + 1)?;
				None
			}
		};

		let start = decoder.offset();
		let value_header = pull(&mut decoder)?;
		skip_item(&mut decoder, &mut scratch, value_header, depth + 1)?;
		let end = decoder.offset();

		if let Some(index) = key_index {
			if found[index].is_some() {
				bail!("cbor map has a duplicate `{}` field", keys[index]);
			}
			found[index] = Some(start..end);
		}
		remaining = remaining.map(|count| count - 1);
	}

	Ok(found.map(|range| range.map(|range| &item[range])))
}

/// Shapes raw cbor action args as an array, the form actors receive: a missing,
/// `null` or `undefined` value becomes `[]`, an array is kept, and any other
/// value is wrapped as `[value]`.
pub(super) fn cbor_action_args_array(args: Option<&[u8]>) -> Vec<u8> {
	let Some(args) = args else {
		return vec![CBOR_EMPTY_ARRAY];
	};
	let first = args.first().copied().unwrap_or(CBOR_NULL);
	if first == CBOR_NULL || first == CBOR_UNDEFINED {
		return vec![CBOR_EMPTY_ARRAY];
	}
	// Major type 4 is an array, of any length encoding.
	if first >> 5 == 4 {
		return args.to_vec();
	}

	let mut array = Vec::with_capacity(args.len() + 1);
	array.push(CBOR_ARRAY_OF_ONE);
	array.extend_from_slice(args);
	array
}

/// A container that `skip_item` has entered but not finished.
enum OpenContainer {
	/// A definite-length array, map or tag with this many items left. A map
	/// counts its keys and values separately.
	Items(usize),
	IndefiniteArray,
	/// An indefinite-length map. A break is only valid before a key.
	IndefiniteMap {
		awaiting_value: bool,
	},
}

/// Reads the rest of the item whose `header` was just pulled. `depth` is the
/// nesting level of that item. Open containers live on a heap stack bounded by
/// `CBOR_MAX_NESTING`, so the call stack does not grow with the input.
fn skip_item(
	decoder: &mut MessageDecoder<'_>,
	scratch: &mut Scratch,
	header: Header,
	depth: usize,
) -> Result<()> {
	let mut open: Vec<OpenContainer> = Vec::new();
	let mut next = Some(header);
	loop {
		let header = match next.take() {
			Some(header) => header,
			None => pull(decoder)?,
		};
		if !matches!(header, Header::Break) && depth + open.len() > CBOR_MAX_NESTING {
			bail!("cbor message nests deeper than {CBOR_MAX_NESTING} levels");
		}

		let completed = match header {
			Header::Positive(_) | Header::Negative(_) | Header::Float(_) | Header::Simple(_) => {
				true
			}
			Header::Break => match open.pop() {
				Some(OpenContainer::IndefiniteArray)
				| Some(OpenContainer::IndefiniteMap {
					awaiting_value: false,
				}) => true,
				Some(OpenContainer::Items(_))
				| Some(OpenContainer::IndefiniteMap {
					awaiting_value: true,
				})
				| None => bail!("cbor break outside an indefinite-length item"),
			},
			Header::Bytes(None) | Header::Text(None) => {
				bail!("cbor indefinite-length byte and text strings are not supported");
			}
			Header::Bytes(len) => {
				let mut segments = decoder.bytes(len);
				while let Some(mut segment) = segments.pull().map_err(malformed)? {
					while segment.pull(scratch).map_err(malformed)?.is_some() {}
				}
				true
			}
			Header::Text(len) => {
				let mut segments = decoder.text(len);
				while let Some(mut segment) = segments.pull().map_err(malformed)? {
					while segment.pull(scratch).map_err(malformed)?.is_some() {}
				}
				true
			}
			Header::Array(Some(0)) | Header::Map(Some(0)) => true,
			Header::Tag(_) => {
				open.push(OpenContainer::Items(1));
				false
			}
			Header::Array(Some(len)) => {
				open.push(OpenContainer::Items(len));
				false
			}
			Header::Map(Some(len)) => {
				let Some(items) = len.checked_mul(2) else {
					bail!("cbor map declares more entries than a message can hold");
				};
				open.push(OpenContainer::Items(items));
				false
			}
			Header::Array(None) => {
				open.push(OpenContainer::IndefiniteArray);
				false
			}
			Header::Map(None) => {
				open.push(OpenContainer::IndefiniteMap {
					awaiting_value: false,
				});
				false
			}
		};
		if !completed {
			continue;
		}

		// A finished item counts toward the container around it, which may
		// finish in turn.
		loop {
			match open.last_mut() {
				None => return Ok(()),
				Some(OpenContainer::Items(left)) => {
					*left -= 1;
					if *left > 0 {
						break;
					}
					open.pop();
				}
				Some(OpenContainer::IndefiniteArray) => break,
				Some(OpenContainer::IndefiniteMap { awaiting_value }) => {
					*awaiting_value = !*awaiting_value;
					break;
				}
			}
		}
	}
}

/// Reads a definite-length text item whose header was just pulled. Returns its
/// bytes when they fit in `MAX_FIELD_KEY_LEN`, or `None` for longer text.
fn read_short_text<'s>(
	decoder: &mut MessageDecoder<'_>,
	scratch: &'s mut Scratch,
	len: usize,
) -> Result<Option<&'s [u8]>> {
	let mut segments = decoder.text(Some(len));
	// A definite-length text item is one segment. Text that fits the scratch
	// buffer is read as one UTF-8 checked chunk at its start.
	while let Some(mut segment) = segments.pull().map_err(malformed)? {
		while segment.pull(scratch).map_err(malformed)?.is_some() {}
	}
	if len > MAX_FIELD_KEY_LEN {
		return Ok(None);
	}
	Ok(Some(&scratch[..len]))
}

fn pull(decoder: &mut MessageDecoder<'_>) -> Result<Header> {
	decoder.pull().map_err(malformed)
}

fn malformed(error: ciborium_ll::Error<std::io::Error>) -> anyhow::Error {
	match error {
		ciborium_ll::Error::Io(_) => anyhow!("cbor message ends inside an item"),
		ciborium_ll::Error::Syntax(offset) => anyhow!("malformed cbor at byte {offset}"),
	}
}
