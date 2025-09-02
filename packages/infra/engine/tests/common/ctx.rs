use anyhow::*;
use gas::prelude::*;
use rivet_service_manager::{Service, ServiceKind};
use std::time::Duration;

pub struct TestOpts {
	pub datacenters: usize,
}

impl TestOpts {
	pub fn new(datacenters: usize) -> Self {
		Self { datacenters }
	}
}

impl Default for TestOpts {
	fn default() -> Self {
		Self { datacenters: 1 }
	}
}

pub struct TestCtx {
	dcs: Vec<TestDatacenter>,
	pub opts: TestOpts,
}

pub struct TestDatacenter {
	pub config: rivet_config::Config,
	pub pools: rivet_pools::Pools,
	pub test_deps: rivet_test_deps::TestDeps,
	pub workflow_ctx: StandaloneCtx,
	engine_handle: tokio::task::JoinHandle<Result<()>>,
}

impl TestCtx {
	/// Creates a test context with multiple datacenters
	pub async fn new_multi(dc_count: usize) -> Result<Self> {
		Self::new_with_opts(TestOpts::new(dc_count)).await
	}

	/// Creates a test context with custom options
	pub async fn new_with_opts(opts: TestOpts) -> Result<Self> {
		// Set up logging
		let _ = tracing_subscriber::fmt()
			.with_env_filter("info,rivet=debug,pegboard=debug,epoxy=off,test=debug")
			.with_ansi(false)
			.with_test_writer()
			.try_init();

		// Initialize test dependencies for all DCs
		assert!(opts.datacenters >= 1, "datacenters must be at least 1");
		let dc_count = opts.datacenters;
		tracing::info!("setting up test dependencies for {} DCs", dc_count);
		let dc_labels: Vec<u16> = (1..=dc_count as u16).collect();
		let test_deps_list = rivet_test_deps::TestDeps::new_multi(&dc_labels)
			.await?
			.into_iter();

		// Setup all datacenters
		let mut dcs = Vec::new();
		for test_deps in test_deps_list {
			let dc = Self::setup_instance(test_deps).await?;
			dcs.push(dc);
		}

		Ok(Self { dcs, opts })
	}

	async fn setup_instance(test_deps: rivet_test_deps::TestDeps) -> Result<TestDatacenter> {
		let config = test_deps.config().clone();
		let pools = test_deps.pools().clone();

		// Start the service manager with all required services
		let dc_label = config.dc_label();
		tracing::info!(dc_label, "starting engine services for DC");
		let engine_handle = tokio::spawn({
			let config = config.clone();
			let pools = pools.clone();
			async move {
				let services = vec![
					Service::new("api-public", ServiceKind::ApiPublic, |config, pools| {
						Box::pin(rivet_api_public::start(config, pools))
					}),
					Service::new("api-peer", ServiceKind::ApiPeer, |config, pools| {
						Box::pin(rivet_api_peer::start(config, pools))
					}),
					Service::new("guard", ServiceKind::Standalone, |config, pools| {
						Box::pin(rivet_guard::start(config, pools))
					}),
					Service::new(
						"pegboard-runner-ws",
						ServiceKind::ApiPublic,
						|config, pools| Box::pin(pegboard_runner_ws::start(config, pools)),
					),
					Service::new(
						"workflow-worker",
						ServiceKind::Standalone,
						|config, pools| Box::pin(rivet_workflow_worker::start(config, pools)),
					),
					Service::new("bootstrap", ServiceKind::Oneshot, |config, pools| {
						Box::pin(rivet_bootstrap::start(config, pools))
					}),
				];

				rivet_service_manager::start(config, pools, services).await
			}
		});

		// Wait for ports to open
		tracing::info!(dc_label, "waiting for services to be ready");
		tokio::join!(
			wait_for_port("api-public", test_deps.api_public_port()),
			wait_for_port("guard", test_deps.guard_port()),
			wait_for_port("api-peer", test_deps.api_peer_port()),
			wait_for_port("pegboard", test_deps.pegboard_port()),
		);

		// Create workflow context for assertions
		let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
		let workflow_ctx = StandaloneCtx::new(
			db::DatabaseKv::from_pools(pools.clone()).await?,
			config.clone(),
			pools.clone(),
			cache,
			"test",
			Id::new_v1(config.dc_label()),
			Id::new_v1(config.dc_label()),
		)?;

		Ok(TestDatacenter {
			config,
			pools,
			test_deps,
			workflow_ctx,
			engine_handle,
		})
	}

	pub fn leader_dc(&self) -> &TestDatacenter {
		&self.dcs[0]
	}

	pub fn get_dc(&self, label: u16) -> &TestDatacenter {
		self.dcs
			.iter()
			.find(|dc| dc.config.dc_label() == label)
			.unwrap_or_else(|| panic!("No datacenter found with label {}", label))
	}

	pub async fn shutdown(self) {
		tracing::info!("shutting down multi-DC test context");
		for dc in self.dcs {
			dc.shutdown().await;
		}
	}
}

impl TestDatacenter {
	pub fn api_peer_port(&self) -> u16 {
		self.test_deps.api_peer_port()
	}

	pub fn guard_port(&self) -> u16 {
		self.test_deps.guard_port()
	}

	async fn shutdown(self) {
		tracing::info!(
			dc_label = self.config.dc_label(),
			"shutting down test instance"
		);
		self.engine_handle.abort();
	}
}

pub async fn wait_for_port(service_name: &str, port: u16) {
	let addr = format!("127.0.0.1:{}", port);
	let start = std::time::Instant::now();
	let timeout = Duration::from_secs(30);

	tracing::info!("waiting for {} on port {}", service_name, port);

	loop {
		match tokio::net::TcpStream::connect(&addr).await {
			std::result::Result::Ok(_) => {
				tracing::info!("{} is ready on port {}", service_name, port);
				return;
			}
			std::result::Result::Err(e) => {
				if start.elapsed() > timeout {
					panic!(
						"Timeout waiting for {} on port {} after {:?}: {}",
						service_name, port, timeout, e
					);
				}
				// Check less frequently to avoid spamming
				tokio::time::sleep(Duration::from_millis(100)).await;
			}
		}
	}
}
