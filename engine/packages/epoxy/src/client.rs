//! Caller context for Epoxy operations backed by Depot.

use crate::{
	consensus::{Consensus, ReplicaTransport},
	depot::replica::Replica,
	membership,
	ops::EpoxyOperation,
	transport::HttpTransport,
};
use anyhow::Result;
use epoxy_protocol::protocol;
use std::sync::Arc;

#[derive(Clone)]
pub struct Client {
	pub(crate) replica: Replica,
	background: Arc<rivet_env::TaskScope>,
	pub(crate) transport: Arc<dyn ReplicaTransport>,
	_version_discovery: Option<Arc<VersionDiscovery>>,
}

struct VersionDiscovery {
	transport: Arc<HttpTransport>,
	task: tokio::sync::Mutex<Option<rivet_env::TaskHandle>>,
}

impl Client {
	pub async fn shutdown(&self) -> Result<()> {
		self.background.shutdown().await;
		if let Some(discovery) = &self._version_discovery
			&& let Some(task) = discovery.task.lock().await.take()
		{
			task.await?;
		}
		Ok(())
	}

	pub fn cancel_background(&self) {
		self.background.cancel();
	}

	/// Starts discovery after the process owner has finished constructing clients.
	pub async fn start_version_discovery(&self) -> Result<()> {
		if let Some(discovery) = &self._version_discovery {
			let mut task = discovery.task.lock().await;
			anyhow::ensure!(task.is_none(), "Epoxy version discovery already started");
			let transport = discovery.transport.clone();
			*task = Some(self.background.spawn(async move {
				if let Err(error) = crate::services::version_discovery::start(transport).await {
					tracing::error!(%error, "Epoxy client version discovery stopped");
				}
			}));
		}
		Ok(())
	}

	pub fn env(&self) -> &Arc<dyn rivet_env::Env> {
		self.replica.client.env()
	}

	pub fn with_depot(
		config: &rivet_config::Config,
		depot: depot_client::Client,
		http: Arc<dyn rivet_env::capabilities::http::HttpClient>,
	) -> Result<Self> {
		let replica = Replica::new(depot.clone(), config.epoxy_replica_id());
		let epoxy_topology = membership::epoxy_topology(config)?;
		let transport = Arc::new(
			HttpTransport::new(replica.clone(), Arc::new(epoxy_topology), http)?
				.with_config(config.clone()),
		);
		let mut client = Self::with_transport(replica, transport.clone());
		client._version_discovery = Some(Arc::new(VersionDiscovery {
			transport,
			task: tokio::sync::Mutex::new(None),
		}));
		Ok(client)
	}

	pub fn with_transport(replica: Replica, transport: Arc<dyn ReplicaTransport>) -> Self {
		let background = Arc::new(replica.client.env().task_scope());
		Self {
			background,
			replica,
			transport,
			_version_discovery: None,
		}
	}

	/// Proposes with the local replica backed by its Depot volume. An unknown transport
	/// outcome may include a committed write; callers must preserve retry semantics.
	#[tracing::instrument(skip_all, fields(replica_id = self.replica_id()))]
	pub async fn propose(
		&self,
		input: crate::ops::propose::Input,
	) -> Result<crate::ops::propose::ProposalResult> {
		crate::metrics::observe(
			"operation",
			"epoxy.propose",
			crate::ops::propose::execute(self, &input),
		)
		.await
	}

	/// Shares protocol discovery with the peer router owned by this client.
	pub(crate) fn http_transport(&self) -> Result<Arc<HttpTransport>> {
		self._version_discovery
			.as_ref()
			.map(|discovery| discovery.transport.clone())
			.ok_or_else(|| anyhow::anyhow!("peer router requires an HTTP-backed Epoxy client"))
	}

	pub fn replica_id(&self) -> u64 {
		self.replica.replica_id()
	}

	#[tracing::instrument(skip_all, fields(replica_id = self.replica_id(), operation = std::any::type_name::<I>()))]
	pub async fn op<I: EpoxyOperation>(&self, input: I) -> Result<I::Output> {
		crate::metrics::observe("operation", std::any::type_name::<I>(), input.execute(self)).await
	}

	pub async fn cluster_config(&self) -> Result<protocol::ClusterConfig> {
		use anyhow::Context;
		self.replica
			.membership()
			.await?
			.context("Epoxy membership has not been initialized")
	}

	/// Consensus on this datacenter's replica, every key's only voting replica.
	pub fn consensus(&self) -> Consensus {
		Consensus::new(self.replica.clone(), self.transport.clone())
	}
}


