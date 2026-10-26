//! Owns one application volume client, its payloads, caller replies and transport tasks.

use super::{
	VolumeClientMachine,
	effect::VolumeClientEffect as Effect,
	event::VolumeClientEvent as Input,
	executor::{
		Payloads, VolumeClientTransport, entry_encoded_bytes, entry_exceeds_bytes, payload_digest,
		requests::RequestExecutor,
	},
	snapshot::VolumeClientSnapshot,
	state::{RecoveryPhase, VolumeClientPhase, VolumeClientState},
	types::*,
};
use parking_lot::Mutex;
use rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody;
use rivet_env::{Env, TaskScope};
use rivet_state_machine_ephemeral::{EffectContext, Executor, PublishObserver, Runner};
use std::{
	collections::BTreeMap,
	sync::{
		Arc, OnceLock, Weak,
		atomic::{AtomicBool, Ordering},
	},
};
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Clone, Debug)]
pub struct LogHistory {
	pub cold: rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
	pub committed: u64,
	pub entries: Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntry>,
	/// The volume holds no rows at all: nothing reached cold, there is no fork base, and the
	/// only committed entry is this opening's claim entry at sequence 1, written under a claim the
	/// maintainer granted. A reader may then skip reading cold and replaying the log. Pages
	/// from `read_committed` leave it false.
	pub empty: bool,
	/// Nothing reached cold and there is no fork base, and `entries` holds every committed
	/// entry, so a reader can build the whole volume from `entries` without reading cold rows
	/// or the log. Set for a volume this client created with its initial writes.
	pub complete: bool,
}

/// A commit barrier is registered before its future is returned to the caller.
pub type CommitTicket =
	std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64, WriteError>> + Send + 'static>>;

enum Reply {
	Open(Result<(), WriteError>),
	Committed(Result<u64, WriteError>),
	Write(Result<WriteOutcome, WriteError>),
	Close(Result<(), WriteError>),
}

#[derive(Default)]
struct Callers {
	next: u64,
	waiting: BTreeMap<RequestId, oneshot::Sender<Reply>>,
}

impl Callers {
	/// Registered only after reserving an inbox slot; the core bounds outstanding caller work.
	fn register(
		&mut self,
		closing: &AtomicBool,
	) -> Result<(RequestId, oneshot::Receiver<Reply>), WriteError> {
		if closing.load(Ordering::Acquire) {
			return Err(WriteError::Fenced { token: None });
		}
		self.next = self
			.next
			.checked_add(1)
			.ok_or_else(WriteError::client_overloaded)?;
		let id = RequestId(self.next);
		let (send, receive) = oneshot::channel();
		self.waiting.insert(id, send);
		Ok((id, receive))
	}

	fn reply(&mut self, id: RequestId, reply: Reply) {
		if let Some(send) = self.waiting.remove(&id) {
			let _ = send.send(reply);
		}
	}
}

/// Clones submit work; the separate Volume owner must remain alive and join shutdown.
#[derive(Clone)]
pub struct VolumeHandle {
	tasks: Weak<TaskScope>,
	transport: Arc<dyn VolumeClientTransport>,
	clock: Arc<dyn rivet_env::Clock>,
	pub snapshot: watch::Receiver<VolumeClientSnapshot>,
	inbox: mpsc::WeakSender<Input>,
	payloads: Arc<Mutex<Payloads>>,
	callers: Arc<Mutex<Callers>>,
	closing: Arc<AtomicBool>,
	/// The bodies of this opening's initial writes, so a reader of a volume it created can
	/// rebuild it without reading the log. `None` when it had none.
	initial: Arc<Mutex<Option<Vec<VolumeLogEntryBody>>>>,
}

impl VolumeHandle {
	pub fn spawn_cleanup(
		&self,
		cleanup: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>>,
	) -> Result<(), WriteError> {
		if self.is_fenced() {
			return Err(WriteError::Fenced { token: None });
		}
		self.tasks
			.upgrade()
			.ok_or(WriteError::Fenced { token: None })?
			.spawn(cleanup);
		Ok(())
	}
	/// Local revocation is visible before the queued fence reaches the runner.
	pub fn is_fenced(&self) -> bool {
		self.closing.load(Ordering::Acquire)
	}

