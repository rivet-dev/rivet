//! Explicit coordinator task ownership under the existing singleton authority.

use super::{
	Snapshot, TabletCoordinatorMachine, effect::Effect, event::Event, state::State, store,
};
use crate::balancing_config::TabletBalancingConfig;
use anyhow::{Context, Result};
use object_store::ObjectStore;
use parking_lot::Mutex;
use rivet_data::generated::depot_storage_worker_protocol_v1::{
	GrantWritten, ReadinessPurpose, StopReadiness,
};
use rivet_data::generated::depot_tablet_coordinator_protocol_v1::{
	TabletCoordinatorRejectResponse, TabletCoordinatorRequest, TabletCoordinatorResponse,
};
use rivet_env::{Env, TaskScope};
use rivet_singleton_coordinator::{
	authority::Authority, runtime::SingletonServices,
	singleton_coordinator_machine::types::CoordinatorTenure,
};
use rivet_state_machine_ephemeral::{EffectContext, Executor, Runner, RunnerHandle};
use std::{
	collections::BTreeMap,
	sync::{
		Arc,
		atomic::{AtomicBool, AtomicU64, Ordering},
	},
};
use tokio::sync::{mpsc, oneshot, watch};



/// Implementations check current singleton and worker lease evidence with a fresh clock.
pub trait CoordinatorAuthority: Send + Sync + 'static {
	fn permits(&self) -> bool;
}

impl CoordinatorAuthority for Authority {
	fn permits(&self) -> bool {
		Authority::permits(self)
	}
}

/// Application-owned discovery and maintainer transport; never consulted by the pure core.
pub trait StopTransport: Send + Sync + 'static {
	fn disk_incarnation(
		&self,
		_worker: [u8; 16],
	) -> futures_util::future::BoxFuture<'_, Result<[u8; 16]>> {
		Box::pin(async { anyhow::bail!("disk admission transport unavailable") })
	}
	fn topology(&self) -> futures_util::future::BoxFuture<'_, Result<super::types::StopTopology>>;
	fn readiness(
		&self,
		worker: [u8; 16],
		request: rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness,
	) -> futures_util::future::BoxFuture<
		'_,
		rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
	>;
	fn handoff_readiness(
		&self,
		_worker: [u8; 16],
		_request: rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness,
	) -> futures_util::future::BoxFuture<
		'_,
		Result<rivet_data::generated::depot_storage_worker_protocol_v1::HandoffReadinessResponse>,
	> {
		Box::pin(async { anyhow::bail!("enhanced handoff readiness is unsupported") })
	}

	/// Tells `worker` that a confirmed manifest changed the grant of a tablet it maintained or
	/// now maintains, so it reads the manifest instead of waiting for its poll. Best effort: the
	/// caller only logs a failure, because the poll still delivers the grant.
	fn grant_written(
		&self,
		_worker: [u8; 16],
		_request: rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness,
	) -> futures_util::future::BoxFuture<'_, Result<()>> {
		Box::pin(async { anyhow::bail!("grant notices are unsupported") })
	}

	/// Waits for the next unexpired storage discovery observation that differs from
	/// `previous`, refreshing discovery through topology when the current view expires.
	/// Topology owns transient refresh retries; an error means the subscription cannot continue.
	fn next_storage_workers(
		&self,
		previous: Option<super::types::StopTopology>,
	) -> futures_util::future::BoxFuture<'_, Result<super::types::StopTopology>>;
}

type Replies = Arc<Mutex<BTreeMap<u64, oneshot::Sender<TabletCoordinatorResponse>>>>;

/// Each tablet's maintainer and grant epoch as of the last confirmed manifest, kept only to
/// log grant changes when a write confirms.
type ConfirmedGrants = Arc<Mutex<BTreeMap<[u8; 16], (Option<[u8; 16]>, u64)>>>;

/// Cloneable admission handle; holding it does not keep a revoked coordinator alive.
#[derive(Clone)]
pub struct Client {
	stop_transport: Option<Arc<dyn StopTransport>>,
	inbox: mpsc::WeakSender<Event>,
	authority: Arc<dyn CoordinatorAuthority>,
	alive: Arc<AtomicBool>,
	replies: Replies,
	next_request: Arc<AtomicU64>,
}

impl Client {
	/// Recheck immediately before network delivery; an earlier completed command cannot restore a lost tenure.
	pub fn may_serve(&self) -> bool {
		self.alive.load(Ordering::Acquire) && self.authority.permits()
	}

