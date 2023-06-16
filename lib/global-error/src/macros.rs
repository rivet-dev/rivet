#[macro_export]
macro_rules! location {
	() => {
		$crate::Location::new(file!(), line!(), column!())
	};
}

#[macro_export]
macro_rules! err_code {
	// Code with a builder body (for metadata) as well as context keys
	($code:ident $body:tt, $($key:ident = $value:expr),+ $(,)?) => {{
		let body = $crate::ErrorBuilder $body;
		$crate::GlobalError::bad_request_builder($crate::__formatted_error::code::$code)
			.context(IntoIterator::into_iter([$((stringify!($key).to_string(), $value.to_string()),)+])
				.collect::<std::collections::HashMap<_, _>>()
			)
			.metadata(body.metadata)?
			.build()
	}};
	// Code with a builder body (for metadata)
	($code:ident $body:tt) => {{
		let body = $crate::ErrorBuilder $body;
		$crate::GlobalError::bad_request_builder($crate::__formatted_error::code::$code)
			.metadata(body.metadata)?
			.build()
	}};
	// Code with context keys
	($code:ident, $($key:ident = $value:expr),+ $(,)?) => {
		$crate::GlobalError::bad_request_builder($crate::__formatted_error::code::$code)
			.context(IntoIterator::into_iter([$((stringify!($key).to_string(), $value.to_string()),)+])
				.collect::<std::collections::HashMap<_, _>>()
			)
			.build()
	};
	// Just a code
	($code:ident) => {
		$crate::GlobalError::bad_request($crate::__formatted_error::code::$code)
	};
}
pub use err_code;

#[macro_export]
macro_rules! internal_panic {
	($msg:expr) => {{
		return Err(Into::into($crate::ext::AssertionError::Panic {
			message: $msg,
			location: $crate::location!(),
		}));
	}};
}
pub use internal_panic;

#[macro_export]
macro_rules! retry_panic {
	($msg:expr) => {{
		let mut err = GlobalError::from($crate::ext::RetryError {
			message: $msg,
			location: $crate::location!(),
		});
		if let GlobalError::Internal {
			ref mut retry_immediately,
			..
		} = err
		{
			*retry_immediately = true;
		}

		return Err(err);
	}};
}
pub use retry_panic;

#[macro_export]
macro_rules! internal_assert {
	($expr:expr, $msg:expr) => {{
		let val = $expr;
		if !val {
			return Err(Into::into($crate::ext::AssertionError::Assert {
				val: format!("{:?}", val),
				message: $msg,
				location: $crate::location!(),
			}));
		}
	}};
	($expr:expr $(,)?) => {{
		$crate::internal_assert!($expr, "assertion failed")
	}};
}
pub use internal_assert;

#[macro_export]
macro_rules! internal_assert_eq {
	($left:expr, $right:expr, $msg:expr) => {{
		match (&$left, &$right) {
			(val_left, val_right) => {
				if !(*val_left == *val_right) {
					return Err(Into::into($crate::ext::AssertionError::AssertEq {
						val_left: format!("{:?}", val_left),
						val_right: format!("{:?}", val_right),
						message: $msg,
						location: $crate::location!(),
					}));
				}
			}
		}
	}};
	($left:expr, $right:expr $(,)?) => {{
		$crate::internal_assert_eq!($left, $right, "assertion failed")
	}};
}
pub use internal_assert_eq;

#[macro_export]
macro_rules! internal_unwrap {
	($expr:expr, $msg:expr) => {{
		$crate::internal_unwrap_owned!(&$expr, $msg)
	}};
	($expr:expr $(,)?) => {{
		$crate::internal_unwrap_owned!(&$expr)
	}};
}
pub use internal_unwrap;

#[macro_export]
macro_rules! internal_unwrap_owned {
	($expr:expr, $msg:expr) => {{
		#[allow(match_result_ok)]
		if let Some(val) = $expr {
			val
		} else {
			return Err(Into::into($crate::ext::AssertionError::Unwrap {
				message: $msg,
				location: $crate::location!(),
			}));
		}
	}};
	($expr:expr $(,)?) => {{
		$crate::internal_unwrap_owned!($expr, "attempt to unwrap null value")
	}};
}
pub use internal_unwrap_owned;

#[macro_export]
macro_rules! panic_with {
	($code:ident $body:tt, $($key:ident = $value:expr),+ $(,)?) => {{
		return Err($crate::err_code!($code $body, $($key = $value),+));
	}};
	($code:ident $body:tt) => {{
		return Err($crate::err_code!($code $body));
	}};
	($code:ident, $($key:ident = $value:expr),+ $(,)?) => {{
		return Err($crate::err_code!($code, $($key = $value),+));
	}};
	($code:ident) => {{
		return Err($crate::err_code!($code));
	}};
}
pub use panic_with;