	/// Revokes this local client's admission after an application-level uncertain outcome.
	pub fn fence(&self) {
		self.closing.store(true, Ordering::Release);
		if let Some(inbox) = self.inbox.upgrade() {
			let _ = inbox.try_send(Input::Fence);
		}
	}

	/// Offers a newer tablet replica set from the owner's manifest view. Best effort: a full
	/// inbox drops it, and the owner offers it again at the next manifest change.
	pub fn observe_replica_set(
		&self,
		replica_set: rivet_data::generated::shared_depot_types_v1::TabletReplicaSet,
	) {
		if replica_set.tablet_replica_set_version
			<= self
				.snapshot
				.borrow()
				.replica_set
				.tablet_replica_set_version
		{
			return;
		}
		if let Some(inbox) = self.inbox.upgrade() {
			let _ = inbox.try_send(Input::ReplicaSetObserved { replica_set });
		}
	}

	/// Captures history bounds without retaining the committed payloads in RAM. When this
	/// opening's claim entry is the only committed entry above cold, `entries` holds it, so a
	/// replay reads nothing back.
	pub async fn history(&self) -> Result<LogHistory, WriteError> {
		if let Some(history) = self.empty_history() {
			return Ok(history);
		}
		self.ready().await?;
		let snapshot = self.snapshot.borrow();
		if self.is_fenced() || snapshot.phase != VolumeClientPhase::Appending {
			return Err(WriteError::Fenced { token: None });
		}
		// No write ever committed when nothing above a zero cold position is committed, or when
		// the only committed entry is this opening's claim entry at sequence 1. Every claim is
		// granted by the maintainer, which therefore knows the volume.
		let empty = snapshot.cold.volume_seq == 0
			&& !snapshot.has_base_chain
			&& snapshot.granted_claim
			&& (snapshot.committed == 0
				|| (snapshot.committed == 1 && snapshot.claim_entry_volume_seq == Some(1)));
		// When the claim entry is the only committed entry above cold, this client already knows
		// it exactly, so a replay needs no read of it from a replica.
		let entries = match (
			snapshot.claim_entry_volume_seq,
			snapshot.claim_entry_written_at_unix_ms,
		) {
			(Some(volume_seq), Some(written_at_unix_ms))
				if snapshot.committed == volume_seq
					&& snapshot.cold.volume_seq.checked_add(1) == Some(volume_seq) =>
			{
				vec![rivet_data::generated::depot_log_shard_v1::VolumeLogEntry {
					written_at_unix_ms,
					volume_seq,
					client_epoch: snapshot.client.client_epoch,
					body: rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody::ClaimEntry,
				}]
			}
			_ => Vec::new(),
		};
		// A volume this client created with its initial writes, and has not written since, is
		// exactly those writes.
		if let Some(created) = &snapshot.created
			&& entries.is_empty()
			&& snapshot.committed == created.volume_seq
			&& snapshot.next_volume_seq == created.volume_seq + 1
			&& snapshot.cold.volume_seq == 0
			&& !snapshot.has_base_chain
			&& let Some(initial) = self.initial.lock().clone()
			&& initial.len() as u64 == created.volume_seq
		{
			let entries = initial
				.into_iter()
				.zip(1..)
				.map(|(body, volume_seq)| {
					rivet_data::generated::depot_log_shard_v1::VolumeLogEntry {
						written_at_unix_ms: 0,
						volume_seq,
						client_epoch: created.client_epoch,
						body,
					}
				})
				.collect();
			return Ok(LogHistory {
				cold: snapshot.cold.clone(),
				committed: snapshot.committed,
				entries,
				empty: false,
				complete: true,
			});
		}
		Ok(LogHistory {
			cold: snapshot.cold.clone(),
			committed: snapshot.committed,
			entries,
			empty,
			complete: false,
		})
	}

	/// Where this client's open committed its initial writes, when the open created the volume
	/// with them. `None` before the open answers and for every other open; then the writes may
	/// or may not be in the volume's history, so the owner reads before writing them.
	pub fn created(
		&self,
	) -> Option<rivet_data::generated::shared_depot_types_v1::VolumeLogPosition> {
		self.snapshot.borrow().created.clone()
	}

	/// The values the creating open read back from an existing volume, one per requested key,
	/// when the maintainer could answer them; see `VolumeOpenResponse.read`.
	pub fn read_back(&self) -> Option<Vec<Option<vbare::Bytes>>> {
		self.snapshot.borrow().read_back.as_deref().cloned()
	}

