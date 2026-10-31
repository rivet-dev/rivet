//! Explicit owner of the shard runner and its filesystem tasks. The bounded inbox
//! feeds the core, which owns request count and byte admission budgets. Topology
//! lease admission belongs to the caller; wire headers are checked by the core.
//! Every mutating command carries authority checked at admission and reply.

use super::{
	LogShardMachine,
	effect::LogShardEffect,
	event::LogShardEvent,
	executor::{LogShardExecutor, ReadPlan},
	snapshot::{LogShardSnapshot, Resource, ResourceUsage},
	state::recovery::RecoveryOutcome,
	state::{LogShardPhase, LogShardState},
	types::{
		DiskReadError, LogShardCommand, LogShardConfig, LogShardRequest, RequestId,
		TabletManifestView, WriteToDiskError,
	},
};
use anyhow::{Context, Result};
use parking_lot::Mutex as SyncMutex;
use rivet_data::generated::depot_storage_worker_protocol_v1::{
	RejectResponse, StorageWorkerResponse,
};
use rivet_env::{Env, TaskScope};
use rivet_state_machine_ephemeral::{CompletionSender, EffectContext, Executor, Runner};
use rivet_topology::worker::WorkerId;
use std::{
	collections::BTreeMap,
	path::PathBuf,
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
};
use tokio::sync::{Semaphore, mpsc, oneshot, watch};

type Reply = StorageWorkerResponse;

/// Each outstanding request's reply sender, with when it was submitted for
/// `depot_log_shard_barrier_wait_seconds`.
type Responses = Arc<SyncMutex<BTreeMap<RequestId, (std::time::Instant, oneshot::Sender<Reply>)>>>;

/// Delivers replica status notices to a maintainer worker. The owner installs it once
/// the transport exists; notices before that count as failed.
pub trait StatusNotifier: Send + Sync + 'static {
	fn notify(
		&self,
		worker: WorkerId,
		request: rivet_data::generated::depot_client_protocol_v1::ReplicaStatusNoticeRequest,
	) -> std::pin::Pin<Box<dyn std::future::Future<Output = super::types::NoticeOutcome> + Send>>;
}

type Notifier = Arc<std::sync::OnceLock<Arc<dyn StatusNotifier>>>;

pub struct Shard {
	inbox: Option<mpsc::Sender<LogShardEvent>>,
	pub snapshot: watch::Receiver<LogShardSnapshot>,
	manifest_refresh: watch::Receiver<()>,
	stopped: Option<oneshot::Receiver<()>>,
	tasks: Arc<TaskScope>,
	responses: Responses,
	next_id: AtomicU64,
	notifier: Notifier,
	/// Stamps each submission so the core can refuse requests that waited too long.
	clock: Arc<dyn rivet_env::Clock>,
	/// Retry hint on a submission refused because the inbox is full.
	overload_retry_after_ms: u64,
}

