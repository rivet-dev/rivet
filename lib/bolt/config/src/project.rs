use std::collections::HashMap;

use serde::Deserialize;

pub fn decode(s: &str) -> Result<Project, toml::de::Error> {
	toml::from_str(s)
}

/// Configuration for the Bolt.toml at the root of the project.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Project {
	pub services: HashMap<String, service::ServiceConfig>,
}

pub mod service {
	use serde::Deserialize;
	use std::collections::HashMap;

	#[derive(Deserialize, Clone, Debug)]
	#[serde(rename_all = "kebab-case", deny_unknown_fields)]
	pub struct ServiceConfig {
		#[serde(flatten)]
		pub service: Service,

		pub build: Build,

		#[serde(flatten)]
		pub kind: ServiceKind,

		/// Secrets that need to be exposed for this service.
		#[serde(default)]
		pub secrets: HashMap<String, Secret>,

		pub resources: ServiceResourcesMap,
	}

	#[derive(Deserialize, Clone, Debug)]
	#[serde(rename_all = "kebab-case", deny_unknown_fields)]
	pub struct Build {
		pub package: String,
		pub args: Vec<String>,
	}

	#[derive(Deserialize, Clone, Debug)]
	#[serde(rename_all = "kebab-case", deny_unknown_fields)]
	pub struct Service {
		pub name: String,
		/// The Nomad job priority.
		///
		/// Used if nodes start failing and some services need to be shut down because of capacity
		/// constraints.
		#[serde(default)]
		pub priority: Option<usize>,
		/// If true, this service will do nothing and sleep indefinitely.
		#[serde(default)]
		pub noop: bool,
		/// If this service should only be used for tests.
		#[serde(default)]
		pub test_only: bool,
		/// If this service should only be used for load tests.
		#[serde(default)]
		pub load_test: bool,
	}

	#[derive(Deserialize, Clone, Debug)]
	#[serde(rename_all = "kebab-case", deny_unknown_fields)]
	pub struct Database {}

	#[derive(Deserialize, Clone, Debug)]
	#[serde(rename_all = "kebab-case", deny_unknown_fields)]
	pub struct Secret {
		#[serde(default)]
		pub optional: bool,
	}

	// Externally tagged
	#[derive(Deserialize, Clone, Debug)]
	pub enum ServiceKind {
		#[serde(rename = "headless", rename_all = "kebab-case")]
		Headless { singleton: bool },

		#[serde(rename = "oneshot", rename_all = "kebab-case")]
		Oneshot {},

		#[serde(rename = "periodic", rename_all = "kebab-case")]
		Periodic {
			/// See https://www.nomadproject.io/docs/job-specification/periodic#cron
			cron: String,
		},

		#[serde(rename = "api", rename_all = "kebab-case")]
		Api {
			#[serde(default)]
			disabled: bool,
			#[serde(default)]
			port: Option<u16>,
			singleton: bool,
			router: Option<ServiceRouter>,
		},
	}

	#[derive(Deserialize, Clone, Debug)]
	#[serde(rename_all = "kebab-case", deny_unknown_fields)]
	pub struct ServiceRouter {
		pub mounts: Vec<ServiceMount>,
	}

	#[derive(Deserialize, Clone, Debug)]
	#[serde(rename_all = "kebab-case", deny_unknown_fields)]
	pub struct ServiceMount {
		#[serde(default)]
		pub deprecated: bool,
		#[serde(default)]
		pub subdomain: Option<String>,
		#[serde(default)]
		pub paths: Vec<String>,
		#[serde(default)]
		pub strip_prefix: Option<String>,
		#[serde(default)]
		pub add_path: Option<String>,
	}

	#[derive(Deserialize, Clone, Debug)]
	#[serde(rename_all = "kebab-case", deny_unknown_fields)]
	pub struct ServiceResourcesMap {
		pub single_node: super::super::ns::ServiceResources,
		pub distributed: super::super::ns::ServiceResources,
	}

	impl Service {
		pub fn name_snake(&self) -> String {
			heck::SnakeCase::to_snake_case(self.name.as_str())
		}

		pub fn name_screaming_snake(&self) -> String {
			heck::ShoutySnakeCase::to_shouty_snake_case(self.name.as_str())
		}

		pub fn name_camel_case(&self) -> String {
			heck::CamelCase::to_camel_case(self.name.as_str())
		}

		pub fn name_single_word(&self) -> String {
			self.name.replace('-', "")
		}
	}

	impl ServiceKind {
		/// The service's router used to configure how it's exposed to the world.
		pub fn router(&self) -> Option<&ServiceRouter> {
			match self {
				ServiceKind::Api {
					router: Some(router),
					..
				} => Some(router),
				_ => None,
			}
		}

		/// Determines if the service has a server. This is different than `self.router().is_some()`
		/// because this will be true for any services that are internally-facing HTTP servers, such as
		/// `api-job`.
		pub fn has_server(&self) -> bool {
			matches!(self, ServiceKind::Api { .. })
		}

		pub fn short(&self) -> &str {
			match self {
				ServiceKind::Headless { .. } => "headless",
				ServiceKind::Oneshot { .. } => "oneshot",
				ServiceKind::Periodic { .. } => "periodic",
				ServiceKind::Api { .. } => "api",
			}
		}
	}

	impl Service {
		pub fn priority(&self) -> usize {
			let priority = self.priority.unwrap_or(10);
			assert!(priority <= 100);
			priority
		}
	}
}
