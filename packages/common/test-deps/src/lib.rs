use anyhow::*;
use futures_util::future;
use url::Url;

mod datacenter;

pub use datacenter::*;
pub use rivet_test_deps_docker::*;
use uuid::Uuid;

pub struct TestDeps {
	pub pools: rivet_pools::Pools,
	pub config: rivet_config::Config,
	container_names: Vec<String>,
	api_public_port: u16,
	api_peer_port: u16,
	pegboard_port: u16,
	guard_port: u16,
	stop_docker_containers_on_drop: bool,
}

impl TestDeps {
	pub async fn new() -> Result<Self> {
		TestDeps::new_with_test_id(Uuid::new_v4()).await
	}

	pub async fn new_with_test_id(test_id: Uuid) -> Result<Self> {
		TestDeps::new_multi_with_test_id(&[1], test_id)
			.await?
			.into_iter()
			.next()
			.context("no dc")
	}

	pub async fn new_multi(dc_ids: &[u16]) -> Result<Vec<Self>> {
		Self::new_multi_with_test_id(dc_ids, Uuid::new_v4()).await
	}

	pub async fn new_multi_with_test_id(dc_ids: &[u16], test_id: Uuid) -> Result<Vec<Self>> {
		tracing::info!(?dc_ids, "setting up test dependencies");

		let mut datacenters = Vec::with_capacity(dc_ids.len());
		let mut ports = Vec::with_capacity(dc_ids.len());

		for &dc_id in dc_ids {
			let api_peer_port = portpicker::pick_unused_port().context("api_peer_port")?;
			let guard_port = portpicker::pick_unused_port().context("guard_port")?;

			datacenters.push(rivet_config::config::topology::Datacenter {
				name: format!("dc-{dc_id}"),
				datacenter_label: dc_id,
				is_leader: dc_id == dc_ids[0], // First DC in list is leader
				api_peer_url: Url::parse(&format!("http://127.0.0.1:{api_peer_port}"))?,
				guard_url: Url::parse(&format!("http://127.0.0.1:{guard_port}"))?,
			});
			ports.push((api_peer_port, guard_port));
		}

		// Create futures for each datacenter
		let futures =
			datacenters
				.iter()
				.zip(ports.into_iter())
				.map(|(dc, (api_peer_port, guard_port))| {
					setup_single_datacenter(
						test_id,
						dc.datacenter_label,
						datacenters.clone(),
						api_peer_port,
						guard_port,
					)
				});

		// Execute all futures concurrently
		let deps = future::try_join_all(futures).await?;

		Ok(deps)
	}

	pub fn nats_address(&self) -> Option<String> {
		match self.config.pubsub() {
			rivet_config::config::PubSub::Nats(nats) => nats.addresses.first().cloned(),
			_ => None,
		}
	}

	pub fn api_public_port(&self) -> u16 {
		self.api_public_port
	}

	pub fn api_peer_port(&self) -> u16 {
		self.api_peer_port
	}

	pub fn pegboard_port(&self) -> u16 {
		self.pegboard_port
	}

	pub fn guard_port(&self) -> u16 {
		self.guard_port
	}

	pub fn pools(&self) -> &rivet_pools::Pools {
		&self.pools
	}

	pub fn config(&self) -> &rivet_config::Config {
		&self.config
	}

	/// Will not stop docker containers on drop. Useful if we need to preserve the containers to
	/// recreate test deps with the same test ID, for example if simulating restarting a cluster.
	pub fn dont_stop_docker_containers_on_drop(&mut self) {
		self.stop_docker_containers_on_drop = false;
	}
}

impl Drop for TestDeps {
	fn drop(&mut self) {
		if self.stop_docker_containers_on_drop {
			// Clean up containers synchronously to ensure they're stopped
			tracing::info!("cleaning up test containers");
			for container_name in &self.container_names {
				// Use std::process::Command for synchronous execution in drop
				let _ = std::process::Command::new("docker")
					.arg("stop")
					.arg(container_name)
					.output();
				let _ = std::process::Command::new("docker")
					.arg("rm")
					.arg(container_name)
					.output();
			}
			tracing::info!("test containers cleaned up");
		} else {
			tracing::info!("skipping dropping docker containers");
		}
	}
}
