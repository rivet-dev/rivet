use lazy_static::lazy_static;
use regex::Regex;

use crate::check;

lazy_static! {
	static ref SPACE_REPLACE: Regex = Regex::new(r#" +"#).unwrap();
}

/// Formats a user's biography properly. Assumes util::check::biography succeeded before this function
pub fn biography<T: AsRef<str>>(s: T) -> String {
	let s = s.as_ref();

	// Get chars (filtered to only have MAX_NEW_LINES new lines)
	let mut accum = 0;
	let chars = s
		.chars()
		.into_iter()
		.filter(|c| {
			if c == &'\n' {
				accum += 1;

				accum <= check::MAX_NEW_LINES
			} else {
				true
			}
		})
		.collect::<Vec<char>>();

	if let Ok(string) = truncate_at_code_point(&chars, check::MAX_BIOGRAPHY_LEN) {
		// Replace chains of spaces
		SPACE_REPLACE.replace_all(&string, " ").into_owned()
	} else {
		"".to_owned()
	}
}

pub fn truncate_at_code_point(
	chars: &Vec<char>,
	length: usize,
) -> Result<String, std::string::FromUtf8Error> {
	let mut accum = 0;

	String::from_utf8(
		chars
			.iter()
			.map(|c| Vec::from(c.encode_utf8(&mut [0u8; 8]).as_bytes()))
			.filter(|c| {
				accum += c.len();

				accum < length + 1
			})
			.flatten()
			.collect(),
	)
}