impl Shard {
	/// Starts shard recovery immediately; construction of Env itself starts no services.
	pub fn start(
		env: Arc<dyn Env>,
		root: PathBuf,
		worker: WorkerId,
		config: LogShardConfig,
	) -> Result<Self> {
		// The owner retains tasks and replies; the executor holds a weak task handle.
		let state = LogShardState::new(config.clone(), worker.clone())?;
		let tasks = Arc::new(env.task_scope());
		let responses = Arc::new(SyncMutex::new(BTreeMap::new()));
		// One queue preserves effect order across reads, writes, checkpoints and recovery.
		// Reads only resolve their plan in that order; disk readers run them beside it.
		let (io, mut receiver) = mpsc::unbounded_channel::<IoWork>();
		let fs = env.fs();
		let io_config = config.clone();
		let readers = DiskReaders {
			permits: Arc::new(Semaphore::new(config.disk_read_concurrency as usize)),
			tasks: Arc::downgrade(&tasks),
		};
		let spare_tasks = Arc::downgrade(&tasks);
		tasks.spawn(rivet_metrics::monitor_task("log_shard_disk", async move {
			let mut storage: Option<LogShardExecutor> = None;
			while let Some((effect, inbox, completion)) = receiver.recv().await {
				let effect = match effect {
					LogShardEffect::ReadRequest {
						request_id,
						deadline_boot_ms,
						volume_log_key,
						state,
						first_volume_seq,
						max_bytes,
						reason,
					} => {
						crate::metrics::DISK_FALLBACK_READS
							.with_label_values(&[reason.label()])
							.inc();
						let plan = match storage.as_ref() {
							Some(storage) => storage
								.plan_read(&volume_log_key, &state, first_volume_seq)
								.map_err(|error| DiskReadError::Unavailable(format!("{error:#}"))),
							None => Err(DiskReadError::Unavailable("shard not recovered".into())),
						};
						readers.start(DiskRead {
							request_id,
							deadline_boot_ms,
							volume_log_key,
							state,
							max_bytes,
							plan,
							inbox,
							completion,
						});
						continue;
					}
					effect => effect,
				};
				let operation = io_operation_label(&effect);
				let timer = operation.map(|operation| {
					crate::metrics::IO_DURATION
						.with_label_values(&[operation])
						.start_timer()
				});
				let event = run_io(
					effect,
					&inbox,
					&mut storage,
					fs.clone(),
					root.clone(),
					worker.clone(),
					&io_config,
					&spare_tasks,
				)
				.await;
				drop(timer);
				// The effect's time includes its wait behind earlier disk operations.
				completion.finish(&());
				if let Some(event) = event {
					let _ = inbox.send(event);
				}
			}
		}));
		let (manifest_refresh_sender, manifest_refresh) = watch::channel(());
		let notifier: Notifier = Default::default();
		let executor_responses = responses.clone();
		let executor_notifier = notifier.clone();
		let executor_tasks = Arc::downgrade(&tasks);
		// Disk completions use the runner's completion channel, so an admitted write's
		// replies never wait behind queued requests and the disk task never blocks on a
		// full inbox.
		let handle = Runner::<LogShardMachine>::spawn_with_completions(
			env.clone(),
			state,
			move |_, completions| {
				Arc::new(ShardExecutor {
					manifest_refresh: manifest_refresh_sender,
					io,
					completions,
					responses: executor_responses,
					notifier: executor_notifier,
					tasks: executor_tasks,
				})
			},
		);

		// Observe published resource usage without extending the runner inbox lifetime.
		let mut snapshots = handle.snapshot.clone();
		let inbox = handle.inbox.downgrade();
		let signal_clock = env.clock();
		tasks.spawn(async move {
			let mut near = std::collections::BTreeSet::new();
			let mut pin_age = crate::signals::SegmentPinAge::default();
			loop {
				let (mut resources, retained_entries, retained_entry_bytes) = {
					let snapshot = snapshots.borrow_and_update();
					if let Ok(now_boot_ms) = signal_clock.boot_ms() {
						pin_age.observe(&snapshot, now_boot_ms);
					}
					(
						snapshot.resources.clone(),
						snapshot.retained_entries,
						snapshot.retained_entry_bytes,
					)
				};
				crate::metrics::observe_retained_entries(retained_entries, retained_entry_bytes);
				if let Some(inbox) = inbox.upgrade() {
					resources.push(ResourceUsage {
						resource: Resource::Inbox,
						used: (inbox.max_capacity() - inbox.capacity()) as u64,
						capacity: inbox.max_capacity() as u64,
					});
				}
				crate::metrics::observe(resources, &mut near);
				if snapshots.changed().await.is_err() {
					break;
				}
			}
		});

		Ok(Self {
			inbox: Some(handle.inbox),
			snapshot: handle.snapshot,
			manifest_refresh,
			stopped: Some(handle.stopped),
			tasks,
			responses,
			next_id: AtomicU64::new(1),
			notifier,
			clock: env.clock(),
			overload_retry_after_ms: config.overload_retry_after_ms,
		})
	}

