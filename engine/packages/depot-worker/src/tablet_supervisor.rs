//! One storage worker's plain supervision loop. Revocation precedes joining and replacement.
//! Revoked maintainers close in an owned set that the caller polls through `join_closed`,
//! so a slow SlateDB close never blocks admission or reply delivery for other tablets.

use crate::{
	tablet_coordinator_machine::{
		state::validate,
		store,
		types::{ConditionalRead, ReadResult},
	},
	tablet_maintainer_machine::{
		executor::cold_store::ColdStoreConfig,
		runner::{
			InboxDelivery, Maintainer, MaintainerAuthority, MaintainerDependencies,
			ReplicaTransport, SplitReporter, UpdateSlot,
		},
		state::MaintainerPhase,
		types::MaintainerConfig,
	},
};
use anyhow::{Result, ensure};
use depot_log::log_shard_machine::{
	runner::Shard,
	types::{TabletManifestEntry, TabletManifestView},
};
use depot_types::keys::s3::TabletVolumePrefix;
use futures_util::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use object_store::ObjectStore;
use parking_lot::RwLock;
use rivet_data::generated::{
	depot_client_protocol_v1::{
		ClientRequest, ClientResponse, OperatorCachesFailed, OperatorCachesOutcome,
		OperatorDroppedCaches, OperatorResponse, OperatorTabletOutcome, OperatorTabletRejected,
		VolumeRejectResponse,
	},
	depot_storage_worker_protocol_v1::TabletGrant,
	depot_tablet_manifest_v1::{Tablet, TabletManifest},
};
use rivet_env::{Env, capabilities::slatedb::SlateDbFactory};
use rivet_keys::keys::codec::KeyAddress;
use rivet_topology::{
	Topology,
	heartbeat::HeartbeatState,
	topology_machine::{snapshot::TopologySnapshot, types::LeaseStatus},
	worker::WorkerId,
};
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
	time::Duration,
};
use tokio::sync::{oneshot, watch};

/// A local worker's exact, live topology registration. Checked at every admission and reply.
pub trait WorkerLease: Send + Sync + 'static {
	fn permits(&self, worker: WorkerId) -> bool;

	/// Whether the lease also admits new ownership: starting a newly granted tablet,
	/// requesting fresh grants and opening or creating volumes. False once the confirmed
	/// renewal is half a serving window old, while `permits` still holds; existing work keeps
	/// being served and only new ownership is refused, with a retryable refusal.
	fn admits_new_ownership(&self, worker: WorkerId) -> bool;

	/// The boot time at which a currently live lease stops permitting service, so the
	/// supervisor can revoke its maintainers then. `None` while the lease is not live, and
	/// `u64::MAX` for a lease that never expires.
	fn deadline_boot_ms(&self) -> Option<u64>;

	/// Subscribes to changes of what the methods above answer: the lease's status, deadline,
	/// admission of new ownership and heartbeat state. The supervisor subscribes before it
	/// first reads the lease, so a change after any read wakes it; the passing of a deadline
	/// is not a change, since the supervisor arms a timer for it.
	fn changes(&self) -> Result<Box<dyn LeaseChanges>>;
}

/// A subscription to one lease's changes; see `WorkerLease::changes`.
pub trait LeaseChanges: Send + 'static {
	/// Resolves `true` at the first lease change after the subscription was taken, then at
	/// the first change after each previous resolution, and `false` once the lease can never
	/// change again.
	fn changed(&mut self) -> BoxFuture<'_, bool>;
}

/// A `watch` channel whose every send is a lease change.
impl<T: Send + Sync + 'static> LeaseChanges for watch::Receiver<T> {
	fn changed(&mut self) -> BoxFuture<'_, bool> {
		Box::pin(async move { watch::Receiver::changed(self).await.is_ok() })
	}
}

/// The parts of a topology snapshot that decide what `WorkerLease` answers for this process.
/// The registration identity is fixed for the process's lifetime, so it is not part of it.
#[derive(Clone, PartialEq, Eq)]
struct LeaseView {
	status: LeaseStatus,
	heartbeat_state: Option<HeartbeatState>,
}

impl LeaseView {
	fn of(snapshot: &TopologySnapshot) -> Self {
		Self {
			status: snapshot.status.clone(),
			heartbeat_state: snapshot.heartbeat_state,
		}
	}
}

/// Topology publishes one snapshot for every change of its state, including endpoint lookups,
/// so this resolves only when the lease view itself differs from the last one seen. `seen`
/// is the view at the last wake, not at the last read, which loses nothing because a lease
/// view never returns to an earlier value: live deadlines only advance, and the fenced,
/// releasing and stopped statuses are terminal.
struct TopologyLeaseChanges {
	snapshots: watch::Receiver<TopologySnapshot>,
	seen: LeaseView,
}

impl LeaseChanges for TopologyLeaseChanges {
	fn changed(&mut self) -> BoxFuture<'_, bool> {
		Box::pin(async move {
			let seen = self.seen.clone();
			let changed = self
				.snapshots
				.wait_for(|snapshot| LeaseView::of(snapshot) != seen)
				.await;
			let view = match changed {
				Ok(snapshot) => LeaseView::of(&snapshot),
				Err(_) => return false,
			};
			self.seen = view;
			true
		})
	}
}

impl WorkerLease for Topology {
	fn deadline_boot_ms(&self) -> Option<u64> {
		let snapshot = self.snapshot().ok()?;
		match snapshot.status {
			LeaseStatus::Live {
				deadline_boot_ms, ..
			} => Some(deadline_boot_ms),
			LeaseStatus::Observer
			| LeaseStatus::Registering
			| LeaseStatus::Fenced { .. }
			| LeaseStatus::Releasing
			| LeaseStatus::Stopped => None,
		}
	}

	fn permits(&self, worker: WorkerId) -> bool {
		self.permits_worker(&worker).unwrap_or(false)
	}

	fn admits_new_ownership(&self, worker: WorkerId) -> bool {
		self.permits_worker(&worker).unwrap_or(false) && self.may_take_ownership().unwrap_or(false)
	}

	fn changes(&self) -> Result<Box<dyn LeaseChanges>> {
		let snapshots = Topology::subscribe(self)?;
		let seen = LeaseView::of(&snapshots.borrow());
		Ok(Box::new(TopologyLeaseChanges { snapshots, seen }))
	}
}

/// A holder is gone once topology's watch of its process resolves: its worker's lease expired or
/// was released, or another process of that worker claimed it. Topology keeps one watch per
/// process for every tablet on this worker.
impl crate::tablet_maintainer_machine::runner::HolderWatches for Topology {
	fn watch_holder<'a>(
		&'a self,
		holder: &'a rivet_data::generated::shared_depot_types_v1::VolumeHolder,
	) -> futures_util::future::BoxFuture<
		'a,
		anyhow::Result<futures_util::future::BoxFuture<'static, ()>>,
	> {
		Box::pin(async move {
			let worker = WorkerId::from_bytes(holder.worker_id);
			let instance =
				rivet_topology::worker::WorkerInstanceId::from_bytes(holder.worker_incarnation);
			let handle = self.watch(&worker, instance).await?;
			let gone: futures_util::future::BoxFuture<'static, ()> =
				Box::pin(async move { handle.gone().await });
			Ok(gone)
		})
	}
}

/// The runtime capabilities shared by maintainers; policy and state remain inside each machine.
pub struct SupervisorDependencies {
	pub fork_peer: Option<Arc<dyn crate::tablet_maintainer_machine::runner::ForkPeer>>,
	pub base_reader: Option<Arc<dyn crate::filesystem::base::BaseChunkReader>>,
	/// The local shard receives the same manifest as this worker’s maintainers.
	pub log_shard: Arc<Shard>,
	pub env: Arc<dyn Env>,
	pub lease: Arc<dyn WorkerLease>,
	/// Selects the versions each new grant writes its tablet's data at; the worker's
	/// topology in production.
	pub versions: Arc<dyn rivet_topology::versions::WriteVersions>,
	pub cold_store: Arc<dyn SlateDbFactory>,
	pub replicas: Arc<dyn ReplicaTransport>,
	pub split_reporter: Arc<dyn SplitReporter>,
	/// Tells maintainers when a volume's holder is gone; see `HolderWatches`.
	pub holder_watches: Arc<dyn crate::tablet_maintainer_machine::runner::HolderWatches>,
	pub cold_config: ColdStoreConfig,
	pub maintainer_config: MaintainerConfig,
}

struct GrantAuthority {
	worker: WorkerId,
	lease: Arc<dyn WorkerLease>,
	manifest: Arc<RwLock<Option<TabletManifest>>>,
	/// A stopped instance cannot be authorized by an unchanged cached manifest.
	active: std::sync::atomic::AtomicBool,
}

impl MaintainerAuthority for GrantAuthority {
	fn may_serve(&self, grant: &TabletGrant) -> bool {
		self.active.load(std::sync::atomic::Ordering::Acquire)
			&& self.lease.permits(self.worker.clone())
			&& self.manifest.read().as_ref().is_some_and(|manifest| {
				manifest.tablets.iter().any(|tablet| {
					tablet.id == grant.tablet_id
						&& tablet.tablet_grant_epoch == grant.tablet_grant_epoch
						&& tablet.maintainer == Some(*self.worker.as_bytes())
				})
			})
	}
}

