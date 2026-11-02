//! Peer transport for Depot-backed Epoxy replicas.

mod debug;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use axum::{
	Json, Router,
	body::Bytes,
	extract::{Path, State},
	http::StatusCode,
	routing::{get, post},
};
use base64::Engine;
use epoxy_protocol::{OLDEST_PROTOCOL_VERSION, PROTOCOL_VERSION, protocol, versioned};
use rivet_config::Config;

use crate::{
	client::Client,
	depot::replica::Replica,
	membership,
	ops::propose::{Command, CommandKind, Input, Proposal, ProposalResult, SetCommand},
	transport::HttpTransport,
};

#[derive(Clone)]
struct PeerState {
	replica: Replica,
	config: Config,
	client: Client,
	transport: Arc<HttpTransport>,
}

impl PeerState {
	async fn require_membership(&self) -> Result<(), StatusCode> {
		self.client.cluster_config().await.map_err(|error| {
			tracing::warn!(%error, "Epoxy membership is not ready");
			StatusCode::SERVICE_UNAVAILABLE
		})?;
		Ok(())
	}
}

/// Builds routes for the process peer listener without binding or starting tasks.
/// The caller starts discovery explicitly and shuts the client down after draining HTTP.
pub fn router(config: Config, client: Client) -> Result<Router> {
	let state = PeerState {
		replica: client.replica.clone(),
		config,
		transport: client.http_transport()?,
		client,
	};
	Ok(Router::new()
		.route("/v{version}/epoxy/message", post(message))
		.route("/epoxy/protocol-version", get(protocol_version))
		.route("/epoxy/coordinator/state", get(coordinator_state))
		.route("/epoxy/replica/key/{key}", get(debug::key))
		.route("/epoxy/replica/key/{key}/fanout", get(debug::fanout))
		.route("/epoxy/replica/debug", get(replica_debug))
		.route("/epoxy/replica/kv/{key}/local", get(kv_local))
		.route("/epoxy/replica/kv/{key}/optimistic", get(kv_optimistic))
		.route("/epoxy/replica/kv/{key}", get(kv_linearizable).put(kv_set))
		.with_state(state))
}

/// Owns a standalone peer listener for hosts that do not already serve UPS.
#[tracing::instrument(skip_all, fields(replica_id = config.epoxy_replica_id()))]
pub async fn serve(
	config: Config,
	depot: depot_client::Client,
	ups: universalpubsub::PubSub,
	http: Arc<dyn rivet_env::capabilities::http::HttpClient>,
	server: Arc<dyn rivet_env::capabilities::http::HttpServer>,
) -> Result<()> {
	let client = Client::with_depot(&config, depot, http)?;
	let router = router(config.clone(), client.clone())?.merge(ups.router());
	let addr = SocketAddr::from((config.api_peer().host()?, config.api_peer().port()));
	let listener = server.bind(addr).await?;
	tracing::info!(%addr, "Epoxy peer protocol listening");
	client.start_version_discovery().await?;
	ups.mark_listening();
	let shutdown = ups.clone();
	let result = listener
		.serve(
			router,
			Box::pin(async move { shutdown.requests_drained().await }),
		)
		.await;
	client.shutdown().await?;
	result
}

async fn protocol_version(
	State(state): State<PeerState>,
) -> Json<crate::transport::ProtocolVersionResponse> {
	Json(crate::transport::ProtocolVersionResponse {
		protocol_version: state.transport.local_version(),
	})
}

#[derive(serde::Serialize)]
struct CoordinatorStateResponse {
	config: crate::types::ClusterConfig,
}

/// The fixed single-datacenter membership.
async fn coordinator_state(
	State(state): State<PeerState>,
) -> Result<Json<CoordinatorStateResponse>, StatusCode> {
	let config = state.client.cluster_config().await.map_err(|error| {
		tracing::warn!(%error, "failed to inspect Epoxy membership");
		StatusCode::INTERNAL_SERVER_ERROR
	})?;
	Ok(Json(CoordinatorStateResponse {
		config: config.into(),
	}))
}

#[derive(serde::Serialize)]
struct ReplicaDebugResponse {
	config: crate::types::ClusterConfig,
	computed: ReplicaDebugComputed,
}

#[derive(serde::Serialize)]
struct ReplicaDebugComputed {
	this_replica_id: protocol::ReplicaId,
	this_replica_status: Option<crate::types::ReplicaStatus>,
	is_coordinator: bool,
	quorum_members: Vec<protocol::ReplicaId>,
	quorum_sizes: QuorumSizes,
	replica_counts: ReplicaCounts,
}