	/// Waits until a volume opened empty has sent its claim entry, or until the open ends any
	/// other way. Returns `true` when writes may be admitted now and ride right behind the
	/// claim entry, `false` when the caller must wait for [`VolumeHandle::ready`]. It only watches
	/// the published snapshot, so it sends nothing to the runner.
	pub async fn claim_entry_sent(&self) -> Result<bool, WriteError> {
		let mut snapshot = self.snapshot.clone();
		let settled = snapshot
			.wait_for(|view| match &view.phase {
				VolumeClientPhase::Opening { .. } => false,
				VolumeClientPhase::Recovering(
					RecoveryPhase::Claiming
					| RecoveryPhase::Replaying { .. }
					| RecoveryPhase::Matching,
				) => !view.opened_empty,
				VolumeClientPhase::Recovering(_)
				| VolumeClientPhase::Draining { .. }
				| VolumeClientPhase::Closing { .. }
				| VolumeClientPhase::Stopped
				| VolumeClientPhase::Appending
				| VolumeClientPhase::Fenced { .. } => true,
			})
			.await
			.map_err(|_| WriteError::Fenced { token: None })?
			.clone();
		match &settled.phase {
			VolumeClientPhase::Fenced { error } => Err(error.clone()),
			VolumeClientPhase::Recovering(RecoveryPhase::CommittingClaimEntry { .. })
			| VolumeClientPhase::Appending => Ok(settled.opened_empty && !self.is_fenced()),
			VolumeClientPhase::Opening { .. }
			| VolumeClientPhase::Recovering(_)
			| VolumeClientPhase::Draining { .. }
			| VolumeClientPhase::Closing { .. }
			| VolumeClientPhase::Stopped => Ok(false),
		}
	}

	/// The history of a volume opened empty whose claim entry is still in flight: nothing above
	/// cold but the claim entry, which commits before any later entry. `None` otherwise.
	fn empty_history(&self) -> Option<LogHistory> {
		let snapshot = self.snapshot.borrow();
		if self.is_fenced()
			|| !snapshot.opened_empty
			|| !matches!(
				snapshot.phase,
				VolumeClientPhase::Recovering(RecoveryPhase::CommittingClaimEntry { .. })
			) {
			return None;
		}
		let volume_seq = snapshot.claim_entry_volume_seq?;
		let written_at_unix_ms = snapshot.claim_entry_written_at_unix_ms?;
		if snapshot.cold.volume_seq.checked_add(1) != Some(volume_seq) {
			return None;
		}
		Some(LogHistory {
			cold: snapshot.cold.clone(),
			committed: volume_seq,
			entries: vec![rivet_data::generated::depot_log_shard_v1::VolumeLogEntry {
				written_at_unix_ms,
				volume_seq,
				client_epoch: snapshot.client.client_epoch,
				body: rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody::ClaimEntry,
			}],
			empty: snapshot.cold.volume_seq == 0 && !snapshot.has_base_chain && volume_seq == 1,
			complete: false,
		})
	}

