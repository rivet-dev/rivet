use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};

pub const MAX_IDENT_LEN: usize = 64;
pub const MAX_DISPLAY_NAME_LEN: usize = 128;

lazy_static! {
	static ref BCRYPT: Regex = RegexBuilder::new(r#"^\$2[ayb]?\$[0-9]{2}\$[A-Za-z0-9\./]+$"#)
		.build()
		.unwrap();
}

/// Determines if the given string is a safe identifier.
///
/// All characters must be lowercase alphanumeric or a dash without a repeating double dash.
///
/// Double dashes are used as separators in DNS and path components internally.
pub fn ident(s: impl AsRef<str>) -> bool {
	ident_with_len(s, false, MAX_IDENT_LEN)
}

pub fn ident_with_len(s: impl AsRef<str>, lenient: bool, len: usize) -> bool {
	let s = s.as_ref();
	!s.is_empty()
		&& s.len() <= len
		&& !s.starts_with('-')
		&& !s.ends_with('-')
		&& s.chars().all(|c| match c {
			'0'..='9' | 'a'..='z' | '-' => true,
			'A'..='Z' | '_' if lenient => true,
			_ => false,
		}) && (lenient || !s.contains("--"))
}

pub fn display_name(s: impl AsRef<str>) -> bool {
	display_name_with_len(s, MAX_DISPLAY_NAME_LEN)
}

fn display_name_with_len(s: impl AsRef<str>, len: usize) -> bool {
	let s = s.as_ref();

	if s.is_empty() || s.len() > len {
		return false;
	}

	let chars: Vec<char> = s.chars().collect();

	// Check for non-space whitespace
	if chars.iter().any(|c| c != &' ' && c.is_whitespace()) {
		return false;
	}

	// Check for trailing whitespace
	if let (Some(first), Some(last)) = (chars.first(), chars.last()) {
		if first.is_whitespace() || last.is_whitespace() {
			return false;
		}
	}

	// Check for more than 1 whitespace in a row
	let mut last_whitespace = false;
	for c in chars {
		let is_whitespace = c.is_whitespace();

		if is_whitespace && last_whitespace {
			return false;
		}

		last_whitespace = is_whitespace;
	}

	true
}

/// Checks if a string is a valid bcrypt hash.
pub fn bcrypt(s: impl AsRef<str>) -> bool {
	let s = s.as_ref();

	BCRYPT.is_match(s)
}

#[cfg(test)]
mod tests {
	#[test]
	fn ident() {
		assert!(super::ident("x".repeat(super::MAX_IDENT_LEN)));
		assert!(!super::ident("x".repeat(super::MAX_IDENT_LEN + 1)));
		assert!(super::ident("test"));
		assert!(super::ident("test-123"));
		assert!(super::ident("test-123-abc"));
		assert!(!super::ident("test--123"));
		assert!(!super::ident("test-123-"));
		assert!(!super::ident("-test-123"));
		assert!(!super::ident("test_123"));
		assert!(!super::ident("test-ABC"));
	}

	#[test]
	fn ident_with_custom_len() {
		let max_len = super::MAX_IDENT_LEN * 2;
		assert!(super::ident_with_len("x".repeat(max_len), false, max_len));
		assert!(!super::ident_with_len(
			"x".repeat(max_len + 1),
			false,
			max_len
		));
		assert!(super::ident_with_len("test", false, max_len));
		assert!(super::ident_with_len("test-123", false, max_len));
		assert!(super::ident_with_len("test-123-abc", false, max_len));
		assert!(!super::ident_with_len("test--123", false, max_len));
		assert!(!super::ident_with_len("test-123-", false, max_len));
		assert!(!super::ident_with_len("-test-123", false, max_len));
		assert!(!super::ident_with_len("test_123", false, max_len));
		assert!(!super::ident("test-ABC"));
	}

	#[test]
	fn ident_with_custom_len_lenient() {
		let max_len = super::MAX_IDENT_LEN * 2;
		assert!(super::ident_with_len("x".repeat(max_len), true, max_len));
		assert!(!super::ident_with_len(
			"x".repeat(max_len + 1),
			true,
			max_len
		));
		assert!(super::ident_with_len("test", true, max_len));
		assert!(super::ident_with_len("test-123", true, max_len));
		assert!(super::ident_with_len("test-123-abc", true, max_len));
		assert!(super::ident_with_len("test--123", true, max_len));
		assert!(!super::ident_with_len("test-123-", true, max_len));
		assert!(!super::ident_with_len("-test-123", true, max_len));
		assert!(super::ident_with_len("test_123", true, max_len));
		assert!(super::ident_with_len("test_123-abc", true, max_len));
		assert!(super::ident_with_len("test_123_abc", true, max_len));
		assert!(super::ident_with_len("test-ABC", true, max_len));
	}
}
