use lazy_static::lazy_static;
use std::env;
use uuid::Uuid;

lazy_static! {
	static ref SERVICE_NAME: String =
		env::var("RIVET_SERVICE_NAME").unwrap_or_else(|_| "rivet".to_string());
	static ref SERVICE_INSTANCE: String =
		env::var("RIVET_SERVICE_INSTANCE").unwrap_or_else(|_| Uuid::new_v4().to_string());
	static ref SOURCE_HASH: String =
		env::var("RIVET_SOURCE_HASH").unwrap_or_else(|_| "unknown".to_string());
}

/// Generic name used to differentiate pools of servers.
pub fn service_name() -> &'static str {
	&SERVICE_NAME
}

/// Unique string for this instance of the service.
pub fn service_instance() -> &'static str {
	&SERVICE_INSTANCE
}

/// Returns the source hash. Used to identify if the source code has changed in order to purge
/// caches.
///
/// This will use the `RIVET_SOURCE_HASH` and fall back to the Git hash otherwise.
pub fn source_hash() -> &'static str {
	&SOURCE_HASH
}