struct RunningTablet {
	/// The assignment this maintainer last accepted. It trails the manifest while an
	/// `AssignmentUpdated` waits for inbox space, and the next reconcile offers it again.
	tablet: Tablet,
	/// Last coherent global version queued with this exact assignment.
	manifest_version: u64,
	maintainer: Maintainer,
	authority: Arc<GrantAuthority>,
	/// Set when the manifest's retired parents must be offered to this maintainer.
	/// Cleared only once the update is queued in the maintainer's inbox.
	retired_pending: bool,
	/// An inbox slot reserved after an update found the inbox full, held until the next
	/// reconcile offers the update with it.
	update_slot: Option<UpdateSlot>,
	/// A reservation for `update_slot` is in `TabletSupervisor::inbox_slots`. No update is
	/// offered meanwhile; the slot's arrival wakes the supervisor to offer it.
	reserving_slot: bool,
	/// Boot time this instance started, for the move timing logs.
	started_boot_ms: u64,
	/// Whether the first request routed to this instance was logged.
	first_request_logged: std::sync::atomic::AtomicBool,
}

/// How a pending update is offered to a maintainer's inbox.
enum UpdateOffer {
	/// With the slot reserved for it, which never finds the inbox full.
	Reserved(UpdateSlot),
	/// By a plain try, which may find the inbox full.
	Unreserved,
	/// Not at all until the reservation in flight resolves.
	Reserving,
}

impl RunningTablet {
	/// How the next pending update is offered. Takes the reserved slot, so the caller must
	/// offer an update with it.
	fn update_offer(&mut self) -> UpdateOffer {
		if let Some(slot) = self.update_slot.take() {
			return UpdateOffer::Reserved(slot);
		}
		if self.reserving_slot {
			return UpdateOffer::Reserving;
		}
		UpdateOffer::Unreserved
	}

	/// Logs the first request this instance receives, once, so a move's timeline shows when
	/// clients found the new maintainer and in which phase it was.
	fn observe_request(&self, env: &Arc<dyn Env>, kind: &'static str) {
		// A load first keeps the common case free of a shared write.
		if self
			.first_request_logged
			.load(std::sync::atomic::Ordering::Relaxed)
			|| self
				.first_request_logged
				.swap(true, std::sync::atomic::Ordering::Relaxed)
		{
			return;
		}
		let phase = self.maintainer.snapshot.borrow().phase.clone();
		let Ok(now) = env.clock().observe() else {
			return;
		};
		tracing::info!(
			tablet_id = %uuid::Uuid::from_bytes(self.tablet.id),
			grant_epoch = self.tablet.tablet_grant_epoch,
			kind,
			?phase,
			since_start_ms = now.boot_ms.saturating_sub(self.started_boot_ms),
			since_grant_ms = now.unix_ms.saturating_sub(self.tablet.granted_at),
			"tablet maintainer received its first request"
		);
	}
}

/// A request in flight for fresh grants. The service loop polls it without waiting, so a
/// slow coordinator reply never stalls reconciliation of other tablets.
struct PendingRecovery {
	grants: Vec<TabletGrant>,
	future: BoxFuture<'static, ()>,
}

/// The outcome of one revoked maintainer's shutdown.
struct ClosedTablet {
	tablet_id: [u8; 16],
	tablet_grant_epoch: u64,
	result: Result<()>,
	close_ms: u64,
}

pub struct TabletSupervisor {
	alarm_manifest:
		Arc<RwLock<Option<rivet_data::generated::depot_tablet_manifest_v1::AlarmTabletManifest>>>,
	alarms: Option<crate::alarms::AlarmDriver>,
	manifest_refresh: Option<watch::Sender<()>>,
	commands: Option<crate::command::volume_host::Factory>,
	worker: WorkerId,
	deps: SupervisorDependencies,
	manifest: Arc<RwLock<Option<TabletManifest>>>,
	running: BTreeMap<[u8; 16], RunningTablet>,
	/// Grants this process never opens a tablet store at. The tablet store accepts each grant's
	/// leadership epoch once, and claim epochs are unique only per grant lifetime, so each of
	/// these tablets needs a fresh grant first: every grant this worker held in the first
	/// manifest this supervisor reconciled, which an earlier process of this worker may have
	/// opened, and every grant this process already started a maintainer at.
	spent: BTreeSet<([u8; 16], u64)>,
	/// The first manifest was reconciled and its held grants are in `spent`.
	inherited_spent: bool,
	/// The highest manifest version reconciled, which `close` does not reset.
	reconciled_version: u64,
	/// Spent grants a fresh-grant request already named, with the boot time from which a
	/// request may name them again. A request's reply only triggers the manifest refresh that
	/// carries the new grant, so a grant is not named again before that has had time to arrive.
	recovery_requested: BTreeMap<([u8; 16], u64), u64>,
	/// Spent grants whose maintainer reported a terminal recovery failure.
	terminal_recovery: BTreeSet<([u8; 16], u64)>,
	/// Revoked maintainers still joining their shutdown. Each future owns its maintainer, so
	/// dropping the supervisor hands the remaining cleanup to `Maintainer::drop`.
	closing: FuturesUnordered<BoxFuture<'static, ClosedTablet>>,
	/// Tablets with a predecessor in `closing`. No replacement starts until it has closed,
	/// because two local owners of one tablet would contend for the same SlateDB.
	closing_tablets: BTreeSet<[u8; 16]>,
	recovery: Option<PendingRecovery>,
	/// One future per started maintainer that resolves once its core reaches `Fenced` or
	/// `Stopped`, or its snapshot closes, so revocation does not wait for a poll.
	terminals: FuturesUnordered<BoxFuture<'static, ()>>,
	/// One reservation per maintainer whose inbox refused an update, resolving with the slot
	/// the next reconcile offers it with; see `RunningTablet::reserving_slot`.
	inbox_slots: FuturesUnordered<BoxFuture<'static, (TabletGrant, Option<UpdateSlot>)>>,
	/// Wakes `wait_for_work` when the worker's lease changes. Taken before the lease is first
	/// read, so no change is missed.
	lease_changes: LeaseSubscription,
}

enum LeaseSubscription {
	/// The lease has not been read yet.
	Unsubscribed,
	Open(Box<dyn LeaseChanges>),
	/// The subscription closed, which `wait_for_work` reported once as a final change: topology's
	/// runner may stop without publishing a terminal status, and the lease then permits nothing.
	Closed,
}

/// What `wait_for_work` observed beside a close or a deadline.
enum SupervisorSignal {
	/// The lease changed, or `open` is false and its subscription closed.
	Lease { open: bool },
	/// A reserved inbox slot arrived, or `None` because its maintainer stopped.
	Slot(TabletGrant, Option<UpdateSlot>),
	/// A maintainer fenced or stopped, a recovery request finished when `recovered`, or a
	/// command host moved its idle queue head.
	Maintainer { recovered: bool },
}

/// An operator request for this worker's tablets; see `TabletSupervisor::operator`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorRequest {
	Tier {
		tablet: Option<[u8; 16]>,
	},
	DropCaches {
		tablet: Option<[u8; 16]>,
		quiet_ms: u64,
		limit: u32,
	},
}

/// One maintainer's reply to an operator request, as reported to the operator.
fn operator_outcome(tablet_id: [u8; 16], response: ClientResponse) -> OperatorTabletOutcome {
	let reason = match response {
		ClientResponse::OperatorTabletReport(report) => {
			return OperatorTabletOutcome::OperatorTabletReport(report);
		}
		ClientResponse::VolumeRejectResponse(reason) => reason,
		// A maintainer answers an operator request only with a report or a rejection.
		ClientResponse::FsReadResponse(_)
		| ClientResponse::VolumeCheckpointResponse(_)
		| ClientResponse::FsForkReferenceResponse(_)
		| ClientResponse::FsForkResponse(_)
		| ClientResponse::VolumeCreateResponse(_)
		| ClientResponse::VolumeDeleteResponse
		| ClientResponse::VolumeOpenResponse(_)
		| ClientResponse::VolumeCloseResponse
		| ClientResponse::ReplicaStatusNoticeResponse(_)
		| ClientResponse::OperatorResponse(_)
		| ClientResponse::RepairInspection(_)
		| ClientResponse::RepairResponse(_)
		| ClientResponse::RepairDiagnosticsPage(_)
		| ClientResponse::RepairAuditPage(_) => depot_types::client::not_maintainer(),
	};
	OperatorTabletOutcome::OperatorTabletRejected(OperatorTabletRejected { tablet_id, reason })
}

/// Why `TabletSupervisor::wait_for_work` returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupervisorWake {
	/// A lease, recovery or command idle deadline is due.
	Deadline,
	/// A revoked maintainer finished closing.
	Join,
	/// The worker's lease changed: a confirmed renewal, which also ends a degraded phase, or
	/// a fence, release or stop. Reported once more when the lease subscription closes.
	Lease,
	/// A maintainer's inbox has room for an update it refused.
	Inbox,
	/// A maintainer fenced or stopped, a recovery request finished, or a command host
	/// moved its idle queue head.
	Maintainer,
}

impl SupervisorWake {
	pub fn as_str(self) -> &'static str {
		match self {
			SupervisorWake::Deadline => "deadline",
			SupervisorWake::Join => "join",
			SupervisorWake::Lease => "lease",
			SupervisorWake::Inbox => "inbox",
			SupervisorWake::Maintainer => "maintainer",
		}
	}
}

/// When a live lease stops permitting service, so the supervisor revokes its maintainers
/// then. A deadline that already passed arms nothing: topology may keep publishing the expired
/// snapshot while a renewal is uncertain, and its next lease change wakes the supervisor.
fn lease_wake_deadline_boot_ms(deadline: Option<u64>, now: u64) -> Option<u64> {
	deadline.filter(|deadline| *deadline > now)
}

/// How long a spent grant waits before a fresh-grant request names it again.
const RECOVERY_RETRY_MS: u64 = 1000;

