//! Bounded peer requests with per-peer protocol discovery: a peer's version is probed on
//! first use and after a version error, never on a timer.

use crate::{
	consensus::{ReplicaTransport, ResponseFuture},
	depot::replica::Replica,
	membership,
};
use anyhow::{Context, Result, ensure};
use epoxy_protocol::{OLDEST_PROTOCOL_VERSION, PROTOCOL_VERSION, protocol, versioned};
use futures_util::{StreamExt, stream::FuturesUnordered};
use rivet_data::generated::epoxy_control_state_v1;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;

pub struct HttpTransport {
	pub(crate) client: Arc<dyn rivet_env::capabilities::http::HttpClient>,
	local: Replica,
	epoxy_topology: Arc<epoxy_control_state_v1::EpoxyTopology>,
	config: Option<rivet_config::Config>,
	peer_versions: RwLock<BTreeMap<String, u16>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProtocolVersionResponse {
	pub protocol_version: u16,
}

impl HttpTransport {
	pub fn new(
		local: Replica,
		epoxy_topology: Arc<epoxy_control_state_v1::EpoxyTopology>,
		client: Arc<dyn rivet_env::capabilities::http::HttpClient>,
	) -> Result<Self> {
		Ok(Self {
			client,
			local,
			epoxy_topology,
			config: None,
			peer_versions: RwLock::new(BTreeMap::new()),
		})
	}

	pub fn with_config(mut self, config: rivet_config::Config) -> Self {
		self.config = Some(config);
		self
	}

	/// Regional version selection belongs to shared version management. Until its
	/// Depot migration, Engine supplies the compiled version through this config.
	pub fn local_version(&self) -> u16 {
		self.config.as_ref().map_or(PROTOCOL_VERSION, |config| {
			config.protocols().epoxy.version().min(PROTOCOL_VERSION)
		})
	}

	fn epoxy_topology(&self) -> Result<epoxy_control_state_v1::EpoxyTopology> {
		self.config
			.as_ref()
			.map(membership::epoxy_topology)
			.transpose()
			.map(|topology| topology.unwrap_or_else(|| (*self.epoxy_topology).clone()))
	}

	// Installed membership owns addresses. Topology supplies only unadmitted peers
	// needed for bootstrap and health checks; it never overlays an installed address.
	async fn peer_addresses(&self) -> Result<BTreeMap<u64, String>> {
		let mut addresses: BTreeMap<_, _> = self
			.epoxy_topology()?
			.replicas
			.into_iter()
			.map(|r| (r.replica_id, r.api_peer_url.to_string()))
			.collect();
		if let Some(installed) = self.local.membership().await? {
			addresses.extend(
				installed
					.replicas
					.into_iter()
					.map(|r| (r.replica_id, r.api_peer_url.to_string())),
			);
		}
		Ok(addresses)
	}

	#[tracing::instrument(skip_all)]
	pub async fn refresh_versions(&self) -> Result<()> {
		let urls: Vec<_> = self
			.peer_addresses()
			.await?
			.into_iter()
			.filter(|(id, _)| *id != self.local.replica_id())
			.map(|(_, url)| url)
			.collect();
		self.peer_versions
			.write()
			.await
			.retain(|url, _| urls.contains(url));
		let mut pending = urls
			.into_iter()
			.map(|address| async move {
				let result = self.probe_version(&address).await;
				(address, result)
			})
			.collect::<FuturesUnordered<_>>();
		while let Some((url, result)) = pending.next().await {
			match result {
				Ok(version) => {
					self.peer_versions.write().await.insert(url, version);
				}
				Err(error) => {
					if clock_failure(&error) {
						return Err(error);
					}
					tracing::warn!(%error, "Epoxy peer version probe failed; retaining previously negotiated version")
				}
			}
		}
		Ok(())
	}

	/// Asks one peer for its protocol version.
	async fn probe_version(&self, address: &str) -> Result<u16> {
		let mut url = url::Url::parse(address)?;
		url.set_path("/epoxy/protocol-version");
		let response = self
			.http(
				rivet_env::capabilities::http::Method::GET,
				url,
				Vec::new(),
				Duration::from_secs(5),
			)
			.await?;
		let value: ProtocolVersionResponse = serde_json::from_slice(&response)?;
		ensure!(
			value.protocol_version >= OLDEST_PROTOCOL_VERSION,
			"Epoxy peer speaks a released Rivet 2.x protocol version that this build cannot talk to"
		);
		crate::metrics::VERSION_PROBES.inc();
		Ok(value.protocol_version)
	}

