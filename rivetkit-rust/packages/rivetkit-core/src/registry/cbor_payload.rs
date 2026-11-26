//! Converts CBOR action results and event payloads into the JSON json clients
//! receive. cbor and bare clients get these payloads as raw CBOR.
//!
//! The JSON is what the TypeScript runtime sent before rivetkit-core existed:
//! `jsonStringifyCompat` of the value. The runtime encodes a value with
//! `encodeCborCompat`, which already writes `bigint`, `Uint8Array`, `Set` and
//! `undefined` as `["$Name", ...]` arrays, so only the values cbor-x writes as
//! CBOR tags need a rule. Each one follows what cbor-x decodes it to and what
//! `JSON.stringify` then writes:
//!
//! - Tag 1 (`Date`) becomes `Date.prototype.toISOString()` text, or `null` for
//!   an invalid date.
//! - Tag 259 (`Map`), tag 258 (`Set`) and tag 27 (`Error`, `RegExp`) become `{}`.
//! - Tag 64 (`Uint8Array`) and a plain byte string become `["$Uint8Array",
//!   "<base64>"]`. The other typed arrays become an object of their elements
//!   keyed by index, with 64-bit elements as `["$BigInt", "<decimal>"]`.
//! - Tags 2 and 3 (bignums) become `["$BigInt", "<decimal>"]`.
//!
//! A value JSON cannot hold this way is an error, never `null`: any other tag,
//! a bignum wider than 128 bits, or a map key that is not text or an integer.

use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use ciborium::Value as CborValue;
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};

const CBOR_TAG_DATE_EPOCH: u64 = 1;
const CBOR_TAG_BIGNUM_POSITIVE: u64 = 2;
const CBOR_TAG_BIGNUM_NEGATIVE: u64 = 3;
const CBOR_TAG_GENERIC_OBJECT: u64 = 27;
const CBOR_TAG_UINT8_ARRAY: u64 = 64;
const CBOR_TAG_SET: u64 = 258;
const CBOR_TAG_MAP: u64 = 259;

/// The largest time value a JavaScript `Date` can hold, in milliseconds.
const JS_DATE_MAX_EPOCH_MS: f64 = 8.64e15;
const MS_PER_DAY: i64 = 86_400_000;

const JSON_COMPAT_BIGINT: &str = "$BigInt";
const JSON_COMPAT_UINT8_ARRAY: &str = "$Uint8Array";

/// The element type of a typed array tag that cbor-x writes. cbor-x writes
/// little-endian tags only, since every host the runtime supports is
/// little-endian.
#[derive(Clone, Copy)]
enum TypedArrayElement {
	U8,
	U16,
	U32,
	U64,
	I8,
	I16,
	I32,
	I64,
	F32,
	F64,
}

impl TypedArrayElement {
	fn from_tag(tag: u64) -> Option<Self> {
		match tag {
			68 => Some(Self::U8),
			69 => Some(Self::U16),
			70 => Some(Self::U32),
			71 => Some(Self::U64),
			72 => Some(Self::I8),
			77 => Some(Self::I16),
			78 => Some(Self::I32),
			79 => Some(Self::I64),
			85 => Some(Self::F32),
			86 => Some(Self::F64),
			_ => None,
		}
	}

	fn size(self) -> usize {
		match self {
			Self::U8 | Self::I8 => 1,
			Self::U16 | Self::I16 => 2,
			Self::U32 | Self::I32 | Self::F32 => 4,
			Self::U64 | Self::I64 | Self::F64 => 8,
		}
	}

	/// Converts one little-endian element, whose length is `self.size()`.
	fn to_json(self, bytes: &[u8]) -> JsonValue {
		let mut raw = [0u8; 8];
		raw[..bytes.len()].copy_from_slice(bytes);
		match self {
			Self::U8 => JsonValue::from(raw[0]),
			Self::U16 => JsonValue::from(u16::from_le_bytes([raw[0], raw[1]])),
			Self::U32 => JsonValue::from(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]])),
			Self::U64 => json_compat_bigint(u64::from_le_bytes(raw).to_string()),
			Self::I8 => JsonValue::from(raw[0] as i8),
			Self::I16 => JsonValue::from(i16::from_le_bytes([raw[0], raw[1]])),
			Self::I32 => JsonValue::from(i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]])),
			Self::I64 => json_compat_bigint(i64::from_le_bytes(raw).to_string()),
			Self::F32 => json_float(f64::from(f32::from_le_bytes([
				raw[0], raw[1], raw[2], raw[3],
			]))),
			Self::F64 => json_float(f64::from_le_bytes(raw)),
		}
	}
}