	/// Re-read one bounded committed page from a matched, claimed replica. The
	/// returned cold position tells callers when a flush has retired their range.
	pub async fn read_committed(
		&self,
		first: u64,
		max_bytes: u32,
	) -> Result<LogHistory, WriteError> {
		self.ready().await?;
		let snapshot = self.snapshot.borrow().clone();
		if self.is_fenced() || snapshot.phase != VolumeClientPhase::Appending {
			return Err(WriteError::Fenced { token: None });
		}
		let key = snapshot
			.volume_log_key
			.clone()
			.ok_or(WriteError::RecoveryFailed)?;
		let mut result = LogHistory {
			cold: snapshot.cold.clone(),
			committed: snapshot.committed,
			entries: Vec::new(),
			empty: false,
			complete: false,
		};
		if first <= result.cold.volume_seq || first > result.committed {
			return Ok(result);
		}
		let max_bytes = max_bytes.min(snapshot.config.max_append_bytes);
		for replica in snapshot
			.replicas
			.iter()
			.filter(|r| r.matched && r.acked >= first)
		{
			let read = rivet_env::timeout(
				self.clock.as_ref(),
				std::time::Duration::from_millis(snapshot.config.append_timeout_ms),
				self.transport.read(
					replica.worker_id.clone(),
					(*key).clone(),
					first,
					1024,
					max_bytes,
				),
			)
			.await;
			let current = self.snapshot.borrow();
			if self.is_fenced()
				|| current.client != snapshot.client
				|| current.phase != VolumeClientPhase::Appending
			{
				return Err(WriteError::Fenced { token: None });
			}
			drop(current);
			let (state, mut entries) = match read {
                Ok(Ok(page)) => page,
                Ok(Err(super::event::RpcError::Rejected(
                    rivet_data::generated::depot_storage_worker_protocol_v1::RejectResponse::BelowCold(reply),
                ))) => (reply.state, Vec::new()),
                _ => continue,
            };
			if state.replica_incarnation != replica.replica_incarnation
				|| state.claimed_client.as_ref() != Some(&snapshot.client)
				|| state.status
					!= rivet_data::generated::shared_depot_types_v1::VolumeLogReplicaStatus::InSync
			{
				continue;
			}
			if state.cold.volume_seq >= first {
				result.cold = state.cold;
				return Ok(result);
			}
			entries.retain(|e| e.volume_seq <= snapshot.committed.min(replica.acked));
			if entries.is_empty() {
				continue;
			}
			let mut next = first;
			if entries
				.iter()
				.map(depot_types::log_shard::entry_bytes)
				.sum::<u64>()
				> u64::from(max_bytes)
				|| entries.iter().any(|entry| {
					let invalid = entry.volume_seq != next
						|| entry.client_epoch > snapshot.client.client_epoch
						|| entry.volume_seq > state.appended.volume_seq;
					next = next.saturating_add(1);
					invalid
				}) {
				return Err(WriteError::RecoveryFailed);
			}
			result.entries = entries;
			return Ok(result);
		}
		Err(WriteError::QuorumUnavailable { token: None })
	}

	/// Convenience for diagnostics and small histories. Filesystem replay uses
	/// read_committed directly so transaction size does not become a RAM limit.
	pub async fn retained_history(&self) -> Result<LogHistory, WriteError> {
		let mut history = self.history().await?;
		// `history` may already hold the claim entry, the only entry above cold.
		let mut next = history
			.entries
			.last()
			.map_or(history.cold.volume_seq, |entry| entry.volume_seq)
			+ 1;
		let max_bytes = self.snapshot.borrow().config.max_append_bytes;
		while next <= history.committed {
			let page = self.read_committed(next, max_bytes).await?;
			if page.cold.volume_seq >= next || page.entries.is_empty() {
				return Err(WriteError::RecoveryFailed);
			}
			for entry in page
				.entries
				.into_iter()
				.take_while(|e| e.volume_seq <= history.committed)
			{
				next = entry.volume_seq + 1;
				history.entries.push(entry);
			}
		}
		Ok(history)
	}
	/// Admits application bytes encoded as `payload_version` of `payload_family`, which the
	/// shards record in their version marks without decoding the bytes. Family 0 is
	/// untracked. A separate commit ticket establishes durability.
	pub async fn admit_payload(
		&self,
		payload_family: rivet_data::families::VersionFamily,
		payload_version: u16,
		payload: impl Into<vbare::Bytes>,
	) -> Result<WriteOutcome, WriteError> {
		self.admit_write(VolumeLogEntryBody::PayloadWrite(
			rivet_data::generated::depot_log_shard_v1::PayloadWrite {
				payload_family: payload_family.0,
				payload_version,
				client_seq: 0,
				request_id: [0; 16],
				payload_digest: [0; 16],
				payload: payload.into(),
				codec: rivet_data::generated::depot_log_shard_v1::PayloadCodec::None,
			},
		))
		.await
	}

	/// Explicitly waits for the claim entry's quorum with its own deadline, without requiring a write.
	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn ready(&self) -> Result<(), WriteError> {
		let inbox = self
			.inbox
			.upgrade()
			.ok_or(WriteError::Fenced { token: None })?;
		let permit = inbox
			.try_reserve()
			.map_err(|_| WriteError::client_overloaded())?;
		let (request_id, response) = self.callers.lock().register(&self.closing)?;
		permit.send(Input::OpenRequest { request_id });
		match response.await {
			Ok(Reply::Open(result)) => result,
			_ => Err(WriteError::Fenced { token: None }),
		}
	}