/// Whether `request_fresh_grants` may ask for a fresh grant for `tablet`: this worker still
/// owns it at a spent grant, and no maintainer of it runs or is still closing here.
fn recoverable(
	tablet: &Tablet,
	worker: &WorkerId,
	spent: &BTreeSet<([u8; 16], u64)>,
	terminal_recovery: &BTreeSet<([u8; 16], u64)>,
	closing_tablets: &BTreeSet<[u8; 16]>,
	running: &dyn Fn(&[u8; 16]) -> bool,
) -> bool {
	tablet.maintainer == Some(*worker.as_bytes())
		&& !closing_tablets.contains(&tablet.id)
		&& !running(&tablet.id)
		&& spent.contains(&(tablet.id, tablet.tablet_grant_epoch))
		&& !terminal_recovery.contains(&(tablet.id, tablet.tablet_grant_epoch))
}

/// A fresh epoch cannot make a terminal opening failure recover on the same worker.
fn stop_terminal_recovery(
	terminal_recovery: &mut BTreeSet<([u8; 16], u64)>,
	grant: ([u8; 16], u64),
	failure: &crate::tablet_maintainer_machine::state::RecoveryFailure,
) {
	terminal_recovery.insert(grant);
	tracing::error!(
		tablet_id = ?grant.0,
		grant_epoch = grant.1,
		?failure,
		"tablet recovery requires a placement or capacity change; automatic regrant stopped"
	);
}

/// Reserves an inbox slot for `running`'s pending update, so its arrival wakes the supervisor
/// to offer the update again; see `RunningTablet::reserving_slot`.
fn reserve_update_slot(
	inbox_slots: &mut FuturesUnordered<BoxFuture<'static, (TabletGrant, Option<UpdateSlot>)>>,
	running: &mut RunningTablet,
) {
	running.reserving_slot = true;
	let grant = TabletGrant {
		tablet_id: running.tablet.id,
		tablet_grant_epoch: running.tablet.tablet_grant_epoch,
	};
	let reserve = running.maintainer.reserve_update();
	inbox_slots.push(async move { (grant, reserve.await) }.boxed());
}

/// When the supervisor next asks for fresh grants, or `None` while no spent grant can be
/// replaced. A tablet still closing has no deadline: its close wakes the loop, and a deadline
/// of now would wake it again at once without changing anything.
fn recovery_deadline_boot_ms(
	manifest: &TabletManifest,
	worker: &WorkerId,
	spent: &BTreeSet<([u8; 16], u64)>,
	terminal_recovery: &BTreeSet<([u8; 16], u64)>,
	closing_tablets: &BTreeSet<[u8; 16]>,
	running: &dyn Fn(&[u8; 16]) -> bool,
	requested: &BTreeMap<([u8; 16], u64), u64>,
	now_boot_ms: u64,
) -> Option<u64> {
	manifest
		.tablets
		.iter()
		.filter(|tablet| {
			recoverable(
				tablet,
				worker,
				spent,
				terminal_recovery,
				closing_tablets,
				running,
			)
		})
		.map(|tablet| {
			requested
				.get(&(tablet.id, tablet.tablet_grant_epoch))
				.copied()
				.unwrap_or(0)
				.max(now_boot_ms)
		})
		.min()
}

/// Unflushed applied bytes at which a tablet runs its pass: three quarters of the L0 SST
/// size SlateDB uploads a memtable at, leaving room for the rows other store work writes.
fn tablet_flush_trigger(cold_config: &ColdStoreConfig) -> u64 {
	((cold_config.l0_sst_size_bytes as u64).saturating_mul(3) / 4).max(1)
}

/// Captures the admitted tablet grant, so a later manifest cannot authorize an old reply.
pub struct VolumeReplyAuthority {
	snapshot: watch::Receiver<crate::tablet_maintainer_machine::MaintainerSnapshot>,
	grant: TabletGrant,
	authority: Arc<GrantAuthority>,
}

impl VolumeReplyAuthority {
	/// A core-detected fence revokes delivery without waiting for the supervisor poll.
	pub fn may_reply(&self) -> bool {
		!matches!(
			self.snapshot.borrow().phase,
			MaintainerPhase::Fenced | MaintainerPhase::Stopped
		) && self.authority.may_serve(&self.grant)
	}
}

/// Enhanced readiness cannot outlive the global CAS precondition that its create barrier protects.
pub(crate) struct HandoffReplyAuthority {
	volume: VolumeReplyAuthority,
	manifest: Arc<RwLock<Option<TabletManifest>>>,
	version: u64,
}
impl HandoffReplyAuthority {
	pub(crate) fn may_reply(&self) -> bool {
		self.volume.may_reply()
			&& self
				.manifest
				.read()
				.as_ref()
				.is_some_and(|manifest| manifest.tablet_manifest_version == self.version)
	}
}

impl TabletSupervisor {
	pub fn new(worker: WorkerId, deps: SupervisorDependencies) -> Self {
		Self {
			manifest_refresh: None,
			commands: None,
			alarms: None,
			worker,
			deps,
			manifest: Arc::new(RwLock::new(None)),
			alarm_manifest: Arc::new(RwLock::new(None)),
			running: BTreeMap::new(),
			spent: BTreeSet::new(),
			inherited_spent: false,
			reconciled_version: 0,
			recovery_requested: BTreeMap::new(),
			terminal_recovery: BTreeSet::new(),
			closing: FuturesUnordered::new(),
			closing_tablets: BTreeSet::new(),
			recovery: None,
			terminals: FuturesUnordered::new(),
			inbox_slots: FuturesUnordered::new(),
			lease_changes: LeaseSubscription::Unsubscribed,
		}
	}

	/// Subscribes to lease changes once, before the first read of the lease, so a change
	/// after any read wakes `wait_for_work`.
	fn subscribe_lease(&mut self) -> Result<()> {
		if matches!(self.lease_changes, LeaseSubscription::Unsubscribed) {
			self.lease_changes = LeaseSubscription::Open(self.deps.lease.changes()?);
		}
		Ok(())
	}

	/// Records a resolved inbox reservation for the maintainer it was taken from. One for a
	/// maintainer revoked since is dropped; its tablet's next lifetime reserves its own.
	fn accept_slot(&mut self, grant: TabletGrant, slot: Option<UpdateSlot>) {
		let Some(running) = self.running.get_mut(&grant.tablet_id) else {
			return;
		};
		if running.tablet.tablet_grant_epoch != grant.tablet_grant_epoch {
			return;
		}
		running.reserving_slot = false;
		running.update_slot = slot;
	}

	pub fn with_alarms(mut self, driver: Arc<dyn crate::alarms::DepotDriver>) -> Result<Self> {
		self.deps.maintainer_config.alarms.validate()?;
		self.alarms = Some(crate::alarms::AlarmDriver::new(
			driver,
			self.deps.maintainer_config.alarms.max_callbacks,
		)?);
		Ok(self)
	}

	/// Routing and readiness misses share the process owner's manifest wake channel.
	pub fn with_manifest_refresh(mut self, refresh: watch::Sender<()>) -> Self {
		self.manifest_refresh = Some(refresh);
		self
	}

	pub fn maintainer(&self, tablet_id: &[u8; 16]) -> Option<&Maintainer> {
		self.running
			.get(tablet_id)
			.map(|running| &running.maintainer)
	}

	/// Routes once against the installed manifest and preserves the caller when queueing work.
	/// The returned guard must be checked again immediately before writing the network reply.
	pub fn volume_request(
		&self,
		source_worker_id: WorkerId,
		request: ClientRequest,
		deadline_boot_ms: u64,
	) -> Result<(VolumeReplyAuthority, oneshot::Receiver<ClientResponse>), VolumeRejectResponse> {
		if let ClientRequest::ReplicaStatusNoticeRequest(request) = request {
			return self.status_notices(request);
		}
		if matches!(
			request,
			ClientRequest::OperatorTierRequest(_) | ClientRequest::OperatorDropCachesRequest(_)
		) {
			// The service loop runs these through `operator`, which spans many tablets.
			return Err(depot_types::client::overloaded());
		}
		let hash = depot_types::partition_key_hash(
			depot_types::client::volume_key(&request).ok_or(depot_types::client::overloaded())?,
		);
		let Some(running) = self
			.running
			.values()
			.find(|running| running.tablet.hash_range.contains(hash))
		else {
			self.routing_miss();
			return Err(depot_types::client::not_maintainer());
		};
		running.observe_request(&self.deps.env, "volume");
		if matches!(
			request,
			ClientRequest::VolumeOpenRequest(_) | ClientRequest::VolumeCreateRequest(_)
		) && !self.deps.lease.admits_new_ownership(self.worker.clone())
		{
			crate::metrics::LEASE_DEGRADED_REFUSALS
				.with_label_values(&["volume_open"])
				.inc();
			return Err(depot_types::client::overloaded());
		}
		let response = match request {
			ClientRequest::FsForkRequest(request) => {
				running.maintainer.fork(request.spec, deadline_boot_ms)
			}
			ClientRequest::FsForkReferenceRequest(request) => {
				running.maintainer.fork_reference(request, deadline_boot_ms)
			}
			ClientRequest::VolumeCheckpointRequest(request) => {
				running.maintainer.checkpoint(request, deadline_boot_ms)
			}
			ClientRequest::FsReadRequest(request) => running
				.maintainer
				.filesystem_read(request, deadline_boot_ms),
			ClientRequest::VolumeCreateRequest(request) => running.maintainer.create_volume(
				source_worker_id,
				request.volume_key,
				request.retention,
				request.client_incarnation,
				Some(request.holder),
				deadline_boot_ms,
			),
			ClientRequest::VolumeOpenRequest(request) => {
				running
					.maintainer
					.open_volume(source_worker_id, request, deadline_boot_ms)
			}
			ClientRequest::VolumeCloseRequest(request) => running
				.maintainer
				.close_volume(request.volume_log_key.into(), request.client),
			ClientRequest::VolumeDeleteRequest(request) => running
				.maintainer
				.delete_volume(request.volume_log_key.into()),
			ClientRequest::RepairInspectRequest(request) => {
				running.maintainer.repair_inspect(request.volume_key)
			}
			ClientRequest::RepairRequest(request) => running.maintainer.repair(request),
			ClientRequest::RepairDiagnosticsRequest(request) => {
				running.maintainer.repair_diagnostics(request)
			}
			ClientRequest::RepairAuditListRequest(request) => {
				running.maintainer.repair_audit_list(request)
			}
			// Notices and operator requests name no single volume and were rejected above.
			ClientRequest::ReplicaStatusNoticeRequest(_)
			| ClientRequest::OperatorTierRequest(_)
			| ClientRequest::OperatorDropCachesRequest(_) => {
				return Err(depot_types::client::overloaded());
			}
		}?;
		Ok((
			VolumeReplyAuthority {
				snapshot: running.maintainer.snapshot.clone(),
				grant: TabletGrant {
					tablet_id: running.tablet.id,
					tablet_grant_epoch: running.tablet.tablet_grant_epoch,
				},
				authority: running.authority.clone(),
			},
			response,
		))
	}

