//! Regional filesystem routing uses the shared HTTP boundary and current context client.
use rivet_data::generated::{depot_client_protocol_v2::*, shared_depot_fs_read_v2::*};
use rivet_engine_context::EngineContext;

/// Cloning a context does not start services. Resolve its current installed client
/// at each call so startup ordering cannot capture a permanently unready handle.
pub(crate) struct BaseReader(pub EngineContext);
#[async_trait::async_trait]
impl depot_worker::filesystem::base::BaseLayerReader for BaseReader {
	async fn read(
		&self,
		link: &depot_fs_core::FsBaseLink,
		ino: u64,
		indices: &[u64],
		deadline_boot_ms: u64,
	) -> depot_worker::filesystem::session::Result<Vec<depot_fs_core::forks::ChunkLookup>> {
		use depot_worker::filesystem::session::Error;
		let region = link
			.region
			.unwrap_or(self.0.config().topology().datacenter_label);
		let request = FsReadRequest {
			target: FsReadTarget::VolumeLogKey(
				rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
					volume_key: link.volume_key.clone(),
					volume_incarnation: link.incarnation,
				},
			),
			client_epoch: None,
			operation: FsReadOperation::FsBaseChunksRequest(FsBaseChunksRequest {
				link: link.clone(),
				ino,
				indices: indices.to_vec(),
			}),
		};
		let response = self
			.0
			.depot()
			.request_in_region(
				region,
				ClientRequest::FsReadRequest(request),
				deadline_boot_ms,
			)
			.await
			.map_err(|error| transport_error(&error))?;
		match response {
			ClientResponse::FsReadResponse(FsReadResponse::FsBaseChunksResponse(response))
				if response.chunks.len() == indices.len() =>
			{
				response
					.chunks
					.into_iter()
					.map(|v| v.try_into().map_err(|_| Error::Corrupt))
					.collect()
			}
			ClientResponse::FsReadResponse(FsReadResponse::FsReadError(error)) => {
				Err(depot_worker::filesystem::base::base_read_error(error))
			}
			ClientResponse::VolumeRejectResponse(reject) => Err(reject_error(&reject)),
			_ => Err(Error::Corrupt),
		}
	}

	async fn rows(
		&self,
		link: &depot_fs_core::FsBaseLink,
		read: FsBaseRead,
		deadline_boot_ms: u64,
	) -> depot_worker::filesystem::session::Result<FsBaseRowsPage> {
		self.base_rows(link, read, deadline_boot_ms).await
	}
}

impl BaseReader {
	/// Sends one base rows read to the link's region and answers its page.
	async fn base_rows(
		&self,
		link: &depot_fs_core::FsBaseLink,
		read: FsBaseRead,
		deadline_boot_ms: u64,
	) -> depot_worker::filesystem::session::Result<FsBaseRowsPage> {
		use depot_worker::filesystem::session::Error;
		let region = link
			.region
			.unwrap_or(self.0.config().topology().datacenter_label);
		let request = FsReadRequest {
			target: FsReadTarget::VolumeLogKey(
				rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
					volume_key: link.volume_key.clone(),
					volume_incarnation: link.incarnation,
				},
			),
			client_epoch: None,
			operation: FsReadOperation::FsBaseRowsRequest(FsBaseRowsRequest {
				link: link.clone(),
				read,
			}),
		};
		let response = self
			.0
			.depot()
			.request_in_region(
				region,
				ClientRequest::FsReadRequest(request),
				deadline_boot_ms,
			)
			.await
			.map_err(|error| transport_error(&error))?;
		match response {
			ClientResponse::FsReadResponse(FsReadResponse::FsBaseRowsPage(page)) => Ok(page),
			ClientResponse::FsReadResponse(FsReadResponse::FsReadError(error)) => {
				tracing::debug!(?error, "base rows read refused by the parent's maintainer");
				Err(depot_worker::filesystem::base::base_read_error(error))
			}
			ClientResponse::VolumeRejectResponse(reject) => {
				tracing::debug!(
					?reject,
					"base rows read rejected by the parent's maintainer"
				);
				Err(reject_error(&reject))
			}
			_ => Err(Error::Corrupt),
		}
	}
}

/// The base read error for a base layer request that failed in transport: `Overloaded`,
/// which the actor's read retries, when the request was refused before it ran for a reason
/// that passes (`depot_client::refused_before_running`), else `BaseUnavailable`.
fn transport_error(error: &anyhow::Error) -> depot_worker::filesystem::session::Error {
	use depot_worker::filesystem::session::Error;
	if depot_client::refused_before_running(error) {
		Error::Overloaded
	} else {
		Error::BaseUnavailable
	}
}

/// The base read error for a base layer request the parent's maintainer rejected:
/// `Overloaded` for a rejection that passes, such as shed load, a repair claim or a moved or
/// splitting maintainer, else `BaseUnavailable`.
fn reject_error(reject: &VolumeRejectResponse) -> depot_worker::filesystem::session::Error {
	use depot_worker::filesystem::session::Error;
	if depot_client::reject_refused_before_running(reject) {
		Error::Overloaded
	} else {
		Error::BaseUnavailable
	}
}

impl depot_worker::group_events::GroupCommands for BaseReader {
	fn group_read(
		&self,
		group: rivet_keys::keys::codec::GroupId,
		request: rivet_data::generated::depot_group_v1::GroupReadRequest,
	) -> depot_worker::group_events::GroupCommandFuture<
		'_,
		rivet_data::generated::depot_group_v1::GroupReadResponse,
	> {
		Box::pin(async move {
			let volume = depot_types::keys::partition::GroupVolume::new(group).volume_key();
			match self
				.0
				.depot()
				.command::<depot_protocol::command::GroupRead>(&volume, request)
				.await
			{
				Ok(response) => Ok(response),
				// A group whose volume was never written has no record and no forks.
				Err(error) if depot_client::groups::is_volume_not_found(&error) => {
					Ok(rivet_data::generated::depot_group_v1::GroupReadResponse {
						commit_lag: 0,
						record: None,
						children: Vec::new(),
					})
				}
				Err(error) => Err(error.to_string()),
			}
		})
	}

	fn events_read(
		&self,
		shard: u64,
		request: rivet_data::generated::depot_group_v1::EventsReadRequest,
	) -> depot_worker::group_events::GroupCommandFuture<
		'_,
		rivet_data::generated::depot_group_v1::EventsReadResponse,
	> {
		Box::pin(async move {
			let volume = depot_types::keys::partition::EventShardVolume::new(shard).volume_key();
			match self
				.0
				.depot()
				.command::<depot_protocol::command::EventsRead>(&volume, request)
				.await
			{
				Ok(response) => Ok(response),
				// A shard nothing was posted to yet is empty and its tail is zero.
				Err(error) if depot_client::groups::is_volume_not_found(&error) => {
					Ok(rivet_data::generated::depot_group_v1::EventsReadResponse {
						rows: Vec::new(),
						next: 0,
						oldest: 0,
					})
				}
				Err(error) => Err(error.to_string()),
			}
		})
	}
}

impl depot_worker::tablet_maintainer_machine::runner::ForkPeer for BaseReader {
	fn request(
		&self,
		region: u16,
		request: ClientRequest,
		deadline_boot_ms: u64,
	) -> depot_worker::tablet_maintainer_machine::runner::ForkPeerFuture<'_> {
		Box::pin(async move {
			self.0
				.depot()
				.request_in_region(region, request, deadline_boot_ms)
				.await
				.map_err(|e| e.to_string())
		})
	}
}