	/// Submits immutable mutation bytes with the original retry token, returning after a disk quorum.
	///
	/// A retry must carry exactly the bytes its token was issued for. The digest check that
	/// refuses a retry with other bytes catches accidental mismatches only, not bytes chosen to
	/// collide.
	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn write(
		&self,
		body: VolumeLogEntryBody,
		retry_of: Option<WriteToken>,
	) -> Result<WriteOutcome, WriteError> {
		self.submit_write(body, retry_of, true).await
	}

	/// Returns once the core has fixed the mutation's position. Replication continues
	/// even if the caller drops its receipt; use a commit ticket before acknowledging data.
	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn admit_write(&self, body: VolumeLogEntryBody) -> Result<WriteOutcome, WriteError> {
		self.submit_write(body, None, false).await
	}

	/// Both public write paths transfer the same immutable payload and admission metadata.
	async fn submit_write(
		&self,
		body: VolumeLogEntryBody,
		retry_of: Option<WriteToken>,
		sync: bool,
	) -> Result<WriteOutcome, WriteError> {
		let inbox = self
			.inbox
			.upgrade()
			.ok_or(WriteError::Fenced { token: None })?;
		let permit = inbox
			.try_reserve()
			.map_err(|_| WriteError::client_overloaded())?;

		// A new write toward shards that are refusing this process's appends would only queue
		// behind their retries, so it is refused before taking a position. A retry joins an
		// entry already admitted and is never refused here.
		if retry_of.is_none()
			&& let Some(retry_after_ms) = self.shards_refusing_ms()
		{
			super::metrics::record_shard_refused_write();
			return Err(WriteError::Overloaded {
				retry_after_ms,
				token: None,
			});
		}

		// Hash, encode and measure before taking the payloads lock. The runner's executor takes
		// the same lock synchronously for every append and release, so payload-sized work must
		// not run under it. The digest covers the decoded bytes, so it is taken first.
		let (max_append_bytes, max_mutation_bytes, codec) = {
			let snapshot = self.snapshot.borrow();
			(
				snapshot.config.max_append_bytes,
				snapshot.max_mutation_bytes,
				snapshot.config.payload_codec.clone(),
			)
		};
		let payload_digest = payload_digest(&body).map_err(|_| WriteError::RetryConflict)?;
		let body = super::metrics::encode(codec, body);
		// An entry above the configured append limit exceeds every shard's limit, which only
		// decreases from it, so it is refused for good before it is retained. The machine
		// applies the limit the shards have reported since.
		if entry_exceeds_bytes(&body, max_append_bytes) {
			return Err(WriteError::EntryTooLarge {
				max_bytes: max_mutation_bytes,
			});
		}
		let encoded_bytes = entry_encoded_bytes(&body).map_err(|_| WriteError::RetryConflict)?;
		if encoded_bytes > max_append_bytes {
			return Err(WriteError::EntryTooLarge {
				max_bytes: max_mutation_bytes,
			});
		}

		// Failed admission must release its body.
		let payload = self
			.payloads
			.lock()
			.insert_measured(body)
			.map_err(|_| WriteError::RetryConflict)?;
		let registered = self.callers.lock().register(&self.closing);
		let (request_id, response) = match registered {
			Ok(registered) => registered,
			Err(error) => {
				self.payloads.lock().release(payload);
				return Err(error);
			}
		};
		permit.send(Input::WriteRequest {
			request_id,
			payload,
			payload_digest,
			retry_of,
			encoded_bytes,
			sync,
		});
		drop(inbox);
		match response.await {
			Ok(Reply::Write(result)) => result,
			_ => Err(WriteError::QuorumUnavailable { token: None }),
		}
	}

	/// How long until a quorum of this volume's log shards accepts appends from this process
	/// again, while the shards' latest refusals or slow answers to any of its clients still
	/// deny a quorum. `None` when a quorum accepts now.
	fn shards_refusing_ms(&self) -> Option<u64> {
		let now_boot_ms = self.clock.boot_ms().ok()?;
		let snapshot = self.snapshot.borrow();
		super::membership::refused_quorum_ms(
			&snapshot.replica_set,
			|worker| self.transport.refusing_until(worker, now_boot_ms),
			now_boot_ms,
		)
	}

	/// Enqueues the barrier now, before returning its ticket. Polling tickets in a
	/// different order cannot move their captured positions or start new writes.
	/// Dropping a ticket abandons only its reply, never the admitted mutations.
	pub fn wait_for_commit(&self) -> Result<CommitTicket, WriteError> {
		self.commit_barrier(None)
	}

