//! Explicit task owner for one grant. The store loop receives store effects in the
//! order the state machine admitted them and runs each on its own task, beside the
//! others the core admitted; only opening, closing and split preparation run inline.
//! Filesystem queries and lifecycle row lookups run on a separate read task as soon as
//! the state machine admits them, beside any mutation. Replica RPCs run in the
//! core's bounded windows and check current authority at dispatch and reply.

use super::executor::cold_store::{ColdStore, ColdStoreConfig};
use super::{
	MaintainerSnapshot, TabletMaintainerMachine, effect::MaintainerEffect, event::MaintainerEvent,
	state::MaintainerState, types::*,
};
use crate::object_store_metrics::{ObjectStorePurpose, with_purpose};
use anyhow::{Context, Result};
use futures_util::{
	FutureExt,
	future::{AbortHandle, Abortable, BoxFuture},
};
use parking_lot::Mutex as SyncMutex;
use rivet_data::generated::{
	depot_client_protocol_v1::{ClientResponse, VolumeRejectResponse},
	depot_kv_command_v1::{OverloadReason, UnavailableReason},
	depot_storage_worker_protocol_v1::{StorageWorkerRequest, StorageWorkerResponse, TabletGrant},
	depot_tablet_coordinator_protocol_v1::{SplitPrepared, TabletCoordinatorResponse},
	depot_tablet_manifest_v1::Tablet,
	shared_depot_fs_read_v1::{FsReadOperation, FsReadRequest},
};
use rivet_env::{
	Env, TaskHandle, TaskScope,
	capabilities::{rand::RngCore, slatedb::SlateDbFactory},
};
use rivet_state_machine_ephemeral::{EffectContext, Executor, Runner};
use rivet_topology::worker::WorkerId;
use std::{
	collections::BTreeMap,
	future::Future,
	pin::Pin,
	sync::{
		Arc, OnceLock, Weak,
		atomic::{AtomicBool, AtomicU64, Ordering},
	},
	time::Duration,
};
use tokio::sync::{mpsc, oneshot, watch};

pub type ReplicaFuture<'a> =
	Pin<Box<dyn Future<Output = Result<StorageWorkerResponse, String>> + Send + 'a>>;

/// Production resolves exact worker identities through topology; fixed-grant tests use real in-process shards.
///
/// A transport fixes a request's place in its destination's send order before the request's
/// future first yields. The production transport takes the request's turn on the worker's
/// ordered UPS lane there, so one worker receives the maintainer's control requests, such as a
/// forget and the load that follows it, in the order the state machine emitted them. A
/// transport that yields first still works, but loses that order.
pub trait ReplicaTransport: Send + Sync + 'static {
	fn request(&self, worker: WorkerId, request: StorageWorkerRequest) -> ReplicaFuture<'_>;
}

/// The supervisor checks both the latest topology lease and the unchanged tablet assignment.
/// Implementations read their injected clock on every call; cached readiness is insufficient.
pub trait MaintainerAuthority: Send + Sync + 'static {
	fn may_serve(&self, grant: &TabletGrant) -> bool;
}

pub type SplitReportFuture<'a> =
	Pin<Box<dyn Future<Output = Result<TabletCoordinatorResponse, String>> + Send + 'a>>;

/// Resolves once the process that holds a volume's claim is gone, through topology's shared
/// watch of that process. Every maintainer on this worker watching one holder shares one
/// topology watch. The outer future resolves once the watch is taken; the inner one resolves
/// when the process is gone and never otherwise. Dropping the inner future ends this watch.
pub trait HolderWatches: Send + Sync + 'static {
	fn watch_holder<'a>(
		&'a self,
		holder: &'a rivet_data::generated::shared_depot_types_v1::VolumeHolder,
	) -> futures_util::future::BoxFuture<
		'a,
		anyhow::Result<futures_util::future::BoxFuture<'static, ()>>,
	>;
}

/// Holder watches for tests that do not exercise holders: every watch starts and none ever
/// reports its holder gone, so parked volumes stay loaded, as under a live holder.
#[cfg(any(test, feature = "test-support"))]
pub struct LiveHolders;

#[cfg(any(test, feature = "test-support"))]
impl HolderWatches for LiveHolders {
	fn watch_holder<'a>(
		&'a self,
		_holder: &'a rivet_data::generated::shared_depot_types_v1::VolumeHolder,
	) -> futures_util::future::BoxFuture<
		'a,
		anyhow::Result<futures_util::future::BoxFuture<'static, ()>>,
	> {
		Box::pin(async {
			let never: futures_util::future::BoxFuture<'static, ()> =
				Box::pin(std::future::pending());
			Ok(never)
		})
	}
}

/// Reports split preparation and retired-store deletion through coordinator discovery.
/// Retry and completion decisions remain in the maintainer state machine.
pub trait SplitReporter: Send + Sync + 'static {
	/// Requests a new grant for this exact fenced lifetime after its store has closed.
	fn recover(
		&self,
		_request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::RecoverTablets,
	) -> SplitReportFuture<'_> {
		Box::pin(async { Err("tablet recovery transport unavailable".into()) })
	}
	/// Reports replica coverage through the same grant-checked coordinator transport.
	fn report_catch_up(
		&self,
		_request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::CatchUpReport,
	) -> SplitReportFuture<'_> {
		Box::pin(async { Err("catch-up report transport unavailable".into()) })
	}

	/// Reports the tablet data's version marks through the same grant-checked transport.
	fn report_version_marks(
		&self,
		_request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::VersionMarksReport,
	) -> SplitReportFuture<'_> {
		Box::pin(async { Err("version marks report transport unavailable".into()) })
	}

	fn report(&self, request: SplitPrepared) -> SplitReportFuture<'_>;
	fn report_with_alarms(
		&self,
		_report: SplitPrepared,
		_manifest_id: u64,
		_maintainer: [u8; 16],
	) -> SplitReportFuture<'_> {
		Box::pin(async { Err("alarm split transport unavailable".into()) })
	}
	fn alarm_index_ready(
		&self,
		_report: rivet_data::generated::depot_tablet_coordinator_protocol_v1::AlarmIndexReady,
	) -> SplitReportFuture<'_> {
		Box::pin(async { Err("alarm readiness transport unavailable".into()) })
	}
	fn report_deleted(
		&self,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::RetiredTabletDeleted,
	) -> SplitReportFuture<'_>;
	/// Reports a split child's import of its parent's maintainer log through the same
	/// grant-checked transport.
	fn report_parent_log_imported(
		&self,
		_request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::ParentLogImported,
	) -> SplitReportFuture<'_> {
		Box::pin(async { Err("parent log import report transport unavailable".into()) })
	}
}

pub type ForkPeerFuture<'a> =
	Pin<Box<dyn Future<Output = Result<ClientResponse, String>> + Send + 'a>>;
/// Sends one exact request; retry policy and durable identity belong to the core.
pub trait ForkPeer: Send + Sync + 'static {
	fn request(
		&self,
		region: u16,
		request: rivet_data::generated::depot_client_protocol_v1::ClientRequest,
		deadline_boot_ms: u64,
	) -> ForkPeerFuture<'_>;
}

/// Runtime-only adapters never enter the deterministic state.
pub struct MaintainerDependencies {
	pub fork_peer: Option<Arc<dyn ForkPeer>>,
	pub base_reader: Option<Arc<dyn crate::filesystem::base::BaseChunkReader>>,
	pub env: Arc<dyn Env>,
	pub cold_store: Arc<dyn SlateDbFactory>,
	pub replicas: Arc<dyn ReplicaTransport>,
	pub authority: Arc<dyn MaintainerAuthority>,
	pub cold_config: ColdStoreConfig,
	pub split_reporter: Arc<dyn SplitReporter>,
	/// Watches holder processes, so a gone holder's parked volumes unload.
	pub holder_watches: Arc<dyn HolderWatches>,
}

// Submission and Drop are synchronous; the mutex protects only oneshot bookkeeping.
type StopResponses = Arc<
	SyncMutex<
		BTreeMap<
			RequestId,
			oneshot::Sender<
				rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
			>,
		>,
	>,
>;

type Responses = Arc<SyncMutex<BTreeMap<RequestId, oneshot::Sender<ClientResponse>>>>;

type KvResponses = Arc<
	SyncMutex<
		BTreeMap<
			RequestId,
			oneshot::Sender<rivet_data::generated::depot_kv_command_v1::KvCommandResponse>,
		>,
	>,
>;
type KvHost = Arc<OnceLock<Arc<crate::command::volume_host::VolumeHost>>>;

/// Outcome of offering a supervisor update to a maintainer's bounded inbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InboxDelivery {
	/// The update is queued behind every event already in the inbox.
	Queued,
	/// The inbox is full and nothing was queued. The caller must offer it again.
	Full,
}

/// One inbox slot held for a supervisor update; see `Maintainer::reserve_update`. The slot
/// counts against the inbox until an update is offered with it or it is dropped.
pub struct UpdateSlot(mpsc::OwnedPermit<MaintainerEvent>);

struct BulkPacing {
	budget: Arc<crate::filesystem::bulk_budget::BulkReadBudget>,
	min_chunks: usize,
}

impl BulkPacing {
	/// Counts the chunk runs that use bulk scan settings, before the request can take a read slot.
	fn bytes(&self, request: &FsReadRequest) -> u64 {
		let chunks = match &request.operation {
			FsReadOperation::FsReadChunks(read) => read
				.ranges
				.iter()
				.filter(|range| range.count as usize >= self.min_chunks)
				.fold(0u64, |sum, range| {
					sum.saturating_add(u64::from(range.count))
				}),
			FsReadOperation::FsBaseChunksRequest(read) => {
				let mut indices = read.indices.clone();
				indices.sort_unstable();
				let mut chunks = 0u64;
				let mut run = 0usize;
				let mut previous = None;
				for index in indices {
					if previous.is_some_and(|previous: u64| index.saturating_sub(previous) > 1) {
						if run >= self.min_chunks {
							chunks = chunks.saturating_add(run as u64);
						}
						run = 0;
					}
					run += 1;
					previous = Some(index);
				}
				if run >= self.min_chunks {
					chunks = chunks.saturating_add(run as u64);
				}
				chunks
			}
			FsReadOperation::FsReadGet(read) if self.min_chunks == 1 => {
				if matches!(
					depot_fs_core::store::RowKey::from(read.key.clone()),
					depot_fs_core::store::RowKey::Chunk(_, _)
				) {
					1
				} else {
					0
				}
			}
			FsReadOperation::FsReadOpen(_)
			| FsReadOperation::FsReadClose(_)
			| FsReadOperation::FsReadKeys(_)
			| FsReadOperation::FsReadRaw(_)
			| FsReadOperation::FsReadGet(_)
			| FsReadOperation::FsReadScan(_)
			| FsReadOperation::FsForkMetadataRequest(_)
			| FsReadOperation::FsBaseKeysRequest(_) => 0,
		};
		chunks.saturating_mul(depot_fs_core::CHUNK_BYTES as u64)
	}
}

pub struct Maintainer {
	alarm_tasks: Arc<SyncMutex<BTreeMap<OpId, AbortHandle>>>,
	kv_host: KvHost,
	kv_responses: KvResponses,
	
	inbox: Option<mpsc::Sender<MaintainerEvent>>,
	/// Opens queued in the inbox, not yet taken by the runner, at which a new open is
	/// refused at submit.
	max_queued_opens: u64,
	/// Opens this handle placed in the inbox, counted before the send so it never trails the
	/// core's `opens_received`.
	opens_submitted: AtomicU64,
	/// Whether the queued opens last crossed the warning threshold, so the warning fires once
	/// per approach to the cap rather than on every submit.
	open_queue_warned: AtomicBool,
	/// The tablet's load record, which tracks waiting opens and counts completions.
	open_load: Arc<crate::metrics::TabletLoad>,
	pub snapshot: watch::Receiver<MaintainerSnapshot>,
	stopped: Option<oneshot::Receiver<()>>,
	close_processed: Option<oneshot::Receiver<()>>,
	io_stopped: Option<oneshot::Receiver<Result<(), String>>>,
	env: Arc<dyn Env>,
	tasks: Arc<TaskScope>,
	bulk_pacing: Option<BulkPacing>,
	responses: Responses,
	stop_responses: StopResponses,
	next_request: AtomicU64,
	grant: TabletGrant,
	authority: Arc<dyn MaintainerAuthority>,
}

impl Maintainer {
	/// Starts the ordered store owner and the pure runner. Merely constructing adapters starts no services.
	pub fn start(
		tablet: Tablet,
		config: MaintainerConfig,
		deps: MaintainerDependencies,
	) -> Result<Self> {
		Self::start_with_alarms(tablet, config, deps, None, None)
	}