	#[tracing::instrument(skip_all)]
	pub async fn request(&self, request: TabletCoordinatorRequest) -> TabletCoordinatorResponse {
		self.request_envelope(rivet_data::generated::depot_tablet_coordinator_protocol_v1::AlarmCoordinatorRequest::TabletCoordinatorRequest(request)).await
	}
	/// Sends a coordinator command that retains the exact alarm-index source and grant.
	pub async fn request_envelope(
		&self,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::AlarmCoordinatorRequest,
	) -> TabletCoordinatorResponse {
		if !self.alive.load(Ordering::Acquire) || !self.authority.permits() {
			return not_coordinator();
		}
		let Ok(id) = self
			.next_request
			.try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
		else {
			return not_coordinator();
		};
		let (send, receive) = oneshot::channel();
		{
			let mut replies = self.replies.lock();
			if replies.len() >= 128 {
				return TabletCoordinatorResponse::TabletCoordinatorRejectResponse(
					TabletCoordinatorRejectResponse::Overloaded,
				);
			}
			replies.insert(id, send);
		}
		let _pending = PendingReply {
			id,
			replies: self.replies.clone(),
			inbox: self.inbox.clone(),
		};
		let Some(inbox) = self.inbox.upgrade() else {
			return not_coordinator();
		};
		let event = if let rivet_data::generated::depot_tablet_coordinator_protocol_v1::AlarmCoordinatorRequest::TabletCoordinatorRequest(TabletCoordinatorRequest::RequestStop(request)) = request {
			let Some(transport) = &self.stop_transport else {
				return not_coordinator();
			};
			let Ok(topology) = transport.topology().await else {
				return TabletCoordinatorResponse::TabletCoordinatorRejectResponse(
					TabletCoordinatorRejectResponse::Overloaded,
				);
			};
			Event::RequestStop {
				request_id: id,
				request,
				topology,
			}
		} else {
			Event::AlarmCommand {
				request_id: id,
				request,
			}
		};
		if inbox.send(event).await.is_err() {
			return not_coordinator();
		}
		// A caller waiting for its reply must not keep the stopped runner inbox alive.
		drop(inbox);
		let response = receive.await.unwrap_or_else(|_| not_coordinator());
		if self.alive.load(Ordering::Acquire) && self.authority.permits() {
			response
		} else {
			not_coordinator()
		}
	}
}

struct PendingReply {
	id: u64,
	replies: Replies,
	inbox: mpsc::WeakSender<Event>,
}

impl Drop for PendingReply {
	fn drop(&mut self) {
		if self.replies.lock().remove(&self.id).is_some() {
			if let Some(inbox) = self.inbox.upgrade() {
				let _ = inbox.try_send(Event::CancelCommand {
					request_id: self.id,
				});
			}
		}
	}
}

pub struct Coordinator {
	pub client: Client,
	pub snapshot: watch::Receiver<Snapshot>,
	runner: Option<RunnerHandle<TabletCoordinatorMachine>>,
	executor: Arc<ManifestExecutor>,
}

impl Coordinator {
	/// Startup reads and conditionally rewrites the manifest before admitting confirmed replies.
	pub fn start(
		env: Arc<dyn Env>,
		store: Arc<dyn ObjectStore>,
		authority: Arc<dyn CoordinatorAuthority>,
	) -> Self {
		Self::start_with_stop(env, store, authority, None)
	}

	pub fn start_with_stop(
		env: Arc<dyn Env>,
		store: Arc<dyn ObjectStore>,
		authority: Arc<dyn CoordinatorAuthority>,
		stop_transport: Option<Arc<dyn StopTransport>>,
	) -> Self {
		Self::start_configured(
			env,
			store,
			authority,
			stop_transport,
			super::state::balancing::Settings::default(),
		)
	}

	pub fn start_configured(
		env: Arc<dyn Env>,
		store: Arc<dyn ObjectStore>,
		authority: Arc<dyn CoordinatorAuthority>,
		stop_transport: Option<Arc<dyn StopTransport>>,
		settings: super::state::balancing::Settings,
	) -> Self {
		Self::start_balancing(
			env,
			store,
			authority,
			stop_transport,
			Some(settings),
			watch::channel(true).1,
			super::state::DEFAULT_LEARNING_DEADLINE_MS,
		)
	}

