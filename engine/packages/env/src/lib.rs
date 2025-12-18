use lazy_static::lazy_static;
use std::env;

lazy_static! {
	static ref SERVICE_NAME: String =
		env::var("RIVET_SERVICE_NAME").unwrap_or_else(|_| "rivet".to_string());
}

/// Generic name used to differentiate pools of servers.
pub fn service_name() -> &'static str {
	&SERVICE_NAME
}