	/// Installs the transport for replica status notices. Only the first call takes
	/// effect.
	pub fn install_status_notifier(&self, notifier: Arc<dyn StatusNotifier>) {
		let _ = self.notifier.set(notifier);
	}

	/// Enqueues one decoded request; the core checks its authority and budgets.
	pub fn submit(
		&self,
		command: LogShardCommand,
	) -> std::result::Result<oneshot::Receiver<Reply>, RejectResponse> {
		self.submit_until(command, u64::MAX)
	}

	/// Submits with the receiving host's caller deadline. Mutations keep their admitted
	/// outcome semantics; reads use it to bound queueing and disk work.
	pub fn submit_until(
		&self,
		command: LogShardCommand,
		deadline_boot_ms: u64,
	) -> std::result::Result<oneshot::Receiver<Reply>, RejectResponse> {
		// Wrapping IDs correlate only bounded outstanding work, not disk log positions.
		let request_id = RequestId(self.next_id.fetch_add(1, Ordering::Relaxed));
		let request = LogShardRequest {
			deadline_boot_ms,
			request_id,
			command,
		};

		// Register delivery before enqueueing so a fast completion cannot outrun its receiver.
		let (sender, receiver) = oneshot::channel();
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let submitted = std::time::Instant::now();
		self.responses
			.lock()
			.insert(request_id, (submitted, sender));

		let result = self.enqueue(vec![request]);

		if let Err(reason) = result {
			self.responses.lock().remove(&request_id);
			crate::metrics::record_rejection(&reason);
			return Err(reason);
		}

		Ok(receiver)
	}

	/// Sends one submission, stamped with the current boot time, without waiting for room.
	/// A full inbox refuses it with a retry hint; a closed inbox or a failed clock is
	/// unavailable even if the last published snapshot was serving.
	fn enqueue(&self, requests: Vec<LogShardRequest>) -> std::result::Result<(), RejectResponse> {
		let Some(inbox) = &self.inbox else {
			return Err(self.not_serving());
		};
		let Ok(submitted_boot_ms) = self.clock.boot_ms() else {
			return Err(self.not_serving());
		};
		let count = requests.len();
		let event = LogShardEvent::Submitted {
			submitted_boot_ms,
			requests,
		};
		match inbox.try_send(event) {
			Ok(()) => Ok(()),
			Err(mpsc::error::TrySendError::Full(_)) => {
				crate::metrics::record_admission_shed(
					crate::metrics::AdmissionShed::InboxFull,
					count,
				);
				Err(RejectResponse::Overloaded(
					rivet_data::generated::depot_storage_worker_protocol_v1::Overloaded {
						retry_after_ms: self.overload_retry_after_ms,
					},
				))
			}
			Err(mpsc::error::TrySendError::Closed(_)) => Err(self.not_serving()),
		}
	}

	/// Submits a coalesced batch as one inbox event, so the core admits its items together and
	/// may write them in one disk batch. Each item keeps its own budget checks and reply. A full
	/// or closed inbox rejects every item alike.
	pub fn submit_many(
		&self,
		commands: Vec<LogShardCommand>,
	) -> std::result::Result<Vec<oneshot::Receiver<Reply>>, RejectResponse> {
		self.submit_many_until(commands, u64::MAX)
	}

