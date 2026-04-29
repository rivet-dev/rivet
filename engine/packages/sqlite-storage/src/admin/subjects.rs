use std::{borrow::Cow, fmt};

pub const SQLITE_OP_SUBJECT: &str = "sqlite.op";

#[derive(Clone, Copy, Debug, Default)]
pub struct SqliteOpSubject;

impl fmt::Display for SqliteOpSubject {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(SQLITE_OP_SUBJECT)
	}
}

impl universalpubsub::Subject for SqliteOpSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed(SQLITE_OP_SUBJECT))
	}

	fn as_str(&self) -> Option<&str> {
		Some(SQLITE_OP_SUBJECT)
	}
}