#[derive(serde::Serialize)]
struct QuorumSizes {
	fast: usize,
	slow: usize,
	all: usize,
	any: usize,
}

#[derive(serde::Serialize)]
struct ReplicaCounts {
	active: usize,
	learning: usize,
	joining: usize,
}

async fn replica_debug(
	State(state): State<PeerState>,
) -> Result<Json<ReplicaDebugResponse>, StatusCode> {
	let config = state.client.cluster_config().await.map_err(|error| {
		tracing::warn!(%error, "failed to inspect Epoxy replica");
		StatusCode::INTERNAL_SERVER_ERROR
	})?;
	let replica_id = state.replica.replica_id();
	let quorum_members: Vec<_> = config
		.replicas
		.iter()
		.filter(|member| member.status == protocol::ReplicaStatus::Active)
		.map(|member| member.replica_id)
		.collect();
	let n = quorum_members.len();
	let slow = n / 2 + 1;
	let computed = ReplicaDebugComputed {
		this_replica_id: replica_id,
		this_replica_status: config
			.replicas
			.iter()
			.find(|member| member.replica_id == replica_id)
			.map(|member| member.status.clone().into()),
		is_coordinator: config.coordinator_replica_id == replica_id,
		quorum_members,
		quorum_sizes: QuorumSizes {
			fast: n - ((slow - 1) / 2),
			slow,
			all: n,
			any: usize::from(n > 0),
		},
		replica_counts: ReplicaCounts {
			active: config
				.replicas
				.iter()
				.filter(|member| member.status == protocol::ReplicaStatus::Active)
				.count(),
			learning: config
				.replicas
				.iter()
				.filter(|member| member.status == protocol::ReplicaStatus::Learning)
				.count(),
			joining: config
				.replicas
				.iter()
				.filter(|member| member.status == protocol::ReplicaStatus::Joining)
				.count(),
		},
	};
	Ok(Json(ReplicaDebugResponse {
		config: config.into(),
		computed,
	}))
}

#[derive(serde::Serialize)]
struct KvResponse {
	exists: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	value: Option<String>,
}

fn decode_key(key: &str) -> Result<Vec<u8>, StatusCode> {
	base64::engine::general_purpose::STANDARD
		.decode(key)
		.map_err(|_| StatusCode::BAD_REQUEST)
}

fn kv_response(value: Option<protocol::CommittedValue>) -> Json<KvResponse> {
	Json(KvResponse {
		exists: value.is_some(),
		value: value
			.and_then(|value| value.value)
			.map(|value| base64::engine::general_purpose::STANDARD.encode(value)),
	})
}

async fn kv_local(
	State(state): State<PeerState>,
	Path(key): Path<String>,
) -> Result<Json<KvResponse>, StatusCode> {
	state.require_membership().await?;
	let key = decode_key(&key)?;
	let value = state
		.replica
		.get(&key)
		.await
		.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
	Ok(kv_response(value))
}

async fn kv_optimistic(
	State(state): State<PeerState>,
	Path(key): Path<String>,
) -> Result<Json<KvResponse>, StatusCode> {
	state.require_membership().await?;
	let key = decode_key(&key)?;
	let value = state
		.client
		.consensus()
		.get(&key)
		.await
		.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
	Ok(kv_response(value))
}

