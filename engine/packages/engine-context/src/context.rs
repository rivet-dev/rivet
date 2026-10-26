use std::sync::Arc;

use anyhow::*;
use depot_client::Client as DepotClient;
use rivet_config::Config;
use rivet_topology::topology::Topology;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{Error, NodeId, ObjectStores, UpsPool, depot::DepotPool};

pub struct EngineAdapters {
	pub fs: rivet_env::fs::Fs,
	pub object_stores: ObjectStores,
	pub http: Arc<dyn rivet_env::capabilities::http::HttpClient>,
	pub http_server: Arc<dyn rivet_env::capabilities::http::HttpServer>,
	pub depot_databases: Arc<dyn rivet_env::capabilities::slatedb::SlateDbFactory>,
}

// Context clones share capabilities; EngineBackground owns process task lifetime.
pub(crate) struct EngineContextInner {
	pub(crate) env: Arc<dyn rivet_env::Env>,
	pub(crate) fs: rivet_env::fs::Fs,

	pub(crate) depot_databases: Arc<dyn rivet_env::capabilities::slatedb::SlateDbFactory>,
	pub(crate) shutdown: rivet_env::capabilities::shutdown::Shutdown,
	pub(crate) http: Arc<dyn rivet_env::capabilities::http::HttpClient>,
	pub(crate) http_server: Arc<dyn rivet_env::capabilities::http::HttpServer>,
	pub(crate) config: Config,
	pub(crate) node_id: NodeId,
	pub(crate) worker_identity: rivet_topology::worker::WorkerIdentity,
	background_started: AtomicBool,
	known_namespaces: std::sync::RwLock<std::collections::HashSet<String>>,
	ingress: crate::ingress::IngressDrain,
	pub(crate) ups: Option<UpsPool>,
	pub(crate) object_stores: ObjectStores,
	pub(crate) depot: DepotPool,
	/// One Epoxy client shared by unscoped callers, so its membership cache and background
	/// tasks are not rebuilt for every operation.
	pub(crate) epoxy: std::sync::OnceLock<epoxy::client::Client>,
	pub(crate) jwt: rivet_auth_jwt::runtime::Cache,
	/// One cache budget for every filesystem this process opens, shared by the volume client
	/// and the storage worker's command host.
	pub(crate) filesystem_cache: Arc<depot_client::filesystem::CacheBudget>,
}

#[derive(Clone)]
pub struct EngineContext(
	Arc<EngineContextInner>,
	Option<Arc<crate::singleton_scope::SingletonScope>>,
);

impl EngineContext {
	pub fn jwt(&self) -> &rivet_auth_jwt::runtime::Cache {
		&self.0.jwt
	}
	#[tracing::instrument(skip_all)]
	pub async fn new(
		env: Arc<dyn rivet_env::Env>,
		shutdown: rivet_env::capabilities::shutdown::Shutdown,
		config: Config,
		adapters: EngineAdapters,
	) -> Result<EngineContext> {
		let worker_identity =
			rivet_topology::worker::WorkerIdentity::new(&mut env.rng("depot.identity"));
		Self::new_with_identity(env, shutdown, config, adapters, worker_identity).await
	}

	/// Uses the identity selected by bootstrap before constructing worker-addressed clients.
	/// Storage bootstrap restores the folder's worker ID and draws a fresh process incarnation.
	#[tracing::instrument(skip_all)]
	pub async fn new_with_identity(
		env: Arc<dyn rivet_env::Env>,
		shutdown: rivet_env::capabilities::shutdown::Shutdown,
		config: Config,
		adapters: EngineAdapters,
		worker_identity: rivet_topology::worker::WorkerIdentity,
	) -> Result<EngineContext> {
		let EngineAdapters {
			fs,
			object_stores,
			http,
			http_server,
			depot_databases,
		} = adapters;

		let node_id = NodeId::new(&mut env.rng("process.node_id"));

		let topology = Topology::new(
			env.clone(),
			object_stores.lease.clone(),
			rivet_topology::TopologyConfig {
				endpoint_capacity: (config.depot()).endpoint_cache_capacity(),
				lookup_request_capacity: config.depot().lookup_request_capacity(),
				..Default::default()
			},
		)?;

		let ups = crate::db::ups::setup(
			env.clone(),
			&config,
			&worker_identity,
			topology.clone(),
			http.clone(),
		)
		.await?;

		let mut depot_client = DepotClient::new(ups.clone(), topology.clone()).with_regions(
			Arc::new(depot_client::regional::Regions::new(
				config.topology().clone(),
				http.clone(),
				env.clock(),
			)),
		);
		if let Some(floor) = config.depot().command_timeout_floor() {
			depot_client = depot_client.with_command_timeout_floor(floor);
		}
		let depot = DepotPool {
			ready: Default::default(),
			volumes: Default::default(),
			service: Default::default(),
			client: depot_client,
			topology,
		};
		let filesystem_cache = depot_client::filesystem::CacheBudget::new(
			config.depot().filesystem.max_process_cache_bytes,
		);
		let context = EngineContext(
			Arc::new(EngineContextInner {
				jwt: rivet_auth_jwt::runtime::Cache::new(
					env.clone(),
					object_stores.lease.clone(),
					config.clone(),
				),
				fs,
				depot_databases,
				http,
				http_server,
				env,
				shutdown,
				config,
				node_id,
				worker_identity,
				background_started: AtomicBool::new(false),
				known_namespaces: Default::default(),
				ingress: Default::default(),
				ups: Some(ups),
				object_stores,
				depot,
				epoxy: std::sync::OnceLock::new(),
				filesystem_cache,
			}),
			None,
		);

		Ok(context)
	}