	/// Records the result of probing `address` on its first use. A failed probe leaves the
	/// peer unprobed, so its next request probes again beside it.
	async fn record_probe(&self, address: &str, probed: Result<u16>) -> Result<()> {
		match probed {
			Ok(version) => {
				self.peer_versions
					.write()
					.await
					.insert(address.to_owned(), version);
				Ok(())
			}
			Err(error) => {
				if clock_failure(&error) {
					return Err(error);
				}
				tracing::warn!(%error, "Epoxy peer version probe failed; it is retried on the next request");
				Ok(())
			}
		}
	}
}

impl ReplicaTransport for HttpTransport {
	fn admit(
		&self,
	) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>> {
		Box::pin(async move {
			ensure!(
				self.local_version() >= OLDEST_PROTOCOL_VERSION,
				"Epoxy protocol admission is not ready"
			);
			Ok(())
		})
	}

	fn request(&self, request: protocol::Request) -> ResponseFuture<'_> {
		Box::pin(async move {
			self.admit().await?;
			if request.to_replica_id == self.local.replica_id() {
				return self.local.handle(request).await;
			}
			let addresses = self.peer_addresses().await?;
			let address = addresses
				.get(&request.to_replica_id)
				.context("Epoxy target replica is absent from membership")?;
			// A peer is probed on its first use and after a version error, never on a timer.
			// The first request does not wait for the probe: it speaks the local version, as
			// an unprobed peer always did, while the probe runs beside it.
			let local_version = self.local_version();
			let cached = self.peer_versions.read().await.get(address).copied();
			let version = cached.unwrap_or(local_version).min(local_version);
			let mut url = url::Url::parse(address)?;
			url.set_path(&format!("/v{version}/epoxy/message"));
			let payload = versioned::encode_request(request, version)?;
			let send = async {
				let response = self
					.http(
						rivet_env::capabilities::http::Method::POST,
						url,
						payload,
						crate::consts::REQUEST_TIMEOUT,
					)
					.await?;
				versioned::decode_response(&response, version)
			};
			let result = match cached {
				Some(_) => send.await,
				None => {
					let (result, probed) = futures_util::join!(send, self.probe_version(address));
					self.record_probe(address, probed).await?;
					result
				}
			};
			// A rejected request or an undecodable reply may mean the peer's version
			// changed. Forget it, so the next request probes again.
			if let Err(error) = &result
				&& version_error(error)
			{
				self.peer_versions.write().await.remove(address);
			}
			result
		})
	}
}

impl HttpTransport {
	pub(crate) async fn http(
		&self,
		method: rivet_env::capabilities::http::Method,
		url: url::Url,
		body: Vec<u8>,
		timeout: Duration,
	) -> Result<bytes::Bytes> {
		let request = rivet_env::capabilities::http::Request::builder()
			.method(method)
			.uri(url.as_str())
			.body(bytes::Bytes::from(body))?;
		let response = rivet_env::timeout(
			self.local.client.env().clock().as_ref(),
			timeout,
			self.client.execute(request, 64 * 1024 * 1024),
		)
		.await??;
		let status = response.status();
		if status.is_client_error() || status.is_server_error() {
			return Err(PeerHttpStatus(status.as_u16()).into());
		}
		Ok(response.into_body())
	}
}

/// A peer answered with an HTTP error status.
#[derive(Debug)]
struct PeerHttpStatus(u16);

impl std::fmt::Display for PeerHttpStatus {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "Epoxy peer returned HTTP {}", self.0)
	}
}

impl std::error::Error for PeerHttpStatus {}

/// Whether a request failure may come from a version mismatch: the peer rejected the
/// request, or its reply did not decode. Timeouts and transport failures do not qualify.
fn version_error(error: &anyhow::Error) -> bool {
	match error.downcast_ref::<PeerHttpStatus>() {
		Some(PeerHttpStatus(status)) => (400..500).contains(status),
		None => {
			error.downcast_ref::<rivet_env::TimeoutError>().is_none()
				&& error.downcast_ref::<rivet_env::ClockError>().is_none()
				&& error.chain().all(|cause| !cause.is::<std::io::Error>())
		}
	}
}

fn clock_failure(error: &anyhow::Error) -> bool {
	error.downcast_ref::<rivet_env::ClockError>().is_some()
		|| error
			.downcast_ref::<rivet_env::TimeoutError>()
			.is_some_and(|error| matches!(error, rivet_env::TimeoutError::Clock(_)))
}


