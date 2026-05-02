use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerlessMetadataEnvoy {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub kind: Option<ServerlessMetadataEnvoyKind>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerlessMetadataEnvoyKind {
	#[serde(rename = "serverless")]
	Serverless {},
	#[serde(rename = "normal")]
	Normal {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessMetadataRunner {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerlessMetadataPayload {
	pub runtime: String,
	pub version: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub envoy_protocol_version: Option<u16>,
	#[serde(default)]
	pub actor_names: HashMap<String, ActorName>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub envoy: Option<ServerlessMetadataEnvoy>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub runner: Option<ServerlessMetadataRunner>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub client_endpoint: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub client_namespace: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub client_token: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorName {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

/// Typed shape stored under the actor metadata `preload` key.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerlessActorPreload {
	#[serde(default)]
	pub keys: Vec<Vec<u8>>,
	#[serde(default)]
	pub prefixes: Vec<ServerlessActorPreloadPrefix>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerlessActorPreloadPrefix {
	pub prefix: Vec<u8>,
	pub max_bytes: u64,
	#[serde(default)]
	pub partial: bool,
}