	/// The core checks this immutable history before capturing the barrier's position.
	/// Reopening retires pending barriers; a completed success stays valid after a handoff.
	pub fn wait_for_commit_at(&self, history: HistoryIdentity) -> Result<CommitTicket, WriteError> {
		self.commit_barrier(Some(history))
	}

	fn commit_barrier(&self, history: Option<HistoryIdentity>) -> Result<CommitTicket, WriteError> {
		let inbox = self
			.inbox
			.upgrade()
			.ok_or(WriteError::Fenced { token: None })?;
		let permit = inbox
			.try_reserve()
			.map_err(|_| WriteError::client_overloaded())?;
		let (request_id, response) = self.callers.lock().register(&self.closing)?;
		permit.send(Input::WaitForCommitRequest {
			request_id,
			history,
		});
		Ok(Box::pin(async move {
			match response.await {
				Ok(Reply::Committed(result)) => result,
				_ => Err(WriteError::QuorumUnavailable { token: None }),
			}
		}))
	}

	/// Confirms the current tail with fresh replies from a quorum, detecting a superseding
	/// client even when all requested data is already cached. Adds no volume log entry.
	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn confirm(&self) -> Result<u64, WriteError> {
		self.ready().await?;
		let inbox = self
			.inbox
			.upgrade()
			.ok_or(WriteError::Fenced { token: None })?;
		let permit = inbox
			.try_reserve()
			.map_err(|_| WriteError::client_overloaded())?;
		let (request_id, response) = self.callers.lock().register(&self.closing)?;
		permit.send(Input::ConfirmRequest { request_id });
		drop(inbox);
		match response.await {
			Ok(Reply::Committed(result)) => result,
			_ => Err(WriteError::QuorumUnavailable { token: None }),
		}
	}

	/// Drains accepted writes and lets the core send VolumeClose before releasing its transport owner.
	#[tracing::instrument(skip_all)]
	pub async fn close(&self) -> Result<(), WriteError> {
		if self.snapshot.borrow().phase == VolumeClientPhase::Stopped {
			return Ok(());
		}
		self.ready().await?;
		let inbox = self
			.inbox
			.upgrade()
			.ok_or(WriteError::Fenced { token: None })?;
		let permit = inbox
			.try_reserve()
			.map_err(|_| WriteError::client_overloaded())?;
		let (request_id, response) = self.callers.lock().register(&self.closing)?;
		permit.send(Input::CloseRequest { request_id });
		drop(inbox);
		match response.await {
			Ok(Reply::Close(result)) => result,
			_ => Err(WriteError::QuorumUnavailable { token: None }),
		}
	}
}

/// Observes every changed client snapshot on the runner task.
pub type VolumeObserver = Arc<dyn PublishObserver<VolumeClientSnapshot>>;

/// Owns the runner and every transport task. Drop schedules cleanup; shutdown proves it joined.
pub struct Volume {
	inbox: Option<mpsc::Sender<Input>>,
	handle: Option<VolumeHandle>,
	stopped: Option<oneshot::Receiver<()>>,
	env: Arc<dyn Env>,
	tasks: Arc<TaskScope>,
	closing: Arc<AtomicBool>,
}

impl Volume {
	/// The transport factory receives a guard checked at replica dispatch, including after UPS waits.
	/// Lifecycle transport retains its worker lease guard so the core can send its final close.
	pub fn start(
		env: Arc<dyn Env>,
		state: VolumeClientState,
		make_transport: impl FnOnce(
			Arc<dyn Fn() -> bool + Send + Sync>,
		) -> anyhow::Result<Arc<dyn VolumeClientTransport>>,
	) -> anyhow::Result<Self> {
		Self::start_observed(env, state, None, make_transport)
	}

	/// Like [`Volume::start`], and also reports the initial and every changed snapshot to
	/// `observer` on the runner task. The runner owns the observer and drops it when it stops.
	pub fn start_observed(
		env: Arc<dyn Env>,
		state: VolumeClientState,
		observer: Option<VolumeObserver>,
		make_transport: impl FnOnce(
			Arc<dyn Fn() -> bool + Send + Sync>,
		) -> anyhow::Result<Arc<dyn VolumeClientTransport>>,
	) -> anyhow::Result<Self> {
		Self::start_with_initial(env, state, observer, Vec::new(), make_transport)
	}

