//! Owns the volume clients of operator repair claims and maps their requests onto the tablet's
//! replica transport.
//!
//! Each claim's client is an ordinary serving volume client. Its open is answered locally
//! with the maintainer's record and a claim at the epoch the maintainer issued, exactly as a
//! recovery client's is, and every replica request it makes first checks the maintainer's
//! grant and lease, so a client of a revoked maintainer sends nothing more. One task owns
//! each client and runs the core's appends and close in order.

use super::repair::RepairTransport;
use crate::tablet_maintainer_machine::{
	event::{MaintainerEvent, RepairEvent},
	runner::{MaintainerAuthority, ReplicaFuture, ReplicaTransport},
	types::{OpId, RepairAppendFailure, RepairClaimOpen, RepairClaimOpened},
};
use depot_types::VolumeLogKey;
use depot_volume::volume_client_machine::{
	executor::VolumeClientTransport,
	runner::{Volume, VolumeHandle},
	state::VolumeClientState,
	types::{ErrorDisposition, VolumeClientConfig},
};
use parking_lot::Mutex;
use rivet_data::generated::{
	depot_log_shard_v1::{PayloadWrite, VolumeLogEntryBody},
	depot_storage_worker_protocol_v1::{StorageWorkerRequest, TabletGrant},
	shared_depot_types_v1::VolumeLogPosition,
};
use rivet_env::{Env, TaskHandle};
use rivet_topology::worker::WorkerId;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::mpsc;

/// Fixed-grant runtime capabilities of the repair clients.
pub(crate) struct ClaimAdapters {
	pub replicas: Arc<dyn ReplicaTransport>,
	pub authority: Arc<dyn MaintainerAuthority>,
	pub grant: TabletGrant,
	pub env: Arc<dyn Env>,
}

/// What the core asks a claim's owner task to do after it opened.
enum Command {
	Append { op_id: OpId, body: PayloadWrite },
	Close { op_id: OpId },
}

struct Client {
	commands: mpsc::UnboundedSender<Command>,
	task: TaskHandle,
}

/// The repair clients of one maintainer lifetime, by volume, and the owners of clients that are
/// still closing, which `stop` joins too.
#[derive(Default)]
pub(crate) struct RepairClients {
	clients: Mutex<BTreeMap<VolumeLogKey, Client>>,
	closing: Mutex<Vec<TaskHandle>>,
}

impl RepairClients {
	/// Starts the owner task of one claim's client, which opens it and reports the open.
	pub(crate) fn open(
		&self,
		op_id: OpId,
		request: RepairClaimOpen,
		adapters: ClaimAdapters,
		inbox: mpsc::Sender<MaintainerEvent>,
	) {
		let (commands, receiver) = mpsc::unbounded_channel();
		let env = adapters.env.clone();
		let key = request.volume_log_key.clone();
		let owner = env.spawn(Box::pin(run(op_id, request, adapters, inbox, receiver)));
		let monitor_env = env.clone();
		// A panicked owner cannot prove its client stopped writing; end the process rather
		// than release the grant behind it.
		let task = env.spawn(Box::pin(async move {
			if let Err(error) = owner.await {
				tracing::error!(%error, "repair claim owner panicked");
				match monitor_env.exit(70).await {}
			}
		}));
		let mut clients = self.clients.lock();
		clients.retain(|_, client| !client.task.is_finished());
		clients.insert(key, Client { commands, task });
	}

	/// Sends one append to the claim's client. A claim with no live client answers that
	/// nothing was written.
	pub(crate) fn append(
		&self,
		op_id: OpId,
		key: &VolumeLogKey,
		body: PayloadWrite,
		env: &Arc<dyn Env>,
		inbox: mpsc::Sender<MaintainerEvent>,
	) {
		let sent = self.clients.lock().get(key).is_some_and(|client| {
			client
				.commands
				.send(Command::Append { op_id, body })
				.is_ok()
		});
		if !sent {
			answer(
				env,
				inbox,
				RepairEvent::ClaimAppendResponse {
					op_id,
					result: Err(RepairAppendFailure::NotExecuted(
						"the repair client is not running".into(),
					)),
				},
			);
		}
	}

