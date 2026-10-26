//! The existing peer inspection surface, backed by Depot records.

use super::{PeerState, decode_key};
use axum::{
	Json,
	extract::{Path, State},
	http::StatusCode,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
pub(super) struct KeyResponse {
	replica_id: u64,
	key: String,
	state: KeyState,
}

#[derive(Serialize, Deserialize)]
struct KeyState {
	#[serde(skip_serializing_if = "Option::is_none")]
	current_ballot: Option<Ballot>,
	#[serde(skip_serializing_if = "Option::is_none")]
	accepted: Option<AcceptedValue>,
	#[serde(skip_serializing_if = "Option::is_none")]
	committed: Option<CommittedValue>,
}

#[derive(Serialize, Deserialize)]
struct Ballot {
	counter: u64,
	replica_id: u64,
}

impl From<epoxy_protocol::protocol::Ballot> for Ballot {
	fn from(value: epoxy_protocol::protocol::Ballot) -> Self {
		Self {
			counter: value.counter,
			replica_id: value.replica_id,
		}
	}
}

#[derive(Serialize, Deserialize)]
struct AcceptedValue {
	value: Option<String>,
	ballot: Ballot,
	version: u64,
	mutable: bool,
}

#[derive(Serialize, Deserialize)]
struct CommittedValue {
	value: Option<String>,
	version: u64,
	mutable: bool,
}

fn encode(value: Option<Vec<u8>>) -> Option<String> {
	value.map(|v| base64::engine::general_purpose::STANDARD.encode(v))
}

pub(super) async fn key(
	State(state): State<PeerState>,
	Path(key): Path<String>,
) -> Result<Json<KeyResponse>, StatusCode> {
	let bytes = decode_key(&key)?;
	let record = state.replica.inspect(&bytes).await.map_err(|error| {
		tracing::warn!(%error, "failed to inspect Epoxy key");
		StatusCode::INTERNAL_SERVER_ERROR
	})?;
	Ok(Json(KeyResponse {
		replica_id: state.replica.replica_id(),
		key,
		state: KeyState {
			current_ballot: record.promised.map(Into::into),
			accepted: record.accepted.map(|v| AcceptedValue {
				value: encode(v.value.map(Into::into)),
				ballot: v.ballot.into(),
				version: v.version,
				mutable: v.mutable,
			}),
			committed: record.committed.map(|v| CommittedValue {
				value: encode(v.value.map(Into::into)),
				version: v.version,
				mutable: v.mutable,
			}),
		},
	}))
}

#[derive(Serialize)]
pub(super) struct FanoutResponse {
	replicas: BTreeMap<u64, KeyResponse>,
	errors: BTreeMap<u64, String>,
}

pub(super) async fn fanout(
	State(state): State<PeerState>,
	Path(key_name): Path<String>,
) -> Result<Json<FanoutResponse>, StatusCode> {
	let config = state
		.client
		.cluster_config()
		.await
		.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
	let local = key(State(state.clone()), Path(key_name.clone())).await?.0;
	let mut result = FanoutResponse {
		replicas: [(local.replica_id, local)].into(),
		errors: BTreeMap::new(),
	};
	let replies = futures_util::future::join_all(
		config
			.replicas
			.iter()
			.filter(|r| r.replica_id != state.replica.replica_id())
			.map(|replica| async {
				let response: anyhow::Result<KeyResponse> = async {
					let mut url = url::Url::parse(&replica.api_peer_url)?;
					url.path_segments_mut()
						.map_err(|_| anyhow::anyhow!("invalid Epoxy peer URL"))?
						.clear()
						.extend(["epoxy", "replica", "key", &key_name]);
					let response = state
						.transport
						.http(
							rivet_env::capabilities::http::Method::GET,
							url,
							Vec::new(),
							crate::consts::REQUEST_TIMEOUT,
						)
						.await?;
					Ok(serde_json::from_slice(&response)?)
				}
				.await;
				(replica.replica_id, response)
			}),
	)
	.await;
	for (id, reply) in replies {
		match reply {
			Ok(value) => {
				result.replicas.insert(id, value);
			}
			Err(error) => {
				result.errors.insert(id, error.to_string());
			}
		}
	}
	Ok(Json(result))
}
