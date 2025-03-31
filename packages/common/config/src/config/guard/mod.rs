use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Configuration structs
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
#[derive(Default)]
pub struct Guard {
	pub http_port: u16, // Port for HTTP traffic
	pub https: Option<Https>, // Optional HTTPS configuration
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
#[derive(Default)]
pub struct Https {
	pub port: u16,      // Port for HTTPS traffic
	pub tls: Tls,       // TLS configuration
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
#[derive(Default)]
pub struct Tls {
	pub cert_dir: PathBuf,
	pub job_cert_path: PathBuf,
	pub job_key_path: PathBuf,
	pub api_cert_path: PathBuf,
	pub api_key_path: PathBuf,
}
