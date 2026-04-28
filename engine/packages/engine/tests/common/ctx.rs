use anyhow::*;
use gas::prelude::*;
use rivet_service_manager::{Service, ServiceKind};
use std::io;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// Process-wide capture of formatted tracing output. Tests that need to make assertions about
/// engine log lines (for example to detect a leaked sqlite open_dbs entry) read from this buffer.
/// The buffer is appended to by every tracing event regardless of whether any test reads it.
static LOG_CAPTURE: OnceLock<Arc<parking_lot::Mutex<Vec<u8>>>> = OnceLock::new();

fn log_capture() -> Arc<parking_lot::Mutex<Vec<u8>>> {
	LOG_CAPTURE
		.get_or_init(|| Arc::new(parking_lot::Mutex::new(Vec::new())))
		.clone()
}

/// Returns a snapshot of all tracing output captured so far across the process. Used by tests
/// that need to assert on specific engine log lines.
pub fn captured_logs_snapshot() -> String {
	let guard = log_capture();
	let buf = guard.lock();
	String::from_utf8_lossy(&buf).into_owned()
}

/// `MakeWriter` that tees each tracing line into the test stdout writer (so libtest captures
/// it for failed-test output) and into the process-wide capture buffer used by assertions.
struct TeeMakeWriter {
	capture: Arc<parking_lot::Mutex<Vec<u8>>>,
}

struct TeeWriter<'a> {
	test_writer: tracing_subscriber::fmt::TestWriter,
	capture: parking_lot::MutexGuard<'a, Vec<u8>>,
}

impl<'a> io::Write for TeeWriter<'a> {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		// Forced-sync context: tracing-subscriber MakeWriter is invoked synchronously inside the
		// emit path, so we must use a parking_lot guard rather than an async lock.
		self.capture.extend_from_slice(buf);
		self.test_writer.write(buf)
	}

	fn flush(&mut self) -> io::Result<()> {
		self.test_writer.flush()
	}
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for TeeMakeWriter {
	type Writer = TeeWriter<'a>;

	fn make_writer(&'a self) -> Self::Writer {
		TeeWriter {
			test_writer: tracing_subscriber::fmt::TestWriter::new(),
			capture: self.capture.lock(),
		}
	}
}

pub struct TestOpts {
	pub datacenters: usize,
	pub timeout_secs: u64,
	pub pegboard_outbound: bool,
	pub auth_admin_token: Option<String>,
}

impl TestOpts {
	pub fn new(datacenters: usize) -> Self {
		Self {
			datacenters,
			timeout_secs: 10,
			pegboard_outbound: false,
			auth_admin_token: None,
		}
	}

	pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
		self.timeout_secs = timeout_secs;
		self
	}

	pub fn with_pegboard_outbound(mut self) -> Self {
		self.pegboard_outbound = true;
		self
	}

	pub fn with_auth_admin_token(mut self, token: impl Into<String>) -> Self {
		self.auth_admin_token = Some(token.into());
		self
	}
}

impl Default for TestOpts {
	fn default() -> Self {
		Self {
			datacenters: 1,
			timeout_secs: 10,
			pegboard_outbound: false,
			auth_admin_token: None,
		}
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
		// Set up logging. The custom `MakeWriter` tees each line into both the libtest
		// stdout writer and a process-wide capture buffer (`captured_logs_snapshot`).
		let _ = tracing_subscriber::fmt()
			.with_env_filter("info")
			.with_ansi(false)
			.with_writer(TeeMakeWriter {
				capture: log_capture(),
			})
			.try_init();

		// Initialize test dependencies for all DCs
		assert!(opts.datacenters >= 1, "datacenters must be at least 1");
		let dc_count = opts.datacenters;
		tracing::info!("setting up test dependencies for {} DCs", dc_count);
		let dc_labels: Vec<u16> = (1..=dc_count as u16).collect();
		let test_deps_list = rivet_test_deps::TestDeps::new_multi(&dc_labels)
			.await?
			.into_iter();

		// Setup all datacenters in parallel so each DC's epoxy/peer endpoints can reach the
		// others without hitting a startup race (sequential setup would let DC1's epoxy try to
		// contact DC2 before DC2's API server is listening, which puts DC1 into a long backoff
		// loop).
		let setup_futures = test_deps_list.map(|test_deps| {
			Self::setup_instance(
				test_deps,
				opts.pegboard_outbound,
				opts.auth_admin_token.clone(),
			)
		});
		let mut dcs: Vec<TestDatacenter> = futures_util::future::try_join_all(setup_futures).await?;
		dcs.sort_by_key(|dc| dc.config.dc_label());

		Ok(Self { dcs, opts })
	}

	async fn setup_instance(
		test_deps: rivet_test_deps::TestDeps,
		include_pegboard_outbound: bool,
		auth_admin_token: Option<String>,
	) -> Result<TestDatacenter> {
		let config = if let Some(admin_token) = auth_admin_token {
			let mut root = (**test_deps.config()).clone();
			root.auth = Some(rivet_config::config::auth::Auth {
				admin_token: rivet_config::secret::Secret::new(admin_token),
			});
			rivet_config::Config::from_root(root)
		} else {
			test_deps.config().clone()
		};
		let pools = test_deps.pools().clone();

		// Start the service manager with all required services
		let dc_label = config.dc_label();
		tracing::info!(dc_label, "starting engine services for DC");
		let engine_handle = tokio::spawn({
			let config = config.clone();
			let pools = pools.clone();
			async move {
				let mut services = vec![
					Service::new(
						"api-peer",
						ServiceKind::ApiPeer,
						|config, pools| Box::pin(rivet_api_peer::start(config, pools)),
						false,
					),
					Service::new(
						"guard",
						ServiceKind::Standalone,
						|config, pools| Box::pin(rivet_guard::start(config, pools)),
						true,
					),
					Service::new(
						"workflow-worker",
						ServiceKind::Standalone,
						|config, pools| Box::pin(rivet_workflow_worker::start(config, pools)),
						true,
					),
					Service::new(
						"bootstrap",
						ServiceKind::Oneshot,
						|config, pools| Box::pin(rivet_bootstrap::start(config, pools)),
						false,
					),
				];

				if include_pegboard_outbound {
					services.push(Service::new(
						"pegboard_outbound",
						ServiceKind::Standalone,
						|config, pools| Box::pin(pegboard_outbound::start(config, pools)),
						true,
					));
				}

				rivet_service_manager::start(config, pools, services).await
			}
		});

		// Wait for ports to open
		tracing::info!(dc_label, "waiting for services to be ready");
		tokio::join!(
			wait_for_port("api-peer", test_deps.api_peer_port()),
			wait_for_port("guard", test_deps.guard_port()),
		);

		// Create workflow context for assertions
		let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
		let workflow_ctx = StandaloneCtx::new(
			db::DatabaseKv::new(config.clone(), pools.clone()).await?,
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
