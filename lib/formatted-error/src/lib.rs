mod utils;
include!(concat!(env!("OUT_DIR"), "/gen.rs"));

/// Creates a formatted error from an error code. It is recommended that you use the
/// `err_code` macro to create an error.
pub fn parse(code: &str) -> FormattedError {
	ERROR_REGISTRY
		.get(code)
		.cloned()
		.or_else(|| ERROR_REGISTRY.get(code::UNKNOWN_ERROR).cloned())
		.expect("unknown error should be present")
}
