use std::{borrow::Cow, fmt::Display};

pub trait Subject: Display {
	/// Used for metrics.
	fn root<'a>() -> Option<Cow<'a, str>> {
		None
	}

	fn as_str(&self) -> Option<&str> {
		None
	}

	fn as_cow<'a>(&'a self) -> Cow<'a, str> {
		if let Some(subject) = self.as_str() {
			Cow::Borrowed(subject)
		} else {
			Cow::Owned(self.to_string())
		}
	}
}

impl Subject for &str {
	fn as_str(&self) -> Option<&str> {
		Some(self)
	}
}

impl Subject for &String {
	fn as_str(&self) -> Option<&str> {
		Some(self)
	}
}