	/// Like `submit_many`, preserving the deadline of the transport batch for every read.
	pub fn submit_many_until(
		&self,
		commands: Vec<LogShardCommand>,
		deadline_boot_ms: u64,
	) -> std::result::Result<Vec<oneshot::Receiver<Reply>>, RejectResponse> {
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let submitted = std::time::Instant::now();
		let mut requests = Vec::with_capacity(commands.len());
		let mut receivers = Vec::with_capacity(commands.len());
		{
			// Register delivery before enqueueing so a fast completion cannot outrun its receiver.
			let mut responses = self.responses.lock();
			for command in commands {
				let request_id = RequestId(self.next_id.fetch_add(1, Ordering::Relaxed));
				let (sender, receiver) = oneshot::channel();
				responses.insert(request_id, (submitted, sender));
				requests.push(LogShardRequest {
					deadline_boot_ms,
					request_id,
					command,
				});
				receivers.push(receiver);
			}
		}
		let ids = requests
			.iter()
			.map(|request| request.request_id)
			.collect::<Vec<_>>();

		let result = self.enqueue(requests);

		if let Err(reason) = result {
			let mut responses = self.responses.lock();
			for id in &ids {
				responses.remove(id);
				crate::metrics::record_rejection(&reason);
			}
			return Err(reason);
		}

		Ok(receivers)
	}

	/// Wakeup hints for the manifest poller; the queued requests retain their original deadlines.
	pub fn manifest_refreshes(&self) -> watch::Receiver<()> {
		self.manifest_refresh.clone()
	}

	/// Supplies the supervisor's latest routing view for client replica-set checks.
	pub async fn update_tablet_manifest(&self, tablet_manifest: TabletManifestView) -> Result<()> {
		self.inbox
			.as_ref()
			.context("shard stopped")?
			.send(LogShardEvent::TabletManifestUpdated { tablet_manifest })
			.await?;
		Ok(())
	}

	/// Reports an unavailable shard when admission or the response channel has closed.
	fn not_serving(&self) -> RejectResponse {
		RejectResponse::NotServing
	}

	/// Stop admission and await accepted disk writes before cancelling and joining
	/// remaining owned I/O. No ownership of this service is hidden in context clones.
	pub async fn shutdown(mut self) -> Result<()> {
		// Let recovery settle, then drain accepted work before closing the runner's inbox.
		let result = async {
			while matches!(self.snapshot.borrow().phase, LogShardPhase::Recovering(_)) {
				self.snapshot.changed().await?;
			}

			self.inbox
				.as_ref()
				.context("shard already stopped")?
				.send(LogShardEvent::Drain)
				.await?;

			while self.snapshot.borrow().phase != LogShardPhase::Stopped {
				self.snapshot.changed().await?;
			}

			self.inbox.take();
			self.stopped
				.take()
				.context("missing runner completion")?
				.await?;

			Ok(())
		}
		.await;

		// Join remaining I/O and release reply reservations even if draining failed.
		self.tasks.shutdown().await;
		self.responses.lock().clear();
		result
	}
}

impl Drop for Shard {
	fn drop(&mut self) {
		self.tasks.cancel();
		self.responses.lock().clear();
	}
}

/// One disk operation, its completion channel and its telemetry record, which the disk task
/// finishes when the operation completes.
type IoWork = (
	LogShardEffect,
	CompletionSender<LogShardMachine>,
	rivet_state_machine::EffectCompletion,
);

struct ShardExecutor {
	manifest_refresh: watch::Sender<()>,
	io: mpsc::UnboundedSender<IoWork>,
	/// Where disk work reports back; delivered ahead of queued requests.
	completions: CompletionSender<LogShardMachine>,
	responses: Responses,
	notifier: Notifier,
	/// The owner's task scope, which joins notice sends at shutdown.
	tasks: std::sync::Weak<TaskScope>,
}

