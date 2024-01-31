use std::fmt::Display;

// Helpers
#[doc(hidden)]
pub use formatted_error as __formatted_error;
#[doc(hidden)]
pub struct ErrorBuilder<T: serde::Serialize> {
	pub metadata: T,
}

mod error;
pub mod ext;
pub mod macros;

pub mod prelude {
	pub use crate::{
		error::{GlobalError, GlobalResult},
		ext::*,
		macros::*,
	};
}

pub use crate::error::{GlobalError, GlobalResult};

#[derive(Debug)]
pub struct Location {
	file: &'static str,
	line: u32,
	column: u32,
}

impl Location {
	pub fn new(file: &'static str, line: u32, column: u32) -> Self {
		Location { file, line, column }
	}
}

impl Display for Location {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}:{}", self.file, self.line, self.column)
	}
}