	/// Runs an operator request on the tablets this worker maintains: the named one, or all of
	/// them. Each maintainer reports what it started, and a tablet that cannot serve reports its
	/// rejection. Drop caches then empties the process's SlateDB caches and removes the named
	/// tablets' disk-cached parts. Nothing here waits for the work it starts.
	pub fn operator(&self, request: OperatorRequest) -> BoxFuture<'static, ClientResponse> {
		let tablet = match &request {
			OperatorRequest::Tier { tablet } | OperatorRequest::DropCaches { tablet, .. } => {
				*tablet
			}
		};
		let mut submitted = Vec::new();
		for running in self.running.values() {
			if tablet.is_some_and(|id| id != running.tablet.id) {
				continue;
			}
			let reply = match &request {
				OperatorRequest::Tier { .. } => running.maintainer.tier_now(),
				OperatorRequest::DropCaches {
					quiet_ms, limit, ..
				} => {
					// Closing idle command volumes sends their closes, which begin their
					// unloads, so a repeated request finds them gone.
					let closed = running.maintainer.close_idle_commands();
					if closed > 0 {
						tracing::info!(closed, "operator closed idle command volumes");
					}
					running.maintainer.unload_quiet(*quiet_ms, *limit)
				}
			};
			submitted.push((running.tablet.id, reply));
		}
		// A named tablet this worker does not maintain reports so, rather than an empty list.
		if let Some(id) = tablet
			&& submitted.is_empty()
		{
			submitted.push((id, Err(depot_types::client::not_maintainer())));
		}