	pub(crate) fn start_with_alarms(
		tablet: Tablet,
		config: MaintainerConfig,
		deps: MaintainerDependencies,
		alarm_driver: Option<(crate::alarms::AlarmDriver, WorkerId)>,
		initialization: Option<
			rivet_data::generated::depot_tablet_manifest_v1::AlarmIndexInitialization,
		>,
	) -> Result<Self> {
		let mut state = MaintainerState::new(tablet.clone(), config.clone())?;
		state.alarms.enabled = alarm_driver.is_some() || initialization.is_some();
		state.alarms.initialization = initialization
			.as_ref()
			.filter(|i| {
				i.phase == rivet_data::generated::depot_tablet_manifest_v1::AlarmIndexPhase::CloneNeedsBootstrap
			})
			.and_then(|i| i.source.clone());
		let grant = TabletGrant {
			tablet_id: tablet.id,
			tablet_grant_epoch: tablet.tablet_grant_epoch,
		};
		if let Some((_, worker)) = &alarm_driver {
			state.alarms.owner = Some(rivet_data::generated::shared_depot_alarms_v1::AlarmOwner {
				worker_id: *worker.as_bytes(),
				tablet_id: grant.tablet_id,
				tablet_grant_epoch: grant.tablet_grant_epoch,
			});
			state.alarms.owner_id = *deps.env.rng("depot.alarm.owner").uuid_v4().as_bytes();
		}
		anyhow::ensure!(
			deps.authority.may_serve(&grant),
			"tablet grant is not current"
		);
		let tasks = Arc::new(deps.env.task_scope());
		let kv_host = Arc::new(OnceLock::new());
		let kv_responses = Arc::new(SyncMutex::new(BTreeMap::new()));
		let responses = Arc::new(SyncMutex::new(BTreeMap::new()));
		let stop_responses = Arc::new(SyncMutex::new(BTreeMap::new()));
		let closing = Arc::new(AtomicBool::new(false));
		let io_closing = closing.clone();
		let (io, mut receiver) = mpsc::unbounded_channel::<StoreWork>();
		let factory = deps.cold_store.clone();
		let base_reader = deps.base_reader.clone();
		let transaction_replicas = deps.replicas.clone();
		let settings = deps.cold_config.clone();
		let bulk_pacing = if settings.bulk_run.min_chunks > 0 {
			settings.bulk_budget.clone().map(|budget| BulkPacing {
				budget,
				min_chunks: settings.bulk_run.min_chunks,
			})
		} else {
			None
		};
		let authority = deps.authority.clone();
		let io_grant = grant.clone();
		let guard_authority = authority.clone();
		let guard_grant = grant.clone();
		let guard_closing = closing.clone();
		let mutation_guard: Arc<dyn rivet_env::MutationGuard> = Arc::new(move || {
			!guard_closing.load(Ordering::Acquire) && guard_authority.may_serve(&guard_grant)
		});
		
		
		let (io_done, io_stopped) = oneshot::channel();
		let storage_env = deps.env.clone();
		let (reads, read_dispatches) = mpsc::unbounded_channel::<ReadDispatch>();
		let io_reads = reads.clone();
		let (filesystem, open_filesystem) =
			watch::channel::<Option<Arc<crate::filesystem::store::FilesystemStore>>>(None);
		let request_filesystem = open_filesystem.clone();
		tasks.spawn(run_filesystem_reads(
			read_dispatches,
			open_filesystem,
			authority.clone(),
			grant.clone(),
			closing.clone(),
			deps.env.clone(),
		));
		let upload_env = deps.env.clone();
		let load = crate::metrics::TabletLoadHandle::register(tablet.id);
		let apply_signals = crate::signals::ApplySignals::new(tablet.id);
		let io_apply_signals = apply_signals.clone();
		let runner_load = load.load.clone();
		let open_load = load.load.clone();
		let store_deps = StoreIo {
			initialization,
			factory: factory.clone(),
			config: Arc::new(settings.clone()),
			guard: mutation_guard.clone(),
			env: storage_env.clone(),
			base_reader: base_reader.clone(),
			replicas: transaction_replicas.clone(),
		};
		tasks.spawn(async move {
			let mut storage: Option<Arc<ColdStore>> = None;
			let mut close_result = Ok(());
			// Store operations run on their own tasks beside this loop and each other.
			// The core admits only work whose footprints do not conflict; opening,
			// closing and split preparation stay inline because the core admits
			// nothing beside them. Closing the store joins every task first.
			let operations = upload_env.task_scope();
			let mut running: Vec<rivet_env::TaskHandle> = Vec::new();
			while let Some((effect, inbox, completion, queued_at)) = receiver.recv().await {
				crate::metrics::TABLET_STORE_QUEUE_WAIT.observe(
					crate::metrics::telemetry_elapsed()
						.saturating_sub(queued_at)
						.as_secs_f64(),
				);
				if !matches!(effect, MaintainerEffect::CloseCold)
					&& (!authority.may_serve(&io_grant) || io_closing.load(Ordering::Acquire))
				{
					// Dropping the completion records the fenced effect as dropped.
					let _ = inbox.send(MaintainerEvent::Fence).await;
					continue;
				}
				if matches!(effect, MaintainerEffect::CloseCold) {
					close_filesystem_reads(&io_reads, &filesystem).await;
					for task in running.drain(..) {
						let _ = task.join().await;
					}
				}
				// Apply and flush holds pause inside their own task, so the loop keeps
				// dispatching while one is parked.
				
				running.retain(|task| !task.is_finished());
				if let MaintainerEffect::FilesystemApplyRequest {
					read_timeout_ms,
					read_deadline_boot_ms,
					op_id,
					key,
					previous,
					entries,
					mut resolver,
					source,
					max_read_bytes,
					authority: apply_authority,
					drain_through,
					loaded_record,
				} = effect
				{
					let apply_load = load.load.clone();
					let signal_clock = storage_env.clock();
					let lag = entries
						.first()
						.and_then(|entry| {
							let clocks = signal_clock.observe().ok()?;
							let lag_ms = clocks.unix_ms.saturating_sub(entry.written_at_unix_ms);
							Some(std::time::Duration::from_millis(
								u64::try_from(lag_ms).unwrap_or(0),
							))
						})
						.unwrap_or_default();
					let apply_wait = apply_load.begin_apply(lag);
					let apply_signals = io_apply_signals.clone();
					let fs = storage.as_ref().map(|store| store.filesystem.clone());
					let replicas = transaction_replicas.clone();
					let guard = mutation_guard.clone();
					let clock = storage_env.clock();
					let authority = authority.clone();
					let grant = io_grant.clone();
					
					let panic_inbox = inbox.clone();
					running.push(operations.spawn(fence_on_panic(panic_inbox, async move {
						
						let timer = crate::metrics::TABLET_STORE_OPERATION_DURATION
							.with_label_values(&["filesystem_apply"])
							.start_timer();
						let result = async {
							anyhow::ensure!(
								resolver.position() == &previous,
								"stale filesystem resolver"
							);
							let fs = fs.context("filesystem store unavailable")?;
							let reader = crate::filesystem::apply::ReplicaTransactionReader {
								clock,
								read_timeout_ms,
								read_deadline_boot_ms,
								replicas: replicas.as_ref(),
								source,
								key: key.clone(),
								through: entries
									.last()
									.map_or(previous.volume_seq, |e| e.volume_seq),
								guard,
								max_bytes: max_read_bytes,
								drain_through,
							};
							let alarm = crate::metrics::with_tablet_load(
								apply_load.clone(),
								with_purpose(
									ObjectStorePurpose::Apply,
									fs.apply_resolved_tracking_alarm(
										&key,
										apply_authority,
										loaded_record.as_ref(),
										&mut resolver,
										&entries,
										&reader,
									),
								),
							)
							.await?;
							anyhow::Ok((resolver.position().clone(), alarm))
						}
						.await;
						apply_load.record_store(std::time::Duration::from_secs_f64(
							timer.stop_and_record(),
						));
						completion.finish(&result);
						apply_load.record_apply();
						drop(apply_wait);
						if result.is_ok()
							&& let Ok(clocks) = signal_clock.observe()
						{
							apply_signals.record_apply(&entries, clocks);
						}
						let event = filesystem_apply_event(
							op_id,
							result.map_err(|e| {
								e.downcast::<super::types::FilesystemApplyError>()
									.unwrap_or_else(|e| {
										super::types::FilesystemApplyError::Storage(e.to_string())
									})
							}),
						);
						
						let event = if authority.may_serve(&grant) {
							event
						} else {
							MaintainerEvent::Fence
						};
						let _ = inbox.send(event).await;
					})));
					continue;
				}
				if let MaintainerEffect::FilesystemFlushRequest {
					op_id,
					records,
					deferred_volumes,
					maintainer_log,
				} = effect
				{
					crate::metrics::TABLET_FLUSH_DEFERRED.inc_by(u64::from(deferred_volumes));
					let fs = storage.as_ref().map(|store| store.filesystem.clone());
					let authority = authority.clone();
					let grant = io_grant.clone();
					let upload_load = load.load.clone();
					
					let panic_inbox = inbox.clone();
					running.push(operations.spawn(fence_on_panic(panic_inbox, async move {
						
						// Stage, report staged, then make the records durable. The
						// staged report always precedes the flush response.
						let operation_timer = crate::metrics::TABLET_STORE_OPERATION_DURATION
							.with_label_values(&["filesystem_flush_stage"])
							.start_timer();
						let staged = async {
							let fs = fs.context("filesystem store unavailable")?;
							crate::metrics::with_tablet_load(
								upload_load.clone(),
								with_purpose(
									ObjectStorePurpose::Stage,
									fs.stage_flush_with_log(&records, maintainer_log.as_ref()),
								),
							)
							.await?;
							anyhow::Ok(fs)
						}
						.await;
						drop(operation_timer);
						let fs = match staged {
							Ok(fs) => fs,
							Err(error) => {
								completion.finish(&Err::<(), _>(()));
								let event = MaintainerEvent::FilesystemFlushResponse {
									op_id,
									result: Err(error.to_string()),
								};
								
								let event = if authority.may_serve(&grant) {
									event
								} else {
									MaintainerEvent::Fence
								};
								let _ = inbox.send(event).await;
								return;
							}
						};
						let _ = inbox
							.send(MaintainerEvent::FilesystemFlushStaged { op_id })
							.await;
						let timer = crate::metrics::TABLET_STORE_OPERATION_DURATION
							.with_label_values(&["filesystem_flush_upload"])
							.start_timer();
						let finished = crate::metrics::with_tablet_load(
							upload_load.clone(),
							with_purpose(ObjectStorePurpose::Retention, fs.finish_flush(&records)),
						)
						.await;
						drop(timer);
						if finished.is_ok()
							&& let Some(write) = &maintainer_log
						{
							fs.rows_marked_durable(&write.record.version_marks);
						}
						if let Ok(timings) = &finished {
							upload_load.record_flush(timings);
						}
						let result = finished.map(|_| ()).map_err(|e| e.to_string());
						completion.finish(&result);
						let event = MaintainerEvent::FilesystemFlushResponse { op_id, result };
						
						let event = if authority.may_serve(&grant) {
							event
						} else {
							MaintainerEvent::Fence
						};
						let _ = inbox.send(event).await;
					})));
					continue;
				}
				let inline = matches!(
					effect,
					MaintainerEffect::OpenColdRequest { .. }
						| MaintainerEffect::CloseCold
						| MaintainerEffect::PrepareSplitRequest { .. }
				);
				if !inline {
					let mut shared = storage.clone();
					let store_deps = store_deps.clone();
					let authority = authority.clone();
					let grant = io_grant.clone();
					let store_load = load.load.clone();
					
					let panic_inbox = inbox.clone();
					running.push(operations.spawn(fence_on_panic(panic_inbox, async move {
						let operation_timer = store_operation_label(&effect).map(|operation| {
							crate::metrics::TABLET_STORE_OPERATION_DURATION
								.with_label_values(&[operation])
								.start_timer()
						});
						let purpose = store_operation_purpose(&effect);
						let event = crate::metrics::with_tablet_load(
							store_load.clone(),
							with_purpose(purpose, store_io(effect, &mut shared, &store_deps)),
						)
						.await;
						if let Some(timer) = operation_timer {
							store_load.record_store(std::time::Duration::from_secs_f64(
								timer.stop_and_record(),
							));
						}
						completion.finish(&());
						
						if let Some(event) = event {
							let event = if authority.may_serve(&grant) {
								event
							} else {
								MaintainerEvent::Fence
							};
							let _ = inbox.send(event).await;
						}
					})));
					continue;
				}
				let operation_timer = store_operation_label(&effect).map(|operation| {
					crate::metrics::TABLET_STORE_OPERATION_DURATION
						.with_label_values(&[operation])
						.start_timer()
				});
				let purpose = store_operation_purpose(&effect);
				let bootstrap_tablet =
					if let MaintainerEffect::OpenColdRequest { tablet, .. } = &effect {
						Some(tablet.clone())
					} else {
						None
					};
				// Opening, closing and split preparation run inline, so a panic there must
				// not end this loop with the operation still in flight. A fenced or
				// draining core ignores `Fence` while it waits for its close, so a panicked
				// close reports a failed close instead.
				let closing = matches!(effect, MaintainerEffect::CloseCold);
				let mut event = std::panic::AssertUnwindSafe(with_purpose(
					purpose,
					store_io(effect, &mut storage, &store_deps),
				))
				.catch_unwind()
				.await
				.unwrap_or_else(|_| {
					tracing::error!(
						closing,
						"inline tablet store operation panicked; fencing the maintainer so its supervisor recovers the tablet"
					);
					match closing {
						true => Some(MaintainerEvent::ColdClosed {
							result: Err("cold store close panicked".into()),
						}),
						false => Some(MaintainerEvent::Fence),
					}
				});

				if let (
					Some(tablet),
					Some(initialization),
					Some(MaintainerEvent::OpenColdResponse { op_id, result }),
				) = (bootstrap_tablet, &store_deps.initialization, &mut event)
				{
					if result.is_ok() {
						let bootstrap = async {
							crate::filesystem::alarm_bootstrap::initialize(
								storage
									.as_ref()
									.context("cold store not open")?
									.filesystem(),
								store_deps.factory.as_ref(),
								&tablet,
								initialization,
								store_deps.guard.clone(),
								store_deps.env.as_ref(),
								&inbox,
								*op_id,
							)
							.await
						};
						if let Err(error) = bootstrap.await {
							*result = Err(error.to_string());
						}
					}
				}
				drop(operation_timer);
				completion.finish(&());
				// Publish the handle before the response reaches the core, so a query the
				// core dispatches after observing an open store finds it open.
				filesystem
					.send_replace(storage.as_ref().map(|store| store.filesystem.clone()));
				// The open store reports its memtable budget until it is dropped.
				if let (Some(MaintainerEvent::OpenColdResponse { result: Ok(_), .. }), Some(store)) =
					(&event, storage.as_ref())
				{
					drop(operations.spawn(forward_memtable_budget(
						store.unflushed_full(),
						inbox.clone(),
					)));
				}
				
				if let Some(event) = event {
					if let MaintainerEvent::ColdClosed { result } = &event {
						close_result = result.clone();
					}
					let event = if matches!(event, MaintainerEvent::ColdClosed { .. })
						|| authority.may_serve(&io_grant)
					{
						event
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				}
			}
			close_filesystem_reads(&io_reads, &filesystem).await;
			for task in running.drain(..) {
				let _ = task.join().await;
			}
			if let Some(store) = storage {
				close_result = close_store(store).await;
			}
			let _ = io_done.send(close_result);
		});
		let (notify_closed, close_processed) = oneshot::channel();
		let alarm_tasks = Arc::new(SyncMutex::new(BTreeMap::new()));
		let executor = Arc::new(MaintainerExecutor {
			filesystem: request_filesystem,
			alarm_driver: alarm_driver.map(|(driver, _)| driver),
			alarm_owner: state.alarms.owner.clone(),
			alarm_tasks: alarm_tasks.clone(),
			alarm_scope: Arc::new(deps.env.task_scope()),
			fork_peer: deps.fork_peer,
			fork_tasks: SyncMutex::new(BTreeMap::new()),
			kv_host: kv_host.clone(),
			kv_responses: kv_responses.clone(),
			notify_closed: SyncMutex::new(Some(notify_closed)),
			io,
			reads,
			responses: responses.clone(),
			stop_responses: stop_responses.clone(),
			recovery_jobs: Default::default(),
			repair_clients: Default::default(),
			split_reporter: deps.split_reporter,
			holder_watches: deps.holder_watches,
			holder_watch_cancels: scc::HashMap::new(),
			report_tasks: SyncMutex::new(BTreeMap::new()),
			replicas: deps.replicas,
			authority: deps.authority.clone(),
			grant: grant.clone(),
			tasks: Arc::downgrade(&tasks),
			timeout_ms: config.tuning().operation_timeout_ms,
			local_worker: config.local_worker.clone(),
			replica_tasks: SyncMutex::new(BTreeMap::new()),
			closing,
		});
		crate::metrics::TABLET_MAINTAINER_INBOX_CAPACITY.set(i64::from(config.inbox_capacity));
		crate::metrics::TABLET_VOLUME_OPEN_QUEUE_CAPACITY.set(i64::from(config.max_queued_opens));
		let busy: Arc<dyn rivet_state_machine_ephemeral::BusyObserver> = runner_load;
		let handle = Runner::<TabletMaintainerMachine>::spawn_with_observers(
			deps.env.clone(),
			state,
			None,
			Some(busy),
			|_| executor,
		);
		let mut store_pressure = handle.snapshot.clone();
		let signal_clock = deps.env.clock();
		let mut signals =
			crate::signals::SignalObserver::new(tablet.tablet_grant_epoch, apply_signals);
		tasks.spawn(async move {
			let mut gauges = crate::metrics::StoreGauges::default();
			let mut alarm_gauges = crate::metrics::AlarmGauges::default();
			loop {
				let snapshot = store_pressure.borrow_and_update().clone();
				if let Ok(now_boot_ms) = signal_clock.boot_ms() {
					signals.observe(&snapshot, now_boot_ms);
				}
				alarm_gauges.observe(snapshot.alarms);
				let store = snapshot.store;
				// Telemetry-only clock read for the admission stall integral.
				#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
				let now = std::time::Instant::now();
				gauges.observe(store, now);
				if store_pressure.changed().await.is_err() {
					break;
				}
			}
			signals.close(signal_clock.boot_ms().unwrap_or(0));
		});
		Ok(Self {
			alarm_tasks,
			kv_host,
			kv_responses,
			
			inbox: Some(handle.inbox),
			max_queued_opens: u64::from(config.max_queued_opens),
			opens_submitted: AtomicU64::new(0),
			open_queue_warned: AtomicBool::new(false),
			open_load,
			snapshot: handle.snapshot,
			stopped: Some(handle.stopped),
			close_processed: Some(close_processed),
			io_stopped: Some(io_stopped),
			env: deps.env,
			tasks,
			bulk_pacing,
			responses,
			stop_responses,
			next_request: AtomicU64::new(1),
			grant,
			authority: deps.authority,
		})
	}

	/// Receives retired parents selected from a validated supervisor manifest.
	/// Automatic assignment is not wired yet: nested splits need retained cleanup lineage.
	///
	/// Never waits for inbox space, so a busy tablet cannot stall the supervisor's
	/// admission loop. Without a `slot`, a full inbox returns `InboxDelivery::Full` and the
	/// caller offers the same update again with a slot from `reserve_update`.
	pub fn update_retired_tablets(
		&self,
		slot: Option<UpdateSlot>,
		retired: Vec<rivet_data::generated::depot_tablet_manifest_v1::RetiredTablet>,
		parent_log_recorded: bool,
	) -> Result<InboxDelivery> {
		self.offer_update(
			slot,
			MaintainerEvent::RetiredTabletsUpdated {
				retired,
				parent_log_recorded,
			},
		)
	}

	/// Delivers a newer replica set without reopening SlateDB at the same grant epoch.
	/// Like `update_retired_tablets`, a full inbox returns `InboxDelivery::Full`.
	pub fn update_assignment(&self, tablet: Tablet) -> Result<InboxDelivery> {
		self.offer_update(None, MaintainerEvent::AssignmentUpdated { tablet })
	}

	/// Delivers global CAS fencing together with the exact assignment it describes.
	pub fn observe_manifest(
		&self,
		slot: Option<UpdateSlot>,
		tablet_manifest_version: u64,
		tablet: Tablet,
	) -> Result<InboxDelivery> {
		self.offer_update(
			slot,
			MaintainerEvent::ManifestObserved {
				tablet_manifest_version,
				tablet,
			},
		)
	}

	/// Waits for inbox space for one supervisor update and holds it, so an update refused
	/// with `InboxDelivery::Full` is offered again exactly when it fits, without a retry
	/// timer. Resolves `None` once the maintainer has stopped.
	pub fn reserve_update(&self) -> BoxFuture<'static, Option<UpdateSlot>> {
		let Some(inbox) = self.inbox.clone() else {
			return Box::pin(async { None });
		};
		Box::pin(async move { inbox.reserve_owned().await.ok().map(UpdateSlot) })
	}

	