/// Converts an action result or event payload into the JSON a json client
/// receives. See the module docs for the rules.
pub(super) fn decode_client_payload_json(payload: &[u8]) -> Result<JsonValue> {
	if payload.is_empty() {
		return Ok(JsonValue::Null);
	}

	// Plain JSON values decode straight into `JsonValue`, so ordinary payloads
	// skip the intermediate `CborValue`. `JsonValue` rejects tags, byte strings
	// and non-text keys, which are exactly the values the rules below cover.
	if let Ok(value) = ciborium::from_reader::<JsonValue, _>(payload) {
		return Ok(value);
	}

	let value =
		ciborium::from_reader::<CborValue, _>(payload).context("decode cbor client payload")?;
	cbor_value_to_json(value)
}

fn cbor_value_to_json(value: CborValue) -> Result<JsonValue> {
	let json = match value {
		CborValue::Integer(integer) => json_integer(i128::from(integer)),
		CborValue::Float(float) => json_float(float),
		CborValue::Bytes(bytes) => json_compat_uint8_array(&bytes),
		CborValue::Text(text) => JsonValue::String(text),
		CborValue::Bool(value) => JsonValue::Bool(value),
		CborValue::Null => JsonValue::Null,
		CborValue::Tag(tag, content) => cbor_tag_to_json(tag, *content)?,
		CborValue::Array(items) => JsonValue::Array(
			items
				.into_iter()
				.map(cbor_value_to_json)
				.collect::<Result<_>>()?,
		),
		CborValue::Map(entries) => {
			let mut object = JsonMap::new();
			for (key, value) in entries {
				object.insert(cbor_map_key_to_json(key)?, cbor_value_to_json(value)?);
			}
			JsonValue::Object(object)
		}
		// `ciborium::Value` is `#[non_exhaustive]`, so a future variant needs a
		// rule before it can reach a json client.
		_ => bail!("cbor value has no json form"),
	};
	Ok(json)
}

fn cbor_tag_to_json(tag: u64, content: CborValue) -> Result<JsonValue> {
	match (tag, content) {
		(CBOR_TAG_DATE_EPOCH, content) => Ok(epoch_date_to_json(&content)),
		(CBOR_TAG_MAP | CBOR_TAG_SET | CBOR_TAG_GENERIC_OBJECT, _) => {
			Ok(JsonValue::Object(JsonMap::new()))
		}
		(CBOR_TAG_UINT8_ARRAY, CborValue::Bytes(bytes)) => Ok(json_compat_uint8_array(&bytes)),
		(CBOR_TAG_BIGNUM_POSITIVE | CBOR_TAG_BIGNUM_NEGATIVE, CborValue::Bytes(bytes)) => {
			bignum_to_json(tag == CBOR_TAG_BIGNUM_NEGATIVE, &bytes)
		}
		(tag, CborValue::Bytes(bytes)) => match TypedArrayElement::from_tag(tag) {
			Some(element) => typed_array_to_json(element, &bytes),
			None => bail!("cbor tag {tag} has no json form"),
		},
		(tag, _) => bail!("cbor tag {tag} has no json form"),
	}
}

/// Matches cbor-x, which decodes tag 1 as `new Date(Math.round(seconds * 1000))`,
/// followed by `Date.prototype.toJSON`.
fn epoch_date_to_json(content: &CborValue) -> JsonValue {
	let seconds = match content {
		CborValue::Integer(integer) => i128::from(*integer) as f64,
		CborValue::Float(float) => *float,
		_ => return JsonValue::Null,
	};

	let epoch_ms = js_math_round(seconds * 1000.0);
	if !epoch_ms.is_finite() || epoch_ms.abs() > JS_DATE_MAX_EPOCH_MS {
		return JsonValue::Null;
	}

	JsonValue::String(js_date_iso_string(epoch_ms as i64))
}

/// Rounds like `Math.round`: halves go toward positive infinity. The fraction is
/// compared with 0.5 instead of adding 0.5 to `value`, because that sum rounds
/// to the nearest representable double and moves large odd integers up by one.
fn js_math_round(value: f64) -> f64 {
	let floor = value.floor();
	if value - floor >= 0.5 {
		floor + 1.0
	} else {
		floor
	}
}

