//! Maps client effects to replica messages; the core remains the sole owner of retries and fences.

use super::{UpsLogClient, metrics::RequestTimer};
use crate::volume_client_machine::{
	event::RpcError,
	executor::{TransportFuture, VolumeClientTransport},
};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_log_shard_v1::{VolumeLogEntry, VolumeLogReplicaState},
	depot_storage_worker_protocol_v1::{
		self as wire, StorageWorkerRequest as Request, StorageWorkerResponse as Response,
	},
	shared_depot_types_v1::{ClientIdentity, VolumeLogPosition},
};
use rivet_topology::worker::WorkerId;
use std::sync::Arc;

/// Tablet routing owns lifecycle; replica addresses come directly from client effects.
/// All calls have a bounded transport deadline.
pub trait VolumeLifecycleTransport: Send + Sync {
	fn open(
		&self,
		key: depot_types::VolumeKey,
		client_incarnation: [u8; 16],
		create_if_absent: bool,
		initial: Vec<rivet_data::generated::depot_log_shard_v1::PayloadWrite>,
		read: Option<rivet_data::generated::depot_client_protocol_v1::VolumeReadKeys>,
		deadline_boot_ms: u64,
	) -> TransportFuture<'_, rivet_data::generated::depot_client_protocol_v1::VolumeOpenResponse>;

	fn close(
		&self,
		key: VolumeLogKey,
		client: Option<rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
	) -> TransportFuture<'_, ()>;
}

/// Replica requests use UPS while lifecycle requests use the client's current tablet route.
/// The owner supplies the same dispatch guard to both paths and clears it on fencing.
pub struct UpsVolumeTransport {
	replicas: UpsLogClient,
	lifecycle: Arc<dyn VolumeLifecycleTransport>,
}

impl UpsVolumeTransport {
	pub fn new(replicas: UpsLogClient, lifecycle: Arc<dyn VolumeLifecycleTransport>) -> Self {
		Self {
			replicas,
			lifecycle,
		}
	}
}

impl UpsVolumeTransport {
	/// Preserves the version and fence chosen by the client core for this operation.
	fn header(
		&self,
		key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
	) -> wire::ClientRequestHeader {
		wire::ClientRequestHeader {
			volume_log_key: key.into(),
			client,
			tablet_replica_set_version,
		}
	}

	/// Accepts only the expected response arm; malformed or unrelated replies provide no evidence.
	async fn state(
		&self,
		worker: WorkerId,
		request: Request,
	) -> Result<VolumeLogReplicaState, RpcError> {
		let timer = RequestTimer::start(request_label(&request));
		let result = self.replica_state(worker, request).await;
		timer.finish(&result);
		result
	}

	async fn replica_state(
		&self,
		worker: WorkerId,
		request: Request,
	) -> Result<VolumeLogReplicaState, RpcError> {
		match self
			.replicas
			.request(&worker, request)
			.await
			.map_err(|error| transport_error(&worker, &error))?
		{
			Response::ReplicaStateResponse(response) => Ok(response.state),
			Response::RejectResponse(reason) => Err(RpcError::Rejected(reason)),
			Response::ReadVolumeLogEntriesResponse(_)
			| Response::GetDiskIncarnationResponse(_)
			| Response::ListTabletReplicasResponse(_)
			| Response::ReadVolumeLogStatusResponse(_)
			| Response::ForgetVolumesResponse(_)
			| Response::TrimVolumesResponse(_)
			| Response::StorageWorkerBatchResponse(_) => Err(RpcError::Unavailable),
		}
	}
}