		let drop_caches = match &request {
			OperatorRequest::Tier { .. } => None,
			OperatorRequest::DropCaches { tablet, .. } => {
				Some(tablet.map(|id| TabletVolumePrefix::new(uuid::Uuid::from_bytes(id)).pack()))
			}
		};
		let cold_store = self.deps.cold_store.clone();
		async move {
			let mut tablets = Vec::with_capacity(submitted.len());
			for (tablet_id, reply) in submitted {
				let response = match reply {
					Ok(reply) => reply.await.unwrap_or(ClientResponse::VolumeRejectResponse(
						depot_types::client::not_maintainer(),
					)),
					Err(reason) => ClientResponse::VolumeRejectResponse(reason),
				};
				tablets.push(operator_outcome(tablet_id, response));
			}
			let caches = match drop_caches {
				Some(prefix) => Some(match cold_store.drop_caches(prefix).await {
					Ok(dropped) => {
						OperatorCachesOutcome::OperatorDroppedCaches(OperatorDroppedCaches {
							memory_bytes: dropped.memory_bytes,
							disk_files: dropped.disk_files,
							disk_bytes: dropped.disk_bytes,
						})
					}
					Err(error) => {
						OperatorCachesOutcome::OperatorCachesFailed(OperatorCachesFailed {
							message: format!("{error:#}").into(),
						})
					}
				}),
				None => None,
			};
			ClientResponse::OperatorResponse(OperatorResponse { tablets, caches })
		}
		.boxed()
	}

	/// Publish authority first. A slow close never leaves the predecessor admitting work.
	///
	/// Revoked maintainers are handed to the closing set rather than joined here, so this
	/// returns without waiting for any SlateDB close. Callers must keep polling
	/// `join_closed`; a tablet whose predecessor is still closing starts on a later
	/// reconcile. Maintainer updates never wait for inbox space either. A full inbox
	/// leaves the update pending and reserves the slot that the next reconcile offers it with.
	#[tracing::instrument(skip_all)]
	pub async fn reconcile(&mut self, manifest: TabletManifest) -> Result<()> {
		validate(&manifest)?;
		self.subscribe_lease()?;
		// `close` clears the installed manifest, so an older manifest after it would prune
		// `spent` grants that a newer manifest still names.
		if manifest.tablet_manifest_version < self.reconciled_version {
			return Ok(());
		}
		self.reconciled_version = manifest.tablet_manifest_version;
		if let Some(current) = self.manifest.read().as_ref() {
			if manifest.tablet_manifest_version < current.tablet_manifest_version {
				return Ok(());
			}
			ensure!(
				manifest.tablet_manifest_version != current.tablet_manifest_version
					|| manifest == *current,
				"conflicting manifest at the same version"
			);
		}
		let changed = self.manifest.read().as_ref() != Some(&manifest);
		*self.manifest.write() = Some(manifest.clone());
		crate::metrics::publish_placement(&manifest, self.worker.as_bytes());
		let awaiting_import = manifest
			.retired_tablets
			.iter()
			.filter(|retired| {
				!retired
					.children
					.iter()
					.all(|child| retired.log_imported_by.contains(child))
			})
			.count();
		crate::metrics::RETIRED_LOGS_AWAITING_IMPORT.set(awaiting_import as i64);
		// An earlier process of this worker may have opened the store at any grant it held
		// when this one started, so none of them is opened here. Telling which ones it opened
		// would need a clock comparison that skew breaks; a fresh grant for each costs one
		// batched manifest update per process start.
		if !self.inherited_spent {
			self.inherited_spent = true;
			for tablet in &manifest.tablets {
				if tablet.maintainer == Some(*self.worker.as_bytes()) {
					tracing::info!(
						tablet_id = ?tablet.id,
						grant_epoch = tablet.tablet_grant_epoch,
						"requesting a fresh grant for a tablet this worker held when its process started"
					);
					self.spent.insert((tablet.id, tablet.tablet_grant_epoch));
				}
			}
		}
		let current = |(id, epoch): &([u8; 16], u64)| {
			manifest
				.tablets
				.iter()
				.any(|tablet| &tablet.id == id && &tablet.tablet_grant_epoch == epoch)
		};
		self.spent.retain(current);
		self.recovery_requested.retain(|grant, _| current(grant));
		self.terminal_recovery.retain(current);

		// Revoke all obsolete instances before any one tablet's cleanup begins.
		let mut revoked = vec![];
		for (id, running) in &self.running {
			let fenced = matches!(
				running.maintainer.snapshot.borrow().phase,
				MaintainerPhase::Fenced | MaintainerPhase::Stopped
			);
			let keep = !fenced
				&& self.deps.lease.permits(self.worker.clone())
				&& manifest
					.tablets
					.iter()
					.any(|tablet| same_lifetime(tablet, &running.tablet));
			if !keep {
				running
					.authority
					.active
					.store(false, std::sync::atomic::Ordering::Release);
				running.maintainer.revoke_commands();
				revoked.push(*id);
			}
		}
		for id in revoked {
			let running = self.running.remove(&id).expect("selected running tablet");
			// How long after the newer grant this worker learned it lost the tablet.
			let successor = manifest.tablets.iter().find(|tablet| tablet.id == id);
			let since_grant_ms = successor.and_then(|tablet| {
				let now = self.deps.env.clock().observe().ok()?;
				Some(now.unix_ms.saturating_sub(tablet.granted_at))
			});
			// One borrow at a time: a second read guard can wait behind the runner's send.
			let (phase, loaded_volumes, recovery_failure) = {
				let snapshot = running.maintainer.snapshot.borrow();
				(
					snapshot.phase.clone(),
					snapshot.loaded_volumes,
					snapshot.recovery_failure.clone(),
				)
			};
			if let Some(failure) = &recovery_failure {
				stop_terminal_recovery(
					&mut self.terminal_recovery,
					(id, running.tablet.tablet_grant_epoch),
					failure,
				);
			}
			tracing::info!(
				tablet_id = ?id,
				tablet = %uuid::Uuid::from_bytes(id),
				grant_epoch = running.tablet.tablet_grant_epoch,
				next_grant_epoch = successor.map(|tablet| tablet.tablet_grant_epoch),
				next_maintainer = ?successor.and_then(|tablet| tablet.maintainer).map(uuid::Uuid::from_bytes),
				manifest_version = manifest.tablet_manifest_version,
				?phase,
				?since_grant_ms,
				loaded_volumes,
				"stopping tablet maintainer"
			);
			self.begin_close(running);
		}
		self.request_fresh_grants(&manifest)?;

		// Client routing sees the manifest through the local shard, never an object-store read per append.
		if changed {
			self.deps
				.log_shard
				.update_tablet_manifest(TabletManifestView {
					tablet_manifest_version: manifest.tablet_manifest_version,
					retired_tablets: manifest
						.retired_tablets
						.iter()
						.map(|retired| retired.tablet_id)
						.collect(),
					tablets: manifest
						.tablets
						.iter()
						.map(|tablet| {
							(
								tablet.id,
								TabletManifestEntry {
									maintainer: tablet.maintainer.map(WorkerId::from_bytes),
									disks: tablet.replica_set.disks.clone(),
									tablet_id: tablet.id,
									hash_range: tablet.hash_range,
									members: tablet
										.replica_set
										.members
										.iter()
										.map(|id| WorkerId::from_bytes(*id))
										.collect(),
									tablet_replica_set_version: tablet
										.replica_set
										.tablet_replica_set_version,
									phase: tablet.replica_set.phase.clone(),
								},
							)
						})
						.collect(),
				})
				.await?;
		}

		// Each database gets its share of the process cold-store budgets.
		let maintained = manifest
			.tablets
			.iter()
			.filter(|tablet| tablet.maintainer == Some(*self.worker.as_bytes()))
			.count();
		let cold_config = self.deps.cold_config.for_maintained_tablets(maintained);
		crate::metrics::publish_cold_store_capacity(&self.deps.cold_config, maintained);

		// Each start checks the lease again, and a tablet whose predecessor is still
		// closing waits for `join_closed` so two local owners never share one store. A
		// spent grant waits for its fresh grant instead.
		for tablet in &manifest.tablets {
			if tablet.maintainer != Some(*self.worker.as_bytes())
				|| self.running.contains_key(&tablet.id)
				|| self.closing_tablets.contains(&tablet.id)
				|| self.spent.contains(&(tablet.id, tablet.tablet_grant_epoch))
				|| !self.deps.lease.permits(self.worker.clone())
			{
				continue;
			}
			// A late lease renewal defers new tablets until the next confirmed renewal, which
			// wakes the supervisor as a lease change, or until the lease fences.
			if !self.deps.lease.admits_new_ownership(self.worker.clone()) {
				crate::metrics::LEASE_DEGRADED_REFUSALS
					.with_label_values(&["tablet_start"])
					.inc();
				continue;
			}
			// The grant writes at the versions every live storage worker reads now. A
			// refusal means none is safe to write, so this grant does not start, and the
			// next reconcile offers it again; the worker's other tablets keep serving.
			let write_versions =
				match crate::tablet_maintainer_machine::types::TabletWriteVersions::select(
					self.deps.versions.as_ref(),
				) {
					Ok(versions) => versions,
					Err(error) => {
						tracing::warn!(
							?error,
							tablet_id = %uuid::Uuid::from_bytes(tablet.id),
							"no tablet data version is readable by every live storage worker; check for a deploy that skipped a release"
						);
						continue;
					}
				};
			// Spent before the start, so a later stop of this maintainer, for any reason,
			// leads to a fresh grant and never to a second lifetime under this one.
			self.spent.insert((tablet.id, tablet.tablet_grant_epoch));
			let authority = Arc::new(GrantAuthority {
				worker: self.worker.clone(),
				lease: self.deps.lease.clone(),
				manifest: self.manifest.clone(),
				active: std::sync::atomic::AtomicBool::new(true),
			});
			let maintainer = Maintainer::start_with_alarms(
				tablet.clone(),
				MaintainerConfig {
					write_versions,
					local_worker: Some(self.worker.clone()),
					// A pass runs before SlateDB would upload the memtable at its L0 SST
					// size, so the upload records cold positions.
					tablet_flush_above_bytes: tablet_flush_trigger(&cold_config),
					..self.deps.maintainer_config.clone()
				},
				MaintainerDependencies {
					fork_peer: self.deps.fork_peer.clone(),
					base_reader: self.deps.base_reader.clone(),
					env: self.deps.env.clone(),
					cold_store: self.deps.cold_store.clone(),
					replicas: self.deps.replicas.clone(),
					authority: authority.clone(),
					cold_config: cold_config.clone(),
					split_reporter: self.deps.split_reporter.clone(),
					holder_watches: self.deps.holder_watches.clone(),
				},
				self.alarms
					.clone()
					.map(|driver| (driver, self.worker.clone())),
				self.alarm_manifest.read().as_ref().and_then(|m| {
					m.initialization
						.get(uuid::Uuid::from_bytes(tablet.id).to_string().as_str())
						.cloned()
				}),
			)?;
			if let Some(factory) = &self.commands {
				maintainer.install_commands(factory.clone())?;
			}
			let clock = self.deps.env.clock();
			// Diagnostics never fail a start; an unreadable clock logs zeros.
			let started = clock.observe().unwrap_or(rivet_env::Clocks {
				boot_ms: 0,
				unix_ms: 0,
			});
			let mut snapshot = maintainer.snapshot.clone();
			let tablet_id = tablet.id;
			let grant_epoch = tablet.tablet_grant_epoch;
			let granted_at = tablet.granted_at;
			self.terminals.push(
				async move {
					// The open's end is logged once so a move can be timed to its first
					// serving instant; it resolves the future only at a terminal phase.
					let opened = snapshot
						.wait_for(|snapshot| !matches!(snapshot.phase, MaintainerPhase::Opening))
						.await
						.map(|snapshot| snapshot.clone());
					if let (Ok(opened), Ok(now)) = (opened, clock.observe()) {
						tracing::info!(
							tablet_id = %uuid::Uuid::from_bytes(tablet_id),
							grant_epoch,
							phase = ?opened.phase,
							open_ms = now.boot_ms.saturating_sub(started.boot_ms),
							since_grant_ms = now.unix_ms.saturating_sub(granted_at),
							loaded_volumes = opened.loaded_volumes,
							listed_members = opened.listed_members,
							"tablet maintainer finished opening"
						);
					}
					let _ = snapshot
						.wait_for(|snapshot| {
							matches!(
								snapshot.phase,
								MaintainerPhase::Fenced | MaintainerPhase::Stopped
							)
						})
						.await;
				}
				.boxed(),
			);
			tracing::info!(
				tablet_id = ?tablet.id,
				tablet = %uuid::Uuid::from_bytes(tablet.id),
				grant_epoch = tablet.tablet_grant_epoch,
				manifest_version = manifest.tablet_manifest_version,
				since_grant_ms = started.unix_ms.saturating_sub(tablet.granted_at),
				"started tablet maintainer"
			);
			self.running.insert(
				tablet.id,
				RunningTablet {
					tablet: tablet.clone(),
					manifest_version: 0,
					maintainer,
					authority,
					retired_pending: true,
					update_slot: None,
					reserving_slot: false,
					started_boot_ms: started.boot_ms,
					first_request_logged: std::sync::atomic::AtomicBool::new(false),
				},
			);
		}

		// A replica-set phase does not allocate another store epoch. Keep the existing owner
		// and deliver the new set through its ordered event inbox. A maintainer started above
		// receives the manifest it started under here, in the same reconcile.
		for tablet in &manifest.tablets {
			let Some(running) = self.running.get_mut(&tablet.id) else {
				continue;
			};
			if running.tablet == *tablet
				&& running.manifest_version >= manifest.tablet_manifest_version
			{
				continue;
			}
			let slot = match running.update_offer() {
				UpdateOffer::Reserved(slot) => Some(slot),
				UpdateOffer::Unreserved => None,
				UpdateOffer::Reserving => continue,
			};
			match running.maintainer.observe_manifest(
				slot,
				manifest.tablet_manifest_version,
				tablet.clone(),
			)? {
				InboxDelivery::Queued => {
					running.tablet = tablet.clone();
					running.manifest_version = manifest.tablet_manifest_version;
				}
				InboxDelivery::Full => {
					tracing::debug!(tablet_id = ?tablet.id, "maintainer inbox full, reserving a slot for the assignment update");
					reserve_update_slot(&mut self.inbox_slots, running);
				}
			}
		}

		// The first hash follows nested splits and moves without a stored cleanup owner.
		for running in self.running.values_mut() {
			if changed {
				running.retired_pending = true;
			}
			// Retired parents follow any assignment update still waiting for inbox space,
			// preserving the order in which the maintainer observes manifest changes.
			let assignment_pending = manifest
				.tablets
				.iter()
				.any(|tablet| tablet.id == running.tablet.id && *tablet != running.tablet);
			if !running.retired_pending || assignment_pending {
				continue;
			}
			let retired = manifest
				.retired_tablets
				.iter()
				.filter(|retired| {
					if self.alarm_manifest.read().as_ref().is_some_and(|m| {
						m.holds
							.iter()
							.any(|hold| hold.parent_id == retired.tablet_id)
					}) {
						return false;
					}
					running
						.tablet
						.hash_range
						.contains(retired.partition_key_hash_start)
				})
				.cloned()
				.collect();
			// A split child's import of its parent's log is recorded once the parent's retired
			// record lists the child, or once that record is gone.
			let parent_log_recorded = running.tablet.parent_tablet_id.is_none_or(|parent| {
				manifest
					.retired_tablets
					.iter()
					.find(|retired| retired.tablet_id == parent)
					.is_none_or(|retired| retired.log_imported_by.contains(&running.tablet.id))
			});
			let slot = match running.update_offer() {
				UpdateOffer::Reserved(slot) => Some(slot),
				UpdateOffer::Unreserved => None,
				UpdateOffer::Reserving => continue,
			};
			match running
				.maintainer
				.update_retired_tablets(slot, retired, parent_log_recorded)?
			{
				InboxDelivery::Queued => running.retired_pending = false,
				InboxDelivery::Full => {
					tracing::debug!(tablet_id = ?running.tablet.id, "maintainer inbox full, reserving a slot for the retired tablet update");
					reserve_update_slot(&mut self.inbox_slots, running);
				}
			}
		}
		Ok(())
	}

	/// Asks the coordinator, in one request, for a fresh grant for every tablet this worker
	/// owns at a spent grant once no maintainer of it runs or closes here: at process start
	/// that is every tablet the worker held, later each one whose maintainer stopped. Each
	/// grant is conditional on its exact spent epoch, so a lost or replayed request is safe:
	/// the coordinator skips a grant that already advanced. Only a newer manifest authorizes
	/// starting the replacement.
	///
	/// The request runs as an owned future that each service iteration polls without waiting.
	fn request_fresh_grants(&mut self, manifest: &TabletManifest) -> Result<()> {
		let clock = self.deps.env.clock();
		let now = clock.observe()?.boot_ms;
		if !self.deps.lease.permits(self.worker.clone()) {
			self.recovery = None;
			return Ok(());
		}
		// A fresh grant is new ownership; a late lease renewal asks for none until it confirms.
		// A request already in flight still completes.
		if self.recovery.is_none() && !self.deps.lease.admits_new_ownership(self.worker.clone()) {
			crate::metrics::LEASE_DEGRADED_REFUSALS
				.with_label_values(&["fresh_grant"])
				.inc();
			return Ok(());
		}
		let running = |id: &[u8; 16]| self.running.contains_key(id);
		if let Some(recovery) = &mut self.recovery {
			// A request none of whose grants is still waiting is dropped, so a newer spent
			// grant does not wait out its timeout.
			let waiting = recovery.grants.iter().any(|grant| {
				manifest.tablets.iter().any(|tablet| {
					tablet.id == grant.tablet_id
						&& tablet.tablet_grant_epoch == grant.tablet_grant_epoch
						&& recoverable(
							tablet,
							&self.worker,
							&self.spent,
							&self.terminal_recovery,
							&self.closing_tablets,
							&running,
						)
				})
			});
			if waiting && recovery.future.as_mut().now_or_never().is_none() {
				return Ok(());
			}
			self.recovery = None;
		}

		let grants = manifest
			.tablets
			.iter()
			.filter(|tablet| {
				recoverable(
					tablet,
					&self.worker,
					&self.spent,
					&self.terminal_recovery,
					&self.closing_tablets,
					&running,
				) && self
					.recovery_requested
					.get(&(tablet.id, tablet.tablet_grant_epoch))
					.is_none_or(|retry_at| *retry_at <= now)
			})
			.map(|tablet| TabletGrant {
				tablet_id: tablet.id,
				tablet_grant_epoch: tablet.tablet_grant_epoch,
			})
			.collect::<Vec<_>>();
		if grants.is_empty() {
			return Ok(());
		}
		for grant in &grants {
			self.recovery_requested.insert(
				(grant.tablet_id, grant.tablet_grant_epoch),
				now.saturating_add(RECOVERY_RETRY_MS),
			);
		}

		let request = rivet_data::generated::depot_tablet_coordinator_protocol_v1::RecoverTablets {
			maintainer: *self.worker.as_bytes(),
			grants: grants
				.iter()
				.map(|grant| {
					rivet_data::generated::depot_tablet_coordinator_protocol_v1::SpentTabletGrant {
						tablet_id: grant.tablet_id,
						tablet_grant_epoch: grant.tablet_grant_epoch,
					}
				})
				.collect(),
		};
		let reporter = self.deps.split_reporter.clone();
		let mut future = async move {
			let tablets = request.grants.len();
			let result = rivet_env::timeout(
				clock.as_ref(),
				Duration::from_secs(5),
				reporter.recover(request),
			)
			.await;
			tracing::info!(tablets, ?result, "requested fresh tablet grants");
		}
		.boxed();
		if future.as_mut().now_or_never().is_none() {
			self.recovery = Some(PendingRecovery { grants, future });
		}

		Ok(())
	}

	/// Moves an already revoked maintainer into the closing set. Its tablet cannot start
	/// again on this worker until `join_closed` records the outcome.
	fn begin_close(&mut self, running: RunningTablet) {
		let tablet_id = running.tablet.id;
		let tablet_grant_epoch = running.tablet.tablet_grant_epoch;
		let clock = self.deps.env.clock();
		self.closing_tablets.insert(tablet_id);
		// A reserved inbox slot holds a sender to the maintainer's inbox, which its runner
		// waits to see closed before it stops, so the slot drops before the shutdown.
		let RunningTablet {
			maintainer,
			update_slot,
			..
		} = running;
		drop(update_slot);
		self.closing.push(
			async move {
				let began = clock.observe().map(|clocks| clocks.boot_ms).unwrap_or(0);
				let result = maintainer.shutdown().await;
				let ended = clock
					.observe()
					.map(|clocks| clocks.boot_ms)
					.unwrap_or(began);
				ClosedTablet {
					tablet_id,
					tablet_grant_epoch,
					result,
					close_ms: ended.saturating_sub(began),
				}
			}
			.boxed(),
		);
	}

	/// The earliest time anything this supervisor waits for becomes due: the lease deadline,
	/// the next fresh-grant request, or a command host's idle queue head.
	pub fn next_deadline_boot_ms(&self, now_boot_ms: u64) -> Option<u64> {
		let mut deadlines = Vec::new();
		deadlines.extend(lease_wake_deadline_boot_ms(
			self.deps.lease.deadline_boot_ms(),
			now_boot_ms,
		));
		// A degraded lease asks for no fresh grant, so its recovery deadline would be due at
		// once on every pass; the confirmed renewal that ends the degraded phase wakes the
		// supervisor as a lease change instead.
		if self.recovery.is_none()
			&& self.deps.lease.admits_new_ownership(self.worker.clone())
			&& let Some(manifest) = self.manifest.read().as_ref()
		{
			deadlines.extend(recovery_deadline_boot_ms(
				manifest,
				&self.worker,
				&self.spent,
				&self.terminal_recovery,
				&self.closing_tablets,
				&|id| self.running.contains_key(id),
				&self.recovery_requested,
				now_boot_ms,
			));
		}
		deadlines.extend(
			self.running
				.values()
				.filter_map(|running| running.maintainer.next_command_sweep_at()),
		);
		deadlines.into_iter().min()
	}

	/// Waits until the supervisor has work: a revoked maintainer closed, the lease changed, a
	/// reserved inbox slot arrived, a maintainer fenced, a recovery request finished, a
	/// command host's idle head moved, or `next_deadline_boot_ms` passed. Manifest and stop
	/// changes are the caller's to watch.
	pub async fn wait_for_work(&mut self) -> Result<SupervisorWake> {
		self.subscribe_lease()?;
		let clock = self.deps.env.clock();
		let now = clock.observe()?.boot_ms;
		let deadline = self.next_deadline_boot_ms(now);
		let idle_changed = self
			.commands
			.as_ref()
			.map(|factory| factory.idle_changed.clone());
		let closing = &mut self.closing;
		let terminals = &mut self.terminals;
		let recovery = &mut self.recovery;
		let inbox_slots = &mut self.inbox_slots;
		let lease_changes = &mut self.lease_changes;
		let wake = rivet_env::select_biased!("closed maintainers start replacements before other signals",
			closed = async {
				match closing.next().await {
					Some(closed) => closed,
					None => std::future::pending().await,
				}
			} => {
				self.finish_close(closed);
				SupervisorWake::Join
			},
			signal = async {
				rivet_env::select_biased!("lease changes precede inbox slots and maintainer signals",
					open = async {
						match lease_changes {
							LeaseSubscription::Open(changes) => changes.changed().await,
							LeaseSubscription::Unsubscribed | LeaseSubscription::Closed => {
								std::future::pending().await
							}
						}
					} => SupervisorSignal::Lease { open },
					slot = async {
						match inbox_slots.next().await {
							Some(slot) => slot,
							None => std::future::pending().await,
						}
					} => SupervisorSignal::Slot(slot.0, slot.1),
					recovered = async {
						rivet_env::select_biased!("maintainer signals have no relative order",
							() = async {
								match terminals.next().await {
									Some(()) => {}
									None => std::future::pending().await,
								}
							} => false,
							() = async {
								match recovery {
									Some(pending) => pending.future.as_mut().await,
									None => std::future::pending().await,
								}
							} => true,
							() = async {
								match idle_changed {
									Some(notify) => notify.notified().await,
									None => std::future::pending().await,
								}
							} => false,
						)
					} => SupervisorSignal::Maintainer { recovered },
				)
			} => match signal {
				SupervisorSignal::Lease { open } => {
					if !open {
						self.lease_changes = LeaseSubscription::Closed;
					}
					SupervisorWake::Lease
				}
				SupervisorSignal::Slot(grant, slot) => {
					self.accept_slot(grant, slot);
					SupervisorWake::Inbox
				}
				SupervisorSignal::Maintainer { recovered } => {
					if recovered {
						self.recovery = None;
					}
					SupervisorWake::Maintainer
				}
			},
			result = async {
				match deadline.filter(|deadline| *deadline != u64::MAX) {
					Some(deadline) => clock.sleep_until(deadline).await,
					None => std::future::pending().await,
				}
			} => {
				result?;
				SupervisorWake::Deadline
			},
		);
		Ok(wake)
	}

	/// Drives revoked maintainers' shutdowns and returns once one has closed. Pends
	/// forever while nothing is closing, so callers can select on it beside other work.
	/// Shutdowns only make progress while this future is polled.
	pub async fn join_closed(&mut self) {
		match self.closing.next().await {
			Some(closed) => self.finish_close(closed),
			None => std::future::pending().await,
		}
	}

	/// Records a joined shutdown. Its grant was spent when the maintainer started, so the
	/// tablet waits for a fresh grant whether or not the close failed: a replacement can fence
	/// SlateDB before this revoked owner finishes closing, and that local failure must not stop
	/// unrelated tablets or the shared log shard.
	fn finish_close(&mut self, closed: ClosedTablet) {
		self.closing_tablets.remove(&closed.tablet_id);
		tracing::info!(
			tablet_id = %uuid::Uuid::from_bytes(closed.tablet_id),
			grant_epoch = closed.tablet_grant_epoch,
			close_ms = closed.close_ms,
			ok = closed.result.is_ok(),
			"tablet maintainer closed"
		);
		if let Err(error) = closed.result {
			tracing::warn!(tablet_id = ?closed.tablet_id, grant_epoch = closed.tablet_grant_epoch, %error, "revoked tablet maintainer closed with an error");
		}
	}

	/// Polling only triggers cleanup; each maintainer's data path checks the topology lease directly.
	pub async fn run(
		&mut self,
		mut manifests: watch::Receiver<Option<TabletManifest>>,
		mut stop: watch::Receiver<bool>,
	) -> Result<()> {
		let result = async {
			loop {
				if *stop.borrow() {
					return Ok(());
				}
				let manifest = manifests.borrow_and_update().clone();
				if let Some(manifest) = manifest {
					self.reconcile(manifest).await?;
				} else {
					self.close().await?;
				}
				rivet_env::select_biased!("shutdown and manifest changes precede supervisor deadlines",
				 changed = stop.changed() => { if changed.is_err() { return Ok(()); } },
				 changed = manifests.changed() => { if changed.is_err() { return Ok(()); } },
				 wake = self.wait_for_work() => { wake?; },
				);
			}
		}
		.await;
		let closed = self.close().await;
		result.and(closed)
	}

	/// Revokes every local grant before joining any task; no replacement may start until this returns.
	/// Joins maintainers revoked by an earlier reconcile as well, so none outlives the supervisor.
	pub async fn close(&mut self) -> Result<()> {
		self.recovery = None;
		self.inbox_slots.clear();
		*self.manifest.write() = None;
		for running in self.running.values() {
			running
				.authority
				.active
				.store(false, std::sync::atomic::Ordering::Release);
			running.maintainer.revoke_commands();
		}
		while let Some(closed) = self.closing.next().await {
			self.finish_close(closed);
		}
		let mut result = Ok(());
		for (_, running) in std::mem::take(&mut self.running) {
			// The reserved slot drops before the shutdown, as in `begin_close`.
			let RunningTablet {
				maintainer,
				update_slot,
				..
			} = running;
			drop(update_slot);
			if let Err(error) = maintainer.shutdown().await {
				result = Err(error);
			}
		}
		result
	}
}