	/// Starts the coordinator with automatic placement, or with operator commands alone
	/// when `settings` is absent. `balancing_enabled` is the dynamic switch for the
	/// configured balancer: its current value sets the start position and every later
	/// change is delivered to the core while this coordinator holds its tenure.
	pub fn start_balancing(
		env: Arc<dyn Env>,
		store: Arc<dyn ObjectStore>,
		authority: Arc<dyn CoordinatorAuthority>,
		stop_transport: Option<Arc<dyn StopTransport>>,
		settings: Option<super::state::balancing::Settings>,
		balancing_enabled: watch::Receiver<bool>,
		learning_deadline_ms: u64,
	) -> Self {
		let mut state = State::default();
		state.learning_deadline_ms = learning_deadline_ms;
		state.balancing = match (&stop_transport, settings) {
			(Some(_), Some(settings)) => Some(
				super::state::balancing::Balancing::new(settings)
					.with_enabled(*balancing_enabled.borrow()),
			),
			(None, _) | (_, None) => None,
		};
		let configured_balancing = state.balancing.is_some();
		state.alarms.enabled = true;
		let executor = Arc::new(ManifestExecutor {
			stop_transport: stop_transport.clone(),
			stop_tasks: Mutex::new(BTreeMap::new()),
			timeout_ms: state.operation_timeout_ms,
			store,
			authority: authority.clone(),
			alive: Arc::new(AtomicBool::new(true)),
			tasks: env.task_scope(),
			replies: Arc::new(Mutex::new(BTreeMap::new())),
			grants: Arc::new(Mutex::new(BTreeMap::new())),
		});
		let runner = Runner::spawn(env.clone(), state, executor.clone());
		if let Some(transport) = stop_transport.clone() {
			executor.tasks.spawn(forward_storage_workers(
				transport,
				runner.inbox.downgrade(),
				executor.authority.clone(),
				executor.alive.clone(),
			));
		}
		if configured_balancing {
			executor.tasks.spawn(forward_balancing_enabled(
				balancing_enabled,
				runner.inbox.downgrade(),
				executor.authority.clone(),
				executor.alive.clone(),
			));
		}
		let client = Client {
			stop_transport,
			inbox: runner.inbox.downgrade(),
			authority,
			alive: executor.alive.clone(),
			replies: executor.replies.clone(),
			next_request: Arc::new(AtomicU64::new(1)),
		};
		Self {
			client,
			snapshot: runner.snapshot.clone(),
			runner: Some(runner),
			executor,
		}
	}

	/// Revoke first, then stop the core and join every local effect before relinquishing the service.
	pub async fn shutdown(mut self) -> Result<()> {
		self.executor.alive.store(false, Ordering::Release);
		let mut runner = self.runner.take().context("coordinator already closed")?;
		let _ = runner.inbox.send(Event::Stop).await;
		let _ = runner.snapshot.wait_for(|snapshot| snapshot.stopped).await;
		self.executor.tasks.shutdown().await;
		self.executor.replies.lock().clear();
		drop(runner.inbox);
		runner
			.stopped
			.await
			.context("tablet coordinator runner failed")?;
		Ok(())
	}
}

impl Drop for Coordinator {
	fn drop(&mut self) {
		self.executor.alive.store(false, Ordering::Release);
		self.executor.tasks.cancel();
		self.executor.replies.lock().clear();
	}
}

/// Delivers each changed storage discovery observation to the core while this coordinator
/// holds its tenure. Ends when the coordinator stops or discovery fails permanently.
async fn forward_storage_workers(
	transport: Arc<dyn StopTransport>,
	inbox: mpsc::WeakSender<Event>,
	authority: Arc<dyn CoordinatorAuthority>,
	alive: Arc<AtomicBool>,
) {
	let mut previous = None;
	loop {
		let topology = match transport.next_storage_workers(previous.clone()).await {
			Ok(topology) => topology,
			Err(error) => {
				tracing::error!(
					?error,
					"storage discovery subscription ended; stopping the tablet coordinator"
				);
				// Topology owns refresh retries. A closed subscription or missing topology
				// owner is terminal, so the core must stop using its last observation.
				if let Some(inbox) = inbox.upgrade() {
					let _ = inbox.send(Event::Stop).await;
				}
				return;
			}
		};
		previous = Some(topology.clone());
		if !alive.load(Ordering::Acquire) || !authority.permits() {
			return;
		}
		let Some(inbox) = inbox.upgrade() else {
			return;
		};
		if inbox
			.send(Event::StorageWorkers { topology })
			.await
			.is_err()
		{
			return;
		}
	}
}

