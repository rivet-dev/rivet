pub const MAX_VALUE_LEN: usize = 262_144; // 2^16

pub fn key_directory(key: &str) -> &str {
	let mut chars = key.chars().rev().enumerate().peekable();
	while let Some((i, c)) = chars.next() {
		// Find the last slash in the key
		if c == '/' && !matches!(chars.peek(), Some((_, '\\'))) {
			return &key[..(key.len() - i - 1)];
		}
	}

	""
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn normal() {
		let key = "a/b/c";
		let directory = key_directory(key);
		assert_eq!("a/b", directory);
	}
	#[test]
	fn no_directory() {
		let key = "abc";
		let directory = key_directory(key);
		assert_eq!("", directory);
	}

	#[test]
	fn empty() {
		let key = "";
		let directory = key_directory(key);
		assert_eq!("", directory);
	}

	#[test]
	fn empty_segments() {
		let key = "a//b";
		let directory = key_directory(key);
		assert_eq!("a/", directory);
	}

	#[test]
	fn empty_segments_escaped() {
		let key = "a\\///b";
		let directory = key_directory(key);
		assert_eq!("a\\//", directory);
	}

	#[test]
	fn slash_in_segments() {
		let key = "a/b\\/c";
		let directory = key_directory(key);
		assert_eq!("a", directory);
	}

	#[test]
	fn no_directory_with_slash() {
		let key = "/abc";
		let directory = key_directory(key);
		assert_eq!("", directory);
	}
}
