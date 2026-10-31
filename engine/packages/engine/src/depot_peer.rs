//! Regional filesystem routing uses the shared HTTP boundary and current context client.
use rivet_data::generated::{depot_client_protocol_v1::*, shared_depot_fs_read_v1::*};
use rivet_engine_context::EngineContext;

/// Cloning a context does not start services. Resolve its current installed client
/// at each call so startup ordering cannot capture a permanently unready handle.
pub(crate) struct BaseReader(pub EngineContext);
#[async_trait::async_trait]
impl depot_worker::filesystem::base::BaseChunkReader for BaseReader {
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
			.map_err(|_| Error::BaseUnavailable)?;
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
			ClientResponse::FsReadResponse(FsReadResponse::FsReadError(_))
			| ClientResponse::VolumeRejectResponse(_) => Err(Error::BaseUnavailable),
			_ => Err(Error::Corrupt),
		}
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
