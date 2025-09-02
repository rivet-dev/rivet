use anyhow::{self, Context};
use epoxy::types;
use epoxy_protocol::protocol::ReplicaId;
use gas::{
	prelude::*,
	test::{self, WorkflowTestCtx},
};
// Note: workflows module no longer exposed in tests
use rivet_util::Id;
use serde_json::json;
use std::collections::HashMap;
use url::Url;

pub mod api;
pub mod utils;

pub const THREE_REPLICAS: &[ReplicaId] = &[1, 2, 3];
pub const DEFAULT_REPLICA_IDS: &[ReplicaId] = &[1, 2];

#[derive(Clone)]
pub struct ReplicaMetadata {
	pub api_peer_port: u16,
	pub guard_port: u16,
}

pub struct TestCtx {
	test_id: Uuid,

	pub leader_id: ReplicaId,
	pub coordinator_workflow_id: Id,

	/// Metadata about each replica. This is used to build the config.
	///
	/// Created with add_replica.
	replica_metadata: HashMap<ReplicaId, ReplicaMetadata>,

	/// Actual context for each replica.
	///
	/// Must first create metadata with add_replica.
	///
	/// Created with start_replica & stopped with stop_replica.
	replica_contexts: HashMap<ReplicaId, ReplicaContext>,
}

struct ReplicaContext {
	wf_ctx: WorkflowTestCtx,
	api_server_handle: tokio::task::JoinHandle<()>,
}

impl TestCtx {
	pub async fn new() -> anyhow::Result<Self> {
		Self::new_with(DEFAULT_REPLICA_IDS).await
	}

	pub async fn new_with(replica_ids: &[ReplicaId]) -> anyhow::Result<Self> {
		gas::test::setup_logging();

		let leader_id = replica_ids[0];

		let mut test_ctx = TestCtx {
			test_id: Uuid::new_v4(),
			leader_id,
			coordinator_workflow_id: Id::new_v1(leader_id as u16),
			replica_contexts: HashMap::new(),
			replica_metadata: HashMap::new(),
		};

		// Add replicas. Must do this before starting since start_replica will generate a config
		// from the replica metadata.
		tracing::info!("adding replicas");
		for &replica_id in replica_ids {
			test_ctx.add_replica(replica_id).await?;
		}

		// Start replicas.
		tracing::info!("starting replicas");
		for &replica_id in replica_ids {
			test_ctx.start_replica(replica_id).await?;
		}

		// Start coordinator workflow on leader
		let mut config_sub = test_ctx
			.get_ctx(leader_id)
			.subscribe::<epoxy::workflows::coordinator::ConfigChangeMessage>(
				json!({ "replica": leader_id }),
			)
			.await
			.unwrap();

		let coordinator_workflow_id =
			setup_epoxy_coordinator_wf(&test_ctx.replica_contexts, leader_id).await?;
		test_ctx.coordinator_workflow_id = coordinator_workflow_id;

		tracing::info!("waiting for replicas to become ready");
		loop {
			let config_msg = config_sub.next().await?;
			tracing::info!(?config_msg.config, "received config");
			let all_active = config_msg
				.config
				.replicas
				.iter()
				.all(|r| r.status == types::ReplicaStatus::Active);
			if all_active {
				break;
			}
		}

		Ok(test_ctx)
	}

	pub fn get_ctx(&self, replica_id: ReplicaId) -> &WorkflowTestCtx {
		&self
			.replica_contexts
			.get(&replica_id)
			.expect("replica not started")
			.wf_ctx
	}

	pub async fn add_replica(&mut self, replica_id: ReplicaId) -> anyhow::Result<()> {
		tracing::info!(?replica_id, "adding replica");

		assert!(
			!self.replica_contexts.contains_key(&replica_id),
			"replica already exists"
		);

		let api_peer_port =
			portpicker::pick_unused_port().context("failed to pick API peer port")?;
		let guard_port = portpicker::pick_unused_port().context("failed to pick guard port")?;

		self.replica_metadata.insert(
			replica_id,
			ReplicaMetadata {
				api_peer_port,
				guard_port,
			},
		);

		Ok(())
	}

