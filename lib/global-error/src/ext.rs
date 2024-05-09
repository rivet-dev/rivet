use async_trait::async_trait;

use crate::{bail, GlobalResult, Location};

#[derive(Debug, thiserror::Error)]
pub enum AssertionError {
	#[error("{location} panic: {message}")]
	Panic { message: String, location: Location },

	#[error("{location} {val}: {message}")]
	Assert {
		val: String,
		message: String,
		location: Location,
	},

	#[error("{location} {val_left} != {val_right}: {message}")]
	AssertEq {
		val_left: String,
		val_right: String,
		message: String,
		location: Location,
	},

	#[error("{location} unwrap: {message}")]
	Unwrap { message: String, location: Location },
}

#[derive(Debug, thiserror::Error)]
#[error("{location} retry: {message}")]
pub struct RetryError {
	pub message: String,
	pub location: Location,
}

/// `UnwrapOrAssertError` is a trait used for handling unwrapping of `Result`
/// and `Option` types.
///
/// This trait provides a method `assertion_error_unwrap` that takes a `Result`
/// or `Option` and returns a `Result`. If the original `Result` or `Option` is
/// `Ok` or `Some`, it returns `Ok` with the unwrapped value. If it's `Err` or
/// `None`, it returns `Err` with an `AssertionError::Unwrap` that includes a
/// message and location.
pub trait UnwrapOrAssertError {
	type WrappedType;

	fn assertion_error_unwrap(
		self,
		message: String,
		location: Location,
	) -> Result<Self::WrappedType, crate::ext::AssertionError>;
}

impl<T, E: core::fmt::Debug> UnwrapOrAssertError for Result<T, E> {
	type WrappedType = T;

	fn assertion_error_unwrap(
		self,
		message: String,
		location: Location,
	) -> Result<T, crate::ext::AssertionError> {
		match self {
			Ok(t) => Ok(t),
			Err(e) => Err(crate::ext::AssertionError::Unwrap {
				message: format!("{}: {:?}", message, e),
				location,
			}),
		}
	}
}

impl<T> UnwrapOrAssertError for Option<T> {
	type WrappedType = T;

	fn assertion_error_unwrap(
		self,
		message: String,
		location: Location,
	) -> Result<T, crate::ext::AssertionError> {
		match self {
			Some(t) => Ok(t),
			None => Err(Into::into(crate::ext::AssertionError::Unwrap {
				message,
				location,
			})),
		}
	}
}

// TODO(forest): Can we handle this type with a blanket impl?
impl<'a, T> UnwrapOrAssertError for &'a Option<T> {
	type WrappedType = &'a T;

	fn assertion_error_unwrap(
		self,
		message: String,
		location: Location,
	) -> Result<&'a T, crate::ext::AssertionError> {
		match self {
			Some(t) => Ok(t),
			None => Err(Into::into(crate::ext::AssertionError::Unwrap {
				message,
				location,
			})),
		}
	}
}

// TODO(forest): Can we handle this type with a blanket impl?
impl<'a, T> UnwrapOrAssertError for &'a &'a Option<T> {
	type WrappedType = &'a T;

	fn assertion_error_unwrap(
		self,
		message: String,
		location: Location,
	) -> Result<&'a T, crate::ext::AssertionError> {
		match self {
			Some(t) => Ok(t),
			None => Err(Into::into(crate::ext::AssertionError::Unwrap {
				message,
				location,
			})),
		}
	}
}

#[async_trait]
pub trait ToGlobalError: Sized {
	async fn to_global_error(self) -> GlobalResult<Self>;
}

#[async_trait]
impl ToGlobalError for reqwest::Response {
	async fn to_global_error(self) -> GlobalResult<Self> {
		if self.status().is_success() {
			Ok(self)
		} else {
			let url = self.url().clone();
			let status = self.status();
			let body = self.text().await?;

			bail!(format!("{url} ({status}):\n{body}"));
		}
	}
}