	/// Like [`Volume::start_observed`] for an opener that creates the volume with `initial` as
	/// its first writes. When the open reports them committed, the client appends after them,
	/// which [`VolumeHandle::created`] reports.
	///
	/// Otherwise the client recovers normally, and the recovered history may hold any of: none
	/// of the writes; all of them, from an earlier attempt of this open; a prefix of them, from
	/// a torn create write; or another opener's initial writes, from that opener's failed
	/// create. So the owner must read the recovered volume and decide from its contents, never
	/// from the presence of an entry, before writing again. An owner whose initial writes form
	/// one logical change either sends it as one write or ends it with a completion marker it
	/// checks.
	///
	/// Initial writes over `max_initial_bytes` are not sent: the volume is created empty and
	/// `created` stays `None`. `state` must create if absent.
	pub fn start_with_initial(
		env: Arc<dyn Env>,
		state: VolumeClientState,
		observer: Option<VolumeObserver>,
		initial: Vec<VolumeLogEntryBody>,
		make_transport: impl FnOnce(
			Arc<dyn Fn() -> bool + Send + Sync>,
		) -> anyhow::Result<Arc<dyn VolumeClientTransport>>,
	) -> anyhow::Result<Self> {
		Self::start_with_timers(env, state, observer, initial, None, make_transport)
	}

	/// Like [`Volume::start_with_initial`]. With `timers`, the client's alarms, such as its
	/// idle close, wait in that process-wide deadline queue instead of in a sleep of their
	/// own. The queue's owner must run it until every such client stopped.
	pub fn start_with_timers(
		env: Arc<dyn Env>,
		mut state: VolumeClientState,
		observer: Option<VolumeObserver>,
		initial: Vec<VolumeLogEntryBody>,
		timers: Option<rivet_state_machine_ephemeral::SharedTimers>,
		make_transport: impl FnOnce(
			Arc<dyn Fn() -> bool + Send + Sync>,
		) -> anyhow::Result<Arc<dyn VolumeClientTransport>>,
	) -> anyhow::Result<Self> {
		let snapshots: Arc<OnceLock<watch::Receiver<VolumeClientSnapshot>>> =
			Arc::new(OnceLock::new());
		let closing = Arc::new(AtomicBool::new(false));
		let guard = {
			let snapshots = snapshots.clone();
			let closing = closing.clone();
			Arc::new(move || {
				!closing.load(Ordering::Acquire)
					&& snapshots.get().is_some_and(|snapshot| {
						!matches!(
							snapshot.borrow().phase,
							VolumeClientPhase::Fenced { .. }
								| VolumeClientPhase::Stopped
								| VolumeClientPhase::Closing { .. }
						)
					})
			})
		};
		let transport = make_transport(guard)?;
		let tasks = Arc::new(env.task_scope());
		let mut store = Payloads::default();
		// The digest covers the decoded bytes, so encoding first changes no identity.
		let codec = state.config.payload_codec.clone();
		let initial: Vec<_> = initial
			.into_iter()
			.map(|body| super::metrics::encode(codec.clone(), body))
			.collect();
		let initial_bytes = initial
			.iter()
			.map(|body| entry_encoded_bytes(body).map(u64::from))
			.sum::<anyhow::Result<u64>>()?;
		let initial = if initial_bytes <= state.config.max_initial_bytes {
			initial
		} else {
			tracing::debug!(
				initial_bytes,
				max_initial_bytes = state.config.max_initial_bytes,
				"initial writes exceed the creating open's budget; creating the volume empty"
			);
			Vec::new()
		};
		let initial_bodies = Arc::new(Mutex::new((!initial.is_empty()).then(|| initial.clone())));
		// Each initial write gets the identity an ordinary write would get, in order, so the
		// writes after them continue the client's sequence.
		let mut rng = env.rng("depot.volume.initial_write");
		let mut writes = Vec::with_capacity(initial.len());
		for (body, client_seq) in initial.into_iter().zip(1..) {
			let payload_digest = payload_digest(&body)?;
			let (payload, encoded_bytes) = store.insert(body)?;
			writes.push(super::state::InitialWrite {
				payload,
				encoded_bytes,
				identity: super::types::MutationIdentity {
					client_seq,
					request_id: *rng.uuid_v4().as_bytes(),
					payload_digest,
				},
			});
		}
		state.set_initial_writes(writes)?;
		let payloads = Arc::new(Mutex::new(store));
		let callers = Arc::new(Mutex::new(Callers::default()));
		let runner = Runner::<VolumeClientMachine>::spawn_with_timers(
			env.clone(),
			state,
			observer,
			None,
			timers,
			|snapshot| {
				let _ = snapshots.set(snapshot.clone());
				Arc::new(VolumeExecutor {
					snapshot,
					payloads: payloads.clone(),
					callers: callers.clone(),
					tasks: Arc::downgrade(&tasks),
					closing: closing.clone(),
					transport: transport.clone(),
				})
			},
		);
		Ok(Self {
			inbox: Some(runner.inbox.clone()),
			handle: Some(VolumeHandle {
				tasks: Arc::downgrade(&tasks),
				transport,
				clock: env.clock(),
				snapshot: runner.snapshot,
				inbox: runner.inbox.downgrade(),
				payloads,
				callers,
				closing: closing.clone(),
				initial: initial_bodies,
			}),
			stopped: Some(runner.stopped),
			env,
			tasks,
			closing,
		})
	}

