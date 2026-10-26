//! Regional filesystem RPCs use configured peer URLs and the injected HTTP adapter.
//! The receiving region resolves its own tablet placement through the usual UPS path.
use crate::Client;
use anyhow::{Context, Result, ensure};
use rivet_data::{
	DEPOT_CLIENT_PROTOCOL_VERSION,
	generated::depot_client_protocol_v1::{ClientRequest, ClientResponse},
	versioned::{DepotClientRequestData, DepotClientResponseData},
};
use rivet_env::{
	Clock,
	capabilities::http::{HttpClient, Request},
};
use std::{sync::Arc, time::Duration};
use vbare::Versioned;

pub const PEER_PATH: &str = "/v1/depot/filesystem";
pub const REGION_HEADER: &str = "x-rivet-depot-region";
pub const TIMEOUT_HEADER: &str = "x-rivet-depot-timeout-ms";
pub const MAX_MESSAGE_BYTES: usize = depot_fs_core::wire::DEFAULT_MAX_READ_BYTES + 64 * 1024;

#[derive(Debug, thiserror::Error)]
#[error("filesystem region {0} is unavailable")]
pub struct RegionUnavailable(pub u16);

/// Construction installs routes only. The process owns the HTTP and UPS service lifetimes.
pub struct Regions {
	topology: rivet_config::config::topology::Topology,
	http: Arc<dyn HttpClient>,
	clock: Arc<dyn Clock>,
}
impl Regions {
	pub fn new(
		topology: rivet_config::config::topology::Topology,
		http: Arc<dyn HttpClient>,
		clock: Arc<dyn Clock>,
	) -> Self {
		Self {
			topology,
			http,
			clock,
		}
	}
	pub fn local(&self) -> u16 {
		self.topology.datacenter_label
	}
	pub(crate) fn contains(&self, region: u16) -> bool {
		self.topology.dc_for_label(region).is_some()
	}

	/// One attempt preserves the exact body. The operation owner decides whether an
	/// uncertain command can be retried; HTTP redirects and retries are not requested here.
	async fn request(
		&self,
		region: u16,
		request: ClientRequest,
		deadline: u64,
	) -> Result<ClientResponse> {
		validate_request(region, &request)?;
		let peer = self
			.topology
			.dc_for_label(region)
			.ok_or(RegionUnavailable(region))?;
		let remaining = deadline.saturating_sub(self.clock.observe()?.boot_ms);
		ensure!(remaining > 0, "regional request deadline expired");
		let bytes = DepotClientRequestData::serialize_with_embedded_version(
			&request,
			DEPOT_CLIENT_PROTOCOL_VERSION,
		)?;
		ensure!(
			bytes.len() <= MAX_MESSAGE_BYTES,
			"regional request too large"
		);
		let request = Request::post(peer.peer_url.join(PEER_PATH)?.as_str())
			.header(REGION_HEADER, region.to_string())
			.header(TIMEOUT_HEADER, remaining.to_string())
			.header("content-type", "application/octet-stream")
			.body(bytes.into())?;
		let response = rivet_env::timeout(
			self.clock.as_ref(),
			Duration::from_millis(remaining),
			self.http.execute(request, MAX_MESSAGE_BYTES),
		)
		.await??;
		ensure!(
			self.clock.observe()?.boot_ms < deadline,
			"regional reply arrived after deadline"
		);
		ensure!(
			response.status().is_success(),
			"regional peer rejected request: {}",
			response.status()
		);
		DepotClientResponseData::deserialize_with_embedded_version(response.body())
	}
}

/// Keep the regional peer surface limited to filesystem fork coordination. Exact
/// source identity is checked at both the regional boundary and the tablet owner.
pub fn validate_request(region: u16, request: &ClientRequest) -> Result<()> {
	match request {
		ClientRequest::FsForkRequest(request) => {
			ensure!(
				depot_fs_core::forks::valid_spec(&request.spec, region),
				"invalid fork destination"
			);
			Ok(())
		}
		ClientRequest::FsForkReferenceRequest(request) => {
			ensure!(request.region() == region, "fork reference region mismatch");
			ensure!(
				depot_fs_core::forks::valid_spec(request.spec(), request.spec().destination_region),
				"invalid fork spec"
			);
			Ok(())
		}
		ClientRequest::FsReadRequest(request) => {
			let link = depot_fs_core::wire::source_link(&request.operation)
				.context("not a held-image request")?;
			ensure!(
				depot_fs_core::wire::valid_source_target(request),
				"source identity mismatch"
			);
			ensure!(
				link.region.is_none_or(|r| r == region),
				"source region mismatch"
			);
			Ok(())
		}
		_ => anyhow::bail!("not a regional filesystem request"),
	}
}

impl Client {
	pub fn with_regions(&self, regions: Arc<Regions>) -> Self {
		Self {
			regions: Some(regions),
			..self.clone()
		}
	}
	pub fn local_region(&self) -> Result<u16> {
		Ok(self
			.regions
			.as_ref()
			.context("filesystem regional routing not configured")?
			.local())
	}
	fn regional_admission(&self) -> Result<()> {
		ensure!(self.topology.may_admit()?, "regional caller is not serving");
		if self.dispatch_guard.as_ref().is_some_and(|guard| !guard()) {
			return Err(crate::ScopeRevoked.build());
		}
		Ok(())
	}

	/// A peer request is delivered locally once. It never bounces between regions.
	pub async fn filesystem_peer_request(
		&self,
		region: u16,
		request: ClientRequest,
		deadline_boot_ms: u64,
	) -> Result<ClientResponse> {
		ensure!(self.local_region()? == region, "wrong destination region");
		validate_request(region, &request)?;
		self.regional_admission()?;
		let response = self
			.volume_handle()?
			.read_requests()
			.request_at(request, deadline_boot_ms)
			.await?;
		self.regional_admission()?;
		Ok(response)
	}

	pub async fn request_in_region(
		&self,
		region: u16,
		request: ClientRequest,
		deadline_boot_ms: u64,
	) -> Result<ClientResponse> {
		let regions = self.regions.as_ref().ok_or(RegionUnavailable(region))?;
		self.regional_admission()?;
		let response = if region == regions.local() {
			self.filesystem_peer_request(region, request, deadline_boot_ms)
				.await?
		} else {
			regions.request(region, request, deadline_boot_ms).await?
		};
		self.regional_admission()?;
		Ok(response)
	}
}


