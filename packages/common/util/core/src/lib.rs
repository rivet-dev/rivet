pub use id::Id;
pub use rivet_util_id as id;

pub mod backoff;
pub mod billing;
pub mod check;
pub mod duration;
pub mod faker;
pub mod file_size;
pub mod format;
pub mod future;
pub mod geo;
pub mod math;
pub mod req;
pub mod serde;
pub mod signal;
pub mod sort;
pub mod timestamp;
pub mod url;

/// Slices a string without panicking on char boundaries. Defaults to the left side of the char if a slice
// is invalid. Will still panic if start > end.
pub fn safe_slice(s: &str, start: usize, end: usize) -> &str {
	let mut new_start = 0;
	let mut new_end = s.len();

	for (i, _) in s.char_indices() {
		if i <= start {
			new_start = i;
		}

		if i >= end {
			break;
		}

		new_end = i;
	}

	&s[new_start..=new_end]
}