	pub fn handle(&self) -> VolumeHandle {
		self.handle.as_ref().expect("live volume owner").clone()
	}

	/// Call close before shutdown for graceful release; shutdown also works after failure or cancellation.
	#[tracing::instrument(skip_all)]
	pub async fn shutdown(mut self) -> anyhow::Result<()> {
		self.closing.store(true, Ordering::Release);
		self.inbox.take();
		if let Some(handle) = self.handle.take() {
			handle.callers.lock().waiting.clear();
		}
		self.tasks.shutdown().await;
		let result = if let Some(stopped) = self.stopped.as_mut() {
			stopped.await.map_err(anyhow::Error::from)
		} else {
			Ok(())
		};
		self.stopped.take();
		result
	}
}

impl Drop for Volume {
	fn drop(&mut self) {
		let Some(stopped) = self.stopped.take() else {
			return;
		};
		self.closing.store(true, Ordering::Release);
		self.inbox.take();
		if let Some(handle) = self.handle.take() {
			handle.callers.lock().waiting.clear();
		}
		let tasks = self.tasks.clone();
		self.env.spawn(Box::pin(async move {
			tasks.shutdown().await;
			let _ = stopped.await;
		}));
	}
}

struct VolumeExecutor {
	snapshot: watch::Receiver<VolumeClientSnapshot>,
	payloads: Arc<Mutex<Payloads>>,
	callers: Arc<Mutex<Callers>>,
	tasks: Weak<TaskScope>,
	closing: Arc<AtomicBool>,
	transport: Arc<dyn VolumeClientTransport>,
}

impl Executor<VolumeClientMachine> for VolumeExecutor {
	fn execute(&self, effect: Effect, ctx: EffectContext<VolumeClientMachine>) {
		match effect {
			Effect::OpenResponse { request_id, result } => {
				self.callers.lock().reply(request_id, Reply::Open(result))
			}
			Effect::WriteResponse { request_id, result } => {
				self.callers.lock().reply(request_id, Reply::Write(result))
			}
			Effect::CloseResponse { request_id, result } => {
				self.callers.lock().reply(request_id, Reply::Close(result))
			}
			Effect::ReleasePayload { payload } => self.payloads.lock().release(payload),
			Effect::WaitForCommitResponse { request_id, result }
			| Effect::ConfirmResponse { request_id, result } => {
				self.callers
					.lock()
					.reply(request_id, Reply::Committed(result));
			}
			Effect::RecoveryResponse { .. } => {
				tracing::error!("unrequested application client reply");
			}
			request => {
				let cleanup = matches!(request, Effect::VolumeCloseRequest { .. });
				let snapshot = self.snapshot.clone();
				let closing = self.closing.clone();
				let config = (*snapshot.borrow().config).clone();
				RequestExecutor {
					payloads: self.payloads.clone(),
					tasks: self.tasks.clone(),
					transport: self.transport.clone(),
					config,
					on_revoked: None,
					may_dispatch: Arc::new(move || {
						!closing.load(Ordering::Acquire)
							&& (cleanup
								|| !matches!(
									snapshot.borrow().phase,
									VolumeClientPhase::Fenced { .. } | VolumeClientPhase::Stopped
								))
					}),
				}
				.request(request, ctx);
			}
		}
	}
}
