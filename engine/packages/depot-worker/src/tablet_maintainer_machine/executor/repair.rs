//! Owns recovery clients and maps their requests onto the tablet's replica transport.

use crate::tablet_maintainer_machine::{
	event::MaintainerEvent,
	runner::{MaintainerAuthority, ReplicaTransport},
	types::{OpId, RecoveryRequest},
};
use depot_types::VolumeLogKey;
use depot_volume::volume_client_machine::{
	event::RpcError,
	executor::{
		TransportFuture, VolumeClientTransport,
		recovery::{Recovery, RecoveryAuthority, RecoveryConfig, RecoveryDependencies},
	},
	types::VolumeClientConfig,
};
use parking_lot::Mutex;
use rivet_data::generated::{
	depot_log_shard_v1::{VolumeLogEntry, VolumeLogReplicaState},
	depot_storage_worker_protocol_v1::{
		self as wire, StorageWorkerRequest as Request, StorageWorkerResponse as Response,
		TabletGrant,
	},
	shared_depot_fs_v1::FsVolumeRecord,
	shared_depot_types_v1::{ClientIdentity, VolumeLogPosition},
};
use rivet_env::{Env, TaskHandle};
use rivet_topology::worker::WorkerId;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{mpsc, oneshot};

#[derive(Default)]
pub(crate) struct RecoveryJobs(Mutex<BTreeMap<OpId, Job>>);

struct Job {
	cancel: Option<oneshot::Sender<()>>,
	task: TaskHandle,
}

impl RecoveryJobs {
	/// Cancellation signals the nested owner; aborting its parent future would lose the join barrier.
	pub(crate) fn cancel(&self, id: OpId) {
		if let Some(job) = self.0.lock().get_mut(&id)
			&& let Some(cancel) = job.cancel.take()
		{
			let _ = cancel.send(());
		}
	}

	/// Start only work selected by the core, retaining its task until completion or grant shutdown.
	pub(crate) fn start(
		&self,
		op_id: OpId,
		request: RecoveryRequest,
		adapters: RecoveryAdapters,
		inbox: mpsc::Sender<MaintainerEvent>,
	) {
		let (send, cancel) = oneshot::channel();
		let env = adapters.env.clone();
		let owner = env.spawn(Box::pin(run(op_id, request, adapters, inbox, cancel)));

		// Observe owner panics before forgetting a completed task. Panic cleanup cannot
		// establish a joined nested owner, so it must never be followed by grant release.
		let monitor_env = env.clone();
		let task = env.spawn(Box::pin(async move {
			if let Err(error) = owner.await {
				tracing::error!(%error, "recovery owner panicked");
				match monitor_env.exit(70).await {}
			}
		}));
		let mut jobs = self.0.lock();
		jobs.retain(|_, job| !job.task.is_finished());
		jobs.insert(
			op_id,
			Job {
				cancel: Some(send),
				task,
			},
		);
	}

	/// Take and signal every job synchronously, then join before closing the cold store and releasing the grant.
	pub(crate) fn stop(
		&self,
	) -> impl std::future::Future<Output = Result<(), String>> + Send + 'static {
		let jobs = std::mem::take(&mut *self.0.lock());
		let mut tasks = Vec::with_capacity(jobs.len());
		for (_, mut job) in jobs {
			if let Some(cancel) = job.cancel.take() {
				let _ = cancel.send(());
			}
			tasks.push(job.task);
		}
		async move {
			let mut failure = None;
			for task in tasks {
				if let Err(error) = task.await {
					failure = Some(error.to_string());
				}
			}
			match failure {
				Some(error) => Err(error),
				None => Ok(()),
			}
		}
	}
}

impl Drop for RecoveryJobs {
	fn drop(&mut self) {
		for job in self.0.get_mut().values_mut() {
			if let Some(cancel) = job.cancel.take() {
				let _ = cancel.send(());
			}
		}
	}
}

/// Fixed-grant runtime capabilities are kept out of the serialized recovery work.
pub(crate) struct RecoveryAdapters {
	pub replicas: Arc<dyn ReplicaTransport>,
	pub authority: Arc<dyn MaintainerAuthority>,
	pub grant: TabletGrant,
	pub env: Arc<dyn Env>,
}

/// Hold the nested owner through result delivery and cancellation, then join before replying.
async fn run(
	op_id: OpId,
	request: RecoveryRequest,
	adapters: RecoveryAdapters,
	inbox: mpsc::Sender<MaintainerEvent>,
	cancel: oneshot::Receiver<()>,
) {
	tracing::info!(
		operation_id = op_id.0,
		tablet_id = %uuid::Uuid::from_bytes(adapters.grant.tablet_id),
		grant_epoch = adapters.grant.tablet_grant_epoch,
		volume = ?request.volume_log_key,
		claim_epoch = request.claim_epoch,
		timeout_ms = request.timeout_ms,
		finish_recovery_timeout_ms = ?request.finish_recovery_timeout_ms,
		required_members = ?request.required_members,
		"volume recovery started"
	);
	let transport = Arc::new(RepairTransport {
		key: request.volume_log_key.clone(),
		row: request.row,
		replicas: adapters.replicas,
		claim_epoch: request.claim_epoch,
		tablet_replica_set_version: request.replica_set.tablet_replica_set_version,
	});
	let authority = Arc::new(Authority {
		authority: adapters.authority.clone(),
		grant: adapters.grant.clone(),
	});
	let config = RecoveryConfig {
		client: VolumeClientConfig {
			finish_recovery_timeout_ms: request
				.finish_recovery_timeout_ms
				.unwrap_or(VolumeClientConfig::default().finish_recovery_timeout_ms),
			..Default::default()
		},
		timeout_ms: request.timeout_ms,
		required_members: request.required_members,
	};
	let result = match Recovery::start(
		request.volume_log_key,
		request.replica_set,
		config,
		RecoveryDependencies {
			env: adapters.env,
			transport,
			authority,
		},
	) {
		Ok(mut recovery) => {
			let result = rivet_env::select_biased!("owner cancellation must stop recovery before publishing a result",
				_ = cancel => Err("recovery canceled".to_owned()),
				result = recovery.wait() => result.map_err(|error| format!("{error:?}")),
			);

			// Completion is not observable until all tasks belonging to the child have joined.
			match recovery.shutdown().await {
				Ok(()) => result,
				Err(error) => Err(error.to_string()),
			}
		}
		Err(error) => Err(error.to_string()),
	};
	tracing::info!(
		operation_id = op_id.0,
		tablet_id = %uuid::Uuid::from_bytes(adapters.grant.tablet_id),
		grant_epoch = adapters.grant.tablet_grant_epoch,
		succeeded = result.is_ok(),
		error = ?result.as_ref().err(),
		"volume recovery joined"
	);
	let event = if adapters.authority.may_serve(&adapters.grant) {
		MaintainerEvent::RecoverVolumeResponse { op_id, result }
	} else {
		MaintainerEvent::Fence
	};
	let _ = inbox.send(event).await;
}