impl VolumeClientTransport for UpsVolumeTransport {
	fn open(
		&self,
		key: depot_types::VolumeKey,
		client_incarnation: [u8; 16],
		create_if_absent: bool,
		initial: Vec<rivet_data::generated::depot_log_shard_v1::PayloadWrite>,
		read: Option<rivet_data::generated::depot_client_protocol_v1::VolumeReadKeys>,
		deadline_boot_ms: u64,
	) -> TransportFuture<'_, rivet_data::generated::depot_client_protocol_v1::VolumeOpenResponse> {
		Box::pin(async move {
			let timer = RequestTimer::start("open");
			let result = self
				.lifecycle
				.open(
					key,
					client_incarnation,
					create_if_absent,
					initial,
					read,
					deadline_boot_ms,
				)
				.await;
			timer.finish(&result);
			result
		})
	}

	fn close(
		&self,
		key: VolumeLogKey,
		client: Option<rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
	) -> TransportFuture<'_, ()> {
		Box::pin(async move {
			let timer = RequestTimer::start("close");
			let result = self.lifecycle.close(key, client).await;
			timer.finish(&result);
			result
		})
	}

	fn claim(
		&self,
		worker: WorkerId,
		key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
	) -> TransportFuture<'_, VolumeLogReplicaState> {
		Box::pin(self.state(
			worker,
			Request::ClaimRequest(wire::ClaimRequest {
				header: wire::ReplicaRequestHeader::ClientRequestHeader(self.header(
					key,
					client,
					tablet_replica_set_version,
				)),
				holder: Some(self.replicas.holder()),
			}),
		))
	}

	fn append(
		&self,
		worker: WorkerId,
		key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
		previous: VolumeLogPosition,
		entries: Vec<VolumeLogEntry>,
		committed: u64,
	) -> TransportFuture<'_, VolumeLogReplicaState> {
		Box::pin(async move {
			let sent_boot_ms = self.replicas.now_boot_ms();
			let result = self
				.state(
					worker.clone(),
					Request::AppendRequest(wire::AppendRequest {
						header: wire::ReplicaRequestHeader::ClientRequestHeader(self.header(
							key,
							client,
							tablet_replica_set_version,
						)),
						previous,
						entries,
						committed_volume_seq: committed,
					}),
				)
				.await;
			// A refusal carries the shard's own hint. Any other answer, a timeout included,
			// tells how long the worker took to answer; only a served append shows it quick
			// again, so an instant transport failure never ends a hold.
			if let Err(RpcError::Rejected(reason)) = &result {
				self.replicas.record_append_refusal(&worker, reason);
			} else if let Some(sent_boot_ms) = sent_boot_ms {
				self.replicas
					.record_append_latency(&worker, sent_boot_ms, result.is_ok());
			}
			result
		})
	}

	fn refusing_until(&self, worker: &WorkerId, now_boot_ms: u64) -> Option<u64> {
		self.replicas.refusing_until(worker, now_boot_ms)
	}

	fn advance_committed(
		&self,
		worker: WorkerId,
		key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
		committed: u64,
	) -> TransportFuture<'_, VolumeLogReplicaState> {
		Box::pin(self.state(
			worker,
			Request::AdvanceCommittedRequest(wire::AdvanceCommittedRequest {
				header: self.header(key, client, tablet_replica_set_version),
				committed_volume_seq: committed,
			}),
		))
	}

	fn truncate(
		&self,
		worker: WorkerId,
		key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
		expected_appended: VolumeLogPosition,
		keep: VolumeLogPosition,
		mark_repairing: bool,
	) -> TransportFuture<'_, VolumeLogReplicaState> {
		Box::pin(self.state(
			worker,
			Request::TruncateRequest(wire::TruncateRequest {
				expected_replica_incarnation: None,
				header: wire::ReplicaRequestHeader::ClientRequestHeader(self.header(
					key,
					client,
					tablet_replica_set_version,
				)),
				expected_appended,
				keep_through_volume_seq: keep.volume_seq,
				mark_repairing,
			}),
		))
	}

	fn mark_in_sync(
		&self,
		worker: WorkerId,
		key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
		replica_incarnation: [u8; 16],
		expected_appended: VolumeLogPosition,
	) -> TransportFuture<'_, VolumeLogReplicaState> {
		Box::pin(self.state(
			worker,
			Request::MarkInSyncRequest(wire::MarkInSyncRequest {
				header: wire::ReplicaRequestHeader::ClientRequestHeader(self.header(
					key,
					client,
					tablet_replica_set_version,
				)),
				replica_incarnation,
				expected_appended,
			}),
		))
	}

	fn read(
		&self,
		worker: WorkerId,
		key: VolumeLogKey,
		start: u64,
		max_entries: u32,
		max_bytes: u32,
	) -> TransportFuture<'_, (VolumeLogReplicaState, Vec<VolumeLogEntry>)> {
		Box::pin(async move {
			let request = Request::ReadVolumeLogEntriesRequest(wire::ReadVolumeLogEntriesRequest {
				include_deleted: false,
				volume_log_key: key.into(),
				first_volume_seq: start,
				max_bytes,
			});
			let timer = RequestTimer::start("read");
			let result = match self.replicas.request(&worker, request).await {
				Ok(Response::ReadVolumeLogEntriesResponse(response)) => {
					// The wire bounds bytes; the client also bounds retained entry handles.
					let mut entries = response.entries;
					entries.truncate(max_entries as usize);
					Ok((response.state, entries))
				}
				Ok(Response::RejectResponse(reason)) => Err(RpcError::Rejected(reason)),
				Ok(Response::ReplicaStateResponse(_))
				| Ok(Response::GetDiskIncarnationResponse(_))
				| Ok(Response::ListTabletReplicasResponse(_))
				| Ok(Response::ReadVolumeLogStatusResponse(_))
				| Ok(Response::ForgetVolumesResponse(_))
				| Ok(Response::TrimVolumesResponse(_))
				| Ok(Response::StorageWorkerBatchResponse(_)) => Err(RpcError::Unavailable),
				Err(error) => Err(transport_error(&worker, &error)),
			};
			timer.finish(&result);
			result
		})
	}
}

/// How long a request refused by UPS admission waits before it is sent again. UPS refusals
/// carry no hint; this matches the log shard's own inbox-full hint.
pub const TRANSPORT_OVERLOAD_RETRY_AFTER_MS: u64 = 100;

/// Classifies a failed replica request. A UPS overload, whether this process's queue to the
/// worker is full or the peer's transport refused admission, sent nothing that ran, so it is a
/// refusal rather than evidence that the replica failed. Every other failure may mean the
/// replica is unreachable.
fn transport_error(worker: &WorkerId, error: &anyhow::Error) -> RpcError {
	if is_ups_overloaded(error) {
		tracing::debug!(%worker, "replica UPS request refused as overloaded");
		return RpcError::Overloaded {
			retry_after_ms: TRANSPORT_OVERLOAD_RETRY_AFTER_MS,
		};
	}
	tracing::debug!(%worker, %error, "replica UPS request failed");
	RpcError::Unavailable
}

/// Whether UPS refused the request at admission, so nothing was sent that ran. Inspects the
/// structured UPS error, never its display text.
pub fn is_ups_overloaded(error: &anyhow::Error) -> bool {
	error.chain().any(|cause| {
		cause
			.downcast_ref::<rivet_error::RivetError>()
			.is_some_and(|error| error.group() == "ups" && error.code() == "overloaded")
	})
}

/// Bounded request label for latency metrics.
fn request_label(request: &Request) -> &'static str {
	depot_types::storage_worker::request_label(request)
}
