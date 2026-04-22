use std::{
	fmt::{self, Display, Formatter},
	iter::Iterator,
};

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
	static ref SPACE_REPLACE: Regex = Regex::new(r#" +"#).unwrap();
}

/// Renders `Some<T>` as `T` and does not render `None`.
pub struct OptDisplay<T: Display>(pub Option<T>);

impl<T: Display> Display for OptDisplay<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		if let Some(value) = &self.0 {
			write!(f, "{}", value)
		} else {
			Ok(())
		}
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

pub fn duration(ms: i64, relative: bool) -> String {
	let neg = ms < 0;
	let ms = ms.abs();
	let mut parts = Vec::with_capacity(5);

	if relative && neg {
		parts.push("in".to_string());
	}

	if ms == 0 {
		parts.push("0ms".to_string());
	} else if ms < 1000 {
		parts.push(format!("{ms}ms"));
	} else {
		let days = ms / 86_400_000;
		let hours = (ms % 86_400_000) / 3_600_000;
		let minutes = (ms % 3_600_000) / 60_000;
		let seconds = (ms % 60_000) / 1_000;

		if days > 0 {
			parts.push(format!("{days}d"));
		}
		if hours > 0 {
			parts.push(format!("{hours}h"));
		}
		if minutes > 0 {
			parts.push(format!("{minutes}m"));
		}
		if ms < 60_000 && seconds > 0 {
			parts.push(format!("{seconds}s"));
		}
	}

	if relative && !neg {
		parts.push("ago".to_string());
	}

	parts.join(" ")
}
