use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};

pub const MAX_IDENT_LEN: usize = 16;
pub const MAX_IDENT_LONG_LEN: usize = 64;
pub const MAX_DISPLAY_NAME_LEN: usize = 24;
pub const MAX_DISPLAY_NAME_LONG_LEN: usize = 128;
pub const MAX_BIOGRAPHY_LEN: usize = 200;
pub const MAX_NEW_LINES: usize = 5;
pub const MAX_DOMAIN_LEN: usize = 255;

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

pub fn ident_long(s: impl AsRef<str>) -> bool {
	ident_with_len(s, false, MAX_IDENT_LONG_LEN)
}

pub fn ident_lenient(s: impl AsRef<str>) -> bool {
	ident_with_len(s, true, MAX_IDENT_LONG_LEN)
}

pub fn ident_with_len(s: impl AsRef<str>, lenient: bool, len: usize) -> bool {
	let s = s.as_ref();
	s.chars().all(|c| match c {
		'0'..='9' | 'a'..='z' | '-' => true,
		'A'..='Z' | '_' if lenient => true,
		_ => false,
	}) && !s.is_empty()
		&& s.len() <= len
		&& !s.starts_with('-')
		&& !s.ends_with('-')
		&& (lenient || !s.contains("--"))
}

/// Same as `ident` but without the length requirement.
pub fn docker_ident(s: impl AsRef<str>) -> bool {
	let s = s.as_ref();
	s.chars().all(|c| match c {
		'0'..='9' | 'a'..='z' | '-' | '_' => true,
		_ => false,
	}) && !s.is_empty()
		&& !s.starts_with('-')
		&& !s.ends_with('-')
}

/// Determines if the given string can be used as a display name.
///
/// We allow all unicode characters limited to 24 bytes, not graphemes.
pub fn display_name(s: impl AsRef<str>) -> bool {
	display_name_with_len(s, MAX_DISPLAY_NAME_LEN)
}

/// See `display_name`. Limited to 64 bytes.
pub fn display_name_long(s: impl AsRef<str>) -> bool {
	display_name_with_len(s, MAX_DISPLAY_NAME_LONG_LEN)
}

fn display_name_with_len(s: impl AsRef<str>, len: usize) -> bool {
	let s = s.as_ref();

	if s.is_empty() || s.len() > len {
		return false;
	}

	let chars: Vec<char> = s.chars().into_iter().collect();

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

/// Determines if the given string can be used as a biography.
///
/// We allow all unicode characters limited to 200 bytes, not graphemes.
pub fn biography(s: impl AsRef<str>) -> bool {
	let s = s.as_ref();

	if s.len() > MAX_BIOGRAPHY_LEN {
		return false;
	}

	let chars: Vec<char> = s.chars().into_iter().collect();

	// Check for whitespace that isn't a space or new line
	if chars
		.iter()
		.any(|c| c != &' ' && c != &'\n' && c.is_whitespace())
	{
		return false;
	}

	// Only allow a total of MAX_NEW_LINES new lines
	if chars
		.iter()
		.fold(0, |s, c| s + if c == &'\n' { 1 } else { 0 })
		> MAX_NEW_LINES
	{
		return false;
	}

	true
}

/// Checks if a domain is valid.
///
/// Will prevent domains from matching Rivet-specific domains if
/// `is_external` is true.
pub fn domain(s: impl AsRef<str>, is_external: bool) -> bool {
	let s = s.as_ref();

	if let (true, Some(domain_main), Some(domain_cdn), Some(domain_job)) = (
		is_external,
		crate::env::domain_main(),
		crate::env::domain_cdn(),
		crate::env::domain_job(),
	) {
		if s.ends_with(&format!(".{domain_main}"))
			|| s.ends_with(&format!(".{domain_cdn}"))
			|| s == domain_main
			|| s == domain_cdn
			|| s == domain_job
		{
			return false;
		}
	}

	!s.is_empty()
		&& s.len() <= MAX_DOMAIN_LEN
		&& s.chars().all(|c| match c {
			'0'..='9' | 'a'..='z' | '-' | '.' => true,
			_ => false,
		})
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
	fn ident_long() {
		assert!(super::ident_long("x".repeat(super::MAX_IDENT_LONG_LEN)));
		assert!(!super::ident_long(
			"x".repeat(super::MAX_IDENT_LONG_LEN + 1)
		));
		assert!(super::ident_long("test"));
		assert!(super::ident_long("test-123"));
		assert!(super::ident_long("test-123-abc"));
		assert!(!super::ident_long("test--123"));
		assert!(!super::ident_long("test-123-"));
		assert!(!super::ident_long("-test-123"));
		assert!(!super::ident_long("test_123"));
		assert!(!super::ident("test-ABC"));
	}

	#[test]
	fn ident_lenient() {
		assert!(super::ident_lenient("x".repeat(super::MAX_IDENT_LONG_LEN)));
		assert!(!super::ident_lenient(
			"x".repeat(super::MAX_IDENT_LONG_LEN + 1)
		));
		assert!(super::ident_lenient("test"));
		assert!(super::ident_lenient("test-123"));
		assert!(super::ident_lenient("test-123-abc"));
		assert!(super::ident_lenient("test--123"));
		assert!(!super::ident_lenient("test-123-"));
		assert!(!super::ident_lenient("-test-123"));
		assert!(super::ident_lenient("test_123"));
		assert!(super::ident_lenient("test_123-abc"));
		assert!(super::ident_lenient("test_123_abc"));
		assert!(super::ident_lenient("test-ABC"));
	}
}