impl Executor<LogShardMachine> for ShardExecutor {
	/// Replies inline and enqueues filesystem operations in exactly the core's effect order.
	fn execute(&self, effect: LogShardEffect, ctx: EffectContext<LogShardMachine>) {
		let inbox = ctx.inbox();
		match effect {
			LogShardEffect::RefreshTabletManifest => {
				self.manifest_refresh.send_replace(());
			}
			LogShardEffect::Reply {
				request_id,
				response,
			} => {
				if let StorageWorkerResponse::RejectResponse(reason) = &response {
					crate::metrics::record_rejection(reason);
				}
				let entry = self.responses.lock().remove(&request_id);
				if let Some((submitted, sender)) = entry {
					crate::metrics::record_barrier_wait(&response, submitted.elapsed());
					let _ = sender.send(response);
				}
			}
			LogShardEffect::Stopped => {}
			LogShardEffect::NotifyMaintainerRequest {
				worker,
				notice_id,
				request,
			} => {
				let inbox = inbox.clone();
				let send = match (self.notifier.get(), self.tasks.upgrade()) {
					(Some(notifier), Some(tasks)) => {
						Some((notifier.notify(worker.clone(), request), tasks))
					}
					(Some(_), None) | (None, Some(_)) | (None, None) => None,
				};
				match send {
					Some((send, tasks)) => {
						ctx.spawn_in(&tasks, async move {
							let outcome = send.await;
							let _ = inbox
								.send(LogShardEvent::NotifyMaintainerResponse {
									worker,
									notice_id,
									outcome,
								})
								.await;
						});
					}
					None => {
						let _ = inbox.try_send(LogShardEvent::NotifyMaintainerResponse {
							worker,
							notice_id,
							outcome: super::types::NoticeOutcome::Failed,
						});
					}
				}
			}
			LogShardEffect::RecoverRequest { .. }
			| LogShardEffect::ReadRequest { .. }
			| LogShardEffect::WriteToDiskRequest { .. }
			| LogShardEffect::WriteCheckpointRequest { .. }
			| LogShardEffect::RemoveSegmentsRequest { .. }
			| LogShardEffect::MarkRepairingRequest { .. } => {
				let _ = self
					.io
					.send((effect, self.completions.clone(), ctx.complete_later()));
			}
		}
	}
}

/// One read handed from the write queue to the disk readers, with its resolved plan.
struct DiskRead {
	deadline_boot_ms: u64,
	request_id: RequestId,
	volume_log_key: depot_types::VolumeLogKey,
	state: rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState,
	max_bytes: u32,
	plan: Result<ReadPlan, DiskReadError>,
	inbox: CompletionSender<LogShardMachine>,
	completion: rivet_state_machine::EffectCompletion,
}

/// Runs planned reads beside the write queue, at most `disk_read_concurrency` at once.
///
/// A plan names batches that are already synced and that no later write moves or
/// overwrites; writes only extend the active segment past its last batch, into zeros when
/// the segment is preallocated, through their own descriptor, so a reader's positional reads
/// see exactly the planned bytes. A roll truncates only the zeros past the sealed segment's
/// last batch. A segment removed after a read
/// was planned holds only entries below cold, and its read fails as `Unavailable`.
struct DiskReaders {
	permits: Arc<Semaphore>,
	tasks: std::sync::Weak<TaskScope>,
}

impl DiskReaders {
	/// Waits for a reader and completes its read within the original deadline. Expiration
	/// releases the queue reservation without taking a reader or starting filesystem work.
	fn start(&self, read: DiskRead) {
		let Some(tasks) = self.tasks.upgrade() else {
			return;
		};
		let permits = self.permits.clone();
		let clock = tasks.env().clock();
		tasks.spawn(async move {
			let DiskRead {
				request_id,
				deadline_boot_ms,
				volume_log_key,
				state,
				max_bytes,
				plan,
				inbox,
				completion,
			} = read;
			let reading = async {
				let _permit = permits
					.acquire_owned()
					.await
					.map_err(|_| DiskReadError::Unavailable("disk readers stopped".into()))?;
				// The clock is checked after admission too, so an expired ready permit cannot
				// start filesystem work in the same poll as its deadline.
				if clock
					.boot_ms()
					.map_err(|error| DiskReadError::Unavailable(error.to_string()))?
					>= deadline_boot_ms
				{
					return Err(DiskReadError::Unavailable(
						"disk read deadline elapsed".into(),
					));
				}
				let _timer = crate::metrics::IO_DURATION
					.with_label_values(&["read"])
					.start_timer();
				match plan {
					Ok(plan) => plan.read(volume_log_key, state, max_bytes).await,
					Err(error) => Err(error),
				}
			};
			let result =
				match rivet_env::timeout_at(clock.as_ref(), deadline_boot_ms, reading).await {
					Ok(result) => result,
					Err(error) => Err(DiskReadError::Unavailable(format!(
						"disk read timed out: {error}"
					))),
				};
			completion.finish(&());
			let _ = inbox.send(LogShardEvent::ReadResponse { request_id, result });
		});
	}
}