#[macro_export]
macro_rules! assert_with {
	($code:ident $body:tt, $($key:ident = $value:expr),+ $(,)?) => {{
		let val = $expr;
		if !val {
			return Err($crate::err_code!($code $body, $($key = $value),+));
		}
	}};
	($expr:expr, $code:ident $body:tt) => {{
		let val = $expr;
		if !val {
			return Err($crate::err_code!($code $body));
		}
	}};
	($expr:expr, $code:ident, $($key:ident = $value:expr),+ $(,)?) => {{
		let val = $expr;
		if !val {
			return Err($crate::err_code!($code, $($key = $value),+));
		}
	}};
	($expr:expr, $code:ident) => {{
		let val = $expr;
		if !val {
			return Err($crate::err_code!($code));
		}
	}};
}
pub use assert_with;

#[macro_export]
macro_rules! assert_eq_with {
	($left:expr, $right:expr, $code:ident $body:tt, $($key:ident = $value:expr),+ $(,)?) => {{
		match (&$left, &$right) {
			(val_left, val_right) => {
				if !(*val_left == *val_right) {
					return Err($crate::err_code!($code $body, $($key = $value),+));
				}
			}
		}
	}};
	($left:expr, $right:expr, $code:ident $body:tt) => {{
		match (&$left, &$right) {
			(val_left, val_right) => {
				if !(*val_left == *val_right) {
					return Err($crate::err_code!($code $body));
				}
			}
		}
	}};
	($left:expr, $right:expr, $code:ident, $($key:ident = $value:expr),+ $(,)?) => {{
		match (&$left, &$right) {
			(val_left, val_right) => {
				if !(*val_left == *val_right) {
					return Err($crate::err_code!($code, $($key = $value),+));
				}
			}
		}
	}};
	($left:expr, $right:expr, $code:ident) => {{
		match (&$left, &$right) {
			(val_left, val_right) => {
				if !(*val_left == *val_right) {
					return Err($crate::err_code!($code));
				}
			}
		}
	}};
}
pub use assert_eq_with;

#[macro_export]
macro_rules! assert_ne_with {
	($left:expr, $right:expr, $code:ident $body:tt, $($key:ident = $value:expr),+ $(,)?) => {{
		match (&$left, &$right) {
			(val_left, val_right) => {
				if !(*val_left != *val_right) {
					return Err($crate::err_code!($code $body, $($key = $value),+));
				}
			}
		}
	}};
	($left:expr, $right:expr, $code:ident $body:tt) => {{
		match (&$left, &$right) {
			(val_left, val_right) => {
				if !(*val_left != *val_right) {
					return Err($crate::err_code!($code $body));
				}
			}
		}
	}};
	($left:expr, $right:expr, $code:ident, $($key:ident = $value:expr),+ $(,)?) => {{
		match (&$left, &$right) {
			(val_left, val_right) => {
				if !(*val_left != *val_right) {
					return Err($crate::err_code!($code, $($key = $value),+));
				}
			}
		}
	}};
	($left:expr, $right:expr, $code:ident) => {{
		match (&$left, &$right) {
			(val_left, val_right) => {
				if !(*val_left != *val_right) {
					return Err($crate::err_code!($code));
				}
			}
		}
	}};
}
pub use assert_ne_with;

#[macro_export]
macro_rules! unwrap_with {
	($expr:expr, $code:ident $body:tt, $($key:ident = $value:expr),+ $(,)?) => {{
		$crate::unwrap_with_owned!(&$expr, $code $body, $code, $($key = $value),+)
	}};
	($expr:expr, $code:ident $body:tt) => {{
		$crate::unwrap_with_owned!(&$expr, $code $body)
	}};
	($expr:expr, $code:ident, $($key:ident = $value:expr),+ $(,)?) => {{
		$crate::unwrap_with_owned!(&$expr, $code, $($key = $value),+)

	}};
	($expr:expr, $code:ident) => {{
		$crate::unwrap_with_owned!(&$expr, $code)
	}};
}
pub use unwrap_with;

#[macro_export]
macro_rules! unwrap_with_owned {
	($expr:expr, $code:ident $body:tt, $($key:ident = $value:expr),+ $(,)?) => {{
		#[allow(match_result_ok)]
		if let Some(val) = $expr {
			val
		} else {
			return Err($crate::err_code!($code $body, $($key = $value),+));
		}
	}};
	($expr:expr, $code:ident $body:tt) => {{
		#[allow(match_result_ok)]
		if let Some(val) = $expr {
			val
		} else {
			return Err($crate::err_code!($code $body));
		}
	}};
	($expr:expr, $code:ident, $($key:ident = $value:expr),+ $(,)?) => {{
		#[allow(match_result_ok)]
		if let Some(val) = $expr {
			val
		} else {
			return Err($crate::err_code!($code, $($key = $value),+));
		}
	}};
	($expr:expr, $code:ident) => {{
		#[allow(match_result_ok)]
		if let Some(val) = $expr {
			val
		} else {
			return Err($crate::err_code!($code));
		}
	}};
}
pub use unwrap_with_owned;