	pub async fn start_replica(&mut self, replica_id: ReplicaId) -> anyhow::Result<()> {
		assert!(
			!self.replica_contexts.contains_key(&replica_id),
			"replica already running"
		);

		tracing::info!(?replica_id, "starting replica");

		let metadata = self.replica_metadata.get(&replica_id).unwrap();

		// Create test deps
		let datacenters = self.build_datacenters()?;
		let test_deps = rivet_test_deps::setup_single_datacenter(
			self.test_id,
			replica_id as u16,
			datacenters,
			metadata.api_peer_port,
			metadata.guard_port,
		)
		.await?;

		// Create test context
		let reg = epoxy::registry()?;
		let test_ctx = test::WorkflowTestCtx::new(reg, test_deps).await?;

		// Start the API server
		let api_handle = api::setup_api_server(
			test_ctx.config().clone(),
			test_ctx.pools().clone(),
			metadata.api_peer_port,
		)
		.await?;

		// Start replica set
		setup_replica_wf(replica_id, &test_ctx).await?;

		// Create context
		self.replica_contexts.insert(
			replica_id,
			ReplicaContext {
				wf_ctx: test_ctx,
				api_server_handle: api_handle,
			},
		);

		Ok(())
	}

	pub async fn stop_replica(
		&mut self,
		replica_id: ReplicaId,
		preserve_data: bool,
	) -> anyhow::Result<()> {
		tracing::info!(?replica_id, "stopping replica");

		let mut replica_context = self.replica_contexts.remove(&replica_id).unwrap();

		// Retain containers so the databases are still there when we restart this replica
		if preserve_data {
			replica_context
				.wf_ctx
				.test_deps
				.dont_stop_docker_containers_on_drop();
		}

		// Stop the workflow worker
		tracing::info!(replica_id, "stopping replica workflow worker");
		replica_context.wf_ctx.shutdown().await?;

		// Stop the API server
		tracing::info!(replica_id, "stopping replica API server");
		replica_context.api_server_handle.abort();
		let _ = (&mut replica_context.api_server_handle).await; // Ignore the result since we aborted it

		tracing::info!(replica_id, "replica stopped");
		Ok(())
	}

	fn build_datacenters(&self) -> Result<Vec<rivet_config::config::topology::Datacenter>> {
		// Build datacenters using our predetermined metadata
		let mut datacenters = Vec::new();
		let replica_ids: Vec<_> = self.replica_metadata.keys().copied().collect();

		for &other_replica_id in &replica_ids {
			let metadata = &self.replica_metadata[&other_replica_id];
			datacenters.push(rivet_config::config::topology::Datacenter {
				name: format!("dc-{}", other_replica_id),
				datacenter_label: other_replica_id as u16,
				is_leader: other_replica_id == self.leader_id,
				api_peer_url: Url::parse(&format!("http://127.0.0.1:{}", metadata.api_peer_port))?,
				guard_url: Url::parse(&format!("http://127.0.0.1:{}", metadata.guard_port))?,
			});
		}

		Ok(datacenters)
	}

	pub fn replica_ids(&self) -> Vec<ReplicaId> {
		self.replica_metadata.keys().cloned().collect()
	}
}

async fn setup_replica_wf(replica_id: ReplicaId, ctx: &WorkflowTestCtx) -> anyhow::Result<Id> {
	tracing::info!(?replica_id, "setting up epoxy replica");

	// Create coordinator if does not exist
	let workflow_id = ctx
		.workflow(epoxy::workflows::replica::Input {})
		.tag("replica", replica_id)
		.dispatch()
		.await?;
	tracing::info!(%workflow_id, ?replica_id, "created epoxy replica");

	Ok(workflow_id)
}

async fn setup_epoxy_coordinator_wf(
	replica_contexts: &HashMap<ReplicaId, ReplicaContext>,
	leader_id: ReplicaId,
) -> anyhow::Result<Id> {
	let leader_ctx = &replica_contexts
		.get(&leader_id)
		.expect("leader not in replica contexts")
		.wf_ctx;

	tracing::info!(
		replica_id = leader_id,
		"setting up epoxy coordinator for leader replica"
	);

	// Create coordinator if does not exist
	let workflow_id = leader_ctx
		.workflow(epoxy::workflows::coordinator::Input {})
		.tag("replica", leader_id)
		.dispatch()
		.await?;
	tracing::info!(%workflow_id, replica_id = leader_id, "created epoxy coordinator");

	// Trigger reconfiguration
	let mut sub = leader_ctx
		.subscribe::<epoxy::workflows::coordinator::ConfigChangeMessage>(
			json!({ "replica": leader_id }),
		)
		.await?;
	leader_ctx
		.signal(epoxy::workflows::coordinator::ReconfigureSignal {})
		.to_workflow_id(workflow_id)
		.send()
		.await?;
	tracing::info!(%workflow_id, replica_id = leader_id, "sent reconfigure signal to epoxy coordinator");
	sub.next().await?;
	tracing::info!(%workflow_id, replica_id = leader_id, "reconfigure complete");

	Ok(workflow_id)
}