/// Membership and its diagnostic timestamp may change without replacing a store owner.
/// Changed ownership, range or split preparation requires the existing close/join path.
fn same_lifetime(left: &Tablet, right: &Tablet) -> bool {
	left.id == right.id
		&& left.tablet_grant_epoch == right.tablet_grant_epoch
		&& left.maintainer == right.maintainer
		&& left.hash_range.start == right.hash_range.start
		&& left.hash_range.end == right.hash_range.end
		&& left.parent_tablet_id == right.parent_tablet_id
		&& left.split == right.split
}

impl Drop for TabletSupervisor {
	fn drop(&mut self) {
		*self.manifest.write() = None;
		for running in self.running.values() {
			running
				.authority
				.active
				.store(false, std::sync::atomic::Ordering::Release);
			running.maintainer.revoke_commands();
		}
	}
}

/// Resolves once any refresh source signals. A source whose sender closed is dropped, so
/// it cannot wake the watcher in a loop; with no source left this pends forever. Returns the
/// position of the source that signalled, for diagnostics.
async fn any_refresh(refreshes: &mut Vec<watch::Receiver<()>>) -> usize {
	loop {
		if refreshes.is_empty() {
			return std::future::pending().await;
		}
		let changes = refreshes
			.iter_mut()
			.map(|refresh| Box::pin(refresh.changed()))
			.collect::<Vec<_>>();
		let (result, index, _) = futures_util::future::select_all(changes).await;
		if result.is_ok() {
			return index;
		}
		refreshes.remove(index);
	}
}

