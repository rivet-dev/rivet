use std::{borrow::Cow, fmt};

pub const SQLITE_COMPACT_SUBJECT: &str = "sqlite.compact";

#[derive(Clone, Copy, Debug, Default)]
pub struct SqliteCompactSubject;

impl fmt::Display for SqliteCompactSubject {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(SQLITE_COMPACT_SUBJECT)
	}
}

impl universalpubsub::Subject for SqliteCompactSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed(SQLITE_COMPACT_SUBJECT))
	}

	fn as_str(&self) -> Option<&str> {
		Some(SQLITE_COMPACT_SUBJECT)
	}
}