	/// Closes the claim's client. A claim with no live client answers at once.
	pub(crate) fn close(
		&self,
		op_id: OpId,
		key: &VolumeLogKey,
		env: &Arc<dyn Env>,
		inbox: mpsc::Sender<MaintainerEvent>,
	) {
		let client = self.clients.lock().remove(key);
		let sent = client
			.as_ref()
			.is_some_and(|client| client.commands.send(Command::Close { op_id }).is_ok());
		// The owner keeps closing after this; `stop` waits for it before the grant ends.
		if let Some(client) = client {
			let mut closing = self.closing.lock();
			closing.retain(|task| !task.is_finished());
			closing.push(client.task);
		}
		if !sent {
			answer(env, inbox, RepairEvent::ClaimClosedResponse { op_id });
		}
	}

	/// Takes every client and drops its command channel, which makes its owner close the
	/// client and stop, then joins them before the maintainer closes its store and releases
	/// its grant.
	pub(crate) fn stop(
		&self,
	) -> impl std::future::Future<Output = Result<(), String>> + Send + 'static {
		let clients = std::mem::take(&mut *self.clients.lock());
		let mut tasks = clients
			.into_values()
			.map(|client| client.task)
			.collect::<Vec<_>>();
		tasks.extend(std::mem::take(&mut *self.closing.lock()));
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

/// Delivers one repair event from a sync executor call.
fn answer(env: &Arc<dyn Env>, inbox: mpsc::Sender<MaintainerEvent>, event: RepairEvent) {
	env.spawn(Box::pin(async move {
		let _ = inbox.send(MaintainerEvent::Repair(event)).await;
	}));
}

/// Replica requests of a repair client, refused once the maintainer's grant or lease ended or
/// the client itself stopped serving.
struct GuardedReplicas {
	inner: Arc<dyn ReplicaTransport>,
	authority: Arc<dyn MaintainerAuthority>,
	grant: TabletGrant,
	serving: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl ReplicaTransport for GuardedReplicas {
	fn request(&self, worker: WorkerId, request: StorageWorkerRequest) -> ReplicaFuture<'_> {
		if !self.authority.may_serve(&self.grant) || !(self.serving)() {
			return Box::pin(async { Err("repair claim authority ended".to_owned()) });
		}
		self.inner.request(worker, request)
	}
}

/// Opens the claim's client, reports the open, then runs the core's commands in order until
/// a close or the end of the lifetime.
async fn run(
	op_id: OpId,
	request: RepairClaimOpen,
	adapters: ClaimAdapters,
	inbox: mpsc::Sender<MaintainerEvent>,
	mut commands: mpsc::UnboundedReceiver<Command>,
) {
	let authority = adapters.authority.clone();
	let grant = adapters.grant.clone();
	let may_reply = || authority.may_serve(&grant);
	let opened = open(&request, &adapters).await;
	let (volume, handle) = match opened {
		Ok((volume, handle, report)) => {
			let event = match may_reply() {
				true => MaintainerEvent::Repair(RepairEvent::ClaimOpenResponse {
					op_id,
					result: Ok(report),
				}),
				false => MaintainerEvent::Fence,
			};
			let _ = inbox.send(event).await;
			(volume, handle)
		}
		Err((volume, error)) => {
			if let Some(volume) = volume
				&& let Err(error) = volume.shutdown().await
			{
				tracing::warn!(%error, "a repair client that failed to open did not shut down cleanly");
			}
			let event = match may_reply() {
				true => MaintainerEvent::Repair(RepairEvent::ClaimOpenResponse {
					op_id,
					result: Err(error),
				}),
				false => MaintainerEvent::Fence,
			};
			let _ = inbox.send(event).await;
			// The core closes a failed claim; answer that close once it comes.
			while let Some(command) = commands.recv().await {
				let event = match command {
					Command::Close { op_id } => RepairEvent::ClaimClosedResponse { op_id },
					Command::Append { op_id, .. } => RepairEvent::ClaimAppendResponse {
						op_id,
						result: Err(RepairAppendFailure::NotExecuted(
							"the repair client did not open".into(),
						)),
					},
				};
				let closing = matches!(event, RepairEvent::ClaimClosedResponse { .. });
				let _ = inbox.send(MaintainerEvent::Repair(event)).await;
				if closing {
					break;
				}
			}
			return;
		}
	};

	let mut close = None;
	while let Some(command) = commands.recv().await {
		match command {
			Command::Append { op_id, body } => {
				let result = handle
					.write(VolumeLogEntryBody::PayloadWrite(body), None)
					.await
					.map(|outcome| outcome.position)
					.map_err(|error| match error.disposition() {
						ErrorDisposition::NotExecuted => {
							RepairAppendFailure::NotExecuted(format!("{error:?}"))
						}
						ErrorDisposition::OutcomeUnknown | ErrorDisposition::Permanent => {
							RepairAppendFailure::Unknown(format!("{error:?}"))
						}
					});
				let event = match may_reply() {
					true => {
						MaintainerEvent::Repair(RepairEvent::ClaimAppendResponse { op_id, result })
					}
					false => MaintainerEvent::Fence,
				};
				let _ = inbox.send(event).await;
			}
			Command::Close { op_id } => {
				close = Some(op_id);
				break;
			}
		}
	}
	// A close or the end of the lifetime closes the client: its writes are committed on a
	// quorum already, so the close only drains it.
	if let Err(error) = handle.close().await {
		tracing::debug!(
			?error,
			"a repair client's close did not finish; shutting it down"
		);
	}
	if let Err(error) = volume.shutdown().await {
		tracing::warn!(%error, "a repair client did not shut down cleanly");
	}
	if let Some(op_id) = close {
		let _ = inbox
			.send(MaintainerEvent::Repair(RepairEvent::ClaimClosedResponse {
				op_id,
			}))
			.await;
	}
}

type Opened = (Volume, VolumeHandle, RepairClaimOpened);

/// Starts the claim's client and waits for its claim entry's quorum.
async fn open(
	request: &RepairClaimOpen,
	adapters: &ClaimAdapters,
) -> Result<Opened, (Option<Volume>, String)> {
	let config = VolumeClientConfig {
		open_timeout_ms: request.timeout_ms,
		quorum_timeout_ms: request.timeout_ms,
		..Default::default()
	};
	let state = VolumeClientState::with_replica_set(
		config,
		request.volume_log_key.clone(),
		request.replica_set.clone(),
	)
	.map_err(|error| (None, error.to_string()))?;
	let replicas = adapters.replicas.clone();
	let authority = adapters.authority.clone();
	let grant = adapters.grant.clone();
	let key = request.volume_log_key.clone();
	let row = request.row.clone();
	let claim_epoch = request.claim_epoch;
	let version = request.replica_set.tablet_replica_set_version;
	let volume = Volume::start(adapters.env.clone(), state, move |serving| {
		Ok(Arc::new(RepairTransport::new(
			key,
			row,
			Arc::new(GuardedReplicas {
				inner: replicas,
				authority,
				grant,
				serving,
			}),
			claim_epoch,
			version,
		)) as Arc<dyn VolumeClientTransport>)
	})
	.map_err(|error| (None, error.to_string()))?;
	let handle = volume.handle();
	let clock = adapters.env.clock();
	let ready = rivet_env::timeout(
		clock.as_ref(),
		std::time::Duration::from_millis(request.timeout_ms),
		handle.ready(),
	)
	.await;
	match ready {
		Ok(Ok(())) => {}
		Ok(Err(error)) => return Err((Some(volume), format!("{error:?}"))),
		Err(_) => {
			return Err((
				Some(volume),
				"the claim entry did not commit in time".into(),
			));
		}
	}
	let snapshot = handle.snapshot.borrow().clone();
	let Some(claim_entry) = snapshot.claim_entry_volume_seq else {
		return Err((Some(volume), "the client opened with no claim entry".into()));
	};
	let (in_sync, incomplete) = snapshot
		.replicas
		.iter()
		.partition::<Vec<_>, _>(|replica| replica.matched && replica.acked >= claim_entry);
	let report = RepairClaimOpened {
		client: snapshot.client.clone(),
		claim_entry: VolumeLogPosition {
			volume_seq: claim_entry,
			client_epoch: snapshot.client.client_epoch,
		},
		in_sync: in_sync
			.into_iter()
			.map(|replica| replica.worker_id.clone())
			.collect(),
		incomplete: incomplete
			.into_iter()
			.map(|replica| replica.worker_id.clone())
			.collect(),
	};
	Ok((volume, handle, report))
}
