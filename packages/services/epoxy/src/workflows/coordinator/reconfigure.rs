use anyhow::*;
use epoxy_protocol::protocol::{self, ReplicaId};
use gas::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types;

use super::State;

pub async fn reconfigure(ctx: &mut WorkflowCtx) -> Result<()> {
	// Check for config changes
	let config_change = ctx.activity(CheckConfigChangesInput {}).await?;

	if let Some(config_change) = config_change {
		// Health check new replicas
		//
		// This will wait for the replica to come online or cancel if the config changes
		let proceed = ctx
			.activity(HealthCheckNewReplicasInput {
				new_replicas: config_change.new_replicas.clone(),
			})
			.await?;
		if !proceed {
			return Ok(());
		}

		// Add replicas as joining state (no epoch increment)
		ctx.activity(AddReplicasAsJoiningInput {
			new_replicas: config_change.new_replicas.clone(),
		})
		.await?;

		// Send begin learning message to new replicas
		let proceed = ctx
			.activity(SendBeginLearningInput {
				new_replicas: config_change.new_replicas.clone(),
				config: config_change.config.clone(),
			})
			.await?;
		if !proceed {
			return Ok(());
		}
	}

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct CheckConfigChangesInput {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
	/// New config to use after changes have applied.
	pub config: types::ClusterConfig,
	/// New replicas to join.
	pub new_replicas: Vec<types::ReplicaConfig>,
}

#[activity(CheckConfigChanges)]
pub async fn check_config_changes(
	ctx: &ActivityCtx,
	_input: &CheckConfigChangesInput,
) -> Result<Option<ConfigChange>> {
	tracing::info!("checking for config changes");

	let state = ctx.state::<State>()?;

	// Get current configuration from rivet_config
	let topology = ctx.config().topology();

	// Build list of replicas from datacenters
	let current_replicas = topology
		.datacenters
		.iter()
		.map(|dc| {
			// Check if this replica exists in our state
			let status = state
				.config
				.replicas
				.iter()
				.find(|x| x.replica_id == dc.datacenter_label as u64)
				.map(|r| r.status.clone())
				.unwrap_or(types::ReplicaStatus::Joining);

			types::ReplicaConfig {
				replica_id: dc.datacenter_label as u64,
				status: status.into(),
				api_peer_url: dc.api_peer_url.to_string(),
				guard_url: dc.guard_url.to_string(),
			}
		})
		.collect::<Vec<types::ReplicaConfig>>();

	// Find new replicas that aren't in the old config
	let new_replicas: Vec<types::ReplicaConfig> = current_replicas
		.iter()
		.filter(|r| {
			!state
				.config
				.replicas
				.iter()
				.any(|x| x.replica_id == r.replica_id)
		})
		.cloned()
		.collect();

	if new_replicas.is_empty() {
		tracing::info!("no new replicas found");
		return Ok(None);
	}

	tracing::info!(
		new_replica_count = new_replicas.len(),
		"found new replicas to add"
	);

	// Build the full config
	let config = types::ClusterConfig {
		coordinator_replica_id: ctx.config().epoxy_replica_id(),
		epoch: state.config.epoch,
		replicas: current_replicas,
	};

	Ok(Some(ConfigChange {
		config,
		new_replicas,
	}))
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct HealthCheckNewReplicasInput {
	pub new_replicas: Vec<types::ReplicaConfig>,
}

#[activity(HealthCheckNewReplicas)]
pub async fn health_check_new_replicas(
	ctx: &ActivityCtx,
	input: &HealthCheckNewReplicasInput,
) -> Result<bool> {
	if should_abort_reconfigure(ctx, &input.new_replicas)? {
		return Ok(false);
	}

	tracing::info!(
		new_replicas = ?input.new_replicas,
		"health checking new replicas"
	);

	// Send health check to each new replica
	let health_check_futures = input.new_replicas.iter().map(|replica| {
		let replica_id = replica.replica_id;

		async move {
			tracing::info!(?replica_id, "sending health check to replica");

			let request = protocol::Request {
				from_replica_id: ctx.config().epoxy_replica_id(),
				to_replica_id: replica_id,
				kind: protocol::RequestKind::HealthCheckRequest,
			};

			crate::http_client::send_message_to_address(
				replica.api_peer_url.clone(),
				replica_id,
				request,
			)
			.await
			.with_context(|| format!("health check failed for replica {}", replica_id))?;

			tracing::info!(?replica_id, "health check successful");
			Ok(())
		}
	});

	futures_util::future::try_join_all(health_check_futures).await?;

	Ok(true)
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct AddReplicasAsJoiningInput {
	pub new_replicas: Vec<types::ReplicaConfig>,
}

#[activity(AddReplicasAsJoining)]
pub async fn add_replicas_as_joining(
	ctx: &ActivityCtx,
	input: &AddReplicasAsJoiningInput,
) -> Result<()> {
	let mut state = ctx.state::<State>()?;

	for replica in &input.new_replicas {
		tracing::info!(?replica, "adding replica in joining state");

		state.config.replicas.push(replica.clone().into());
	}

	tracing::info!("added {} replicas as joining", input.new_replicas.len());

	// IMPORTANT: Do not increment epoch at this stage, despite what the EPaxos paper recommends.
	// See epoxy/README.md for more details.

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct SendBeginLearningInput {
	pub new_replicas: Vec<types::ReplicaConfig>,
	pub config: types::ClusterConfig,
}

#[activity(SendBeginLearning)]
pub async fn send_begin_learning(
	ctx: &ActivityCtx,
	input: &SendBeginLearningInput,
) -> Result<bool> {
	if should_abort_reconfigure(ctx, &input.new_replicas)? {
		return Ok(false);
	}

	let state = ctx.state::<State>()?;

	// Send begin learning message to each new replica
	let config: protocol::ClusterConfig = state.config.clone().into();
	let begin_learning_futures = input.new_replicas.iter().map(|replica| {
		let replica_id = replica.replica_id;
		let config = config.clone();

		async move {
			tracing::info!(?replica_id, "sending begin learning to replica");

			let request = protocol::Request {
				from_replica_id: ctx.config().epoxy_replica_id(),
				to_replica_id: replica_id,
				kind: protocol::RequestKind::BeginLearningRequest(protocol::BeginLearningRequest {
					config: config.clone(),
				}),
			};

			crate::http_client::send_message(&config, replica_id, request).await?;

			tracing::info!(?replica_id, "begin learning sent successfully");
			Ok(())
		}
	});

	futures_util::future::try_join_all(begin_learning_futures).await?;

	Ok(true)
}

/// Returns if the config changed from the proposed changes. If so, abort the reconfiguration.
fn should_abort_reconfigure(
	ctx: &ActivityCtx,
	new_replicas: &Vec<types::ReplicaConfig>,
) -> Result<bool> {
	let topology = ctx.config().topology();
	for replica in new_replicas {
		let Some(current_dc) = topology
			.datacenters
			.iter()
			.find(|x| x.datacenter_label as u64 == replica.replica_id)
		else {
			tracing::info!(
				"config changed during reconfigure (replica removed), aborting reconfigure"
			);
			return Ok(true);
		};

		if url::Url::parse(&replica.api_peer_url)? != current_dc.api_peer_url {
			tracing::info!(
				"config changed during reconfigure (api_peer_url changed), aborting reconfigure"
			);
			return Ok(true);
		}

		if url::Url::parse(&replica.guard_url)? != current_dc.guard_url {
			tracing::info!(
				"config changed during reconfigure (guard_url changed), aborting reconfigure"
			);
			return Ok(true);
		}
	}

	Ok(false)
}