	/// Starts process services once and returns their cancellation/drain owner.
	pub async fn start_background(
		&self,
		registration: Option<rivet_topology::WorkerRegistration>,
	) -> Result<EngineBackground> {
		ensure!(
			self.1.is_none(),
			"a singleton scope cannot own process background startup"
		);
		ensure!(
			!self.0.background_started.swap(true, Ordering::AcqRel),
			"Engine background services already started"
		);
		let owner = EngineBackground(self.clone());
		let participates = registration.is_some();
		self.0.depot.topology.start(registration).await?;
		if participates {
			let service = crate::services::depot_client::start(
				self.0.config.depot(),
				self.ups()?,
				self.topology(),
				self.object_stores().primary.clone(),
				self.filesystem_cache(),
			)?;
			self.0
				.depot
				.volumes
				.set(service.client.clone())
				.map_err(|_| anyhow!("volume client already installed"))?;
			*self.0.depot.service.lock().await = Some(service);
			self.0.depot.ready.notify_waiters();
		}
		Ok(owner)
	}

	/// Namespace names never change or disappear, so positive existence entries never expire.
	pub fn knows_namespace(&self, name: &str) -> bool {
		self.0
			.known_namespaces
			.read()
			.expect("namespace cache poisoned")
			.contains(name)
	}

	pub fn remember_namespace(&self, name: &str) {
		self.0
			.known_namespaces
			.write()
			.expect("namespace cache poisoned")
			.insert(name.to_owned());
	}

	pub fn env(&self) -> &Arc<dyn rivet_env::Env> {
		&self.0.env
	}

	/// The process filesystem cache budget shared by every Depot filesystem owner.
	pub fn filesystem_cache(&self) -> Arc<depot_client::filesystem::CacheBudget> {
		self.0.filesystem_cache.clone()
	}

	/// Own accepted ingress until its HTTP bodies and runner connections finish.
	pub fn ingress(&self) -> &crate::ingress::IngressDrain {
		&self.0.ingress
	}

	pub fn shutdown(&self) -> rivet_env::capabilities::shutdown::Shutdown {
		self.0.shutdown.clone()
	}

	pub fn depot_databases(&self) -> Arc<dyn rivet_env::capabilities::slatedb::SlateDbFactory> {
		self.0.depot_databases.clone()
	}

	pub fn http(&self) -> Arc<dyn rivet_env::capabilities::http::HttpClient> {
		self.1
			.as_ref()
			.map_or_else(|| self.0.http.clone(), |scope| scope.http.clone())
	}

	pub fn fs(&self) -> rivet_env::fs::Fs {
		self.0.fs.clone()
	}

	pub fn http_server(&self) -> Arc<dyn rivet_env::capabilities::http::HttpServer> {
		self.0.http_server.clone()
	}

	// MARK: Getters
	pub fn config(&self) -> &Config {
		&self.0.config
	}

	pub fn node_id(&self) -> NodeId {
		self.0.node_id
	}

