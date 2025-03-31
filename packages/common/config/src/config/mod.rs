use global_error::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod server;
pub mod guard;

pub use server::*;
pub use guard::*;

// IMPORTANT:
//
// Do not use Vec unless it is `Vec<String>`. Use a `HashMap` instead.
//
// This is because all values need to be able to be configured using environment variables.
// config-rs can only parse `Vec<String>` from the environment.

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Root {
	// We store all configs under `server` in order to prevent the config from being mixed up with
	// the client config.
	#[serde(default)]
	pub server: Option<server::Server>,

	#[serde(default)]
	pub guard: Option<guard::Guard>,
}

impl Default for Root {
	fn default() -> Self {
		Self {
			server: Some(server::Server::default()),
			guard: None,
		}
	}
}

impl Root {
	pub fn server(&self) -> GlobalResult<&server::Server> {
		Ok(unwrap_ref!(self.server, "missing server config"))
	}

	pub fn guard(&self) -> GlobalResult<&guard::Guard> {
		Ok(unwrap_ref!(self.guard, "missing server config"))
	}
}