	/// Queues `event` with `slot` when one is held, and otherwise only if the inbox has room.
	fn offer_update(
		&self,
		slot: Option<UpdateSlot>,
		event: MaintainerEvent,
	) -> Result<InboxDelivery> {
		anyhow::ensure!(
			self.authority.may_serve(&self.grant),
			"tablet grant is not current"
		);
		let inbox = self.inbox.as_ref().context("maintainer stopped")?;
		if let Some(UpdateSlot(permit)) = slot {
			permit.send(event);
			return Ok(InboxDelivery::Queued);
		}
		match inbox.try_send(event) {
			Ok(()) => Ok(InboxDelivery::Queued),
			Err(mpsc::error::TrySendError::Full(_)) => {
				crate::metrics::record_maintainer_inbox_full("update");
				Ok(InboxDelivery::Full)
			}
			Err(mpsc::error::TrySendError::Closed(_)) => anyhow::bail!("maintainer stopped"),
		}
	}

	/// Asks the tablet to tier its applied data now. The reply reports what was queued.
	pub fn tier_now(&self) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		self.submit(|request_id| MaintainerEvent::OperatorTierRequest { request_id })
	}

	/// Inspects what this maintainer and its store hold for `volume_key`; see
	/// `state::operator_repair::inspect`.
	pub fn repair_inspect(
		&self,
		volume_key: depot_types::VolumeKey,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		self.submit(|request_id| {
			MaintainerEvent::Repair(super::event::RepairEvent::InspectRequest {
				request_id,
				volume_key,
			})
		})
	}

	/// Runs one mutating operator repair call; see `state::operator_repair::calls`.
	pub fn repair(
		&self,
		request: rivet_data::generated::depot_client_protocol_v1::RepairRequest,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		self.submit(|request_id| {
			MaintainerEvent::Repair(super::event::RepairEvent::Request {
				request_id,
				request,
			})
		})
	}

	/// Lists bounded lifecycle diagnostics for one volume.
	pub fn repair_diagnostics(
		&self,
		request: rivet_data::generated::depot_client_protocol_v1::RepairDiagnosticsRequest,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		self.submit(|request_id| {
			MaintainerEvent::Repair(super::event::RepairEvent::DiagnosticsRequest {
				request_id,
				request,
			})
		})
	}

	/// Lists the repair audit rows of one name.
	pub fn repair_audit_list(
		&self,
		request: rivet_data::generated::depot_client_protocol_v1::RepairAuditListRequest,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		self.submit(|request_id| {
			MaintainerEvent::Repair(super::event::RepairEvent::AuditListRequest {
				request_id,
				request,
			})
		})
	}

	/// Begins the unload of at most `limit` volumes quiet for `quiet_ms`. The reply reports the
	/// unloads begun and the volumes still loaded.
	pub fn unload_quiet(
		&self,
		quiet_ms: u64,
		limit: u32,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		self.submit(|request_id| MaintainerEvent::OperatorUnloadRequest {
			request_id,
			quiet_ms,
			limit,
		})
	}

	/// Registers the reply before admission. A refused inbox consumes no caller reservation.
	pub fn open(
		&self,
		worker: WorkerId,
		volume_key: depot_types::VolumeKey,
		create: bool,
		client_incarnation: Option<[u8; 16]>,
		deadline_boot_ms: u64,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		self.submit_open(|request_id| {
			MaintainerEvent::OpenVolumeRequest(OpenVolume {
				retention: None,
				deadline_boot_ms,
				client_incarnation,
				request_id,
				worker_id: worker,
				holder: None,
				volume_key,
				mode: if create {
					OpenMode::Create
				} else {
					OpenMode::Existing
				},
				initial: Vec::new(),
				read: None,
			})
		})
	}

	pub fn create_volume(
		&self,
		worker: WorkerId,
		volume_key: depot_types::VolumeKey,
		retention: Option<depot_fs_core::FsRetentionPolicy>,
		client_incarnation: [u8; 16],
		holder: Option<rivet_data::generated::shared_depot_types_v1::VolumeHolder>,
		deadline_boot_ms: u64,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		self.submit_open(|request_id| {
			MaintainerEvent::OpenVolumeRequest(OpenVolume {
				request_id,
				worker_id: worker,
				holder,
				volume_key,
				retention,
				deadline_boot_ms,
				mode: OpenMode::Create,
				client_incarnation: Some(client_incarnation),
				initial: Vec::new(),
				read: None,
			})
		})
	}

	/// Both wire paths enter the same serialized lookup and load operation.
	pub fn open_volume(
		&self,
		worker: WorkerId,
		request: rivet_data::generated::depot_client_protocol_v1::VolumeOpenRequest,
		deadline_boot_ms: u64,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		let holder = Some(request.holder.clone());
		self.submit_open(|request_id| {
			MaintainerEvent::OpenVolumeRequest(OpenVolume {
				mode: if request.create_if_absent.is_some() {
					OpenMode::CreateIfAbsent
				} else {
					OpenMode::Existing
				},
				initial: request
					.create_if_absent
					.as_ref()
					.map(|options| options.initial.clone())
					.unwrap_or_default(),
				read: request
					.create_if_absent
					.as_ref()
					.and_then(|options| options.read.clone()),
				retention: request
					.create_if_absent
					.and_then(|options| options.retention),
				client_incarnation: Some(request.client_incarnation),
				volume_key: request.volume_key,
				holder,
				worker_id: worker,
				request_id,
				deadline_boot_ms,
			})
		})
	}

	/// Deletes an exact incarnation; a quorum reply precedes cold-store cleanup.
	pub fn delete_volume(
		&self,
		key: depot_types::VolumeLogKey,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		let reply = self.submit(|request_id| MaintainerEvent::DeleteVolumeRequest {
			request_id,
			volume_log_key: key.clone(),
		})?;
		if let Some(host) = self.kv_host.get() {
			host.invalidate(&key);
		}
		Ok(reply)
	}

	pub fn fork(
		&self,
		spec: depot_fs_core::FsForkSpec,
		deadline_boot_ms: u64,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		self.submit(|request_id| MaintainerEvent::ForkRequest {
			request_id,
			spec,
			deadline_boot_ms,
		})
	}

	pub fn fork_reference(
		&self,
		request: rivet_data::generated::depot_client_protocol_v1::FsForkReferenceRequest,
		deadline_boot_ms: u64,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		self.submit(|request_id| MaintainerEvent::ForkReferenceRequest {
			request_id,
			request,
			deadline_boot_ms,
		})
	}
	pub fn checkpoint(
		&self,
		request: rivet_data::generated::depot_client_protocol_v1::VolumeCheckpointRequest,
		deadline_boot_ms: u64,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		self.submit(|request_id| MaintainerEvent::CheckpointRequest {
			request_id,
			request,
			deadline_boot_ms,
		})
	}

	pub fn filesystem_read(
		&self,
		request: FsReadRequest,
		deadline_boot_ms: u64,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		if let Some(pacing) = &self.bulk_pacing {
			let bytes = pacing.bytes(&request);
			if bytes > 0 {
				if !self.authority.may_serve(&self.grant) {
					return Err(depot_types::client::not_maintainer());
				}
				let now = self
					.env
					.clock()
					.observe()
					.map_err(|_| depot_types::client::overloaded())?
					.boot_ms;
				let ready = pacing.budget.reserve(bytes, now);
				if ready > now {
					return self.submit_paced_read(request, deadline_boot_ms, ready, now);
				}
			}
		}
		self.submit(|request_id| MaintainerEvent::FilesystemQueryRequest {
			request_id,
			request,
			deadline_boot_ms,
		})
	}

	/// Holds a paced request outside the maintainer inbox and read window until its budget is due.
	fn submit_paced_read(
		&self,
		request: FsReadRequest,
		deadline_boot_ms: u64,
		ready_boot_ms: u64,
		now_boot_ms: u64,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		let inbox = self
			.inbox
			.as_ref()
			.cloned()
			.ok_or_else(depot_types::client::overloaded)?;
		let id = self
			.next_request
			.try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
			.map_err(|_| depot_types::client::overloaded())?;
		let request_id = RequestId(id);
		let (tx, rx) = oneshot::channel();
		let clock = self.env.clock();
		let authority = self.authority.clone();
		let grant = self.grant.clone();
		let responses = self.responses.clone();
		crate::metrics::FS_BULK_BUDGET_WAITS.inc();
		crate::metrics::FS_BULK_BUDGET_WAIT_SECONDS
			.observe((ready_boot_ms - now_boot_ms) as f64 / 1000.0);
		self.tasks.spawn(async move {
			if clock.sleep_until(ready_boot_ms).await.is_err()
				|| clock
					.observe()
					.map_or(true, |now| now.boot_ms >= deadline_boot_ms)
			{
				let _ = tx.send(ClientResponse::VolumeRejectResponse(
					depot_types::client::overloaded(),
				));
				return;
			}
			if !authority.may_serve(&grant) {
				let _ = tx.send(ClientResponse::VolumeRejectResponse(
					depot_types::client::not_maintainer(),
				));
				return;
			}
			responses.lock().insert(request_id, tx);
			if inbox
				.try_send(MaintainerEvent::FilesystemQueryRequest {
					request_id,
					request,
					deadline_boot_ms,
				})
				.is_err()
			{
				if let Some(tx) = responses.lock().remove(&request_id) {
					crate::metrics::record_maintainer_inbox_full("request");
					let _ = tx.send(ClientResponse::VolumeRejectResponse(
						depot_types::client::overloaded(),
					));
				}
			}
		});
		Ok(rx)
	}

	/// Acknowledges close promptly; the core unloads only after the final row reaches cold storage.
	pub fn close_volume(
		&self,
		key: depot_types::VolumeLogKey,
		client: Option<rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		// A raw client, such as lazy creation's cleanup entry, may release its own claim
		// while the native KV host still owns the volume. Acknowledge it without unloading.
		let retained = self
			.kv_host
			.get()
			.is_some_and(|host| host.retains_volume(&key, client.as_ref()));
		if retained {
			if !self.authority.may_serve(&self.grant) {
				return Err(depot_types::client::not_maintainer());
			}
			let (reply, receive) = oneshot::channel();
			let _ = reply.send(ClientResponse::VolumeCloseResponse);
			return Ok(receive);
		}

		self.submit(|request_id| MaintainerEvent::CloseVolumeRequest {
			client,
			request_id,
			volume_log_key: key,
		})
	}

	/// Submits a volume open and records its maintainer-side latency when the reply arrives.
	///
	/// An open's UPS deadline keeps running while it waits in the inbox. Once
	/// `max_queued_opens` opens are queued and not yet taken by the runner, another would
	/// most likely expire before admission, so it is refused at once and the opener backs
	/// off instead. Other inbox events do not count: they are cheap to take and a burst of
	/// them says nothing about how long an open will wait.
	fn submit_open(
		&self,
		input: impl FnOnce(RequestId) -> MaintainerEvent,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		if !self.authority.may_serve(&self.grant) {
			return Err(depot_types::client::not_maintainer());
		}
		let queued = self.queued_opens();
		crate::metrics::TABLET_VOLUME_OPEN_INBOX_DEPTH.observe(queued as f64);
		self.warn_near_open_cap(queued);
		if queued >= self.max_queued_opens {
			crate::metrics::TABLET_VOLUME_OPEN_REJECTED
				.with_label_values(&["inbox_queue"])
				.inc();
			return Err(depot_types::client::overloaded());
		}

		// Count the open before it can reach the runner, so the core's received count
		// never runs ahead of this one.
		let open_wait = self.open_load.begin_open();
		self.opens_submitted.fetch_add(1, Ordering::AcqRel);
		let reply = match self.submit(input) {
			Ok(reply) => reply,
			Err(reject) => {
				self.opens_submitted.fetch_sub(1, Ordering::AcqRel);
				return Err(reject);
			}
		};
		let (tx, rx) = oneshot::channel();
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let started = std::time::Instant::now();
		let open_load = self.open_load.clone();
		self.tasks.spawn(async move {
			let label = match reply.await {
				Ok(response) => {
					let label = open_reply_label(&response);
					let _ = tx.send(response);
					label
				}
				Err(_) => "cancelled",
			};
			drop(open_wait);
			open_load.record_open_completed(label);
			crate::metrics::TABLET_VOLUME_OPEN_DURATION
				.with_label_values(&[label])
				.observe(started.elapsed().as_secs_f64());
		});
		Ok(rx)
	}

	fn submit(
		&self,
		input: impl FnOnce(RequestId) -> MaintainerEvent,
	) -> Result<oneshot::Receiver<ClientResponse>, VolumeRejectResponse> {
		if !self.authority.may_serve(&self.grant) {
			return Err(depot_types::client::not_maintainer());
		}
		let id = self
			.next_request
			.try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
			.map_err(|_| depot_types::client::overloaded())?;
		let id = RequestId(id);
		let (tx, rx) = oneshot::channel();
		self.responses.lock().insert(id, tx);
		let request = input(id);
		if self
			.inbox
			.as_ref()
			.is_none_or(|inbox| inbox.try_send(request).is_err())
		{
			self.responses.lock().remove(&id);
			crate::metrics::record_maintainer_inbox_full("request");
			return Err(depot_types::client::overloaded());
		}
		Ok(rx)
	}

	/// Opens this handle placed in the inbox that the core has not taken yet. The snapshot
	/// is published after each transition, so an open counts as queued until the transition
	/// that took it has run.
	fn queued_opens(&self) -> u64 {
		let received = self.snapshot.borrow().opens_received;
		self.opens_submitted
			.load(Ordering::Acquire)
			.saturating_sub(received)
	}

	/// Warns once when the queued opens reach three quarters of the cap, and rearms once they
	/// fall below half of it, so sustained pressure does not flood the log.
	fn warn_near_open_cap(&self, queued: u64) {
		let warn_at = self.max_queued_opens.saturating_mul(3) / 4;
		if queued >= warn_at.max(1) {
			if !self.open_queue_warned.swap(true, Ordering::AcqRel) {
				tracing::warn!(
					tablet_id = %uuid::Uuid::from_bytes(self.grant.tablet_id),
					queued,
					cap = self.max_queued_opens,
					"volume opens are queuing near depot.maintainer_max_queued_opens; the tablet maintainer runner is not keeping up with opens"
				);
			}
		} else if queued < self.max_queued_opens / 2 {
			self.open_queue_warned.store(false, Ordering::Release);
		}
	}

	/// Best-effort writer hint. Lost notifications are covered by status polling.
	pub fn filesystem_committed(&self, key: depot_types::VolumeLogKey, through: u64) -> Result<()> {
		anyhow::ensure!(
			self.authority.may_serve(&self.grant),
			"maintainer grant revoked"
		);
		self.inbox
			.as_ref()
			.context("maintainer stopped")?
			.try_send(MaintainerEvent::FilesystemCommittedRequest { key, through })?;
		Ok(())
	}

	/// Offers one tablet's replica status notices from a member's log shard. The core
	/// checks the sender, grant and replica set before applying them. A full inbox queues
	/// nothing and the caller refuses the notice, which the shard resends.
	pub fn status_notices(
		&self,
		worker: WorkerId,
		tablet_grant_epoch: u64,
		tablet_replica_set_version: u64,
		notices: Vec<crate::tablet_maintainer_machine::types::StatusNotice>,
	) -> InboxDelivery {
		let Some(inbox) = self.inbox.as_ref() else {
			return InboxDelivery::Full;
		};
		match inbox.try_send(MaintainerEvent::ReplicaStatusNotices {
			worker_id: worker,
			tablet_grant_epoch,
			tablet_replica_set_version,
			notices,
		}) {
			Ok(()) => InboxDelivery::Queued,
			Err(_) => InboxDelivery::Full,
		}
	}

	/// Cold-store close joins SlateDB's own tasks before the supervisor may release the grant.
	pub async fn shutdown(mut self) -> Result<()> {
		self.revoke_commands();
		if let Some(host) = self.kv_host.get() {
			host.shutdown().await;
		}
		if let Some(inbox) = self.inbox.as_ref() {
			let _ = inbox.send(MaintainerEvent::Drain).await;
		}
		if let Some(closed) = self.close_processed.as_mut() {
			let _ = closed.await;
		}
		self.close_processed.take();
		self.inbox.take();
		// Retain the receivers until each await finishes: cancellation of shutdown
		// transfers cleanup to Drop without abandoning a partly opened SlateDB.
		let stopped = self
			.stopped
			.as_mut()
			.context("missing runner completion")?
			.await;
		self.stopped.take();
		self.inbox.take();
		let closed = self
			.io_stopped
			.as_mut()
			.context("missing store completion")?
			.await;
		self.io_stopped.take();
		self.tasks.shutdown().await;
		self.responses.lock().clear();
		self.stop_responses.lock().clear();
		stopped.context("maintainer runner failed")?;
		closed
			.context("maintainer store task failed")?
			.map_err(anyhow::Error::msg)?;
		Ok(())
	}
}

