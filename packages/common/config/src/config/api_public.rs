use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Configuration for the public API service.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApiPublic {
	/// Flag to enable verbose error reporting.
	pub verbose_errors: Option<bool>,
	/// Flag to respect the X-Forwarded-For header for client IP addresses.
	///
	/// Will be ignored in favor of CF-Connecting-IP if DNS provider is
	/// configured as Cloudflare.
	pub respect_forwarded_for: Option<bool>,
}

impl ApiPublic {
	pub fn verbose_errors(&self) -> bool {
		self.verbose_errors.unwrap_or(true)
	}

	pub fn respect_forwarded_for(&self) -> bool {
		self.respect_forwarded_for.unwrap_or(false)
	}
}