/// Delivers each change of the dynamic balancing switch to the core while this coordinator
/// holds its tenure. The start position was read when the state was built, so only later
/// changes are sent. Ends when the coordinator stops or the switch's publisher is gone.
async fn forward_balancing_enabled(
	mut enabled: watch::Receiver<bool>,
	inbox: mpsc::WeakSender<Event>,
	authority: Arc<dyn CoordinatorAuthority>,
	alive: Arc<AtomicBool>,
) {
	loop {
		if enabled.changed().await.is_err() {
			return;
		}
		let enabled = *enabled.borrow_and_update();
		if !alive.load(Ordering::Acquire) || !authority.permits() {
			return;
		}
		let Some(inbox) = inbox.upgrade() else {
			return;
		};
		if inbox
			.send(Event::BalancingEnabled { enabled })
			.await
			.is_err()
		{
			return;
		}
	}
}

struct ManifestExecutor {
	stop_transport: Option<Arc<dyn StopTransport>>,
	stop_tasks: Mutex<BTreeMap<[u8; 16], rivet_env::TaskHandle>>,
	timeout_ms: u64,
	store: Arc<dyn ObjectStore>,
	authority: Arc<dyn CoordinatorAuthority>,
	alive: Arc<AtomicBool>,
	tasks: TaskScope,
	replies: Replies,
	grants: ConfirmedGrants,
}

/// Returns one grant notice per worker that was or now is the maintainer of a tablet whose
/// maintainer or grant epoch a confirmed write changed, since workers otherwise learn of a
/// move only at their manifest poll. The receiver reads only the manifest version; the tablet
/// fields name the first changed tablet of that worker and are informational, and the worker
/// incarnation is unused.
///
/// Also logs every such tablet, so a move can be timed from its decision (`granted_at`) to
/// this confirmation and on to each worker's reaction. The first observation after a takeover
/// only seeds the view, and a read (`confirmed` absent) never returns notices.
fn observe_grants(
	grants: &ConfirmedGrants,
	manifest: &rivet_data::generated::depot_tablet_manifest_v1::TabletManifest,
	confirmed: Option<(i64, u64)>,
) -> Vec<([u8; 16], StopReadiness)> {
	let mut previous = grants.lock();
	let seeded = !previous.is_empty();
	let mut notices = Vec::new();
	for tablet in &manifest.tablets {
		let current = (tablet.maintainer, tablet.tablet_grant_epoch);
		let before = previous.insert(tablet.id, current);
		let Some((now_unix_ms, write_ms)) = confirmed else {
			continue;
		};
		if !seeded || before == Some(current) {
			continue;
		}
		let from = before.and_then(|(maintainer, _)| maintainer);
		tracing::info!(
			tablet_id = %uuid::Uuid::from_bytes(tablet.id),
			from = ?from.map(uuid::Uuid::from_bytes),
			to = ?tablet.maintainer.map(uuid::Uuid::from_bytes),
			grant_epoch = tablet.tablet_grant_epoch,
			manifest_version = manifest.tablet_manifest_version,
			since_grant_ms = now_unix_ms.saturating_sub(tablet.granted_at),
			write_ms,
			"tablet grant written"
		);

		for worker in [from, tablet.maintainer].into_iter().flatten() {
			let notified = notices
				.iter()
				.any(|(notified, _): &([u8; 16], StopReadiness)| *notified == worker);
			if notified {
				continue;
			}
			notices.push((
				worker,
				StopReadiness {
					tablet_id: tablet.id,
					tablet_grant_epoch: tablet.tablet_grant_epoch,
					tablet_replica_set_version: tablet.replica_set.tablet_replica_set_version,
					worker_id: worker,
					worker_incarnation: [0; 16],
					purpose: ReadinessPurpose::GrantWritten(GrantWritten {
						tablet_manifest_version: manifest.tablet_manifest_version,
					}),
				},
			));
		}
	}
	notices
}

