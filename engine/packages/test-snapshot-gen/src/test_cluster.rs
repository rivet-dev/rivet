use std::collections::HashMap;

use anyhow::{Context, Result};
use epoxy::types;
use epoxy_protocol::protocol::ReplicaId;
use gas::prelude::{TestCtx as WorkflowTestCtx, *};
use rivet_util::Id;
use url::Url;

struct ReplicaMetadata {
	api_peer_port: u16,
	guard_port: u16,
}

struct ReplicaContext {
	wf_ctx: WorkflowTestCtx,
	api_server_handle: tokio::task::JoinHandle<()>,
}

pub struct TestCluster {
	test_id: Uuid,
	leader_id: ReplicaId,
	coordinator_workflow_id: Id,
	replica_metadata: HashMap<ReplicaId, ReplicaMetadata>,
	replica_contexts: HashMap<ReplicaId, ReplicaContext>,
}

impl TestCluster {
	pub async fn new(replica_ids: &[ReplicaId]) -> Result<Self> {
		let leader_id = replica_ids[0];

		let mut cluster = TestCluster {
			test_id: Uuid::new_v4(),
			leader_id,
			coordinator_workflow_id: Id::new_v1(leader_id as u16),
			replica_contexts: HashMap::new(),
			replica_metadata: HashMap::new(),
		};

		// Add metadata for all replicas before starting any.
		for &replica_id in replica_ids {
			let api_peer_port =
				portpicker::pick_unused_port().context("failed to pick API peer port")?;
			let guard_port =
				portpicker::pick_unused_port().context("failed to pick guard port")?;
			cluster.replica_metadata.insert(
				replica_id,
				ReplicaMetadata {
					api_peer_port,
					guard_port,
				},
			);
		}

		// Start each replica.
		for &replica_id in replica_ids {
			cluster.start_replica(replica_id).await?;
		}

		// Start coordinator workflow on leader.
		let mut config_sub = cluster
			.get_ctx(leader_id)
			.subscribe::<epoxy::workflows::coordinator::ConfigChangeMessage>(("replica", leader_id))
			.await?;

		let coordinator_workflow_id =
			setup_coordinator(&cluster.replica_contexts, leader_id).await?;
		cluster.coordinator_workflow_id = coordinator_workflow_id;

		// Wait for all replicas to become active.
		tracing::info!("waiting for replicas to become ready");
		loop {
			let config_msg = config_sub.next().await?;
			let all_active = config_msg
				.config
				.replicas
				.iter()
				.all(|r| r.status == types::ReplicaStatus::Active);
			if all_active {
				break;
			}
		}

		Ok(cluster)
	}

	pub fn get_ctx(&self, replica_id: ReplicaId) -> &WorkflowTestCtx {
		&self
			.replica_contexts
			.get(&replica_id)
			.expect("replica not started")
			.wf_ctx
	}

	pub fn replica_ids(&self) -> Vec<ReplicaId> {
		let mut ids = self.replica_metadata.keys().copied().collect::<Vec<_>>();
		ids.sort_unstable();
		ids
	}

	#[allow(dead_code)]
	pub fn leader_id(&self) -> ReplicaId {
		self.leader_id
	}

	pub async fn shutdown(&mut self) -> Result<()> {
		tokio::time::sleep(std::time::Duration::from_millis(200)).await;

		let mut ids = self.replica_contexts.keys().copied().collect::<Vec<_>>();
		ids.sort_unstable();

		for replica_id in ids {
			let mut ctx = self.replica_contexts.remove(&replica_id).unwrap();
			ctx.wf_ctx.shutdown().await?;
			ctx.api_server_handle.abort();
			let _ = (&mut ctx.api_server_handle).await;
		}

		Ok(())
	}

	async fn start_replica(&mut self, replica_id: ReplicaId) -> Result<()> {
		tracing::info!(%replica_id, "starting replica");

		let metadata = self.replica_metadata.get(&replica_id).unwrap();
		let datacenters = self.build_datacenters()?;

		let test_deps = rivet_test_deps::setup_single_datacenter(
			self.test_id,
			replica_id as u16,
			datacenters,
			metadata.api_peer_port,
			metadata.guard_port,
		)
		.await?;

		let reg = epoxy::registry()?;
		let test_ctx = WorkflowTestCtx::new_with_deps(reg, test_deps).await?;

		let api_handle = setup_api_server(
			test_ctx.config().clone(),
			test_ctx.pools().clone(),
			metadata.api_peer_port,
		)
		.await?;

		// Start replica workflow.
		let workflow_id = test_ctx
			.workflow(epoxy::workflows::replica::Input {})
			.tag("replica", replica_id)
			.dispatch()
			.await?;
		tracing::info!(%workflow_id, %replica_id, "created epoxy replica");

		self.replica_contexts.insert(
			replica_id,
			ReplicaContext {
				wf_ctx: test_ctx,
				api_server_handle: api_handle,
			},
		);

		Ok(())
	}

	fn build_datacenters(
		&self,
	) -> Result<Vec<rivet_config::config::topology::Datacenter>> {
		let mut datacenters = Vec::new();
		let mut ids: Vec<_> = self.replica_metadata.keys().copied().collect();
		ids.sort_unstable();

		for &id in &ids {
			let metadata = &self.replica_metadata[&id];
			datacenters.push(rivet_config::config::topology::Datacenter {
				name: format!("dc-{}", id),
				datacenter_label: id as u16,
				is_leader: id == self.leader_id,
				peer_url: Url::parse(&format!(
					"http://127.0.0.1:{}",
					metadata.api_peer_port
				))?,
				public_url: Url::parse(&format!(
					"http://127.0.0.1:{}",
					metadata.guard_port
				))?,
				proxy_url: None,
				valid_hosts: None,
			});
		}

		Ok(datacenters)
	}
}

async fn setup_coordinator(
	replica_contexts: &HashMap<ReplicaId, ReplicaContext>,
	leader_id: ReplicaId,
) -> Result<Id> {
	let leader_ctx = &replica_contexts
		.get(&leader_id)
		.expect("leader not in replica contexts")
		.wf_ctx;

	let workflow_id = leader_ctx
		.workflow(epoxy::workflows::coordinator::Input {})
		.tag("replica", leader_id)
		.dispatch()
		.await?;

	let mut sub = leader_ctx
		.subscribe::<epoxy::workflows::coordinator::ConfigChangeMessage>(("replica", leader_id))
		.await?;
	leader_ctx
		.signal(epoxy::workflows::coordinator::Reconfigure {})
		.to_workflow_id(workflow_id)
		.send()
		.await?;
	sub.next().await?;

	Ok(workflow_id)
}

async fn setup_api_server(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	port: u16,
) -> Result<tokio::task::JoinHandle<()>> {
	let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

	let router = rivet_api_builder::create_router("api-peer-test", config, pools, |router| {
		epoxy::http_routes::mount_routes(router)
	})
	.await?;

	let listener = tokio::net::TcpListener::bind(addr).await?;
	let handle = tokio::spawn(async move {
		axum::serve(listener, router).await.unwrap();
	});

	Ok(handle)
}
