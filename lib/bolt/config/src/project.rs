use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;

pub fn decode(s: &str) -> Result<Project, toml::de::Error> {
	toml::from_str(s)
}

/// Configuration for the Bolt.toml at the root of the project.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Project {
	#[serde(default)]
	pub additional_roots: HashMap<String, AdditionalRoot>,
	// pub services: Services,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdditionalRoot {
	pub path: PathBuf,
}

// #[derive(Clone, Debug, Deserialize)]
// #[serde(deny_unknown_fields)]
// pub struct Services {
// 	pub sql: Vec<SqlService>,
// 	pub buckets: Vec<S3Bucket>,
// 	pub redis: Vec<RedisService>,
// }
//
// #[derive(Clone, Debug, Deserialize)]
// #[serde(deny_unknown_fields)]
// pub struct SqlService {
// 	pub kind: SqlServiceKind,
// 	pub path: String,
// 	pub db_name: String,
// }
//
// #[derive(Deserialize, Clone, Debug)]
// pub enum SqlServiceKind {
// 	#[serde(rename = "cockroachdb")]
// 	CockroachDB,
// 	#[serde(rename = "clickhouse")]
// 	ClickHouse,
// }
//
// #[derive(Clone, Debug, Deserialize)]
// #[serde(deny_unknown_fields)]
// pub struct S3Bucket {
// 	pub name: String,
// }
//
// #[derive(Clone, Debug, Deserialize)]
// #[serde(deny_unknown_fields)]
// pub struct RedisService {
// 	pub name: String,
// 	pub ephemeral: bool,
// }