impl Drop for Maintainer {
	fn drop(&mut self) {
		self.revoke_commands();
		let Some(io_stopped) = self.io_stopped.take() else {
			return;
		};
		let inbox = self.inbox.take();
		let stopped = self.stopped.take();
		let close_processed = self.close_processed.take();
		let tasks = self.tasks.clone();
		let kv_host = self.kv_host.get().cloned();
		let responses = self.responses.clone();
		// Ordinary Drop transfers ownership to a cleanup task. Only a process crash
		// may abandon a pending SlateDB open. Supervisors await shutdown before
		// releasing a grant; dropping this handle alone is not proof of closure.
		self.env.spawn(Box::pin(async move {
			if let Some(host) = kv_host {
				host.shutdown().await;
			}
			if let Some(inbox) = &inbox {
				let _ = inbox.send(MaintainerEvent::Drain).await;
			}
			if let Some(closed) = close_processed {
				let _ = closed.await;
			}
			drop(inbox);
			if let Some(stopped) = stopped {
				let _ = stopped.await;
			}
			let _ = io_stopped.await;
			tasks.shutdown().await;
			responses.lock().clear();
		}));
	}
}

/// One tablet store effect, its completion inbox and its telemetry record, which the store
/// loop finishes when the effect's work completes.
/// One store effect, its reply inbox, its completion and when it was queued, for the
/// queue wait metric.
type StoreWork = (
	MaintainerEffect,
	mpsc::Sender<MaintainerEvent>,
	rivet_state_machine::EffectCompletion,
	std::time::Duration,
);

struct MaintainerExecutor {
	alarm_driver: Option<crate::alarms::AlarmDriver>,
	alarm_owner: Option<rivet_data::generated::shared_depot_alarms_v1::AlarmOwner>,
	alarm_tasks: Arc<SyncMutex<BTreeMap<OpId, AbortHandle>>>,
	alarm_scope: Arc<TaskScope>,
	filesystem: watch::Receiver<Option<Arc<crate::filesystem::store::FilesystemStore>>>,
	fork_peer: Option<Arc<dyn ForkPeer>>,
	fork_tasks: SyncMutex<BTreeMap<OpId, TaskHandle>>,
	kv_host: KvHost,
	kv_responses: KvResponses,
	split_reporter: Arc<dyn SplitReporter>,
	holder_watches: Arc<dyn HolderWatches>,
	/// One watch task per watched holder, ended by dropping its sender. Only `execute`, a sync
	/// context, touches this map, so it uses the sync methods.
	holder_watch_cancels: scc::HashMap<super::state::holders::HolderKey, oneshot::Sender<()>>,
	report_tasks: SyncMutex<BTreeMap<OpId, TaskHandle>>,
	recovery_jobs: super::executor::repair::RecoveryJobs,
	repair_clients: super::executor::repair_claims::RepairClients,
	notify_closed: SyncMutex<Option<oneshot::Sender<()>>>,
	io: mpsc::UnboundedSender<StoreWork>,
	/// Filesystem queries bypass the ordered IO loop so a slow mutation never delays them.
	reads: mpsc::UnboundedSender<ReadDispatch>,
	responses: Responses,
	stop_responses: StopResponses,
	replicas: Arc<dyn ReplicaTransport>,
	authority: Arc<dyn MaintainerAuthority>,
	grant: TabletGrant,
	tasks: Weak<TaskScope>,
	timeout_ms: u64,
	/// The maintainer's own worker, which labels replica requests as local or remote.
	local_worker: Option<WorkerId>,
	replica_tasks: SyncMutex<BTreeMap<OpId, TaskHandle>>,
	closing: Arc<AtomicBool>,
}