	/// Select scope-bound clients without changing process identity, Topology or ordinary clients.
	/// The coordinator runtime remains the sole owner of the shared election/lease.
	pub fn with_singleton_scope(
		&self,
		authority: rivet_singleton_coordinator::authority::Authority,
	) -> Result<Self> {
		ensure!(self.1.is_none(), "singleton scopes cannot be nested");
		ensure!(authority.permits(), "singleton authority already ended");
		let scope =
			crate::singleton_scope::SingletonScope::new(authority, self.depot(), self.http())?;
		Ok(Self(self.0.clone(), Some(Arc::new(scope))))
	}
	pub fn singleton_authority(
		&self,
	) -> Result<&rivet_singleton_coordinator::authority::Authority> {
		self.1
			.as_ref()
			.map(|scope| &scope.authority)
			.ok_or_else(|| anyhow!("service requires an elected singleton scope"))
	}
	/// Call after scoped service futures have been canceled and joined. Ordinary clients remain active.
	pub async fn close_singleton_scope(&self) -> Result<()> {
		self.1.as_ref().context("not a singleton scope")?.close();
		Ok(())
	}

	// MARK: Pool lookups
	pub fn ups(&self) -> Result<UpsPool> {
		self.0.ups.clone().ok_or(Error::MissingUpsPool.into())
	}

	pub fn worker_identity(&self) -> &rivet_topology::worker::WorkerIdentity {
		&self.0.worker_identity
	}

	/// The process's Epoxy client. A singleton scope dispatches through its own Depot guard and
	/// HTTP client, so it builds a scoped client instead of sharing the process one.
	pub fn epoxy(&self) -> Result<epoxy::client::Client> {
		// Before startup installs volume access, the Depot client is incomplete; do not cache
		// a client built from it.
		if self.1.is_some() || self.0.depot.volumes.get().is_none() {
			return epoxy::client::Client::with_depot(self.config(), self.depot(), self.http());
		}
		if let Some(client) = self.0.epoxy.get() {
			return Ok(client.clone());
		}

		let client = epoxy::client::Client::with_depot(self.config(), self.depot(), self.http())?;
		Ok(self.0.epoxy.get_or_init(|| client).clone())
	}

	pub fn depot(&self) -> DepotClient {
		self.1.as_ref().map_or_else(
			|| {
				if let Some(volumes) = self.0.depot.volumes.get() {
					// The service is constructed from this context's UPS identity.
					self.0
						.depot
						.client
						.with_volumes(volumes.clone())
						.expect("shared worker identity")
				} else {
					self.0.depot.client.clone()
				}
			},
			|scope| scope.depot.clone(),
		)
	}

	/// Waits for process startup to install volume access before a peer service captures a client.
	/// This does not start services; the caller must cancel the wait if startup is aborted.
	pub async fn wait_depot_ready(&self) -> DepotClient {
		loop {
			let ready = self.0.depot.ready.notified();
			if self.0.depot.volumes.get().is_some() {
				return self.depot();
			}
			ready.await;
		}
	}

	/// Closes process-owned openers while UPS and the manifest watcher can still send requests.
	pub async fn close_depot_clients(&self) -> Result<()> {
		ensure!(
			self.1.is_none(),
			"a singleton scope cannot close process clients"
		);
		let mut service = self.0.depot.service.lock().await;
		if let Some(service) = service.as_mut() {
			service.close().await?;
		}
		service.take();
		Ok(())
	}

	pub fn topology(&self) -> Arc<Topology> {
		self.0.depot.topology.clone()
	}

	pub fn object_stores(&self) -> &ObjectStores {
		&self.0.object_stores
	}
}

/// Process lifetime guard. Dropping it cancels tasks even while context clones survive.
pub struct EngineBackground(EngineContext);

impl EngineBackground {
	pub async fn shutdown(self) -> Result<()> {
		if let Some(epoxy) = self.0.0.epoxy.get() {
			epoxy.shutdown().await?;
		}
		self.0.close_depot_clients().await?;
		// The worker releases its heartbeat after closing storage. If it did not,
		// this background guard cannot certify cleanup and must leave the lease to expire.
		let topology = self.0.topology();
		let _ = topology
			.fence(rivet_topology::FenceReason::DependentFailed)
			.await;
		topology.shutdown().await
	}
}

impl Drop for EngineBackground {
	fn drop(&mut self) {
		let topology = self.0.topology();
		let ctx = self.0.clone();
		self.0.env().spawn(Box::pin(async move {
			// Drop cannot confirm dependent cleanup; leave the heartbeat to expire.
			let _ = topology
				.fence(rivet_topology::FenceReason::DependentFailed)
				.await;
			if let Err(error) = ctx.close_depot_clients().await {
				tracing::warn!(%error, "depot client shutdown failed");
			}
			if let Err(error) = topology.shutdown().await {
				tracing::warn!(%error, "topology shutdown failed");
			}
		}));
	}
}