/// Formats like `Date.prototype.toISOString` across the whole JavaScript date
/// range, writing years outside 0..=9999 with a sign and six digits.
fn js_date_iso_string(epoch_ms: i64) -> String {
	let days = epoch_ms.div_euclid(MS_PER_DAY);
	let ms_of_day = epoch_ms.rem_euclid(MS_PER_DAY);
	let (year, month, day) = civil_date_from_days(days);

	let year = if (0..=9999).contains(&year) {
		format!("{year:04}")
	} else if year < 0 {
		format!("-{:06}", -year)
	} else {
		format!("+{year:06}")
	};
	let hours = ms_of_day / 3_600_000;
	let minutes = ms_of_day / 60_000 % 60;
	let seconds = ms_of_day / 1000 % 60;
	let millis = ms_of_day % 1000;
	format!("{year}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.{millis:03}Z")
}

/// Converts days since 1970-01-01 to a proleptic Gregorian `(year, month, day)`
/// using Howard Hinnant's `civil_from_days` algorithm.
fn civil_date_from_days(days: i64) -> (i64, i64, i64) {
	let shifted = days + 719_468;
	let era = shifted.div_euclid(146_097);
	let day_of_era = shifted.rem_euclid(146_097);
	let year_of_era =
		(day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
	let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
	let month_from_march = (5 * day_of_year + 2) / 153;
	let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
	let month = if month_from_march < 10 {
		month_from_march + 3
	} else {
		month_from_march - 9
	};
	let year = year_of_era + era * 400 + i64::from(month <= 2);
	(year, month, day)
}

/// Converts a bignum of at most 128 bits, the widest a Rust actor writes.
fn bignum_to_json(negative: bool, bytes: &[u8]) -> Result<JsonValue> {
	let significant = bytes
		.iter()
		.position(|byte| *byte != 0)
		.map_or(&[][..], |start| &bytes[start..]);
	ensure!(
		significant.len() <= 16,
		"cbor bignum wider than 128 bits has no json form"
	);

	let mut raw = [0u8; 16];
	raw[16 - significant.len()..].copy_from_slice(significant);
	let magnitude = u128::from_be_bytes(raw);

	// A negative bignum holds `-1 - magnitude`.
	let decimal = if negative {
		let Some(absolute) = magnitude.checked_add(1) else {
			bail!("cbor bignum wider than 128 bits has no json form");
		};
		format!("-{absolute}")
	} else {
		magnitude.to_string()
	};
	Ok(json_compat_bigint(decimal))
}

fn typed_array_to_json(element: TypedArrayElement, bytes: &[u8]) -> Result<JsonValue> {
	ensure!(
		bytes.len().is_multiple_of(element.size()),
		"cbor typed array length {} is not a whole number of elements",
		bytes.len()
	);

	let mut object = JsonMap::new();
	for (index, chunk) in bytes.chunks_exact(element.size()).enumerate() {
		object.insert(index.to_string(), element.to_json(chunk));
	}
	Ok(JsonValue::Object(object))
}

fn json_integer(value: i128) -> JsonValue {
	if let Ok(value) = i64::try_from(value) {
		return JsonValue::from(value);
	}
	if let Ok(value) = u64::try_from(value) {
		return JsonValue::from(value);
	}
	json_compat_bigint(value.to_string())
}

/// `JSON.stringify` writes `NaN` and infinities as `null`.
fn json_float(value: f64) -> JsonValue {
	JsonNumber::from_f64(value)
		.map(JsonValue::Number)
		.unwrap_or(JsonValue::Null)
}

fn json_compat_bigint(decimal: String) -> JsonValue {
	JsonValue::Array(vec![
		JsonValue::String(JSON_COMPAT_BIGINT.to_owned()),
		JsonValue::String(decimal),
	])
}

fn json_compat_uint8_array(bytes: &[u8]) -> JsonValue {
	JsonValue::Array(vec![
		JsonValue::String(JSON_COMPAT_UINT8_ARRAY.to_owned()),
		JsonValue::String(STANDARD.encode(bytes)),
	])
}

/// cbor-x decodes a map into an object and writes an integer key as its
/// decimal text. Other keys have no faithful text form, so they are errors.
fn cbor_map_key_to_json(key: CborValue) -> Result<String> {
	match key {
		CborValue::Text(text) => Ok(text),
		CborValue::Integer(integer) => Ok(i128::from(integer).to_string()),
		_ => bail!("cbor map key that is not text or an integer has no json form"),
	}
}
