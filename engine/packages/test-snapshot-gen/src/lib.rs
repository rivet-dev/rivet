use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use epoxy_protocol::protocol::ReplicaId;
use gas::prelude::{TestCtx as WorkflowTestCtx, *};
use url::Url;

/// Base directory where snapshots are stored.
fn snapshots_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("snapshots")
}

/// Return the snapshot directory for a scenario.
pub fn snapshot_dir(scenario: &str) -> PathBuf {
	snapshots_dir().join(scenario)
}

/// Copy a snapshot's RocksDB directories into temp locations and return
/// per-replica paths suitable for constructing test infrastructure.
///
/// Returns a map of `replica_id -> temp_path` where each temp_path is a
/// copy of that replica's RocksDB data directory.
pub fn load_snapshot(
	scenario: &str,
	test_id: uuid::Uuid,
) -> Result<HashMap<ReplicaId, PathBuf>> {
	let dir = snapshot_dir(scenario);
	load_snapshot_from(&dir, test_id)
}

/// Like `load_snapshot` but from an explicit directory.
pub fn load_snapshot_from(
	snapshot_dir: &Path,
	test_id: uuid::Uuid,
) -> Result<HashMap<ReplicaId, PathBuf>> {
	let mut replicas = HashMap::new();

	for entry in std::fs::read_dir(snapshot_dir)
		.with_context(|| format!("failed to read snapshot dir {}", snapshot_dir.display()))?
	{
		let entry = entry?;
		let name = entry.file_name().to_string_lossy().to_string();
		if !name.starts_with("replica-") {
			continue;
		}

		let replica_id: ReplicaId = name
			.strip_prefix("replica-")
			.unwrap()
			.parse()
			.context("invalid replica id in snapshot dir name")?;

		// Copy to the standard test path format so the test infrastructure picks it up.
		let dest = std::env::temp_dir().join(format!("rivet-test-{}-{}", test_id, replica_id));
		if dest.exists() {
			std::fs::remove_dir_all(&dest)?;
		}
		copy_dir_recursive(&entry.path(), &dest)
			.with_context(|| format!("failed to copy snapshot for replica {replica_id}"))?;

		replicas.insert(replica_id, dest);
	}

	Ok(replicas)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
	std::fs::create_dir_all(dst)?;
	for entry in std::fs::read_dir(src)? {
		let entry = entry?;
		let ty = entry.file_type()?;
		let dest_path = dst.join(entry.file_name());
		if ty.is_dir() {
			copy_dir_recursive(&entry.path(), &dest_path)?;
		} else {
			std::fs::copy(entry.path(), &dest_path)?;
		}
	}
	Ok(())
}

/// Helper to build a full test cluster from a pre-existing snapshot.
///
/// This copies the snapshot RocksDB directories into the standard temp
/// locations so that when `rivet_test_deps::setup_single_datacenter` runs
/// it finds the pre-populated database.
pub struct SnapshotTestCtx {
	pub test_id: uuid::Uuid,
	pub leader_id: ReplicaId,
	pub coordinator_workflow_id: rivet_util::Id,
	replica_metadata: HashMap<ReplicaId, ReplicaMetadata>,
	replica_contexts: HashMap<ReplicaId, ReplicaContext>,
}

struct ReplicaMetadata {
	api_peer_port: u16,
	guard_port: u16,
}

struct ReplicaContext {
	wf_ctx: WorkflowTestCtx,
	api_server_handle: tokio::task::JoinHandle<()>,
}

impl SnapshotTestCtx {
	/// Create a test cluster from a snapshot scenario.
	///
	/// Loads the latest snapshot for `scenario`, copies the RocksDB dirs
	/// into temp, then boots the full cluster pointing at that data.
	pub async fn from_snapshot(scenario: &str) -> Result<Self> {
		let test_id = uuid::Uuid::new_v4();
		let snapshot_replicas = load_snapshot(scenario, test_id)?;

		let mut replica_ids: Vec<_> = snapshot_replicas.keys().copied().collect();
		replica_ids.sort_unstable();

		let leader_id = replica_ids[0];
		let mut ctx = SnapshotTestCtx {
			test_id,
			leader_id,
			coordinator_workflow_id: rivet_util::Id::new_v1(leader_id as u16),
			replica_metadata: HashMap::new(),
			replica_contexts: HashMap::new(),
		};

		// Assign ports.
		for &replica_id in &replica_ids {
			let api_peer_port =
				portpicker::pick_unused_port().context("failed to pick API peer port")?;
			let guard_port =
				portpicker::pick_unused_port().context("failed to pick guard port")?;
			ctx.replica_metadata.insert(
				replica_id,
				ReplicaMetadata {
					api_peer_port,
					guard_port,
				},
			);
		}

		// Start replicas pointing at the snapshot data.
		for &replica_id in &replica_ids {
			ctx.start_replica(replica_id).await?;
		}

		Ok(ctx)
	}

	/// Create a test cluster from a snapshot and also start the coordinator + wait for Active.
	pub async fn from_snapshot_with_coordinator(scenario: &str) -> Result<Self> {
		let mut ctx = Self::from_snapshot(scenario).await?;

		let mut config_sub = ctx
			.get_ctx(ctx.leader_id)
			.subscribe::<epoxy::workflows::coordinator::ConfigChangeMessage>((
				"replica",
				ctx.leader_id,
			))
			.await?;

		let coordinator_workflow_id =
			setup_coordinator(&ctx.replica_contexts, ctx.leader_id).await?;
		ctx.coordinator_workflow_id = coordinator_workflow_id;

		tracing::info!("waiting for replicas to become ready");
		loop {
			let config_msg = config_sub.next().await?;
			let all_active = config_msg
				.config
				.replicas
				.iter()
				.all(|r| r.status == epoxy::types::ReplicaStatus::Active);
			if all_active {
				break;
			}
		}

		Ok(ctx)
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

	pub async fn shutdown(&mut self) -> Result<()> {
		tokio::time::sleep(std::time::Duration::from_millis(200)).await;

		let mut ids = self.replica_contexts.keys().copied().collect::<Vec<_>>();
		ids.sort_unstable();

		for replica_id in ids {
			let mut rc = self.replica_contexts.remove(&replica_id).unwrap();
			rc.wf_ctx.shutdown().await?;
			rc.api_server_handle.abort();
			let _ = (&mut rc.api_server_handle).await;
		}

		Ok(())
	}

	async fn start_replica(&mut self, replica_id: ReplicaId) -> Result<()> {
		tracing::info!(%replica_id, "starting replica from snapshot");

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

		// Start the replica workflow.
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
	) -> Result<HashMap<String, rivet_config::config::topology::Datacenter>> {
		let mut datacenters = HashMap::new();
		let mut ids: Vec<_> = self.replica_metadata.keys().copied().collect();
		ids.sort_unstable();

		for &id in &ids {
			let metadata = &self.replica_metadata[&id];
			let name = format!("dc-{}", id);
			datacenters.insert(
				name.clone(),
				rivet_config::config::topology::Datacenter {
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
				},
			);
		}

		Ok(datacenters)
	}
}

async fn setup_coordinator(
	replica_contexts: &HashMap<ReplicaId, ReplicaContext>,
	leader_id: ReplicaId,
) -> Result<rivet_util::Id> {
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