/// Publish monotonic, validated manifest views. Read failures retain the last routing view;
/// topology leases and target-side grant fences continue to determine serving authority.
pub async fn watch_manifest(
	env: Arc<dyn Env>,
	store: Arc<dyn ObjectStore>,
	output: watch::Sender<Option<TabletManifest>>,
	stop: watch::Receiver<bool>,
	refreshes: Vec<watch::Receiver<()>>,
	interval: Duration,
) -> Result<()> {
	watch_manifest_with_alarms(env, store, output, stop, refreshes, interval, None).await
}

pub(crate) async fn watch_manifest_with_alarms(
	env: Arc<dyn Env>,
	store: Arc<dyn ObjectStore>,
	output: watch::Sender<Option<TabletManifest>>,
	mut stop: watch::Receiver<bool>,
	mut refreshes: Vec<watch::Receiver<()>>,
	interval: Duration,
	alarms: Option<
		Arc<RwLock<Option<rivet_data::generated::depot_tablet_manifest_v1::AlarmTabletManifest>>>,
	>,
) -> Result<()> {
	ensure!(
		!interval.is_zero(),
		"manifest poll interval must be positive"
	);
	let clock = env.clock();
	// What woke the read, for the log of each newer manifest.
	let mut trigger = "start";
	// Which entry of `refreshes` signalled a refresh read.
	let mut refresh_source = None;
	// The ETag of the last manifest this watcher decoded and validated. Each later read is
	// conditioned on it, so an unchanged manifest costs one GET and no body. A read that
	// fails or does not decode keeps the previous ETag, so the next read is still conditioned
	// on the last validated manifest; the watcher reads unconditionally only until its first
	// validated manifest, or forever when the store returns no ETag.
	let mut etag: Option<String> = None;
	loop {
		if *stop.borrow() {
			return Ok(());
		}
		// Shutdown must not wait for an unresponsive object store. A failed read
		// retains the current view; a waiting client keeps its own original deadline.
		let read = read_manifest(store.as_ref(), etag.as_deref());
		let result = rivet_env::select_biased!("stop before dispatching or finishing a manifest read",
			changed = stop.changed() => { if changed.is_err() || *stop.borrow() { return Ok(()); } continue; },
			result = rivet_env::timeout(clock.as_ref(), Duration::from_secs(5), read) => result,
		);
		if let Ok(ConditionalRead::Read(ReadResult::FoundAlarm {
			envelope,
			etag: read_etag,
		})) = result
		{
			crate::tablet_coordinator_machine::state::alarms::validate(&envelope)?;
			let stored = crate::tablet_coordinator_machine::types::StoredManifest {
				manifest: envelope.manifest.clone(),
				etag: String::new(),
			};
			validate(&stored.manifest)?;
			let previous = output
				.borrow()
				.as_ref()
				.map(|current| current.tablet_manifest_version);
			let newer =
				previous.is_none_or(|version| stored.manifest.tablet_manifest_version > version);
			if newer {
				tracing::info!(
					manifest_version = stored.manifest.tablet_manifest_version,
					?previous,
					trigger,
					?refresh_source,
					"read a newer tablet manifest"
				);
				if let Some(alarms) = &alarms {
					*alarms.write() = Some(envelope);
				}
				output.send_replace(Some(stored.manifest));
			}
			etag = Some(read_etag);
		}
		trigger = rivet_env::select_biased!("stop manifest polling before another read",
		 changed = stop.changed() => { if changed.is_err() { return Ok(()); } refresh_source = None; "stop" },
		 source = any_refresh(&mut refreshes) => { refresh_source = Some(source); "refresh" },
		 _ = rivet_env::sleep(clock.as_ref(), interval) => { refresh_source = None; "poll" },
		);
	}
}

/// One manifest read for the watcher: conditioned on `etag` when it holds one, so an
/// unchanged manifest returns [`ConditionalRead::Unchanged`] without a body, and a full read
/// otherwise. A store that ignores the condition answers the full object either way.
async fn read_manifest(store: &dyn ObjectStore, etag: Option<&str>) -> ConditionalRead {
	match etag {
		Some(etag) => store::read_alarm_if_none_match(store, etag).await,
		None => ConditionalRead::Read(store::read_alarm(store).await),
	}
}