struct Authority {
	authority: Arc<dyn MaintainerAuthority>,
	grant: TabletGrant,
}

impl RecoveryAuthority for Authority {
	fn may_run(&self) -> bool {
		self.authority.may_serve(&self.grant)
	}
}

/// The core already selected and loaded this incarnation. Its release is processed by
/// RecoverVolumeResponse for a recovery, or by the claim's close for a repair claim.
pub(crate) struct RepairTransport {
	key: VolumeLogKey,
	row: FsVolumeRecord,
	replicas: Arc<dyn ReplicaTransport>,
	/// The claim epoch the maintainer issued for this recovery.
	claim_epoch: u64,
	tablet_replica_set_version: u64,
}

impl RepairTransport {
	pub(crate) fn new(
		key: VolumeLogKey,
		row: FsVolumeRecord,
		replicas: Arc<dyn ReplicaTransport>,
		claim_epoch: u64,
		tablet_replica_set_version: u64,
	) -> Self {
		Self {
			key,
			row,
			replicas,
			claim_epoch,
			tablet_replica_set_version,
		}
	}

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

	async fn state(
		&self,
		worker: WorkerId,
		request: Request,
	) -> Result<VolumeLogReplicaState, RpcError> {
		match self
			.replicas
			.request(worker, request)
			.await
			.map_err(|_| RpcError::Unavailable)?
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

impl VolumeClientTransport for RepairTransport {
	/// The maintainer's repair client takes no caller writes, so it keeps no shard refusals.
	fn refusing_until(&self, _worker: &WorkerId, _now_boot_ms: u64) -> Option<u64> {
		None
	}

	fn open(
		&self,
		key: depot_types::VolumeKey,
		client_incarnation: [u8; 16],
		_create_if_absent: bool,
		_initial: Vec<rivet_data::generated::depot_log_shard_v1::PayloadWrite>,
		_read: Option<rivet_data::generated::depot_client_protocol_v1::VolumeReadKeys>,
		_deadline_boot_ms: u64,
	) -> TransportFuture<'_, rivet_data::generated::depot_client_protocol_v1::VolumeOpenResponse> {
		Box::pin(async move {
			if key == self.key.0.volume_key {
				Ok(
					rivet_data::generated::depot_client_protocol_v1::VolumeOpenResponse {
						record: self.row.clone(),
						// The recovery client claims every member at this granted epoch.
						open_claim: Some(rivet_data::generated::shared_depot_types_v1::OpenClaim {
							client: ClientIdentity {
								client_epoch: self.claim_epoch,
								client_incarnation,
							},
							tablet_replica_set_version: self.tablet_replica_set_version,
							holder: None,
						}),
						states: Vec::new(),
						created: None,
						read: None,
						applied: None,
					},
				)
			} else {
				Err(RpcError::Unavailable)
			}
		})
	}

	fn close(
		&self,
		key: VolumeLogKey,
		_client: Option<rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
	) -> TransportFuture<'_, ()> {
		// The owner retires this internal handle after joining recovery, without sending an application close.
		Box::pin(async move {
			if key == self.key {
				Ok(())
			} else {
				Err(RpcError::Unavailable)
			}
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
				holder: None,
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
		Box::pin(self.state(
			worker,
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
		))
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
		_max_entries: u32,
		max_bytes: u32,
	) -> TransportFuture<'_, (VolumeLogReplicaState, Vec<VolumeLogEntry>)> {
		Box::pin(async move {
			let request = Request::ReadVolumeLogEntriesRequest(wire::ReadVolumeLogEntriesRequest {
				include_deleted: false,
				volume_log_key: key.into(),
				first_volume_seq: start,
				max_bytes,
			});
			match self
				.replicas
				.request(worker, request)
				.await
				.map_err(|_| RpcError::Unavailable)?
			{
				Response::ReadVolumeLogEntriesResponse(response) => {
					Ok((response.state, response.entries))
				}
				Response::RejectResponse(reason) => Err(RpcError::Rejected(reason)),
				Response::ReplicaStateResponse(_)
				| Response::GetDiskIncarnationResponse(_)
				| Response::ListTabletReplicasResponse(_)
				| Response::ReadVolumeLogStatusResponse(_)
				| Response::ForgetVolumesResponse(_)
				| Response::TrimVolumesResponse(_)
				| Response::StorageWorkerBatchResponse(_) => Err(RpcError::Unavailable),
			}
		})
	}
}