impl Executor<TabletMaintainerMachine> for MaintainerExecutor {
	fn execute(&self, effect: MaintainerEffect, ctx: EffectContext<TabletMaintainerMachine>) {
		let inbox = ctx.inbox();
		let env = ctx.env();
		let (effect, split_manifest) = match effect {
			MaintainerEffect::ReportAlarmSplitRequest {
				op_id,
				request,
				manifest_id,
				maintainer,
			} => (
				MaintainerEffect::ReportSplitRequest { op_id, request },
				Some((manifest_id, maintainer)),
			),
			effect => (effect, None),
		};
		match effect {
			MaintainerEffect::RefreshCommandReplicaSets => {
				if !self.closing.load(Ordering::Acquire)
					&& self.authority.may_serve(&self.grant)
					&& let Some(host) = self.kv_host.get()
				{
					host.refresh_replica_sets();
				}
			}
			MaintainerEffect::ReportAlarmIndexReady { request } => {
				let Some(tasks) = self.tasks.upgrade() else {
					return;
				};
				let transport = self.split_reporter.clone();
				let authority = self.authority.clone();
				let grant = self.grant.clone();
				let inbox = inbox.clone();
				let clock = env.clock();
				let timeout = self.timeout_ms;
				ctx.spawn_in(&tasks, async move {
					if !authority.may_serve(&grant) {
						let _ = inbox.send(MaintainerEvent::Fence).await;
						return;
					}
					let result = rivet_env::timeout(
						clock.as_ref(),
						Duration::from_millis(timeout),
						transport.alarm_index_ready(request),
					)
					.await
					.unwrap_or_else(|e| Err(e.to_string()));
					let event = if authority.may_serve(&grant) {
						MaintainerEvent::AlarmIndexReadyResponse { result }
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
			}

			MaintainerEffect::AlarmRequest { op_id, request } => {
				let tasks = self.alarm_scope.clone();
				let driver = self.alarm_driver.clone();
				let owner = self.alarm_owner.clone();
				let fs = self.filesystem.borrow().clone();
				let authority = self.authority.clone();
				let grant = self.grant.clone();
				let closing = self.closing.clone();
				let running = self.alarm_tasks.clone();
				let env = env.clone();
				let (abort, registration) = AbortHandle::new_pair();
				running.lock().insert(op_id, abort);
				let inbox = inbox.clone();
				ctx.spawn_in(&tasks, async move {
					let work = async {
						anyhow::ensure!(
							!closing.load(Ordering::Acquire) && authority.may_serve(&grant),
							"tablet authority lost"
						);
						let fs = fs.context("alarm store not open")?;
						let driver = driver.context("alarm driver not configured")?;
						let owner = owner.context("alarm owner not configured")?;
						crate::alarms::execute(
							&fs,
							&driver,
							owner,
							request,
							env.rng("depot.alarm.retry").next_u32(),
						)
						.await
					};
					// Discard panicked work, but still complete its operation and release capacity.
					let work = std::panic::AssertUnwindSafe(work).catch_unwind();
					let result = Abortable::new(work, registration)
						.await
						.map_err(|_| "alarm operation cancelled".to_owned())
						.and_then(|r| r.map_err(|_| "alarm operation panicked".to_owned()))
						.and_then(|r| r.map_err(|error| error.to_string()));
					running.lock().remove(&op_id);
					let event = if !closing.load(Ordering::Acquire) && authority.may_serve(&grant) {
						MaintainerEvent::AlarmResponse { op_id, result }
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
			}
			MaintainerEffect::CancelAlarmRequest { op_id } => {
				if let Some(abort) = self.alarm_tasks.lock().get(&op_id) {
					abort.abort();
				}
			}
			MaintainerEffect::StopReadinessResponse {
				request_id,
				response,
			} => {
				let response = if self.authority.may_serve(&self.grant) {
					response
				} else {
					rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse::NotServing
				};
				if let Some(reply) = self.stop_responses.lock().remove(&request_id) {
					let _ = reply.send(response);
				}
			}
			MaintainerEffect::ForkPeerRequest {
				op_id,
				region,
				request,
				deadline_boot_ms,
			} => {
				let Some(tasks) = self.tasks.upgrade() else {
					return;
				};
				let peer = self.fork_peer.clone();
				let authority = self.authority.clone();
				let grant = self.grant.clone();
				let inbox = inbox.clone();
				let task = ctx.spawn_in(&tasks, async move {
					if !authority.may_serve(&grant) {
						let _ = inbox.send(MaintainerEvent::Fence).await;
						return;
					}
					let result = match peer {
						Some(peer) => peer.request(region, request, deadline_boot_ms).await,
						None => Err("fork peer transport unavailable".into()),
					};
					let event = if authority.may_serve(&grant) {
						MaintainerEvent::ForkPeerResponse { op_id, result }
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
				let mut pending = self.fork_tasks.lock();
				pending.retain(|_, task| !task.is_finished());
				pending.insert(op_id, task);
			}
			MaintainerEffect::CancelForkPeerRequest { op_id } => {
				if let Some(task) = self.fork_tasks.lock().remove(&op_id) {
					task.abort();
				}
			}
			MaintainerEffect::ExecuteKvCommand {
				deadline_boot_ms,
				request_id,
				request,
				request_bytes,
			} => {
				let Some(reply) = self.kv_responses.lock().remove(&request_id) else {
					return;
				};
				let Some(host) = self.kv_host.get() else {
					let _ = reply.send(crate::kv_refusals::authority_moved());
					return;
				};
				let response = host.submit(
					request,
					usize::try_from(request_bytes).unwrap_or(usize::MAX),
					deadline_boot_ms,
				);
				if let Some(tasks) = self.tasks.upgrade() {
					ctx.spawn_in(&tasks, async move {
						let _ = reply.send(
							response
								.await
								.unwrap_or_else(|_| crate::kv_refusals::authority_moved()),
						);
					});
				}
			}
			MaintainerEffect::KvCommandResponse {
				request_id,
				response,
			} => {
				if let Some(reply) = self.kv_responses.lock().remove(&request_id) {
					let _ = reply.send(response);
				}
			}
			MaintainerEffect::RecordVersionRefused {
				family,
				mark,
				max_read,
			} => {
				tracing::error!(
					tablet_id = %uuid::Uuid::from_bytes(self.grant.tablet_id),
					family,
					schema = rivet_data::families::VersionFamily(family).schema(),
					mark,
					?max_read,
					"tablet data holds a version this binary cannot read; refusing to open. Run a binary that reads it, since versions only move forward"
				);
			}
			MaintainerEffect::RecordUnloadDuration { duration_ms } => {
				crate::metrics::TABLET_UNLOAD_DURATION.observe(duration_ms as f64 / 1000.0)
			}
			MaintainerEffect::ReportCatchUpRequest { op_id, request } => {
				let Some(tasks) = self.tasks.upgrade() else {
					return;
				};
				let transport = self.split_reporter.clone();
				let authority = self.authority.clone();
				let grant = self.grant.clone();
				let inbox = inbox.clone();
				let task = ctx.spawn_in(&tasks, async move {
					if !authority.may_serve(&grant) {
						let _ = inbox.send(MaintainerEvent::Fence).await;
						return;
					}
					let result = transport.report_catch_up(request).await;
					if let Err(error) = &result {
						tracing::warn!(
							%error,
							tablet_id = %uuid::Uuid::from_bytes(grant.tablet_id),
							tablet_grant_epoch = grant.tablet_grant_epoch,
							"catch-up report failed; maintainer will retry while authorized"
						);
					}
					let event = if authority.may_serve(&grant) {
						MaintainerEvent::ReportCatchUpResponse { op_id, result }
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
				let mut tasks = self.report_tasks.lock();
				tasks.retain(|_, task| !task.is_finished());
				tasks.insert(op_id, task);
			}
			MaintainerEffect::ReportRetiredRequest { op_id, request } => {
				let Some(tasks) = self.tasks.upgrade() else {
					return;
				};
				let transport = self.split_reporter.clone();
				let authority = self.authority.clone();
				let grant = self.grant.clone();
				let inbox = inbox.clone();
				let task = ctx.spawn_in(&tasks, async move {
					if !authority.may_serve(&grant) {
						let _ = inbox.send(MaintainerEvent::Fence).await;
						return;
					}
					let result = transport.report_deleted(request).await;
					if let Err(error) = &result {
						tracing::warn!(
							%error,
							tablet_id = %uuid::Uuid::from_bytes(grant.tablet_id),
							tablet_grant_epoch = grant.tablet_grant_epoch,
							"retired tablet report failed; maintainer will retry while authorized"
						);
					}
					let event = if authority.may_serve(&grant) {
						MaintainerEvent::ReportRetiredResponse { op_id, result }
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
				let mut tasks = self.report_tasks.lock();
				tasks.retain(|_, task| !task.is_finished());
				tasks.insert(op_id, task);
			}
			MaintainerEffect::ReportAlarmSplitRequest { .. } => {
				unreachable!("normalized split report")
			}
			MaintainerEffect::ReportParentLogImportedRequest { op_id, request } => {
				let Some(tasks) = self.tasks.upgrade() else {
					return;
				};
				let transport = self.split_reporter.clone();
				let authority = self.authority.clone();
				let grant = self.grant.clone();
				let inbox = inbox.clone();
				let task = ctx.spawn_in(&tasks, async move {
					if !authority.may_serve(&grant) {
						let _ = inbox.send(MaintainerEvent::Fence).await;
						return;
					}
					let result = transport.report_parent_log_imported(request).await;
					if let Err(error) = &result {
						tracing::warn!(
							%error,
							tablet_id = %uuid::Uuid::from_bytes(grant.tablet_id),
							tablet_grant_epoch = grant.tablet_grant_epoch,
							"parent log import report failed; maintainer will retry while authorized"
						);
					}
					let event = if authority.may_serve(&grant) {
						MaintainerEvent::ReportParentLogImportedResponse { op_id, result }
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
				let mut tasks = self.report_tasks.lock();
				tasks.retain(|_, task| !task.is_finished());
				tasks.insert(op_id, task);
			}
			MaintainerEffect::ReportVersionMarksRequest { op_id, request } => {
				let Some(tasks) = self.tasks.upgrade() else {
					return;
				};
				let transport = self.split_reporter.clone();
				let authority = self.authority.clone();
				let grant = self.grant.clone();
				let inbox = inbox.clone();
				let task = ctx.spawn_in(&tasks, async move {
					if !authority.may_serve(&grant) {
						let _ = inbox.send(MaintainerEvent::Fence).await;
						return;
					}
					let result = transport.report_version_marks(request).await;
					if let Err(error) = &result {
						tracing::warn!(
							%error,
							tablet_id = %uuid::Uuid::from_bytes(grant.tablet_id),
							tablet_grant_epoch = grant.tablet_grant_epoch,
							"version marks report failed; maintainer will retry while authorized"
						);
					}
					let event = if authority.may_serve(&grant) {
						MaintainerEvent::ReportVersionMarksResponse { op_id, result }
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
				let mut tasks = self.report_tasks.lock();
				tasks.retain(|_, task| !task.is_finished());
				tasks.insert(op_id, task);
			}
			MaintainerEffect::WatchHolderRequest { holder, watch_id } => {
				let watches = self.holder_watches.clone();
				let Some(tasks) = self.tasks.upgrade() else {
					return;
				};
				// Replacing an older sender ends that holder's previous watch task.
				let (cancel, cancelled) = oneshot::channel::<()>();
				let key = super::state::holders::HolderKey::of(&holder);
				self.holder_watch_cancels.upsert_sync(key, cancel);
				let inbox = inbox.clone();
				let _task = ctx.spawn_in(&tasks, async move {
					let watched = async {
						match watches.watch_holder(&holder).await {
							Ok(gone) => {
								gone.await;
								MaintainerEvent::HolderGone {
									holder: holder.clone(),
								}
							}
							Err(error) => {
								// Topology logs and counts refusals once; this only explains the retry.
								tracing::debug!(
									?error,
									"holder watch refused; the maintainer retries it later"
								);
								MaintainerEvent::HolderWatchRefused {
									holder: holder.clone(),
									watch_id,
								}
							}
						}
					};
					let event = rivet_env::select_biased!("an ended watch never reports its holder",
						_ = cancelled => return,
						event = watched => event,
					);
					let _ = inbox.send(event).await;
				});
			}
			MaintainerEffect::UnwatchHolder { holder } => {
				let key = super::state::holders::HolderKey::of(&holder);
				self.holder_watch_cancels.remove_sync(&key);
			}
			MaintainerEffect::ReportSplitRequest { op_id, request } => {
				let Some(tasks) = self.tasks.upgrade() else {
					return;
				};
				let transport = self.split_reporter.clone();
				let authority = self.authority.clone();
				let grant = self.grant.clone();
				let inbox = inbox.clone();
				let task = ctx.spawn_in(&tasks, async move {
					if !authority.may_serve(&grant) {
						let _ = inbox.send(MaintainerEvent::Fence).await;
						return;
					}
					let result = if let Some((manifest_id, maintainer)) = split_manifest {
						transport
							.report_with_alarms(request, manifest_id, maintainer)
							.await
					} else {
						transport.report(request).await
					};
					if let Err(error) = &result {
						tracing::warn!(
							%error,
							tablet_id = %uuid::Uuid::from_bytes(grant.tablet_id),
							tablet_grant_epoch = grant.tablet_grant_epoch,
							"split report failed; maintainer will retry while authorized"
						);
					}
					let event = if authority.may_serve(&grant) {
						MaintainerEvent::ReportSplitResponse { op_id, result }
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
				let mut tasks = self.report_tasks.lock();
				tasks.retain(|_, task| !task.is_finished());
				tasks.insert(op_id, task);
			}
			MaintainerEffect::CancelSplitReportRequest { op_id }
			| MaintainerEffect::CancelRetiredReportRequest { op_id }
			| MaintainerEffect::CancelParentLogReportRequest { op_id }
			| MaintainerEffect::CancelVersionMarksReportRequest { op_id }
			| MaintainerEffect::CancelCatchUpReportRequest { op_id } => {
				if let Some(task) = self.report_tasks.lock().remove(&op_id) {
					task.abort();
				}
			}
			MaintainerEffect::RecoverVolumeRequest { op_id, request } => {
				self.recovery_jobs.start(
					op_id,
					request,
					super::executor::repair::RecoveryAdapters {
						replicas: self.replicas.clone(),
						authority: self.authority.clone(),
						grant: self.grant.clone(),
						env: env.clone(),
					},
					inbox.clone(),
				);
			}

			MaintainerEffect::CancelRecoveryRequest { op_id } => self.recovery_jobs.cancel(op_id),
			MaintainerEffect::CloseCold => {
				self.closing.store(true, Ordering::Release);
				for abort in self.alarm_tasks.lock().values() {
					abort.abort();
				}
				for (_, task) in std::mem::take(&mut *self.fork_tasks.lock()) {
					task.abort();
				}
				for (_, task) in std::mem::take(&mut *self.report_tasks.lock()) {
					task.abort();
				}
				for (_, task) in std::mem::take(&mut *self.replica_tasks.lock()) {
					task.abort();
				}
				let recoveries = self.recovery_jobs.stop();
				let repair_clients = self.repair_clients.stop();
				let stopped = async move {
					let recovered = recoveries.await;
					let repaired = repair_clients.await;
					recovered.and(repaired)
				};
				let alarms = self.alarm_scope.clone();
				let io = self.io.clone();
				let inbox = inbox.clone();
				let cleanup_env = env.clone();
				// The close is timed until the store loop closes the store.
				let completion = ctx.complete_later();
				cleanup_env.clone().spawn(Box::pin(async move {
					alarms.shutdown().await;
					if let Err(error) = stopped.await {
						// A panicked owner cannot prove nested cleanup; never release its grant.
						tracing::error!(%error, "recovery owner failed during maintainer shutdown");
						match cleanup_env.exit(70).await {}
					}
					let _ = io.send((
						MaintainerEffect::CloseCold,
						inbox,
						completion,
						crate::metrics::telemetry_elapsed(),
					));
				}));
			}

			MaintainerEffect::CloseComplete => {
				if let Some(notify) = self.notify_closed.lock().take() {
					let _ = notify.send(());
				}
			}
			MaintainerEffect::CancelReplicaRequest { op_id } => {
				if let Some(task) = self.replica_tasks.lock().remove(&op_id) {
					task.abort();
				}
			}
			MaintainerEffect::FilesystemQueryRequest {
				op_id,
				request,
				applied,
				settled,
				record,
			} => {
				let _ = self.reads.send(ReadDispatch::Query {
					op_id,
					request,
					applied,
					settled,
					record,
					inbox: inbox.clone(),
				});
			}
			MaintainerEffect::CancelFilesystemQueryRequest { op_id } => {
				let _ = self.reads.send(ReadDispatch::Cancel { op_id });
			}
			MaintainerEffect::ReadRowRequest {
				op_id,
				volume_key,
				incarnation,
			} => {
				let _ = self.reads.send(ReadDispatch::Row {
					op_id,
					volume_key,
					incarnation,
					inbox: inbox.clone(),
				});
			}
			MaintainerEffect::Repair(super::effect::RepairEffect::ClaimOpenRequest {
				op_id,
				request,
			}) => self.repair_clients.open(
				op_id,
				request,
				super::executor::repair_claims::ClaimAdapters {
					replicas: self.replicas.clone(),
					authority: self.authority.clone(),
					grant: self.grant.clone(),
					env: env.clone(),
				},
				inbox.clone(),
			),
			MaintainerEffect::Repair(super::effect::RepairEffect::ClaimAppendRequest {
				op_id,
				volume_log_key,
				body,
			}) => self
				.repair_clients
				.append(op_id, &volume_log_key, body, env, inbox.clone()),
			MaintainerEffect::Repair(super::effect::RepairEffect::ClaimCloseRequest {
				op_id,
				volume_log_key,
			}) => self
				.repair_clients
				.close(op_id, &volume_log_key, env, inbox.clone()),
			MaintainerEffect::Repair(
				effect @ (super::effect::RepairEffect::DiagnosticsRequest { .. }
				| super::effect::RepairEffect::InspectReadRequest { .. }
				| super::effect::RepairEffect::PrepareRequest { .. }
				| super::effect::RepairEffect::AuditListRequest { .. }),
			) => {
				let _ = self.reads.send(ReadDispatch::Repair {
					effect,
					inbox: inbox.clone(),
				});
			}
			MaintainerEffect::ReadOpenKeysRequest {
				op_id,
				key,
				record,
				applied,
				read,
			} => {
				let _ = self.reads.send(ReadDispatch::Keys {
					op_id,
					key,
					record,
					applied,
					read,
					inbox: inbox.clone(),
				});
			}
			MaintainerEffect::Reply {
				request_id,
				response,
			} => {
				let response = if self.authority.may_serve(&self.grant) {
					response
				} else {
					ClientResponse::VolumeRejectResponse(depot_types::client::not_maintainer())
				};
				if let Some(tx) = self.responses.lock().remove(&request_id) {
					let _ = tx.send(response);
				}
			}
			MaintainerEffect::ReplicaRequest {
				op_id,
				worker_id,
				request,
			} => {
				let Some(tasks) = self.tasks.upgrade() else {
					return;
				};
				let transport = self.replicas.clone();
				let authority = self.authority.clone();
				let grant = self.grant.clone();
				let inbox = inbox.clone();
				let clock = env.clock();
				let timeout_ms = self.timeout_ms;
				let locality_label = if self.local_worker.as_ref() == Some(&worker_id) {
					"local"
				} else {
					"remote"
				};
				let task = ctx.track(async move {
					if !authority.may_serve(&grant) {
						let _ = inbox.send(MaintainerEvent::Fence).await;
						return;
					}
					let request_label = replica_request_label(&request);
					#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
					let started = std::time::Instant::now();
					let (result, result_label) = match rivet_env::timeout(
						clock.as_ref(),
						Duration::from_millis(timeout_ms),
						transport.request(worker_id, request),
					)
					.await
					{
						Ok(Ok(StorageWorkerResponse::RejectResponse(reason))) => (
							Ok(StorageWorkerResponse::RejectResponse(reason)),
							"rejected",
						),
						Ok(Ok(response)) => (Ok(response), "ok"),
						Ok(Err(error)) => (Err(error), "error"),
						Err(error) => (Err(error.to_string()), "timeout"),
					};
					crate::metrics::TABLET_REPLICA_REQUEST_DURATION
						.with_label_values(&[request_label, result_label, locality_label])
						.observe(started.elapsed().as_secs_f64());
					let event = if authority.may_serve(&grant) {
						MaintainerEvent::ReplicaResponse { op_id, result }
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
				let Some(task) = start_in_effect_order(&tasks, task) else {
					return;
				};
				let mut pending = self.replica_tasks.lock();
				pending.retain(|_, task| !task.is_finished());
				pending.insert(op_id, task);
			}
			MaintainerEffect::PendingWorkRequest { .. }
			| MaintainerEffect::SweepPendingRequest { .. }
			| MaintainerEffect::MaintainerOpRequest { .. }
			| MaintainerEffect::CheckpointRequest { .. }
			| MaintainerEffect::ForkReferenceRequest { .. }
			| MaintainerEffect::ForkStoreRequest { .. }
			| MaintainerEffect::ForkCleanupRequest { .. }
			| MaintainerEffect::MaterializeRequest { .. }
			| MaintainerEffect::FilesystemLoadRequest { .. }
			| MaintainerEffect::FilesystemApplyRequest { .. }
			| MaintainerEffect::FilesystemFlushRequest { .. }
			| MaintainerEffect::SweepDeletedRequest { .. }
			| MaintainerEffect::PrepareSplitRequest { .. }
			| MaintainerEffect::DeleteRetiredRequest { .. }
			| MaintainerEffect::OpenColdRequest { .. }
			| MaintainerEffect::WriteToColdRequest { .. } => {
				let _ = self.io.send((
					effect,
					inbox.clone(),
					ctx.complete_later(),
					crate::metrics::telemetry_elapsed(),
				));
			}
		}
	}
}

/// Work for the filesystem read task, which owns every query and row lookup task. Reads
/// arrive here from the executor rather than through the ordered IO loop, so a read the
/// core admitted beside a mutation, such as a cold write pass of other volumes, never
/// waits for that mutation to finish.
enum ReadDispatch {
	/// Spawns one query task against the currently open filesystem store, or fences it
	/// when the grant has ended or the store has closed.
	Query {
		op_id: OpId,
		request: rivet_data::generated::shared_depot_fs_read_v1::FsReadRequest,
		applied: Option<rivet_data::generated::shared_depot_types_v1::VolumeLogPosition>,
		settled: Option<rivet_data::generated::shared_depot_types_v1::VolumeLogPosition>,
		record: Option<rivet_data::generated::shared_depot_fs_v1::FsVolumeRecord>,
		inbox: mpsc::Sender<MaintainerEvent>,
	},
	/// Spawns one lifecycle row lookup against the currently open filesystem store, or
	/// fences it when the grant has ended or the store has closed.
	Row {
		op_id: OpId,
		volume_key: depot_types::VolumeKey,
		incarnation: Option<[u8; 16]>,
		inbox: mpsc::Sender<MaintainerEvent>,
	},
	/// Spawns one open's read-back against the currently open filesystem store, or fences it
	/// when the grant has ended or the store has closed.
	Keys {
		op_id: OpId,
		key: depot_types::VolumeLogKey,
		record: rivet_data::generated::shared_depot_fs_v1::FsVolumeRecord,
		applied: rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
		read: rivet_data::generated::depot_client_protocol_v1::VolumeReadKeys,
		inbox: mpsc::Sender<MaintainerEvent>,
	},
	/// Spawns one operator repair read against the currently open filesystem store, or
	/// fences it when the grant has ended or the store has closed.
	Repair {
		effect: super::effect::RepairEffect,
		inbox: mpsc::Sender<MaintainerEvent>,
	},
	/// Aborts and joins one query task. A late completion is ignored by operation identity.
	Cancel { op_id: OpId },
	/// Aborts and joins every query task, then acknowledges. The IO loop sends this before
	/// closing the store, and every later query is fenced.
	Close { closed: oneshot::Sender<()> },
}

/// Owns filesystem query tasks for one maintainer and runs each as soon as it arrives.
/// The core decides which queries may overlap which mutations and bounds outstanding
/// queries by its cold-read windows, so this task never orders or limits them itself.
async fn run_filesystem_reads(
	mut dispatches: mpsc::UnboundedReceiver<ReadDispatch>,
	filesystem: watch::Receiver<Option<Arc<crate::filesystem::store::FilesystemStore>>>,
	authority: Arc<dyn MaintainerAuthority>,
	grant: TabletGrant,
	closing: Arc<AtomicBool>,
	env: Arc<dyn Env>,
) {
	let read_scope = TaskScope::new(env.clone());
	let mut reads = BTreeMap::<OpId, TaskHandle>::new();
	let mut store_closed = false;
	while let Some(dispatch) = dispatches.recv().await {
		match dispatch {
			ReadDispatch::Query {
				op_id,
				request,
				applied,
				settled,
				record,
				inbox,
			} => {
				if store_closed || !authority.may_serve(&grant) || closing.load(Ordering::Acquire) {
					let _ = inbox.send(MaintainerEvent::Fence).await;
					continue;
				}
				let store = filesystem.borrow().clone();
				let authority = authority.clone();
				let grant = grant.clone();
				let closing = closing.clone();
				let task = read_scope.spawn(async move {
					if !authority.may_serve(&grant) || closing.load(Ordering::Acquire) {
						let _ = inbox.send(MaintainerEvent::Fence).await;
						return;
					}
					let result = match store {
						Some(store) => {
							with_purpose(
								ObjectStorePurpose::Query,
								store.read_request(request, applied, settled, record),
							)
							.await
						}
						None => Err("filesystem store not open".into()),
					};
					let event = if authority.may_serve(&grant) && !closing.load(Ordering::Acquire) {
						MaintainerEvent::FilesystemQueryResponse { op_id, result }
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
				reads.retain(|_, task| !task.is_finished());
				reads.insert(op_id, task);
			}
			ReadDispatch::Row {
				op_id,
				volume_key,
				incarnation,
				inbox,
			} => {
				if store_closed || !authority.may_serve(&grant) || closing.load(Ordering::Acquire) {
					let _ = inbox.send(MaintainerEvent::Fence).await;
					continue;
				}
				let store = filesystem.borrow().clone();
				let authority = authority.clone();
				let grant = grant.clone();
				let closing = closing.clone();
				let task = read_scope.spawn(async move {
					let timer = crate::metrics::TABLET_STORE_OPERATION_DURATION
						.with_label_values(&["read_row"])
						.start_timer();
					let result = match store {
						Some(store) => with_purpose(
							ObjectStorePurpose::Lookup,
							store.lifecycle_lookup(&volume_key, incarnation),
						)
						.await
						.map_err(|error| error.to_string()),
						None => Err("filesystem store not open".into()),
					};
					drop(timer);
					let event = if authority.may_serve(&grant) && !closing.load(Ordering::Acquire) {
						MaintainerEvent::ReadRowResponse { op_id, result }
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
				reads.retain(|_, task| !task.is_finished());
				reads.insert(op_id, task);
			}
			ReadDispatch::Keys {
				op_id,
				key,
				record,
				applied,
				read,
				inbox,
			} => {
				if store_closed || !authority.may_serve(&grant) || closing.load(Ordering::Acquire) {
					let _ = inbox.send(MaintainerEvent::Fence).await;
					continue;
				}
				let store = filesystem.borrow().clone();
				let authority = authority.clone();
				let grant = grant.clone();
				let closing = closing.clone();
				let task = read_scope.spawn(async move {
					let timer = crate::metrics::TABLET_STORE_OPERATION_DURATION
						.with_label_values(&["read_open_keys"])
						.start_timer();
					let result = match store {
						Some(store) => store
							.read_keys(&key, record, applied, &read.file, &read.keys)
							.await
							.map_err(|error| error.to_string()),
						None => Err("filesystem store not open".into()),
					};
					drop(timer);
					let event = if authority.may_serve(&grant) && !closing.load(Ordering::Acquire) {
						MaintainerEvent::ReadOpenKeysResponse { op_id, result }
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
				reads.retain(|_, task| !task.is_finished());
				reads.insert(op_id, task);
			}
			ReadDispatch::Repair { effect, inbox } => {
				if store_closed || !authority.may_serve(&grant) || closing.load(Ordering::Acquire) {
					let _ = inbox.send(MaintainerEvent::Fence).await;
					continue;
				}
				let op_id = repair_read_op(&effect);
				let store = filesystem.borrow().clone();
				let authority = authority.clone();
				let grant = grant.clone();
				let closing = closing.clone();
				let task = read_scope.spawn(async move {
					let timer = crate::metrics::TABLET_STORE_OPERATION_DURATION
						.with_label_values(&["repair_read"])
						.start_timer();
					let event = repair_read(store, effect).await;
					drop(timer);
					let event = if authority.may_serve(&grant) && !closing.load(Ordering::Acquire) {
						MaintainerEvent::Repair(event)
					} else {
						MaintainerEvent::Fence
					};
					let _ = inbox.send(event).await;
				});
				reads.retain(|_, task| !task.is_finished());
				reads.insert(op_id, task);
			}
			ReadDispatch::Cancel { op_id } => {
				if let Some(task) = reads.remove(&op_id) {
					task.abort();
					let _ = task.join().await;
				}
			}
			ReadDispatch::Close { closed } => {
				read_scope.shutdown().await;
				reads.clear();
				store_closed = true;
				let _ = closed.send(());
			}
		}
	}
	read_scope.shutdown().await;
}

/// The operation a repair read answers.
fn repair_read_op(effect: &super::effect::RepairEffect) -> OpId {
	use super::effect::RepairEffect;
	match effect {
		RepairEffect::DiagnosticsRequest { op_id, .. }
		| RepairEffect::InspectReadRequest { op_id, .. }
		| RepairEffect::PrepareRequest { op_id, .. }
		| RepairEffect::AuditListRequest { op_id, .. }
		| RepairEffect::ClaimOpenRequest { op_id, .. }
		| RepairEffect::ClaimAppendRequest { op_id, .. }
		| RepairEffect::ClaimCloseRequest { op_id, .. } => *op_id,
	}
}

/// Runs one repair read against `store` and returns the event that answers it.
async fn repair_read(
	store: Option<Arc<crate::filesystem::store::FilesystemStore>>,
	effect: super::effect::RepairEffect,
) -> super::event::RepairEvent {
	use super::{effect::RepairEffect, event::RepairEvent};
	let not_open = || "filesystem store not open".to_owned();
	match effect {
		RepairEffect::DiagnosticsRequest { op_id, request } => RepairEvent::DiagnosticsResponse {
			op_id,
			result: match store {
				Some(store) => store
					.repair_diagnostics(request)
					.await
					.map_err(|e| e.to_string()),
				None => Err(not_open()),
			},
		},
		RepairEffect::InspectReadRequest {
			op_id,
			volume_key,
			incarnations,
		} => RepairEvent::InspectReadResponse {
			op_id,
			result: match store {
				Some(store) => store
					.repair_records(&volume_key, &incarnations)
					.await
					.map_err(|error| error.to_string()),
				None => Err(not_open()),
			},
		},
		RepairEffect::PrepareRequest {
			op_id,
			volume_log_key,
			operation_id,
			reads,
		} => RepairEvent::PrepareResponse {
			op_id,
			result: match store {
				Some(store) => store
					.repair_prepare(&volume_log_key, operation_id, reads)
					.await
					.map_err(|error| error.to_string()),
				None => Err(not_open()),
			},
		},
		RepairEffect::AuditListRequest {
			op_id,
			volume_key,
			after,
			limit,
		} => RepairEvent::AuditListResponse {
			op_id,
			result: match store {
				Some(store) => store
					.repair_audits(&volume_key, after, limit as usize)
					.await
					.map(|(audits, after)| {
						rivet_data::generated::depot_client_protocol_v1::RepairAuditPage {
							audits,
							after,
						}
					})
					.map_err(|error| error.to_string()),
				None => Err(not_open()),
			},
		},
		// The executor routes a claim's client work to the repair clients, never here; a
		// misrouted one fails instead of reaching the store.
		RepairEffect::ClaimOpenRequest { op_id, .. } => RepairEvent::ClaimOpenResponse {
			op_id,
			result: Err("a repair claim's open is not a store read".into()),
		},
		RepairEffect::ClaimAppendRequest { op_id, .. } => RepairEvent::ClaimAppendResponse {
			op_id,
			result: Err(
				crate::tablet_maintainer_machine::types::RepairAppendFailure::NotExecuted(
					"a repair claim's append is not a store read".into(),
				),
			),
		},
		RepairEffect::ClaimCloseRequest { op_id, .. } => RepairEvent::ClaimClosedResponse { op_id },
	}
}

/// Runs one store task and fences the maintainer if it panics. A store operation waits for
/// its own result past any deadline, so a task that ends without reporting must end the
/// lifetime instead of leaving its operation in flight for good.
async fn fence_on_panic(
	inbox: mpsc::Sender<MaintainerEvent>,
	task: impl std::future::Future<Output = ()>,
) {
	let finished = std::panic::AssertUnwindSafe(task).catch_unwind().await;
	if finished.is_err() {
		tracing::error!(
			"tablet store task panicked; fencing the maintainer so its supervisor recovers the tablet"
		);
		let _ = inbox.send(MaintainerEvent::Fence).await;
	}
}

/// Delivers each change of a store's memtable budget to the maintainer, in order, until the
/// store is dropped. Only the latest value matters, so a change made while the previous one
/// is being delivered is reported once with the value it ended at.
async fn forward_memtable_budget(
	mut full: watch::Receiver<bool>,
	inbox: mpsc::Sender<MaintainerEvent>,
) {
	while full.changed().await.is_ok() {
		let event = MaintainerEvent::MemtableBudget {
			full: *full.borrow_and_update(),
		};
		if inbox.send(event).await.is_err() {
			return;
		}
	}
}

/// Joins every filesystem query and withdraws the store handle before the IO loop closes
/// the store, so no read outlives it. A read task that already exited counts as closed.
async fn close_filesystem_reads(
	reads: &mpsc::UnboundedSender<ReadDispatch>,
	filesystem: &watch::Sender<Option<Arc<crate::filesystem::store::FilesystemStore>>>,
) {
	filesystem.send_replace(None);
	let (closed, joined) = oneshot::channel();
	if reads.send(ReadDispatch::Close { closed }).is_ok() {
		let _ = joined.await;
	}
}

/// Capabilities every store operation uses, cloned into each spawned task.
#[derive(Clone)]
struct StoreIo {
	initialization:
		Option<rivet_data::generated::depot_tablet_manifest_v1::AlarmIndexInitialization>,
	factory: Arc<dyn SlateDbFactory>,
	config: Arc<ColdStoreConfig>,
	guard: Arc<dyn rivet_env::MutationGuard>,
	env: Arc<dyn Env>,
	base_reader: Option<Arc<dyn crate::filesystem::base::BaseChunkReader>>,
	replicas: Arc<dyn ReplicaTransport>,
}

/// Closes the store once every task that shared it has been joined. A remaining clone
/// means a task outlived the join and is reported as a close failure. The memtable is not
/// uploaded: the next maintainer replays it from the logs.
async fn close_store(store: Arc<ColdStore>) -> Result<(), String> {
	match Arc::try_unwrap(store) {
		Ok(store) => store
			.close_without_upload()
			.await
			.map_err(|error| error.to_string()),
		Err(_) => Err("cold store is still shared by a store task at close".into()),
	}
}

/// Executes one store effect. Opening, closing and split preparation change or use the
/// handle inline; every other effect runs on its own task with a clone of the handle.
/// Filesystem queries never pass through here; the read task dispatches them against
/// the same grant-owned handle.
fn filesystem_apply_event(
	op_id: OpId,
	result: Result<
		(
			rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
			Option<rivet_data::generated::shared_depot_alarms_v1::AlarmSummary>,
		),
		super::types::FilesystemApplyError,
	>,
) -> MaintainerEvent {
	match result {
		Ok((_, Some(summary))) => MaintainerEvent::FilesystemAlarmApplyResponse { op_id, summary },
		other => MaintainerEvent::FilesystemApplyResponse {
			op_id,
			result: other.map(|(position, _)| position),
		},
	}
}

async fn store_io(
	effect: MaintainerEffect,
	storage: &mut Option<Arc<ColdStore>>,
	deps: &StoreIo,
) -> Option<MaintainerEvent> {
	let factory = deps.factory.as_ref();
	let config = deps.config.as_ref();
	let guard = deps.guard.clone();
	let env = deps.env.as_ref();
	let base_reader = deps.base_reader.clone();
	let replicas = deps.replicas.as_ref();
	match effect {
		MaintainerEffect::MaterializeRequest {
			op_id,
			scope,
			action,
		} => {
			let result = async {
				storage
					.as_ref()
					.context("cold store not open")?
					.filesystem()
					.materialize_step(&scope, action)
					.await
			}
			.await
			.map_err(|e| e.to_string());
			Some(MaintainerEvent::MaterializeResponse { op_id, result })
		}
		MaintainerEffect::ForkCleanupRequest {
			op_id,
			work,
			region,
			action,
			loaded_parent_incarnations,
		} => {
			let result = async {
				storage
					.as_ref()
					.context("cold store not open")?
					.filesystem()
					.fork_cleanup(work, region, action, &loaded_parent_incarnations)
					.await
			}
			.await
			.map_err(|e| e.to_string());
			Some(MaintainerEvent::ForkCleanupResponse { op_id, result })
		}
		MaintainerEffect::ForkStoreRequest {
			op_id,
			spec,
			region,
			action,
			replaced,
		} => {
			let result = async {
				storage
					.as_ref()
					.context("cold store not open")?
					.filesystem
					.fork_store_action(spec, region, action, replaced)
					.await
			}
			.await;
			Some(MaintainerEvent::ForkStoreResponse {
				op_id,
				result: result.map_err(|e| e.to_string()),
			})
		}
		MaintainerEffect::ForkReferenceRequest {
			op_id,
			request,
			region,
			barrier,
			record,
			applied,
			logged,
			loaded_parent_incarnations,
		} => {
			let result = async {
				storage
					.as_ref()
					.context("cold store not open")?
					.filesystem()
					.fork_reference_request(
						request,
						region,
						barrier,
						record,
						applied,
						logged,
						&loaded_parent_incarnations,
					)
					.await
			}
			.await
			.map_err(|e| e.to_string());
			Some(MaintainerEvent::ForkReferenceResponse { op_id, result })
		}
		MaintainerEffect::PendingWorkRequest {
			op_id,
			after,
			limit,
			volume,
		} => {
			let result = async {
				let filesystem = storage
					.as_ref()
					.context("cold store not open")?
					.filesystem();
				match &volume {
					Some(volume) => filesystem.pending_work_of(volume, after, limit).await,
					None => filesystem.pending_work(after, limit).await,
				}
			}
			.await
			.map_err(|e| e.to_string());
			Some(MaintainerEvent::PendingWorkResponse { op_id, result })
		}
		MaintainerEffect::SweepPendingRequest { op_id, scope } => {
			let result = async {
				storage
					.as_ref()
					.context("cold store not open")?
					.filesystem()
					.sweep_pending(&scope)
					.await
			}
			.await
			.map_err(|e| e.to_string());
			Some(MaintainerEvent::SweepPendingResponse { op_id, result })
		}

		MaintainerEffect::MaintainerOpRequest {
			op_id,
			op,
			record,
			applied,
			region,
			loaded_parent_incarnations,
		} => {
			let result = async {
				storage
					.as_ref()
					.context("cold store not open")?
					.filesystem()
					.apply_maintainer_op(op, record, applied, region, &loaded_parent_incarnations)
					.await
			}
			.await
			.map_err(|e| e.to_string());
			Some(MaintainerEvent::MaintainerOpResponse { op_id, result })
		}
		MaintainerEffect::CheckpointRequest {
			op_id,
			request,
			record,
			applied,
			logged,
		} => {
			let result = async {
				let fs = storage
					.as_ref()
					.context("cold store not open")?
					.filesystem();
				match logged {
					Some(created_at) => {
						fs.checkpoint_logged(request, record, applied, created_at)
							.await
					}
					None => fs.checkpoint_request(request, record, applied).await,
				}
			}
			.await
			.map_err(|e| e.to_string());
			Some(MaintainerEvent::CheckpointResponse { op_id, result })
		}

		MaintainerEffect::FilesystemLoadRequest { op_id, key } => {
			let result = async {
				storage
					.as_ref()
					.context("cold store not open")?
					.filesystem()
					.incarnation_row(&key.0.volume_key, key.0.volume_incarnation)
					.await
			}
			.await;
			Some(MaintainerEvent::FilesystemLoadResponse {
				op_id,
				result: result.map_err(|e| e.to_string()),
			})
		}
		MaintainerEffect::FilesystemApplyRequest {
			read_timeout_ms,
			read_deadline_boot_ms,
			op_id,
			key,
			previous,
			entries,
			mut resolver,
			source,
			max_read_bytes,
			authority,
			drain_through,
			loaded_record,
		} => {
			let result = async {
				anyhow::ensure!(
					resolver.position() == &previous,
					"stale filesystem resolver"
				);
				let reader = crate::filesystem::apply::ReplicaTransactionReader {
					clock: env.clock(),
					read_timeout_ms,
					read_deadline_boot_ms,
					replicas,
					source,
					key: key.clone(),
					through: entries.last().map_or(previous.volume_seq, |e| e.volume_seq),
					guard: guard.clone(),
					max_bytes: max_read_bytes,
					drain_through,
				};
				let alarm = storage
					.as_ref()
					.context("cold store not open")?
					.filesystem()
					.apply_resolved_tracking_alarm(
						&key,
						authority,
						loaded_record.as_ref(),
						&mut resolver,
						&entries,
						&reader,
					)
					.await?;
				anyhow::Ok((resolver.position().clone(), alarm))
			}
			.await;
			Some(filesystem_apply_event(
				op_id,
				result.map_err(|e| {
					e.downcast::<super::types::FilesystemApplyError>()
						.unwrap_or_else(|e| {
							super::types::FilesystemApplyError::Storage(e.to_string())
						})
				}),
			))
		}
		MaintainerEffect::FilesystemFlushRequest {
			op_id,
			records,
			maintainer_log,
			..
		} => {
			let result = async {
				let fs = storage
					.as_ref()
					.context("cold store not open")?
					.filesystem();
				fs.stage_flush_with_log(&records, maintainer_log.as_ref())
					.await?;
				fs.finish_flush(&records).await?;
				if let Some(write) = &maintainer_log {
					fs.rows_marked_durable(&write.record.version_marks);
				}
				anyhow::Ok(())
			}
			.await;
			Some(MaintainerEvent::FilesystemFlushResponse {
				op_id,
				result: result.map_err(|e| e.to_string()),
			})
		}

		MaintainerEffect::DeleteRetiredRequest { op_id, retired } => {
			use rivet_keys::keys::codec::KeyAddress;
			let path = depot_types::keys::s3::TabletVolumePrefix::new(uuid::Uuid::from_bytes(
				retired.tablet_id,
			))
			.pack();
			let result = factory
				.delete_unpinned_database(
					path,
					uuid::Uuid::from_bytes(retired.checkpoint_id),
					guard,
				)
				.await;
			if let Err(error) = &result {
				tracing::warn!(%error, tablet_id = %uuid::Uuid::from_bytes(retired.tablet_id), "retired tablet deletion failed; retaining cleanup for retry");
			}
			Some(MaintainerEvent::DeleteRetiredResponse {
				op_id,
				result: result.map_err(|e| e.to_string()),
			})
		}

		MaintainerEffect::PrepareSplitRequest {
			op_id,
			tablet,
			step,
		} => {
			let result = async {
				storage
					.as_ref()
					.context("cold store not open")?
					.prepare_split(factory, &tablet, step, guard)
					.await
			}
			.await;
			if result
				.as_ref()
				.err()
				.and_then(|error| error.downcast_ref::<slatedb::Error>())
				.is_some_and(|error| matches!(error.kind(), slatedb::ErrorKind::Closed(_)))
			{
				Some(MaintainerEvent::Fence)
			} else {
				Some(MaintainerEvent::PrepareSplitResponse {
					op_id,
					result: result.map_err(|error| error.to_string()),
				})
			}
		}
		MaintainerEffect::SweepDeletedRequest {
			op_id,
			volume_log_key,
			max_rows,
		} => {
			let timer = crate::metrics::DELETE_SWEEP_SECONDS.start_timer();
			let result = async {
				storage
					.as_ref()
					.context("cold store not open")?
					.sweep_deleted_volume(&volume_log_key, max_rows)
					.await
			}
			.await;
			drop(timer);
			crate::metrics::DELETE_SWEEP_RESULTS
				.with_label_values(&[if result.is_ok() { "success" } else { "error" }])
				.inc();
			if let Err(error) = &result {
				tracing::warn!(%error, "filesystem deletion sweep failed; fencing maintainer");
			}
			Some(MaintainerEvent::SweepDeletedResponse {
				op_id,
				result: result.map_err(|error| error.to_string()),
			})
		}
		MaintainerEffect::OpenColdRequest {
			op_id,
			tablet,
			rows_version,
		} => {
			let began = env
				.clock()
				.observe()
				.map(|clocks| clocks.boot_ms)
				.unwrap_or(0);
			let tablet_id = tablet.id;
			let grant_epoch = tablet.tablet_grant_epoch;
			let result = async {
				let store = Arc::new(
					ColdStore::open_filesystem(
						factory,
						&tablet,
						config,
						env,
						guard.clone(),
						base_reader,
					)
					.await?,
				);
				let log_key = crate::tablet_maintainer_machine::state::maintainer_log::log_key(
					tablet.id,
					tablet.hash_range.start,
				);
				let record = store
					.filesystem()
					.maintainer_log_record(&log_key.0.volume_key)
					.await?;
				// The record was just read from a store this grant has not written, so its
				// marks are durable.
				store.filesystem().open_row_version(
					rows_version,
					record
						.as_ref()
						.map_or(&[][..], |record| record.version_marks.as_slice()),
				);
				*storage = Some(store);
				Ok::<_, anyhow::Error>(record)
			}
			.await;
			let ended = env
				.clock()
				.observe()
				.map(|clocks| clocks.boot_ms)
				.unwrap_or(began);
			tracing::info!(
				tablet_id = %uuid::Uuid::from_bytes(tablet_id),
				grant_epoch,
				open_cold_ms = ended.saturating_sub(began),
				ok = result.is_ok(),
				"tablet cold store opened"
			);
			Some(MaintainerEvent::OpenColdResponse {
				op_id,
				result: result.map_err(|e| e.to_string()),
			})
		}
		MaintainerEffect::WriteToColdRequest {
			op_id,
			rows,
			upload,
		} => {
			let result = async {
				let rows = rows
					.into_iter()
					.map(|(key, row)| (key.into(), row))
					.collect::<Vec<_>>();
				let store = storage.as_ref().context("cold store not open")?;
				if upload {
					store.write_rows(&rows).await
				} else {
					store.stage_rows(&rows).await
				}
			}
			.await;
			Some(MaintainerEvent::WriteToColdResponse {
				op_id,
				result: result.map_err(|e| e.to_string()),
			})
		}
		MaintainerEffect::CloseCold => {
			let result = match storage.take() {
				Some(store) => close_store(store).await,
				None => Ok(()),
			};
			if let Err(error) = &result {
				tracing::warn!(%error, "cold store close failed after ownership ended");
			}
			Some(MaintainerEvent::ColdClosed { result })
		}
		MaintainerEffect::StopReadinessResponse { .. }
		| MaintainerEffect::FilesystemQueryRequest { .. }
		| MaintainerEffect::CancelFilesystemQueryRequest { .. }
		| MaintainerEffect::ReadRowRequest { .. }
		| MaintainerEffect::ReadOpenKeysRequest { .. }
		| MaintainerEffect::Repair(_)
		| MaintainerEffect::ExecuteKvCommand { .. }
		| MaintainerEffect::KvCommandResponse { .. }
		| MaintainerEffect::RecordVersionRefused { .. }
		| MaintainerEffect::RecordUnloadDuration { .. }
		| MaintainerEffect::ReportAlarmIndexReady { .. }
		| MaintainerEffect::ReportAlarmSplitRequest { .. }
		| MaintainerEffect::ReportSplitRequest { .. }
		| MaintainerEffect::WatchHolderRequest { .. }
		| MaintainerEffect::UnwatchHolder { .. }
		| MaintainerEffect::ReportRetiredRequest { .. }
		| MaintainerEffect::ReportParentLogImportedRequest { .. }
		| MaintainerEffect::CancelParentLogReportRequest { .. }
		| MaintainerEffect::ReportCatchUpRequest { .. }
		| MaintainerEffect::CancelCatchUpReportRequest { .. }
		| MaintainerEffect::CancelRetiredReportRequest { .. }
		| MaintainerEffect::ReportVersionMarksRequest { .. }
		| MaintainerEffect::CancelVersionMarksReportRequest { .. }
		| MaintainerEffect::CancelSplitReportRequest { .. }
		| MaintainerEffect::RecoverVolumeRequest { .. }
		| MaintainerEffect::CancelRecoveryRequest { .. }
		| MaintainerEffect::ReplicaRequest { .. }
		| MaintainerEffect::AlarmRequest { .. }
		| MaintainerEffect::CancelAlarmRequest { .. }
		| MaintainerEffect::Reply { .. }
		| MaintainerEffect::CancelReplicaRequest { .. }
		| MaintainerEffect::ForkPeerRequest { .. }
		| MaintainerEffect::CancelForkPeerRequest { .. }
		| MaintainerEffect::RefreshCommandReplicaSets
		| MaintainerEffect::CloseComplete => None,
	}
}

/// The object-store purpose label for a store operation the generic IO path executes.
fn store_operation_purpose(effect: &MaintainerEffect) -> ObjectStorePurpose {
	match effect {
		MaintainerEffect::FilesystemLoadRequest { .. } => ObjectStorePurpose::Load,
		MaintainerEffect::OpenColdRequest { .. } | MaintainerEffect::ReadOpenKeysRequest { .. } => {
			ObjectStorePurpose::Open
		}
		MaintainerEffect::CheckpointRequest { .. }
		| MaintainerEffect::MaintainerOpRequest { .. }
		| MaintainerEffect::CloseCold
		| MaintainerEffect::DeleteRetiredRequest { .. }
		| MaintainerEffect::FilesystemApplyRequest { .. }
		| MaintainerEffect::FilesystemFlushRequest { .. }
		| MaintainerEffect::ForkCleanupRequest { .. }
		| MaintainerEffect::ForkReferenceRequest { .. }
		| MaintainerEffect::ForkStoreRequest { .. }
		| MaintainerEffect::MaterializeRequest { .. }
		| MaintainerEffect::PendingWorkRequest { .. }
		| MaintainerEffect::PrepareSplitRequest { .. }
		| MaintainerEffect::SweepDeletedRequest { .. }
		| MaintainerEffect::SweepPendingRequest { .. }
		| MaintainerEffect::WriteToColdRequest { .. }
		| MaintainerEffect::StopReadinessResponse { .. }
		| MaintainerEffect::FilesystemQueryRequest { .. }
		| MaintainerEffect::CancelFilesystemQueryRequest { .. }
		| MaintainerEffect::ReadRowRequest { .. }
		| MaintainerEffect::Repair(_)
		| MaintainerEffect::ExecuteKvCommand { .. }
		| MaintainerEffect::KvCommandResponse { .. }
		| MaintainerEffect::RecordVersionRefused { .. }
		| MaintainerEffect::RecordUnloadDuration { .. }
		| MaintainerEffect::ReportAlarmIndexReady { .. }
		| MaintainerEffect::ReportAlarmSplitRequest { .. }
		| MaintainerEffect::ReportSplitRequest { .. }
		| MaintainerEffect::WatchHolderRequest { .. }
		| MaintainerEffect::UnwatchHolder { .. }
		| MaintainerEffect::ReportRetiredRequest { .. }
		| MaintainerEffect::ReportParentLogImportedRequest { .. }
		| MaintainerEffect::CancelParentLogReportRequest { .. }
		| MaintainerEffect::ReportCatchUpRequest { .. }
		| MaintainerEffect::CancelCatchUpReportRequest { .. }
		| MaintainerEffect::CancelRetiredReportRequest { .. }
		| MaintainerEffect::ReportVersionMarksRequest { .. }
		| MaintainerEffect::CancelVersionMarksReportRequest { .. }
		| MaintainerEffect::CancelSplitReportRequest { .. }
		| MaintainerEffect::RecoverVolumeRequest { .. }
		| MaintainerEffect::CancelRecoveryRequest { .. }
		| MaintainerEffect::ReplicaRequest { .. }
		| MaintainerEffect::AlarmRequest { .. }
		| MaintainerEffect::CancelAlarmRequest { .. }
		| MaintainerEffect::Reply { .. }
		| MaintainerEffect::CancelReplicaRequest { .. }
		| MaintainerEffect::ForkPeerRequest { .. }
		| MaintainerEffect::CancelForkPeerRequest { .. }
		| MaintainerEffect::RefreshCommandReplicaSets
		| MaintainerEffect::CloseComplete => ObjectStorePurpose::Maintenance,
	}
}

/// Bounded label for an operation the store IO loop executes, or `None` for effects it
/// never receives.
fn store_operation_label(effect: &MaintainerEffect) -> Option<&'static str> {
	Some(match effect {
		MaintainerEffect::CheckpointRequest { .. } => "checkpoint",
		MaintainerEffect::MaintainerOpRequest { .. } => "maintainer_op",
		MaintainerEffect::CloseCold => "close_cold",
		MaintainerEffect::DeleteRetiredRequest { .. } => "delete_retired",
		MaintainerEffect::FilesystemApplyRequest { .. } => "filesystem_apply",
		MaintainerEffect::FilesystemFlushRequest { .. } => "filesystem_flush",
		MaintainerEffect::FilesystemLoadRequest { .. } => "filesystem_load",
		MaintainerEffect::ForkCleanupRequest { .. } => "fork_cleanup",
		MaintainerEffect::ForkReferenceRequest { .. } => "fork_reference",
		MaintainerEffect::ForkStoreRequest { .. } => "fork_store",
		MaintainerEffect::MaterializeRequest { .. } => "materialize",
		MaintainerEffect::OpenColdRequest { .. } => "open_cold",
		MaintainerEffect::PendingWorkRequest { .. } => "pending_work",
		MaintainerEffect::PrepareSplitRequest { .. } => "prepare_split",
		MaintainerEffect::SweepDeletedRequest { .. } => "sweep_deleted",
		MaintainerEffect::SweepPendingRequest { .. } => "sweep_pending",
		MaintainerEffect::WriteToColdRequest { .. } => "write_to_cold",
		MaintainerEffect::StopReadinessResponse { .. }
		| MaintainerEffect::FilesystemQueryRequest { .. }
		| MaintainerEffect::CancelFilesystemQueryRequest { .. }
		| MaintainerEffect::ReadRowRequest { .. }
		| MaintainerEffect::ReadOpenKeysRequest { .. }
		| MaintainerEffect::Repair(_)
		| MaintainerEffect::ExecuteKvCommand { .. }
		| MaintainerEffect::KvCommandResponse { .. }
		| MaintainerEffect::RecordVersionRefused { .. }
		| MaintainerEffect::RecordUnloadDuration { .. }
		| MaintainerEffect::ReportAlarmIndexReady { .. }
		| MaintainerEffect::ReportAlarmSplitRequest { .. }
		| MaintainerEffect::ReportSplitRequest { .. }
		| MaintainerEffect::WatchHolderRequest { .. }
		| MaintainerEffect::UnwatchHolder { .. }
		| MaintainerEffect::ReportRetiredRequest { .. }
		| MaintainerEffect::ReportParentLogImportedRequest { .. }
		| MaintainerEffect::CancelParentLogReportRequest { .. }
		| MaintainerEffect::ReportCatchUpRequest { .. }
		| MaintainerEffect::CancelCatchUpReportRequest { .. }
		| MaintainerEffect::CancelRetiredReportRequest { .. }
		| MaintainerEffect::ReportVersionMarksRequest { .. }
		| MaintainerEffect::CancelVersionMarksReportRequest { .. }
		| MaintainerEffect::CancelSplitReportRequest { .. }
		| MaintainerEffect::RecoverVolumeRequest { .. }
		| MaintainerEffect::CancelRecoveryRequest { .. }
		| MaintainerEffect::ReplicaRequest { .. }
		| MaintainerEffect::AlarmRequest { .. }
		| MaintainerEffect::CancelAlarmRequest { .. }
		| MaintainerEffect::Reply { .. }
		| MaintainerEffect::CancelReplicaRequest { .. }
		| MaintainerEffect::ForkPeerRequest { .. }
		| MaintainerEffect::CancelForkPeerRequest { .. }
		| MaintainerEffect::RefreshCommandReplicaSets
		| MaintainerEffect::CloseComplete => return None,
	})
}

/// Bounded label for a replica request.
pub(crate) fn replica_request_label(request: &StorageWorkerRequest) -> &'static str {
	depot_types::storage_worker::request_label(request)
}

/// Starts `task` in effect order and spawns its remainder in `tasks`, or returns `None` when it
/// finished at once.
///
/// Spawned tasks start in no particular order: a multi-threaded runtime runs the most recently
/// spawned task first. The first poll therefore runs here, while the executor processes effects
/// in the order the state machine emitted them, so the transport admits each request to its
/// worker's ordered lane in that order. The spawned task continues from where that poll stopped.
#[track_caller]
fn start_in_effect_order<F>(tasks: &TaskScope, task: F) -> Option<TaskHandle>
where
	F: Future<Output = ()> + Send + 'static,
{
	let mut task = Box::pin(task);
	let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
	if task.as_mut().poll(&mut cx).is_ready() {
		return None;
	}
	Some(tasks.spawn(task))
}

/// Bounded label for the maintainer's reply to a volume open.
fn open_reply_label(response: &ClientResponse) -> &'static str {
	match response {
		ClientResponse::VolumeRejectResponse(reason) => match reason {
			VolumeRejectResponse::NotMaintainer(_) => "not_maintainer",
			VolumeRejectResponse::TabletSplitting => "tablet_splitting",
			VolumeRejectResponse::VolumeNotFound => "volume_not_found",
			VolumeRejectResponse::VolumeExists => "volume_exists",
			VolumeRejectResponse::VolumeDeleted => "volume_deleted",
			VolumeRejectResponse::Overloaded(_) => "overloaded",
			VolumeRejectResponse::VolumeRecordMissing(_) => "record_missing",
			VolumeRejectResponse::VolumeUnderRepair(_) => "under_repair",
		},
		ClientResponse::VolumeOpenResponse(_) | ClientResponse::VolumeCreateResponse(_) => "ok",
		ClientResponse::FsReadResponse(_)
		| ClientResponse::VolumeCheckpointResponse(_)
		| ClientResponse::FsForkReferenceResponse(_)
		| ClientResponse::FsForkResponse(_)
		| ClientResponse::VolumeDeleteResponse
		| ClientResponse::VolumeCloseResponse
		| ClientResponse::ReplicaStatusNoticeResponse(_)
		| ClientResponse::OperatorTabletReport(_)
		| ClientResponse::OperatorResponse(_)
		| ClientResponse::RepairInspection(_)
		| ClientResponse::RepairResponse(_)
		| ClientResponse::RepairDiagnosticsPage(_)
		| ClientResponse::RepairAuditPage(_) => "unexpected",
	}
}



impl ReplicaTransport for depot_volume::ups::UpsLogClient {
	fn request(
		&self,
		worker: WorkerId,
		request: rivet_data::generated::depot_storage_worker_protocol_v1::StorageWorkerRequest,
	) -> ReplicaFuture<'_> {
		Box::pin(async move {
			self.request(&worker, request)
				.await
				.map_err(|error| error.to_string())
		})
	}
}

impl Maintainer {
	/// Installs grant-owned runtime capabilities before command admission begins.
	pub fn install_commands(&self, factory: crate::command::volume_host::Factory) -> Result<()> {
		let authority = self.authority.clone();
		let grant = self.grant.clone();
		let alarm_owner = rivet_data::generated::shared_depot_alarms_v1::AlarmOwner {
			worker_id: *factory.pubsub.worker_id().as_bytes(),
			tablet_id: grant.tablet_id,
			tablet_grant_epoch: grant.tablet_grant_epoch,
		};
		let host = crate::command::volume_host::VolumeHost::new(
			factory,
			self.snapshot.clone(),
			Arc::new(move || authority.may_serve(&grant)),
		)?;
		host.set_alarm_owner(alarm_owner);
		anyhow::ensure!(self.kv_host.set(host).is_ok(), "KV host already installed");
		Ok(())
	}

	/// Returns whether this maintainer hosts commands, and so ran the sweep.
	pub fn sweep_commands(&self) {
		if let Some(host) = self.kv_host.get() {
			host.sweep();
		}
	}

	/// Closes the command host's idle volumes now; see `VolumeHost::close_idle_now`.
	pub fn close_idle_commands(&self) -> usize {
		self.kv_host.get().map_or(0, |host| host.close_idle_now())
	}

	/// When the command host next has due idle work, if it has any.
	pub fn next_command_sweep_at(&self) -> Option<u64> {
		self.kv_host.get().and_then(|host| host.next_sweep_at())
	}

	pub fn revoke_commands(&self) {
		for abort in self.alarm_tasks.lock().values() {
			abort.abort();
		}
		if let Some(host) = self.kv_host.get() {
			host.revoke();
		}
		self.kv_responses.lock().clear();
	}

	/// Transfers the request to lifecycle admission without awaiting application execution.
	/// `request_bytes` is the command's received wire size.
	pub fn command(
		&self,
		request: rivet_data::generated::depot_kv_command_v1::KvCommandRequest,
		request_bytes: usize,
		deadline_boot_ms: u64,
	) -> Result<
		oneshot::Receiver<rivet_data::generated::depot_kv_command_v1::KvCommandResponse>,
		rivet_data::generated::depot_kv_command_v1::KvCommandResponse,
	> {
		use rivet_data::generated::depot_kv_command_v1::KvCommandResponse;
		if !self.authority.may_serve(&self.grant) {
			return Err(KvCommandResponse::NotMaintainer);
		}
		let id = RequestId(
			self.next_request
				.try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
				.map_err(|_| crate::kv_refusals::unavailable(UnavailableReason::Internal))?,
		);
		let (tx, rx) = oneshot::channel();
		self.kv_responses.lock().insert(id, tx);
		if self.inbox.as_ref().is_none_or(|inbox| {
			inbox
				.try_send(MaintainerEvent::KvCommandRequest {
					deadline_boot_ms,
					request_id: id,
					request,
					request_bytes: request_bytes as u64,
				})
				.is_err()
		}) {
			self.kv_responses.lock().remove(&id);
			crate::metrics::record_maintainer_inbox_full("kv_command");
			return Err(crate::kv_refusals::overloaded(
				OverloadReason::MaintainerInbox,
			));
		}
		Ok(rx)
	}
}



impl Maintainer {
	/// A check waits for a fresh bounded poll round; transport cancellation does not restart it.
	pub fn stop_readiness(
		&self,
		request: rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness,
	) -> Result<
		oneshot::Receiver<
			rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
		>,
		rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
	> {
		self.submit_readiness(|request_id| MaintainerEvent::StopReadinessRequest {
			request_id,
			request,
		})
	}

	/// Ordinary handoff retains open client claims and only quiesces fresh creates.
	pub fn handoff_readiness(
		&self,
		request: rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness,
		stop_intent: bool,
	) -> Result<
		oneshot::Receiver<
			rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
		>,
		rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
	> {
		self.submit_readiness(|request_id| MaintainerEvent::HandoffReadinessRequest {
			request_id,
			request,
			stop_intent,
		})
	}

	fn submit_readiness(
		&self,
		event: impl FnOnce(RequestId) -> MaintainerEvent,
	) -> Result<
		oneshot::Receiver<
			rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
		>,
		rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
	> {
		use rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse as Response;
		if !self.authority.may_serve(&self.grant) {
			return Err(Response::NotServing);
		}
		let mut pending = self.stop_responses.lock();
		pending.retain(|_, reply| !reply.is_closed());
		if pending.len() >= 4 {
			return Err(super::state::stop::overloaded());
		}
		let id = RequestId(
			self.next_request
				.try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
				.map_err(|_| super::state::stop::overloaded())?,
		);
		let (send, receive) = oneshot::channel();
		pending.insert(id, send);
		if self
			.inbox
			.as_ref()
			.is_none_or(|inbox| inbox.try_send(event(id)).is_err())
		{
			pending.remove(&id);
			crate::metrics::record_maintainer_inbox_full("stop_readiness");
			return Err(super::state::stop::overloaded());
		}
		Ok(receive)
	}
}




