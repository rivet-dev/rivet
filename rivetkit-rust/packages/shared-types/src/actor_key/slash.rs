//! The slash-separated actor key form that the actor REST API and the actor runtime use, and
//! that RivetKit's `serializeActorKey` and `deserializeActorKey` implement.
//!
//! - The empty key `[]` is `/`.
//! - An empty part is `\0`.
//! - Inside a part, `\` becomes `\\` and `/` becomes `\/`.

/// The slash form of the empty key.
pub const EMPTY_KEY: &str = "/";

const SEPARATOR: char = '/';
const ESCAPE: char = '\\';
const EMPTY_PART: &str = "\\0";

/// Encodes `parts` in the slash form.
pub fn serialize<S: AsRef<str>>(parts: &[S]) -> String {
	if parts.is_empty() {
		return EMPTY_KEY.to_owned();
	}

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

/// Decodes a slash-form key. Decoding is total and reads every string the way RivetKit's
/// `deserializeActorKey` does: `""` is the empty key, `\` before any character other than `0`
/// keeps that character, a trailing `\` is kept literally, and a `\0` marker makes its whole part
/// empty.
pub fn deserialize(key: &str) -> Vec<String> {
	if key == EMPTY_KEY {
		return Vec::new();
	}

	let mut parts = Vec::new();
	let mut current = String::new();
	let mut escaping = false;
	let mut empty_part = false;

	for ch in key.chars() {
		if escaping {
			if ch == '0' {
				empty_part = true;
			} else {
				current.push(ch);
			}
			escaping = false;
		} else if ch == ESCAPE {
			escaping = true;
		} else if ch == SEPARATOR {
			if empty_part {
				parts.push(String::new());
				current.clear();
				empty_part = false;
			} else {
				parts.push(std::mem::take(&mut current));
			}
		} else {
			current.push(ch);
		}
	}

	if escaping {
		current.push(ESCAPE);
		parts.push(current);
	} else if empty_part {
		parts.push(String::new());
	} else if !current.is_empty() || !parts.is_empty() {
		parts.push(current);
	}

	parts
}