/// Bounded label for a disk operation, or `None` for effects the IO task never receives.
fn io_operation_label(effect: &LogShardEffect) -> Option<&'static str> {
	match effect {
		LogShardEffect::RecoverRequest { .. } => Some("recover"),
		LogShardEffect::ReadRequest { .. } => Some("read"),
		LogShardEffect::WriteToDiskRequest { .. } => Some("write_to_disk"),
		LogShardEffect::WriteCheckpointRequest { .. } => Some("checkpoint"),
		LogShardEffect::RemoveSegmentsRequest { .. } => Some("remove_segments"),
		LogShardEffect::MarkRepairingRequest { .. } => Some("mark_repairing"),
		LogShardEffect::RefreshTabletManifest
		| LogShardEffect::Reply { .. }
		| LogShardEffect::Stopped
		| LogShardEffect::NotifyMaintainerRequest { .. } => None,
	}
}

/// Runs one operation on the sole filesystem executor. Keeping it in one owned
/// task preserves the disk image captured by a read before subsequent writes.
async fn run_io(
	effect: LogShardEffect,
	inbox: &CompletionSender<LogShardMachine>,
	storage: &mut Option<LogShardExecutor>,
	fs: rivet_env::fs::Fs,
	root: PathBuf,
	worker: WorkerId,
	config: &LogShardConfig,
	spare_tasks: &std::sync::Weak<TaskScope>,
) -> Option<LogShardEvent> {
	let segment_bytes = config.segment_bytes;
	let event = match effect {
		LogShardEffect::RecoverRequest { disk_incarnation } => {
			// Only a missing FORMAT permits initialization; existing shards must recover.
			let result = async {
				let format = root.join(rivet_keys::keys::codec::KeyAddress::pack(
					&depot_types::keys::fs::LogShardKey::new(
						depot_types::keys::fs::LogShardFile::Format,
					),
				));

				let (recovered, outcome) = match fs.read(format).await {
					Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
						let recovered = LogShardExecutor::create(
							fs,
							root,
							&worker,
							uuid::Uuid::from_bytes(disk_incarnation),
							config.max_batch_bytes as usize,
							segment_bytes,
						)
						.await?;
						(recovered, None)
					}

					Err(e) => return Err(e.into()),
					Ok(_) => {
						let (recovered, outcome) = LogShardExecutor::open_with_payload_retention(
							fs,
							root,
							&worker,
							config.max_batch_bytes as usize,
							segment_bytes,
							config.max_retained_payload_bytes_per_volume,
							&|| {
								let _ = inbox.send(LogShardEvent::RecoverProgress);
							},
							&config.read_versions,
						)
						.await?;
						(recovered, Some(outcome))
					}
				};

				let outcome = match outcome {
					Some(outcome) => outcome,
					None => RecoveryOutcome::Clean(super::state::recovery::RecoveredShard {
						checkpoint_bytes: 0,
						retired_volumes: Default::default(),
						disk_incarnation,
						tablet_grants: Default::default(),
						tablet_replica_sets: Default::default(),
						replicas: Vec::new(),
						disk_segments: BTreeMap::new(),
						last_log_seq: 0,
						checkpoint_last_included_log_seq: 0,
						bytes_since_checkpoint: 0,
						changed_since_checkpoint: Default::default(),
						quarantined_bytes: 0,
						stored_max_batch_bytes: 0,
					}),
				};
				let mut recovered = recovered;
				// A corrupt shard preallocates only once repair marking has finished.
				if config.preallocate_segments && matches!(outcome, RecoveryOutcome::Clean(_)) {
					recovered.enable_preallocation(Some(spare_tasks.clone()));
				}
				*storage = Some(recovered);
				Ok::<_, anyhow::Error>(outcome)
			}
			.await;
			LogShardEvent::RecoverResponse(result.map_err(|e| {
				e.downcast_ref::<super::state::recovery::RecoveryError>()
					.cloned()
					.unwrap_or_else(|| super::state::recovery::RecoveryError::Io(format!("{e:#}")))
			}))
		}
		LogShardEffect::WriteToDiskRequest {
			batches,
			fill_wait_ms,
		} => {
			crate::metrics::observe_batch_fill_wait(fill_wait_ms);
			// A write is capped when the filling batch had no room left for another full batch.
			let write_bytes = batches
				.iter()
				.map(|batch| depot_types::log_shard::framed_batch_bytes(batch) as u64)
				.sum::<u64>();
			let frames = batches
				.iter()
				.map(|batch| batch.frames.len())
				.sum::<usize>();
			let capped = write_bytes
				.saturating_add(u64::from(config.max_batch_bytes))
				.saturating_add(depot_types::log_shard::BATCH_FRAMING_BYTES as u64)
				> config.max_write_bytes;
			crate::metrics::observe_disk_write(write_bytes, frames, capped);
			match storage.as_mut() {
				Some(s) => s.write_to_disk_response(&batches).await,
				None => LogShardEvent::WriteToDiskResponse {
					last_log_seq: batches.last().map_or(0, |batch| batch.log_seq),
					result: Err(WriteToDiskError::Write("shard not recovered".into())),
				},
			}
		}
		LogShardEffect::WriteCheckpointRequest {
			checkpoint,
			max_checkpoint_bytes,
			base_bytes,
		} => match storage.as_mut() {
			Some(s) => {
				s.checkpoint_response(&checkpoint, max_checkpoint_bytes, base_bytes)
					.await
			}
			None => LogShardEvent::WriteCheckpointResponse {
				checkpoint_bytes: 0,
				last_included_log_seq: checkpoint.last_included_log_seq,
				result: Err(WriteToDiskError::Write("shard not recovered".into())),
			},
		},
		LogShardEffect::RemoveSegmentsRequest { first_log_seqs } => {
			LogShardEvent::RemoveSegmentsResponse {
				result: match storage.as_mut() {
					Some(s) => s
						.remove_segments(&first_log_seqs)
						.await
						.map_err(|e| WriteToDiskError::Write(format!("{e:#}"))),
					None => Err(WriteToDiskError::Write("shard not recovered".into())),
				},
			}
		}
		LogShardEffect::MarkRepairingRequest {
			retired_volumes,
			tablet_grants,
			tablet_replica_sets,
			repair_marks,
			quarantine_from_log_seq,
			first_log_seq,
		} => LogShardEvent::MarkRepairingResponse(match storage.as_mut() {
			Some(s) => {
				let marked = s
					.mark_repairing(
						&retired_volumes,
						&tablet_grants,
						&tablet_replica_sets,
						&repair_marks,
						quarantine_from_log_seq,
						first_log_seq,
					)
					.await
					.map_err(|e| WriteToDiskError::Write(format!("{e:#}")));
				if config.preallocate_segments && marked.is_ok() {
					s.enable_preallocation(Some(spare_tasks.clone()));
				}
				marked
			}
			None => Err(WriteToDiskError::Write("shard not recovered".into())),
		}),
		// The write queue hands reads to the disk readers before calling this.
		LogShardEffect::ReadRequest { .. }
		| LogShardEffect::RefreshTabletManifest
		| LogShardEffect::Reply { .. }
		| LogShardEffect::Stopped
		| LogShardEffect::NotifyMaintainerRequest { .. } => return None,
	};

	Some(event)
}