async fn kv_linearizable(
	State(state): State<PeerState>,
	Path(key): Path<String>,
) -> Result<Json<KvResponse>, StatusCode> {
	state.require_membership().await?;
	let key = decode_key(&key)?;
	let value = state
		.client
		.consensus()
		.get(&key)
		.await
		.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
	Ok(kv_response(value))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SetKvRequest {
	value: Option<String>,
}

#[derive(serde::Serialize)]
struct SetKvResponse {
	result: String,
}

async fn kv_set(
	State(state): State<PeerState>,
	Path(key): Path<String>,
	Json(body): Json<SetKvRequest>,
) -> Result<Json<SetKvResponse>, StatusCode> {
	state.require_membership().await?;
	let key = decode_key(&key)?;
	let value = body.value.ok_or(StatusCode::BAD_REQUEST)?;
	let value = base64::engine::general_purpose::STANDARD
		.decode(value)
		.map_err(|_| StatusCode::BAD_REQUEST)?;
	let result = state
		.client
		.propose(Input {
			proposal: Proposal {
				commands: vec![Command {
					kind: CommandKind::SetCommand(SetCommand {
						key,
						value: Some(value),
					}),
				}],
			},
			mutable: false,
		})
		.await
		.map_err(|error| {
			tracing::warn!(%error, "Epoxy debug KV proposal failed");
			StatusCode::SERVICE_UNAVAILABLE
		})?;
	let result = match result {
		ProposalResult::Committed => "committed".to_owned(),
		ProposalResult::ConsensusFailed { reason } => format!("consensus_failed: {reason:?}"),
	};
	Ok(Json(SetKvResponse { result }))
}

#[tracing::instrument(skip(state, body))]
async fn message(
	State(state): State<PeerState>,
	Path(version): Path<u16>,
	body: Bytes,
) -> Result<Vec<u8>, (StatusCode, String)> {
	// Versions below the oldest are released Rivet 2.x layouts, which this build cannot decode.
	if !(OLDEST_PROTOCOL_VERSION..=PROTOCOL_VERSION).contains(&version) {
		return Err((
			StatusCode::BAD_REQUEST,
			"unsupported Epoxy protocol version".into(),
		));
	}
	let request = versioned::decode_request(&body, version)
		.map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
	match &request.kind {
		protocol::RequestKind::HealthCheckRequest
		| protocol::RequestKind::BeginLearningRequest(_)
		| protocol::RequestKind::UpdateConfigRequest(_) => {}
		protocol::RequestKind::PreAcceptRequest(_)
		| protocol::RequestKind::PrepareRequest(_)
		| protocol::RequestKind::AcceptRequest(_)
		| protocol::RequestKind::CommitRequest(_)
		| protocol::RequestKind::KvReadStateRequest(_)
		| protocol::RequestKind::KvGetRequest(_)
		| protocol::RequestKind::CoordinatorUpdateReplicaStatusRequest(_)
		| protocol::RequestKind::ChangelogReadRequest(_) => {
			state
				.require_membership()
				.await
				.map_err(|status| (status, "Epoxy membership is not ready".into()))?;
		}
	}
	let epoxy_topology = membership::epoxy_topology(&state.config).map_err(|_| {
		(
			StatusCode::SERVICE_UNAVAILABLE,
			"Epoxy topology unavailable".into(),
		)
	})?;
	let installed = state.client.cluster_config().await.ok();
	if !installed.as_ref().is_some_and(|config| {
		config
			.replicas
			.iter()
			.any(|r| r.replica_id == request.from_replica_id)
	}) && !epoxy_topology
		.replicas
		.iter()
		.any(|r| r.replica_id == request.from_replica_id)
	{
		return Err((StatusCode::FORBIDDEN, "unknown Epoxy sender".into()));
	}
	let request_type = request_kind_label(&request.kind);
	let timer = crate::metrics::REQUEST_DURATION_SECONDS
		.with_label_values(&[request_type])
		.start_timer();
	let result = state.replica.handle(request).await;

	timer.observe_duration();
	crate::metrics::REQUEST_TOTAL
		.with_label_values(&[request_type, if result.is_ok() { "ok" } else { "err" }])
		.inc();
	let response = result.map_err(|error| {
		tracing::warn!(%error, "Epoxy peer request failed");
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			"Epoxy replica request failed".into(),
		)
	})?;
	versioned::encode_response(response, version)
		.map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))
}

fn request_kind_label(kind: &protocol::RequestKind) -> &'static str {
	match kind {
		protocol::RequestKind::UpdateConfigRequest(_) => "update_config",
		protocol::RequestKind::PrepareRequest(_) => "prepare",
		protocol::RequestKind::PreAcceptRequest(_) => "pre_accept",
		protocol::RequestKind::AcceptRequest(_) => "accept",
		protocol::RequestKind::CommitRequest(_) => "commit",
		protocol::RequestKind::ChangelogReadRequest(_) => "changelog_read",
		protocol::RequestKind::HealthCheckRequest => "health_check",
		protocol::RequestKind::CoordinatorUpdateReplicaStatusRequest(_) => {
			"coordinator_update_replica_status"
		}
		protocol::RequestKind::BeginLearningRequest(_) => "begin_learning",
		protocol::RequestKind::KvGetRequest(_) => "kv_get",
		protocol::RequestKind::KvReadStateRequest(_) => "kv_read_state",
	}
}
