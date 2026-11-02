//! The canonical encoding of an actor key.
//!
//! An actor key is a list of string parts. Everything inside the engine that names an actor by
//! key, the keyed actor id, the Epoxy reservation, the stored actor record and the gateway's
//! `rvt-key` query parameter, uses one string: the canonical key [`encode`] produces. Two keys
//! name the same actor only if their canonical strings are equal.
//!
//! The canonical form joins the parts with `,`:
//!
//! - The empty key `[]` is the empty string.
//! - An empty part is `\0`, so `[""]` and `[]` stay distinct.
//! - Inside a part, `\` becomes `\\` and `,` becomes `\,`. Every other character, `/` and
//!   unicode included, is kept as is.
//!
//! For example `["chat", "42"]` is `chat,42`, `["a,b", ""]` is `a\,b,\0` and `["a\\b"]` is
//! `a\\b`.
//!
//! [`decode`] also reads the unescaped `rvt-key` values earlier clients send: an empty part
//! between commas is an empty part, and a `\` that starts no escape is kept literally. Such a
//! value decodes exactly as earlier engines read it unless it contains `\\` or `\,`, or a part
//! that is exactly `\0`. Earlier clients could not tell those keys apart from others either.
//!
//! The actor REST API and the actor runtime still speak the slash form earlier engines used;
//! [`slash`] holds it and [`from_slash`] and [`to_slash`] translate at those boundaries.
//!
//! `rivetkit-typescript/packages/rivetkit/src/actor/keys.ts` is the TypeScript twin. Both run the
//! vectors in `tests/fixtures/actor_key_vectors.json`.

pub mod slash;

const SEPARATOR: char = ',';
const ESCAPE: char = '\\';
const EMPTY_PART: &str = "\\0";

/// Encodes `parts` in the canonical form.
pub fn encode<S: AsRef<str>>(parts: &[S]) -> String {
	let mut key = String::new();
	for (index, part) in parts.iter().enumerate() {
		if index > 0 {
			key.push(SEPARATOR);
		}

		let part = part.as_ref();
		if part.is_empty() {
			key.push_str(EMPTY_PART);
			continue;
		}

		for ch in part.chars() {
			if ch == ESCAPE || ch == SEPARATOR {
				key.push(ESCAPE);
			}
			key.push(ch);
		}
	}
	key
}

/// Decodes a canonical key, or an unescaped `rvt-key` value, into its parts. Decoding is total.
pub fn decode(key: &str) -> Vec<String> {
	if key.is_empty() {
		return Vec::new();
	}

	let mut parts = Vec::new();
	let mut raw = String::new();
	let mut current = String::new();
	let mut chars = key.chars().peekable();
	while let Some(ch) = chars.next() {
		if ch == SEPARATOR {
			parts.push(finish_part(&raw, std::mem::take(&mut current)));
			raw.clear();
			continue;
		}

		raw.push(ch);
		if ch != ESCAPE {
			current.push(ch);
			continue;
		}

		// Only `\\` and `\,` are escapes. Any other backslash is a literal character.
		match chars.peek() {
			Some(&next) if next == ESCAPE || next == SEPARATOR => {
				raw.push(next);
				current.push(next);
				chars.next();
			}
			Some(_) | None => current.push(ESCAPE),
		}
	}
	parts.push(finish_part(&raw, current));
	parts
}

/// A decoded part. A part written as exactly `\0` is the empty part.
fn finish_part(raw: &str, decoded: String) -> String {
	if raw == EMPTY_PART {
		String::new()
	} else {
		decoded
	}
}

/// Rewrites a key string into the canonical form. A canonical key is returned unchanged.
pub fn canonicalize(key: &str) -> String {
	encode(&decode(key))
}

/// Reads the gateway's `rvt-key` query parameter. A missing parameter is the empty key.
pub fn from_gateway_query(rvt_key: Option<&str>) -> Vec<String> {
	rvt_key.map(decode).unwrap_or_default()
}

/// The `rvt-key` value a client sends for `parts`: the canonical key, or `None` for the empty
/// key, whose parameter is omitted.
pub fn to_gateway_query<S: AsRef<str>>(parts: &[S]) -> Option<String> {
	if parts.is_empty() {
		return None;
	}
	Some(encode(parts))
}

/// A slash-form key that is not exactly what [`slash::serialize`] writes for its parts, so it
/// would not come back unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonCanonicalSlashKey;

impl std::fmt::Display for NonCanonicalSlashKey {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("actor key is not in the canonical slash-separated form")
	}
}

impl std::error::Error for NonCanonicalSlashKey {}

/// Translates a key the actor REST API received in the slash form into the canonical form. A
/// key that would not round-trip, such as `""` or `a\b`, is refused, so no two REST keys name
/// one actor.
pub fn from_slash(key: &str) -> Result<String, NonCanonicalSlashKey> {
	let parts = slash::deserialize(key);
	if slash::serialize(&parts) != key {
		return Err(NonCanonicalSlashKey);
	}
	Ok(encode(&parts))
}

/// Translates a canonical key into the slash form the actor REST API returns and the actor
/// runtime reads.
pub fn to_slash(key: &str) -> String {
	slash::serialize(&decode(key))
}
