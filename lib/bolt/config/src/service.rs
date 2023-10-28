use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn decode(s: &str) -> Result<ServiceConfig, toml::de::Error> {
	toml::from_str(s)
}

/// Generalizes the runtime and service kinds in to larger groups. Services in a general group
/// behave similarly. The service class must match between the runtime and service kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentClass {
	Executable,
	NonExecutable,
	Database,
	Cache,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ServiceConfig {
	pub service: Service,

	#[serde(flatten)]
	pub kind: ServiceKind,

	pub runtime: RuntimeKind,

	/// Database dependencies that need a pool for this service.
	#[serde(default)]
	pub databases: HashMap<String, Database>,

	/// Secrets that need to be exposed for this service.
	#[serde(default)]
	pub secrets: HashMap<String, Secret>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Service {
	pub name: String,
	#[serde(default)]
	pub regions: Option<Vec<String>>,
	/// If this service needs to be booted no matter what service is running.
	#[serde(default)]
	pub essential: bool,
	/// The Nomad job priority.
	///
	/// Used if nodes start failing and some services need to be shut down because of capacity
	/// constraints.
	#[serde(default)]
	pub priority: Option<usize>,
	/// If this service should only be used for tests.
	#[serde(default)]
	pub test_only: bool,
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
	Headless {
		#[serde(default = "defaults::singleton")]
		singleton: bool,
	},

	#[serde(rename = "oneshot", rename_all = "kebab-case")]
	Oneshot {},

	#[serde(rename = "periodic", rename_all = "kebab-case")]
	Periodic {
		/// See https://www.nomadproject.io/docs/job-specification/periodic#cron
		cron: String,
		/// See https://www.nomadproject.io/docs/job-specification/periodic#cron
		#[serde(default = "defaults::periodic_prohibit_overlap")]
		prohibit_overlap: bool,
		/// See https://www.nomadproject.io/docs/job-specification/periodic#cron
		#[serde(default = "defaults::periodic_time_zone")]
		time_zone: String,
	},

	#[serde(rename = "operation")]
	Operation {},

	#[serde(rename = "consumer")]
	Consumer {},

	#[serde(rename = "api", rename_all = "kebab-case")]
	Api {
		#[serde(default)]
		port: Option<u16>,
		#[serde(default = "defaults::singleton")]
		singleton: bool,
		router: Option<ServiceRouter>,
	},

	#[serde(rename = "static", rename_all = "kebab-case")]
	Static { router: ServiceRouter },

	#[serde(rename = "database")]
	Database {},

	#[serde(rename = "cache")]
	Cache {},
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "kind")]
pub enum RuntimeKind {
	#[serde(rename = "rust")]
	Rust {},
	#[serde(rename = "crdb")]
	CRDB {},
	#[serde(rename = "clickhouse")]
	ClickHouse {},
	#[serde(rename = "redis")]
	Redis { persistent: bool },
	#[serde(rename = "s3")]
	S3 { upload_policy: UploadPolicy },
	#[serde(rename = "nats")]
	Nats {},
}

// TODO: Unused atm, we need different options to reflect CORS policy for B2.
// TODO: Only `Upload` is supported atm in prod
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum UploadPolicy {
	None,
	Download,
	Public,
	Upload,
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
	pub path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum ServiceDomain {
	Base,
	BaseGame,
	BaseJob,
}

impl Default for ServiceDomain {
	fn default() -> Self {
		Self::Base
	}
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct CargoConfig {
	pub package: CargoPackage,
	pub dependencies: HashMap<String, CargoDependency>,
	#[serde(default)]
	pub dev_dependencies: HashMap<String, CargoDependency>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct CargoPackage {
	pub name: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged, rename_all = "kebab-case")]
pub enum CargoDependency {
	Path { path: String },
	Unknown(serde_json::Value),
}

mod defaults {
	pub fn singleton() -> bool {
		false
	}

	pub fn periodic_prohibit_overlap() -> bool {
		true
	}

	pub fn periodic_time_zone() -> String {
		"UTC".to_owned()
	}
}

impl ServiceConfig {
	pub fn component_class(&self) -> ComponentClass {
		self.kind.component_class()
	}
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

impl RuntimeKind {
	pub fn short(&self) -> &str {
		match self {
			RuntimeKind::Rust { .. } => "rust",
			RuntimeKind::CRDB { .. } => "crdb",
			RuntimeKind::ClickHouse { .. } => "clickhouse",
			RuntimeKind::Redis { .. } => "redis",
			RuntimeKind::S3 { .. } => "s3",
			RuntimeKind::Nats { .. } => "nats",
		}
	}

	pub fn supports_component_class(&self, component_class: &ComponentClass) -> bool {
		match (self, component_class) {
			(RuntimeKind::Rust { .. }, ComponentClass::Executable) => true,
			(RuntimeKind::Rust { .. }, ComponentClass::NonExecutable) => true,
			(
				RuntimeKind::CRDB { .. } | RuntimeKind::ClickHouse { .. } | RuntimeKind::S3 { .. },
				ComponentClass::Database,
			) => true,
			(RuntimeKind::Redis { .. }, ComponentClass::Cache) => true,
			_ => false,
		}
	}
}

impl ServiceKind {
	/// The service's router used to configure how it's exposed to the world.
	pub fn router(&self) -> Option<&ServiceRouter> {
		if let ServiceKind::Api {
			router: Some(router),
			..
		}
		| ServiceKind::Static { router } = self
		{
			Some(router)
		} else {
			None
		}
	}

	/// Determines if the service has a server. This is different than `self.router().is_some()`
	/// because this will be true for any services that are internally-facing HTTP servers, such as
	/// `api-job`.
	pub fn has_server(&self) -> bool {
		if let ServiceKind::Api { .. } | ServiceKind::Static { .. } = self {
			true
		} else {
			false
		}
	}

	pub fn short(&self) -> &str {
		match self {
			ServiceKind::Headless { .. } => "headless",
			ServiceKind::Oneshot { .. } => "oneshot",
			ServiceKind::Periodic { .. } => "periodic",
			ServiceKind::Operation { .. } => "operation",
			ServiceKind::Consumer { .. } => "consumer",
			ServiceKind::Api { .. } => "api",
			ServiceKind::Static { .. } => "static",
			ServiceKind::Database { .. } => "database",
			ServiceKind::Cache { .. } => "cache",
		}
	}

	pub fn component_class(&self) -> ComponentClass {
		match self {
			ServiceKind::Headless { .. }
			| ServiceKind::Oneshot { .. }
			| ServiceKind::Periodic { .. }
			| ServiceKind::Consumer { .. }
			| ServiceKind::Api { .. }
			| ServiceKind::Static { .. } => ComponentClass::Executable,
			ServiceKind::Operation { .. } => ComponentClass::NonExecutable,
			ServiceKind::Database { .. } => ComponentClass::Database,
			ServiceKind::Cache { .. } => ComponentClass::Cache,
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