impl Executor<TabletCoordinatorMachine> for ManifestExecutor {
	fn execute(&self, effect: Effect, ctx: EffectContext<TabletCoordinatorMachine>) {
		let inbox = ctx.inbox().clone();
		let env = ctx.env().clone();
		if let Effect::ObservePlacement {
			status,
			probe_limit_reached,
		} = effect
		{
			for (kind, value) in [
				("in_flight", u64::from(status.disk_probes_in_flight)),
				("waiting", status.waiting_disk_probes as u64),
				("capacity", u64::from(status.disk_probe_capacity)),
			] {
				crate::metrics::COORDINATOR_DISK_PROBES
					.with_label_values(&[kind])
					.set(value.min(i64::MAX as u64) as i64);
			}
			if probe_limit_reached {
				tracing::warn!(
					capacity = status.disk_probe_capacity,
					waiting = status.waiting_disk_probes,
					"tablet disk admission probes reached capacity; waiting workers will be probed as slots free"
				);
			}
			tracing::info!(
				admitted_workers = status.admitted_workers,
				desired_tablets = status.desired_tablets,
				projected_tablets = status.projected_tablets,
				pending_splits = status.pending_splits,
				pending_replacements = status.pending_replacements,
				paused_tablets = status.paused_tablets,
				search_complete = status.search_complete,
				replica_imbalance_ppm = status.replica_imbalance_ppm,
				maintainer_imbalance_ppm = status.maintainer_imbalance_ppm,
				blocked = status
					.blocked
					.as_ref()
					.map_or("none", |reason| reason.as_str()),
				"tablet placement changed"
			);
			for (kind, value) in [
				("desired", status.desired_tablets),
				("projected", status.projected_tablets),
				("admitted_workers", status.admitted_workers),
				("pending_splits", status.pending_splits),
				("pending_replacements", status.pending_replacements),
				("paused", status.paused_tablets),
			] {
				crate::metrics::COORDINATOR_PLACEMENT
					.with_label_values(&[kind])
					.set(value.min(i64::MAX as u64) as i64);
			}
			for (role, value) in [
				("replica", status.replica_imbalance_ppm),
				("maintainer", status.maintainer_imbalance_ppm),
			] {
				crate::metrics::COORDINATOR_WIDTH_IMBALANCE
					.with_label_values(&[role])
					.set(value.min(i64::MAX as u64) as i64);
			}
			let reason = status
				.blocked
				.as_ref()
				.map_or("none", |reason| reason.as_str());
			for label in [
				"none",
				"workers",
				"grace",
				"destination_or_quorum",
				"split",
				"manifest_headroom",
				"disabled",
			] {
				crate::metrics::COORDINATOR_PLACEMENT_BLOCKED
					.with_label_values(&[label])
					.set(i64::from(label == reason));
			}
			return;
		}
		if let Effect::DiskProbeRequest {
			worker,
			instance,
			capacity,
			operation,
		} = effect
		{
			let transport = self.stop_transport.clone();
			let authority = self.authority.clone();
			let alive = self.alive.clone();
			let timeout = std::time::Duration::from_millis(self.timeout_ms);
			ctx.spawn_in(&self.tasks, async move {
				let result = async {
					anyhow::ensure!(
						alive.load(Ordering::Acquire) && authority.permits(),
						"coordinator lost authority"
					);
					let transport = transport.context("disk probe transport unavailable")?;
					let disk = transport.disk_incarnation(worker).await?;
					let topology = transport.topology().await?;
					anyhow::ensure!(
						env.clock().observe()?.boot_ms < topology.valid_until_boot_ms
							&& topology.workers.get(&worker) == Some(&instance)
							&& topology
								.capacities
								.get(&worker)
								.copied()
								.unwrap_or(TabletBalancingConfig::MILLICORES_PER_CORE)
								== capacity,
						"probe topology changed"
					);
					Ok(disk)
				};
				let result = rivet_env::timeout(env.clock().as_ref(), timeout, result)
					.await
					.map_err(|error| error.to_string())
					.and_then(|result: anyhow::Result<_>| {
						result.map_err(|error| error.to_string())
					});
				let event = if alive.load(Ordering::Acquire) && authority.permits() {
					Event::DiskProbeResponse {
						worker,
						operation,
						result,
					}
				} else {
					Event::Stop
				};
				let _ = inbox.send(event).await;
			});
			return;
		}
		if let Effect::ObserveStops {
			entries,
			new_permit,
		} = effect
		{
			crate::metrics::COORDINATOR_STOP_REQUESTS.set(entries.min(i64::MAX as u64) as i64);
			if new_permit {
				crate::metrics::COORDINATOR_STOP_PERMITS.inc();
			}
			return;
		}
		if let Effect::CommandResponse {
			request_id,
			response,
		} = effect
		{
			let response = if self.alive.load(Ordering::Acquire) && self.authority.permits() {
				response
			} else {
				not_coordinator()
			};
			if let Some(reply) = self.replies.lock().remove(&request_id) {
				let _ = reply.send(response);
			}
			return;
		}
		if let Effect::ObserveRegrants { granted, no_member } = effect {
			if granted > 0 {
				tracing::info!(
					granted,
					"re-granted tablets whose maintainer left storage discovery"
				);
				crate::metrics::COORDINATOR_REGRANTS
					.with_label_values(&["granted"])
					.inc_by(granted);
			}
			for tablet_id in no_member {
				crate::metrics::COORDINATOR_REGRANTS
					.with_label_values(&["no_member"])
					.inc();
				tracing::warn!(
					tablet_id = %uuid::Uuid::from_bytes(tablet_id),
					"tablet maintainer left storage discovery and no live member can take the grant; the tablet stays unavailable until a member returns"
				);
			}
			return;
		}
		if let Effect::CancelStopReadinessRequest { worker } = effect {
			if let Some(task) = self.stop_tasks.lock().remove(&worker) {
				task.abort();
			}
			return;
		}
		if let Effect::StopReadinessRequest { operation, check } = effect {
			use futures_util::{StreamExt, stream};
			use rivet_data::generated::depot_storage_worker_protocol_v1::{
				StopReadiness, StopReadinessResponse,
			};
			let transport = self.stop_transport.clone();
			let inbox = inbox.clone();
			let authority = self.authority.clone();
			let alive = self.alive.clone();
			let worker = check.request.worker_id;
			let worker_incarnation = check.request.instance_id;
			let enhanced = matches!(
				check.purpose,
				super::types::StopCheckPurpose::Handoff { .. }
			);
			let requests: Vec<_> = check
				.tablets
				.iter()
				.map(|tablet| (tablet.clone(), check.handoff_request(tablet)))
				.collect();
			let task = ctx.spawn_in(&self.tasks, async move {
				let results: Vec<_> =
					stream::iter(requests.into_iter().map(|(tablet, request)| {
						let transport = transport.clone();
						let authority = authority.clone();
						let alive = alive.clone();
						async move {
							let mut proof = None;
							let mut response = StopReadinessResponse::NotServing;
							if alive.load(Ordering::Acquire) && authority.permits() {
								if let (Some(transport), Some(maintainer)) =
									(transport, tablet.maintainer)
								{
									if let Some(request) = request {
										proof = transport
											.handoff_readiness(maintainer, request)
											.await
											.ok();
									} else {
										response = transport
											.readiness(
												maintainer,
												StopReadiness {
													tablet_replica_set_version: tablet
														.replica_set
														.tablet_replica_set_version,
													tablet_id: tablet.id,
													tablet_grant_epoch: tablet.tablet_grant_epoch,
													worker_id: worker,
													worker_incarnation,
													purpose: ReadinessPurpose::Stop,
												},
											)
											.await;
									}
								}
							}
							(tablet.id, proof, response)
						}
					}))
					.buffer_unordered(16)
					.collect()
					.await;
				let event = if !alive.load(Ordering::Acquire) || !authority.permits() {
					Event::Stop
				} else if enhanced {
					Event::HandoffReadinessResponse {
						operation,
						worker,
						results: results
							.into_iter()
							.map(|(id, proof, _)| (id, proof))
							.collect(),
					}
				} else {
					Event::StopReadinessResponse {
						operation,
						worker,
						results: results
							.into_iter()
							.map(|(id, _, response)| (id, response))
							.collect(),
					}
				};
				let _ = inbox.send(event).await;
			});
			let mut tasks = self.stop_tasks.lock();
			tasks.retain(|_, task| !task.is_finished());
			if let Some(previous) = tasks.insert(worker, task) {
				previous.abort();
			}
			return;
		}

		let store = self.store.clone();
		let authority = self.authority.clone();
		let alive = self.alive.clone();
		let inbox = inbox.clone();
		let runtime = env.clone();
		let timeout = std::time::Duration::from_millis(self.timeout_ms);
		let grants = self.grants.clone();
		let transport = self.stop_transport.clone();
		ctx.spawn_in(&self.tasks, async move {
			if !alive.load(Ordering::Acquire) || !authority.permits() {
				let _ = inbox.send(Event::Stop).await;
				return;
			}
			let clock = runtime.clock();
			let mut notices = Vec::new();
			let event = match effect {
				Effect::ReadManifestRequest { operation } => {
					let result = rivet_env::timeout(
						clock.as_ref(),
						timeout,
						store::read_alarm(store.as_ref()),
					)
					.await
					.unwrap_or_else(|error| super::types::ReadResult::Unknown(error.to_string()));
					match &result {
						super::types::ReadResult::Found(stored) => {
							observe_grants(&grants, &stored.manifest, None);
						}
						super::types::ReadResult::FoundAlarm { envelope, .. } => {
							observe_grants(&grants, &envelope.manifest, None);
						}
						super::types::ReadResult::Missing
						| super::types::ReadResult::Unknown(_)
						| super::types::ReadResult::Invalid(_) => {}
					}
					Event::ReadManifestResponse { operation, result }
				}
				Effect::WriteAlarmManifestRequest {
					operation,
					write,
					envelope,
				} => {
					if let super::types::WriteOrigin::Commands { request_ids, .. } = &write.origin {
						crate::metrics::COORDINATOR_WRITE_COMMANDS
							.observe(request_ids.len() as f64);
					}
					let written = envelope.manifest.clone();
					let issued = clock.observe().map(|clocks| clocks.boot_ms).unwrap_or(0);
					let result = rivet_env::timeout(
						clock.as_ref(),
						timeout,
						store::write_alarm(store.as_ref(), write, envelope),
					)
					.await
					.unwrap_or_else(|error| super::types::WriteResult::Unknown(error.to_string()));
					if matches!(result, super::types::WriteResult::Confirmed { .. })
						&& let Ok(clocks) = clock.observe()
					{
						let write_ms = clocks.boot_ms.saturating_sub(issued);
						notices =
							observe_grants(&grants, &written, Some((clocks.unix_ms, write_ms)));
					}
					Event::WriteManifestResponse { operation, result }
				}
				Effect::WriteManifestRequest { operation, write } => {
					if let super::types::WriteOrigin::Commands { request_ids, .. } = &write.origin {
						crate::metrics::COORDINATOR_WRITE_COMMANDS
							.observe(request_ids.len() as f64);
					}
					let written = write.manifest.clone();
					let issued = clock.observe().map(|clocks| clocks.boot_ms).unwrap_or(0);
					let result = rivet_env::timeout(
						clock.as_ref(),
						timeout,
						store::write(store.as_ref(), write),
					)
					.await
					.unwrap_or_else(|error| super::types::WriteResult::Unknown(error.to_string()));
					if matches!(result, super::types::WriteResult::Confirmed { .. })
						&& let Ok(clocks) = clock.observe()
					{
						let write_ms = clocks.boot_ms.saturating_sub(issued);
						notices =
							observe_grants(&grants, &written, Some((clocks.unix_ms, write_ms)));
					}
					Event::WriteManifestResponse { operation, result }
				}
				Effect::ObserveStops { .. }
				| Effect::ObservePlacement { .. }
				| Effect::DiskProbeRequest { .. }
				| Effect::ObserveRegrants { .. }
				| Effect::StopReadinessRequest { .. }
				| Effect::CancelStopReadinessRequest { .. }
				| Effect::CommandResponse { .. } => unreachable!("replies handled synchronously"),
			};
			let (operation, outcome) = match &event {
				Event::ReadManifestResponse { result, .. } => (
					"read",
					match result {
						super::types::ReadResult::FoundAlarm { .. }
						| super::types::ReadResult::Found(_) => "found",
						super::types::ReadResult::Missing => "missing",
						super::types::ReadResult::Unknown(_) => "unknown",
						super::types::ReadResult::Invalid(_) => "invalid",
					},
				),
				Event::WriteManifestResponse { result, .. } => (
					"write",
					match result {
						super::types::WriteResult::Confirmed { .. } => "confirmed",
						super::types::WriteResult::Precondition => "precondition",
						super::types::WriteResult::Unknown(_) => "unknown",
						super::types::WriteResult::Invalid(_) => "invalid",
					},
				),
				Event::RequestStop { .. }
				| Event::DiskProbeResponse { .. }
				| Event::StopReadinessResponse { .. }
				| Event::HandoffReadinessResponse { .. }
				| Event::CancelCommand { .. }
				| Event::AlarmCommand { .. }
				| Event::Command { .. }
				| Event::StorageWorkers { .. }
				| Event::BalancingEnabled { .. }
				| Event::Stop => {
					unreachable!("I/O response constructed above")
				}
			};
			crate::metrics::TABLET_MANIFEST_IO
				.with_label_values(&[operation, outcome])
				.inc();
			if matches!(outcome, "unknown" | "invalid" | "precondition") {
				tracing::warn!(
					operation,
					outcome,
					"tablet manifest I/O did not confirm the requested operation"
				);
			}
			let serving = alive.load(Ordering::Acquire) && authority.permits();
			let event = if serving { event } else { Event::Stop };
			let _ = inbox.send(event).await;

			// The core already has the write's response, so the notices delay nothing.
			if serving && let Some(transport) = transport {
				let sends = notices.into_iter().map(|(worker, notice)| {
					let transport = transport.clone();
					async move {
						if let Err(error) = transport.grant_written(worker, notice).await {
							tracing::debug!(
								?error,
								worker = %uuid::Uuid::from_bytes(worker),
								"grant notice was not delivered; the worker's manifest poll covers it"
							);
						}
					}
				});
				futures_util::future::join_all(sends).await;
			}
		});
	}
}

