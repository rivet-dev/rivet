use global_error::prelude::*;
use serde::{Deserialize, Serialize};

pub mod server;

pub use server::*;

// IMPORTANT:
//
// Do not use Vec unless it is `Vec<String>`. Use a `HashMap` instead.
//
// This is because all values need to be able to be configured using environment variables.
// config-rs can only parse `Vec<String>` from the environment.

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Root {
	// We store all configs under `client` in order to prevent the config from being mixed up with
	// the server config.
	#[serde(default)]
	pub server: Option<server::Server>,
}

impl Default for Root {
	fn default() -> Self {
		Self {
			server: Some(server::Server::default()),
		}
	}
}

impl Root {
	pub fn server(&self) -> GlobalResult<&server::Server> {
		Ok(unwrap_ref!(self.server, "missing server config"))
	}
}
