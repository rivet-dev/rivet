use anyhow::*;
use epoxy_protocol::protocol::{ReplicaId, SlotId};
use gas::prelude::*;
use indexmap::IndexMap;
use rivet_api_builder::ApiCtx;
use serde::{Deserialize, Serialize};
use universalpubsub::PublishOpts;

#[derive(Serialize, Deserialize)]
pub struct CachePurgeRequest {
	pub base_key: String,
	pub keys: Vec<rivet_cache::RawCacheKey>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CachePurgeResponse {}

pub async fn cache_purge(
	ctx: ApiCtx,
	_path: (),
	_query: (),
	body: CachePurgeRequest,
) -> Result<CachePurgeResponse> {
	ctx.cache()
		.clone()
		.request()
		.purge(&body.base_key, body.keys)
		.await?;

	Ok(CachePurgeResponse {})
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetTracingConfigRequest {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub filter: Option<Option<String>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub sampler_ratio: Option<Option<f64>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetTracingConfigResponse {}

#[tracing::instrument(skip_all)]
pub async fn set_tracing_config(
	ctx: ApiCtx,
	_path: (),
	_query: (),
	body: SetTracingConfigRequest,
) -> Result<SetTracingConfigResponse> {
	// Broadcast message to all services via UPS
	let subject = "rivet.debug.tracing.config";
	let message = serde_json::to_vec(&body)?;

	ctx.ups()?
		.publish(subject, &message, PublishOpts::broadcast())
		.await?;

	tracing::info!(
		filter = ?body.filter,
		sampler_ratio = ?body.sampler_ratio,
		"broadcasted tracing config update"
	);

	Ok(SetTracingConfigResponse {})
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplicaReconfigureRequest {}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplicaReconfigureResponse {}

/// Triggers the epoxy coordinator to reconfigure all replicas.
///
/// Useful when a replica's configuration is outdated for any reason and needs to be re-notified of
/// changes.
///
/// This should never need to be called manually if everything is operating correctly.
pub async fn epoxy_replica_reconfigure(
	ctx: ApiCtx,
	_path: (),
	_query: (),
	_body: ReplicaReconfigureRequest,
) -> Result<ReplicaReconfigureResponse> {
	if ctx.config().is_leader() {
		ctx.signal(epoxy::workflows::coordinator::ReplicaReconfigure {})
			.to_workflow::<epoxy::workflows::coordinator::Workflow>()
			.tag("replica", ctx.config().epoxy_replica_id())
			.send()
			.await?;
	}

	Ok(ReplicaReconfigureResponse {})
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetEpoxyStateResponse {
	pub config: epoxy::types::ClusterConfig,
}

/// Returns the current epoxy coordinator cluster configuration.
///
/// Useful for inspecting the current state of the epoxy cluster, including all replicas and their statuses.
pub async fn get_epoxy_state(ctx: ApiCtx, _path: (), _query: ()) -> Result<GetEpoxyStateResponse> {
	let workflow_id = ctx
		.find_workflow::<epoxy::workflows::coordinator::Workflow>((
			"replica",
			ctx.config().epoxy_replica_id(),
		))
		.await?
		.ok_or_else(|| anyhow!("epoxy coordinator workflow not found"))?;

	let wfs = ctx.get_workflows(vec![workflow_id]).await?;
	let wf = wfs.first().ok_or_else(|| anyhow!("workflow not found"))?;

	let state: epoxy::workflows::coordinator::State =
		wf.parse_state().context("failed to parse workflow state")?;

	Ok(GetEpoxyStateResponse {
		config: state.config,
	})
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetEpoxyStateRequest {
	pub config: epoxy::types::ClusterConfig,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetEpoxyStateResponse {}

/// Overrides the epoxy coordinator cluster configuration and triggers reconfiguration.
///
/// Useful for manually adjusting the cluster state in case the replica status drifts from the
/// state in the coordinator. This will automatically trigger a reconfigure when called.
///
/// This should never need to be called manually if everything is operating correctly.
pub async fn set_epoxy_state(
	ctx: ApiCtx,
	_path: (),
	_query: (),
	body: SetEpoxyStateRequest,
) -> Result<SetEpoxyStateResponse> {
	ensure!(
		body.config.coordinator_replica_id == ctx.config().epoxy_replica_id(),
		"config coordinator_replica_id ({}) does not match current replica id ({})",
		body.config.coordinator_replica_id,
		ctx.config().epoxy_replica_id()
	);

	if ctx.config().is_leader() {
		ctx.signal(epoxy::workflows::coordinator::OverrideState {
			config: body.config,
		})
		.to_workflow::<epoxy::workflows::coordinator::Workflow>()
		.tag("replica", ctx.config().epoxy_replica_id())
		.send()
		.await?;
	}

	Ok(SetEpoxyStateResponse {})
}

#[derive(Serialize, Deserialize)]
pub struct GetEpoxyReplicaDebugResponse {
	pub config: epoxy::types::ClusterConfig,
	pub computed: EpoxyReplicaDebugComputed,
	pub state: EpoxyReplicaDebugState,
}

#[derive(Serialize, Deserialize)]
pub struct EpoxyReplicaDebugComputed {
	pub this_replica_id: ReplicaId,
	pub this_replica_status: Option<epoxy::types::ReplicaStatus>,
	pub is_coordinator: bool,
	pub quorum_members: Vec<ReplicaId>,
	pub quorum_sizes: EpoxyQuorumSizes,
	pub replica_counts: EpoxyReplicaCounts,
}

#[derive(Serialize, Deserialize)]
pub struct EpoxyQuorumSizes {
	pub fast: usize,
	pub slow: usize,
	pub all: usize,
	pub any: usize,
}

#[derive(Serialize, Deserialize)]
pub struct EpoxyReplicaCounts {
	pub active: usize,
	pub learning: usize,
	pub joining: usize,
}

#[derive(Serialize, Deserialize)]
pub struct EpoxyReplicaDebugState {
	pub ballot: EpoxyBallot,
	pub instance_number: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EpoxyBallot {
	pub epoch: u64,
	pub ballot: u64,
	pub replica_id: ReplicaId,
}

/// Returns debug information for this epoxy replica including cluster config, computed values, and state.
///
/// This reads from the replica's local UDB state, which may differ from the coordinator's
/// view if the replica hasn't been reconfigured yet.
pub async fn get_epoxy_replica_debug(
	ctx: ApiCtx,
	_path: (),
	_query: (),
) -> Result<GetEpoxyReplicaDebugResponse> {
	let replica_id = ctx.config().epoxy_replica_id();

	let (config, ballot, instance_number) = ctx
		.udb()?
		.run(|tx| async move {
			let config = epoxy::utils::read_config(&tx, replica_id).await?;
			let ballot = epoxy::replica::ballot::get_ballot(&tx, replica_id).await?;
			let instance_number = epoxy::utils::read_instance_number(&tx, replica_id).await?;
			Result::Ok((config, ballot, instance_number))
		})
		.await?;

	// Compute derived values from config
	let this_replica_status = config
		.replicas
		.iter()
		.find(|r| r.replica_id == replica_id)
		.map(|r| epoxy::types::ReplicaStatus::from(r.status.clone()));
	let is_coordinator = config.coordinator_replica_id == replica_id;

	let quorum_members = epoxy::utils::get_quorum_members(&config);
	let quorum_member_count = quorum_members.len();

	let quorum_sizes = EpoxyQuorumSizes {
		fast: epoxy::utils::calculate_quorum(quorum_member_count, epoxy::utils::QuorumType::Fast),
		slow: epoxy::utils::calculate_quorum(quorum_member_count, epoxy::utils::QuorumType::Slow),
		all: epoxy::utils::calculate_quorum(quorum_member_count, epoxy::utils::QuorumType::All),
		any: epoxy::utils::calculate_quorum(quorum_member_count, epoxy::utils::QuorumType::Any),
	};

	let replica_counts = EpoxyReplicaCounts {
		active: config
			.replicas
			.iter()
			.filter(|r| matches!(r.status, epoxy_protocol::protocol::ReplicaStatus::Active))
			.count(),
		learning: config
			.replicas
			.iter()
			.filter(|r| matches!(r.status, epoxy_protocol::protocol::ReplicaStatus::Learning))
			.count(),
		joining: config
			.replicas
			.iter()
			.filter(|r| matches!(r.status, epoxy_protocol::protocol::ReplicaStatus::Joining))
			.count(),
	};

	Ok(GetEpoxyReplicaDebugResponse {
		config: config.into(),
		computed: EpoxyReplicaDebugComputed {
			this_replica_id: replica_id,
			this_replica_status,
			is_coordinator,
			quorum_members,
			quorum_sizes,
			replica_counts,
		},
		state: EpoxyReplicaDebugState {
			ballot: EpoxyBallot {
				epoch: ballot.epoch,
				ballot: ballot.ballot,
				replica_id: ballot.replica_id,
			},
			instance_number,
		},
	})
}

// MARK: Key debug
#[derive(Serialize, Deserialize, Clone)]
pub struct EpoxyKeyDebugPath {
	pub key: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetEpoxyKeyDebugResponse {
	pub replica_id: ReplicaId,
	pub key: String,
	pub instances: Vec<EpoxyKeyInstance>,
	pub instances_by_status: IndexMap<String, usize>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EpoxyKeyInstance {
	pub replica_id: ReplicaId,
	pub slot_id: SlotId,
	pub log_entry: Option<EpoxyKeyLogEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EpoxyKeyLogEntry {
	pub status: String,
	pub ballot: EpoxyBallot,
	pub seq: u64,
	pub deps: Vec<EpoxyInstance>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EpoxyInstance {
	pub replica_id: ReplicaId,
	pub slot_id: SlotId,
}

/// Returns debug information for a specific key on this replica.
///
/// Shows all instances that have touched this key and their log entry states.
pub async fn get_epoxy_key_debug(
	ctx: ApiCtx,
	path: EpoxyKeyDebugPath,
	_query: (),
) -> Result<GetEpoxyKeyDebugResponse> {
	let replica_id = ctx.config().epoxy_replica_id();

	// Decode key from base64
	let key_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &path.key)
		.context("invalid base64 key")?;

	let instances = ctx
		.udb()?
		.run(|tx| {
			let key_bytes = key_bytes.clone();
			async move {
				let protocol_instances =
					epoxy::utils::read_key_instances(&tx, replica_id, key_bytes).await?;

				let mut instances = Vec::new();
				for instance in protocol_instances {
					let log_entry =
						epoxy::utils::read_log_entry(&tx, replica_id, &instance).await?;
					instances.push(EpoxyKeyInstance {
						replica_id: instance.replica_id,
						slot_id: instance.slot_id,
						log_entry: log_entry.map(|entry| EpoxyKeyLogEntry {
							status: format!("{:?}", entry.state),
							ballot: EpoxyBallot {
								epoch: entry.ballot.epoch,
								ballot: entry.ballot.ballot,
								replica_id: entry.ballot.replica_id,
							},
							seq: entry.seq,
							deps: entry
								.deps
								.into_iter()
								.map(|d| EpoxyInstance {
									replica_id: d.replica_id,
									slot_id: d.slot_id,
								})
								.collect(),
						}),
					});
				}

				Result::Ok(instances)
			}
		})
		.await?;

	// Compute instances by status
	let mut instances_by_status = IndexMap::new();
	for instance in &instances {
		let status = instance
			.log_entry
			.as_ref()
			.map(|e| e.status.clone())
			.unwrap_or_else(|| "Unknown".to_string());
		*instances_by_status.entry(status).or_insert(0) += 1;
	}

	Ok(GetEpoxyKeyDebugResponse {
		replica_id,
		key: path.key,
		instances,
		instances_by_status,
	})
}

#[derive(Serialize, Deserialize)]
pub struct GetEpoxyKeyDebugFanoutResponse {
	pub replicas: IndexMap<ReplicaId, GetEpoxyKeyDebugResponse>,
	pub errors: IndexMap<ReplicaId, String>,
}

/// Returns debug information for a specific key across all replicas in the cluster.
///
/// Fans out to all replicas and aggregates their responses.
pub async fn get_epoxy_key_debug_fanout(
	ctx: ApiCtx,
	path: EpoxyKeyDebugPath,
	_query: (),
) -> Result<GetEpoxyKeyDebugFanoutResponse> {
	let replica_id = ctx.config().epoxy_replica_id();

	// Get cluster config to find all replicas
	let config: epoxy::types::ClusterConfig = ctx
		.udb()?
		.run(|tx| async move {
			let config = epoxy::utils::read_config(&tx, replica_id).await?;
			Result::Ok(config.into())
		})
		.await?;

	// Get local response first
	let local_response = get_epoxy_key_debug(ctx.clone(), path.clone(), ()).await?;

	let mut replicas = IndexMap::new();
	let mut errors = IndexMap::new();

	// Add local response
	replicas.insert(replica_id, local_response);

	// Fan out to other replicas
	let client = rivet_pools::reqwest::client().await?;

	for replica_config in &config.replicas {
		if replica_config.replica_id == replica_id {
			continue;
		}

		let url = format!(
			"{}/epoxy/replica/key/{}",
			replica_config.api_peer_url, path.key
		);

		let response_result = client.get(&url).send().await;
		match response_result {
			std::result::Result::Ok(response) => {
				if response.status().is_success() {
					match response.json::<GetEpoxyKeyDebugResponse>().await {
						std::result::Result::Ok(resp) => {
							replicas.insert(replica_config.replica_id, resp);
						}
						std::result::Result::Err(e) => {
							errors.insert(
								replica_config.replica_id,
								format!("failed to parse response: {}", e),
							);
						}
					}
				} else {
					errors.insert(
						replica_config.replica_id,
						format!("request failed with status: {}", response.status()),
					);
				}
			}
			std::result::Result::Err(e) => {
				errors.insert(replica_config.replica_id, format!("request failed: {}", e));
			}
		}
	}

	Ok(GetEpoxyKeyDebugFanoutResponse { replicas, errors })
}

// MARK: KV get/set
#[derive(Serialize, Deserialize)]
pub struct GetEpoxyKvResponse {
	pub exists: bool,
	/// Base64-encoded value bytes
	#[serde(skip_serializing_if = "Option::is_none")]
	pub value: Option<String>,
}

/// Gets a value from epoxy KV (local replica only). Key and value are base64-encoded.
pub async fn get_epoxy_kv_local(
	ctx: ApiCtx,
	path: EpoxyKeyDebugPath,
	_query: (),
) -> Result<GetEpoxyKvResponse> {
	use base64::Engine;

	let replica_id = ctx.config().epoxy_replica_id();

	let key_bytes = base64::engine::general_purpose::STANDARD
		.decode(&path.key)
		.context("invalid base64 key")?;

	let output = ctx
		.op(epoxy::ops::kv::get_local::Input {
			replica_id,
			key: key_bytes,
		})
		.await?;

	Ok(GetEpoxyKvResponse {
		exists: output.value.is_some(),
		value: output
			.value
			.map(|v| base64::engine::general_purpose::STANDARD.encode(&v)),
	})
}

/// Gets a value from epoxy KV with optimistic read (local + cache + fanout).
/// Key and value are base64-encoded.
pub async fn get_epoxy_kv_optimistic(
	ctx: ApiCtx,
	path: EpoxyKeyDebugPath,
	_query: (),
) -> Result<GetEpoxyKvResponse> {
	use base64::Engine;

	let replica_id = ctx.config().epoxy_replica_id();

	let key_bytes = base64::engine::general_purpose::STANDARD
		.decode(&path.key)
		.context("invalid base64 key")?;

	let output = ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id,
			key: key_bytes,
		})
		.await?;

	Ok(GetEpoxyKvResponse {
		exists: output.value.is_some(),
		value: output
			.value
			.map(|v| base64::engine::general_purpose::STANDARD.encode(&v)),
	})
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetEpoxyKvRequest {
	/// Base64-encoded value bytes. If null, deletes the key.
	pub value: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SetEpoxyKvResponse {
	pub result: String,
}

/// Sets a value in epoxy KV through consensus. Key and value are base64-encoded.
pub async fn set_epoxy_kv(
	ctx: ApiCtx,
	path: EpoxyKeyDebugPath,
	_query: (),
	body: SetEpoxyKvRequest,
) -> Result<SetEpoxyKvResponse> {
	use base64::Engine;
	use epoxy_protocol::protocol;

	let key_bytes = base64::engine::general_purpose::STANDARD
		.decode(&path.key)
		.context("invalid base64 key")?;

	let value_bytes = body
		.value
		.map(|v| {
			base64::engine::general_purpose::STANDARD
				.decode(&v)
				.context("invalid base64 value")
		})
		.transpose()?;

	let result = ctx
		.op(epoxy::ops::propose::Input {
			proposal: protocol::Proposal {
				commands: vec![protocol::Command {
					kind: protocol::CommandKind::SetCommand(protocol::SetCommand {
						key: key_bytes,
						value: value_bytes,
					}),
				}],
			},
			purge_cache: true,
		})
		.await?;

	let result_str = match result {
		epoxy::ops::propose::ProposalResult::Committed => "committed".to_string(),
		epoxy::ops::propose::ProposalResult::ConsensusFailed => "consensus_failed".to_string(),
		epoxy::ops::propose::ProposalResult::CommandError(err) => {
			format!("command_error: {:?}", err)
		}
	};

	Ok(SetEpoxyKvResponse { result: result_str })
}