fn not_coordinator() -> TabletCoordinatorResponse {
	TabletCoordinatorResponse::TabletCoordinatorRejectResponse(
		TabletCoordinatorRejectResponse::NotCoordinator,
	)
}

/// Install beside the existing singleton services. Construction starts no work.
pub struct TabletCoordinatorService {
	/// Automatic placement settings; absent leaves the layout to operator commands.
	settings: Option<super::state::balancing::Settings>,
	/// The dynamic switch for the configured balancer, followed by every tenure.
	balancing_enabled: watch::Receiver<bool>,
	learning_deadline_ms: u64,
	stop_transport: Option<Arc<dyn StopTransport>>,
	pub env: Arc<dyn Env>,
	pub store: Arc<dyn ObjectStore>,
	pub active: watch::Sender<Option<Client>>,
	coordinator: tokio::sync::Mutex<Option<Coordinator>>,
}

impl TabletCoordinatorService {
	pub fn with_balancing(mut self, settings: Option<super::state::balancing::Settings>) -> Self {
		self.settings = settings;
		self
	}
	/// Follows `enabled` as the balancer's switch instead of leaving it on.
	pub fn with_balancing_enabled(mut self, enabled: watch::Receiver<bool>) -> Self {
		self.balancing_enabled = enabled;
		self
	}
	pub fn with_learning_deadline_ms(mut self, learning_deadline_ms: u64) -> Self {
		self.learning_deadline_ms = learning_deadline_ms;
		self
	}
	pub fn with_stop_transport(mut self, transport: Arc<dyn StopTransport>) -> Self {
		self.stop_transport = Some(transport);
		self
	}
	pub fn new(env: Arc<dyn Env>, store: Arc<dyn ObjectStore>) -> Self {
		Self {
			settings: Some(Default::default()),
			balancing_enabled: watch::channel(true).1,
			learning_deadline_ms: super::state::DEFAULT_LEARNING_DEADLINE_MS,
			env,
			store,
			active: watch::channel(None).0,
			stop_transport: None,
			coordinator: tokio::sync::Mutex::new(None),
		}
	}
}

#[async_trait::async_trait]
impl SingletonServices for TabletCoordinatorService {
	async fn run(&self, authority: Authority) -> Result<()> {
		let coordinator = Coordinator::start_balancing(
			self.env.clone(),
			self.store.clone(),
			Arc::new(authority.clone()),
			self.stop_transport.clone(),
			self.settings.clone(),
			self.balancing_enabled.clone(),
			self.learning_deadline_ms,
		);
		self.active.send_replace(Some(coordinator.client.clone()));
		*self.coordinator.lock().await = Some(coordinator);
		authority.revoked().await;
		self.active.send_replace(None);
		Ok(())
	}

	async fn reconcile_stopped(&self, _: &CoordinatorTenure) -> Result<()> {
		self.active.send_replace(None);
		if let Some(coordinator) = self.coordinator.lock().await.take() {
			coordinator.shutdown().await?;
		}
		// Every replacement performs its takeover CAS before serving. Any sent predecessor
		// write either wins before that CAS, or its old ETag is rejected afterward.
		Ok(())
	}
}