impl TabletSupervisor {
	/// Watches routing and alarm initialization together. The returned owned future
	/// must run beside the supervisor so split source holds and serving gates stay current.
	pub fn watch_manifest(
		&self,
		store: Arc<dyn ObjectStore>,
		output: watch::Sender<Option<TabletManifest>>,
		stop: watch::Receiver<bool>,
		refreshes: Vec<watch::Receiver<()>>,
		interval: Duration,
	) -> BoxFuture<'static, Result<()>> {
		let env = self.deps.env.clone();
		let alarms = self.alarm_manifest.clone();
		async move {
			watch_manifest_with_alarms(env, store, output, stop, refreshes, interval, Some(alarms))
				.await
		}
		.boxed()
	}
	pub fn with_commands(mut self, factory: crate::command::volume_host::Factory) -> Result<Self> {
		for running in self.running.values() {
			running.maintainer.install_commands(factory.clone())?;
		}
		self.commands = Some(factory);
		Ok(self)
	}

	pub fn command_limit_bytes(&self) -> usize {
		self.commands
			.as_ref()
			.map_or(0, |factory| factory.config.max_command_bytes)
	}

	/// Hands a log shard's replica status notices to the maintainer of each tablet they
	/// name, and answers at once. A tablet whose maintainer is not serving or whose inbox is
	/// full, or that this worker does not run, is named in the reply, so the shard resends only that tablet's
	/// replicas after a delay. The shard routes by its own manifest view, so an unknown
	/// tablet usually means that view is stale; the shard refreshes it and sends the
	/// replicas to the tablet's new maintainer.
	fn status_notices(
		&self,
		request: rivet_data::generated::depot_client_protocol_v1::ReplicaStatusNoticeRequest,
	) -> Result<(VolumeReplyAuthority, oneshot::Receiver<ClientResponse>), VolumeRejectResponse> {
		let worker = WorkerId::from_bytes(request.worker_id);
		let mut authority = None;
		let mut refused_tablets = Vec::new();
		let mut unknown_tablets = Vec::new();
		for tablet in request.tablets {
			let Some(running) = self.running.get(&tablet.tablet_id) else {
				unknown_tablets.push(tablet.tablet_id);
				continue;
			};
			authority.get_or_insert_with(|| VolumeReplyAuthority {
				snapshot: running.maintainer.snapshot.clone(),
				grant: TabletGrant {
					tablet_id: running.tablet.id,
					tablet_grant_epoch: running.tablet.tablet_grant_epoch,
				},
				authority: running.authority.clone(),
			});
			// A maintainer applies notices only while it serves. One still opening has listed
			// its members already, so a change noticed now would be lost; refusing it makes the
			// shard resend it after a backoff.
			let serving = running.maintainer.snapshot.borrow().phase == MaintainerPhase::Serving;
			if !serving {
				refused_tablets.push(tablet.tablet_id);
				continue;
			}
			let notices = tablet
				.notices
				.into_iter()
				.map(
					|notice| crate::tablet_maintainer_machine::types::StatusNotice {
						volume_log_key: notice.volume_log_key.into(),
						state: notice.state,
						entries: notice.entries,
						cold_write: notice.cold_write.map(|cold| {
							rivet_data::generated::depot_storage_worker_protocol_v1::ColdWriteStatus {
							first_write_unix_ms: cold.first_write_unix_ms,
							committed_bytes: cold.committed_bytes,
							disk_pressure: cold.disk_pressure,
						}
						}),
					},
				)
				.collect();
			let delivery = running.maintainer.status_notices(
				worker.clone(),
				tablet.tablet_grant_epoch,
				tablet.tablet_replica_set_version,
				notices,
			);
			match delivery {
				InboxDelivery::Queued => {}
				InboxDelivery::Full => refused_tablets.push(tablet.tablet_id),
			}
		}
		let Some(authority) = authority else {
			return Err(depot_types::client::not_maintainer());
		};
		let (sender, receiver) = oneshot::channel();
		let _ = sender.send(ClientResponse::ReplicaStatusNoticeResponse(
			rivet_data::generated::depot_client_protocol_v1::ReplicaStatusNoticeResponse {
				refused_tablets,
				unknown_tablets,
			},
		));
		Ok((authority, receiver))
	}

	/// A request named a tablet this worker does not serve. This worker's manifest may be
	/// stale, so the watcher reads it now instead of at its fallback interval.
	fn routing_miss(&self) {
		if let Some(refresh) = &self.manifest_refresh {
			refresh.send_replace(());
		}
	}

	/// The coordinator wrote manifest `version`, changing the grant of a tablet this worker
	/// maintained or now maintains. Reads the manifest now unless this worker already has that
	/// version, so a move starts without waiting for the fallback poll. Authority still comes only
	/// from the manifest read.
	pub fn grant_written(&self, version: u64) {
		let current = self
			.manifest
			.read()
			.as_ref()
			.map(|manifest| manifest.tablet_manifest_version);
		if current.is_none_or(|current| current < version) {
			self.routing_miss();
		}
	}

	/// Sweeps every command host whose idle queue head is due.
	pub fn sweep_due_commands(&self, now_boot_ms: u64) {
		for running in self.running.values() {
			if running
				.maintainer
				.next_command_sweep_at()
				.is_some_and(|due| due <= now_boot_ms)
			{
				running.maintainer.sweep_commands();
			}
		}
	}

	/// `request_bytes` is the command's received wire size.
	pub fn command(
		&self,
		request: rivet_data::generated::depot_kv_command_v1::KvCommandRequest,
		request_bytes: usize,
		deadline_boot_ms: u64,
	) -> Result<
		(
			VolumeReplyAuthority,
			oneshot::Receiver<rivet_data::generated::depot_kv_command_v1::KvCommandResponse>,
		),
		rivet_data::generated::depot_kv_command_v1::KvCommandResponse,
	> {
		let hash = depot_types::partition_key_hash(&request.volume_key);
		let Some(running) = self
			.running
			.values()
			.find(|running| running.tablet.hash_range.contains(hash))
		else {
			self.routing_miss();
			return Err(
				rivet_data::generated::depot_kv_command_v1::KvCommandResponse::NotMaintainer,
			);
		};
		running.observe_request(&self.deps.env, "kv_command");
		let response = running
			.maintainer
			.command(request, request_bytes, deadline_boot_ms)?;
		Ok((
			VolumeReplyAuthority {
				snapshot: running.maintainer.snapshot.clone(),
				grant: TabletGrant {
					tablet_id: running.tablet.id,
					tablet_grant_epoch: running.tablet.tablet_grant_epoch,
				},
				authority: running.authority.clone(),
			},
			response,
		))
	}
}

impl TabletSupervisor {
	/// Only the exact published assignment/version can admit an enhanced owner check.
	pub(crate) fn handoff_readiness(
		&self,
		request: rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness,
	) -> Result<
		(
			HandoffReplyAuthority,
			oneshot::Receiver<
				rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
			>,
		),
		rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
	> {
		use rivet_data::generated::{
			depot_storage_worker_protocol_v1::ReadinessPurpose,
			depot_storage_worker_protocol_v1::{NotReady, StopReadinessResponse as Response},
		};
		let ReadinessPurpose::Handoff(purpose) = &request.purpose else {
			return Err(Response::NotServing);
		};
		let version = purpose.tablet_manifest_version;
		let published = self.manifest.read();
		let Some(manifest) = published.as_ref() else {
			return Err(Response::NotServing);
		};
		let Some(running) = self.running.get(&request.tablet_id) else {
			self.routing_miss();
			return Err(Response::NotServing);
		};
		if manifest.tablet_manifest_version != version || running.manifest_version != version {
			self.routing_miss();
			return Err(Response::NotReady(NotReady {
				blocking_volumes: 0,
				sample: vec![],
			}));
		}
		if request.worker_id != *self.worker.as_bytes()
			|| running.tablet.maintainer != Some(request.worker_id)
			|| running.tablet.tablet_grant_epoch != request.tablet_grant_epoch
			|| running.tablet.replica_set.tablet_replica_set_version
				!= request.tablet_replica_set_version
			|| !manifest
				.tablets
				.iter()
				.any(|tablet| tablet == &running.tablet)
		{
			return Err(Response::NotServing);
		}
		let stop_intent = manifest.stopping.iter().any(|stop| {
			stop.worker_id == request.worker_id && stop.instance_id == request.worker_incarnation
		});
		drop(published);
		let volume = VolumeReplyAuthority {
			snapshot: running.maintainer.snapshot.clone(),
			grant: TabletGrant {
				tablet_id: request.tablet_id,
				tablet_grant_epoch: request.tablet_grant_epoch,
			},
			authority: running.authority.clone(),
		};
		if !volume.may_reply() {
			return Err(Response::NotServing);
		}
		let response = running.maintainer.handoff_readiness(request, stop_intent)?;
		Ok((
			HandoffReplyAuthority {
				volume,
				manifest: self.manifest.clone(),
				version,
			},
			response,
		))
	}

	/// Opening instances reply from their pure core; obsolete grants never supply evidence.
	pub fn stop_readiness(
		&self,
		request: rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness,
	) -> Result<
		(
			VolumeReplyAuthority,
			oneshot::Receiver<
				rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
			>,
		),
		rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
	> {
		use rivet_data::generated::depot_storage_worker_protocol_v1::{
			StaleTabletGrantEpoch, StopReadinessResponse as Response,
		};
		let Some(running) = self.running.get(&request.tablet_id) else {
			self.routing_miss();
			return Err(Response::NotServing);
		};
		if request.tablet_grant_epoch > running.tablet.tablet_grant_epoch
			|| request.tablet_replica_set_version
				> running.tablet.replica_set.tablet_replica_set_version
		{
			self.routing_miss();
		}
		if running.tablet.tablet_grant_epoch != request.tablet_grant_epoch {
			return Err(Response::StaleTabletGrantEpoch(StaleTabletGrantEpoch {
				tablet_grant_epoch: running.tablet.tablet_grant_epoch,
			}));
		}
		let authority = VolumeReplyAuthority {
			snapshot: running.maintainer.snapshot.clone(),
			grant: TabletGrant {
				tablet_id: request.tablet_id,
				tablet_grant_epoch: request.tablet_grant_epoch,
			},
			authority: running.authority.clone(),
		};
		if !authority.may_reply() {
			return Err(Response::NotServing);
		}
		Ok((authority, running.maintainer.stop_readiness(request)?))
	}
}
