use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessMetadataEnvoy {
	pub version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessMetadataRunner {
	pub version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessMetadataPayload {
	pub runtime: String,
	pub version: String,
	#[serde(rename = "envoyProtocolVersion")]
	pub envoy_protocol_version: Option<u16>,
	#[serde(rename = "actorNames", default)]
	pub actor_names: HashMap<String, ActorName>,
	pub envoy: Option<ServerlessMetadataEnvoy>,
	pub runner: Option<ServerlessMetadataRunner>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorName {
	pub metadata: Option<serde_json::Value>,
}
