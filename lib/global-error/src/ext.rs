use crate::Location;

#[derive(Debug, thiserror::Error)]
pub enum AssertionError {
	#[error("{location} panic: {message}")]
	Panic {
		message: &'static str,
		location: Location,
	},

	#[error("{location} {val}: {message}")]
	Assert {
		val: String,
		message: &'static str,
		location: Location,
	},

	#[error("{location} {val_left} != {val_right}: {message}")]
	AssertEq {
		val_left: String,
		val_right: String,
		message: &'static str,
		location: Location,
	},

	#[error("{location} unwrap: {message}")]
	Unwrap {
		message: &'static str,
		location: Location,
	},
}

#[derive(Debug, thiserror::Error)]
#[error("{location} retry: {message}")]
pub struct RetryError {
	pub message: &'static str,
	pub location: Location,
}
