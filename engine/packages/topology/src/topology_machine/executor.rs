use super::{
	TopologyStateMachine,
	effect::*,
	event::*,
	types::{
		EndpointResult, EndpointTimeoutResult, FenceReason, FenceReport, OperationId,
		RenewalLateReport, RequestError,
	},
};
use crate::{
	heartbeat::{Read, Write, read_heartbeat},
	keys::s3::WorkerHeartbeatKey,
	worker::WorkerRole,
};
use anyhow::Result;
use object_store::{ObjectStore, PutMode, UpdateVersion};
use rivet_env::TaskScope;
use rivet_keys::keys::{KeyAddress, codec::FormalKey};
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use rivet_state_machine_ephemeral::{EffectContext, Executor};
use rivet_state_machine_util::IoCall;
use std::{
	collections::BTreeMap,
	sync::{
		Arc, Weak,
		atomic::{AtomicUsize, Ordering},
	},
};
use tokio::sync::{Mutex, OwnedSemaphorePermit, oneshot};

pub(crate) enum ResponseSender {
	ResolveEndpoint(oneshot::Sender<EndpointResult>),
	InvalidateEndpoint(oneshot::Sender<Result<(), RequestError>>),
	InspectHeartbeat(oneshot::Sender<Read>),
}

pub(crate) type Responses = Mutex<BTreeMap<u64, (ResponseSender, OwnedSemaphorePermit)>>;

pub(crate) struct TopologyExecutor {
	pub store: Arc<dyn ObjectStore>,
	pub tasks: Weak<TaskScope>,
	pub lookups: Weak<TaskScope>,
	pub responses: Arc<Responses>,
	pub snapshot: tokio::sync::watch::Receiver<super::snapshot::TopologySnapshot>,
	/// The last lookup usage the core reported, with the gauge guards that count it.
	pub usage: parking_lot::Mutex<LookupUsage>,
	pub capacity: usize,
	pub read_slots: Arc<tokio::sync::Semaphore>,
	/// Timeout reports sent but not yet handled, shared with the `Topology` handle.
	pub timeout_reports: Arc<AtomicUsize>,
	/// Subscriptions resolved by `WatchedProcessGone`.
	pub watches: Arc<crate::watch::WatchRegistry>,
	/// Bounds watch poll reads in flight, separately from endpoint reads.
	pub watch_slots: Arc<tokio::sync::Semaphore>,
	pub _capacity_metric: rivet_metrics::IntGaugeGuard,
	pub _read_capacity_metric: rivet_metrics::IntGaugeGuard,
	pub _request_capacity_metric: rivet_metrics::IntGaugeGuard,
}

/// Endpoint cache entries and pending reads from the core's last `RecordLookupUsage`. Each
/// report replaces the guards, so this executor's share of the process-wide gauges follows its
/// cache and drops to zero with it.
#[derive(Default)]
pub struct LookupUsage {
	entries: usize,
	_entries_metric: Option<IntGaugeGuard>,
	_reads_metric: Option<IntGaugeGuard>,
}

impl Executor<TopologyStateMachine> for TopologyExecutor {
	fn execute(&self, effect: Effect, ctx: EffectContext<TopologyStateMachine>) {
		let store = self.store.clone();
		let read_slots = self.read_slots.clone();
		let (work, timeout_event, tasks, deadline_boot_ms): (
			std::pin::Pin<Box<dyn std::future::Future<Output = Event> + Send>>,
			Event,
			&Weak<TaskScope>,
			u64,
		) = match effect {
			Effect::RecordLookupUsage { entries, reads } => {
				let observed = {
					let mut usage = self.usage.lock();
					let observed = usage.entries;

					// Release the previous report before counting the new one.
					*usage = LookupUsage::default();
					*usage = LookupUsage {
						entries,
						_entries_metric: Some(
							crate::metrics::ENDPOINT_CACHE_ENTRIES.add_guard(entries as i64),
						),
						_reads_metric: Some(crate::metrics::LOOKUP_READS.add_guard(reads as i64)),
					};
					observed
				};

				let threshold = self.capacity - self.capacity / 5;

				if observed < threshold && entries >= threshold {
					tracing::warn!(
						entries,
						capacity = self.capacity,
						"topology endpoint cache near capacity; oldest entries will be evicted"
					);
				}
				return;
			}
			Effect::RecordEndpointTimeout { result } => {
				self.timeout_reports.fetch_sub(1, Ordering::AcqRel);
				let label = match result {
					EndpointTimeoutResult::Invalidated => "invalidated",
					EndpointTimeoutResult::Ignored => "ignored",
				};
				crate::metrics::ENDPOINT_TIMEOUTS
					.with_label_values(&[label])
					.inc();
				return;
			}
			Effect::RecordFence { report } => {
				record_fence(&report);
				return;
			}
			Effect::RecordRenewal {
				latency_ms,
				remaining_lease_ms,
				recovered,
			} => {
				record_renewal(latency_ms, remaining_lease_ms, recovered);
				return;
			}
			Effect::RecordRenewalLate { report } => {
				record_renewal_late(&report);
				return;
			}
			Effect::RecordIncompatibleVersion { incompatibility } => {
				tracing::error!(
					family = incompatibility.family,
					schema = rivet_data::families::VersionFamily(incompatibility.family).schema(),
					selected = incompatibility.selected,
					own_range = ?incompatibility.own_range,
					"other storage workers write a version this binary cannot read; fencing. Roll forward to a binary that reads it, since versions only move forward"
				);
				return;
			}
			Effect::RecordLookupRejected => {
				crate::metrics::LOOKUP_REJECTED.inc();
				tracing::warn!("topology lookup capacity exhausted");
				return;
			}
			Effect::WatchedProcessGone {
				process,
				subscription,
				reason,
			} => {
				let label = match reason {
					super::watch::GoneReason::Expired => "expired",
					super::watch::GoneReason::Released => "released",
					super::watch::GoneReason::Replaced => "replaced",
					super::watch::GoneReason::Missing => "missing",
				};
				crate::metrics::WATCH_GONE.with_label_values(&[label]).inc();
				if let Some(tasks) = self.tasks.upgrade() {
					let watches = self.watches.clone();
					ctx.spawn_in(&tasks, async move {
						watches.fire(&process, subscription).await;
					});
				}
				return;
			}
			Effect::CancelLookups => {
				if let Some(tasks) = self.lookups.upgrade() {
					tasks.cancel();
				}
				return;
			}
			Effect::ResolveEndpointResponse { request_id, result } => {
				self.respond(ctx, request_id, move |sender| {
					if let ResponseSender::ResolveEndpoint(sender) = sender {
						let _ = sender.send(result);
					}
				});
				return;
			}
			Effect::InvalidateEndpointResponse { request_id, result } => {
				self.respond(ctx, request_id, move |sender| {
					if let ResponseSender::InvalidateEndpoint(sender) = sender {
						let _ = sender.send(result);
					}
				});
				return;
			}
			Effect::InspectHeartbeatResponse { request_id, result } => {
				self.respond(ctx, request_id, move |sender| {
					if let ResponseSender::InspectHeartbeat(sender) = sender {
						let _ = sender.send(result);
					}
				});
				return;
			}
			Effect::ReadStorageHeartbeatRequest { worker, generation } => {
				let Some(call) = self
					.snapshot
					.borrow()
					.inspections
					.get(&worker)
					.filter(|call| call.op == generation)
					.copied()
				else {
					return;
				};
				let timeout_event = Event::ReadStorageHeartbeatResponse {
					worker: worker.clone(),
					generation,
					result: Read::Unknown,
				};
				(
					Box::pin(async move {
						{
							let result = match read_slots.try_acquire() {
								Ok(_permit) => {
									read_heartbeat(store.as_ref(), WorkerRole::Storage, &worker)
										.await
								}
								Err(_) => Read::Unknown,
							};
							Event::ReadStorageHeartbeatResponse {
								worker,
								generation,
								result,
							}
						}
					}),
					timeout_event,
					&self.lookups,
					call.deadline_boot_ms,
				)
			}
			Effect::ReadWatchedHeartbeatRequest {
				process,
				generation,
				deadline_boot_ms,
			} => {
				let watch_slots = self.watch_slots.clone();
				let timeout_event = Event::ReadWatchedHeartbeatResponse {
					process: process.clone(),
					generation,
					result: Read::Unknown,
				};
				(
					Box::pin(async move {
						let result = match watch_slots.try_acquire() {
							Ok(_permit) => {
								crate::discovery::read_watched(store.as_ref(), &process.worker_id)
									.await
							}
							Err(_) => {
								crate::metrics::WATCH_POLLS
									.with_label_values(&["overloaded"])
									.inc();
								return Event::ReadWatchedHeartbeatResponse {
									process,
									generation,
									result: Read::Unknown,
								};
							}
						};
						let label = match &result {
							Read::Found(_) => "found",
							Read::Missing => "missing",
							Read::Unknown => "unknown",
						};
						crate::metrics::WATCH_POLLS
							.with_label_values(&[label])
							.inc();
						Event::ReadWatchedHeartbeatResponse {
							process,
							generation,
							result,
						}
					}),
					timeout_event,
					&self.tasks,
					deadline_boot_ms,
				)
			}
			Effect::ReadLocalHeartbeatRequest {
				op,
				worker_registration,
			} => {
				let Some(call) = self.pending_call(op) else {
					return;
				};
				let timeout_event = Event::ReadLocalHeartbeatResponse {
					op,
					result: Read::Unknown,
				};
				(
					Box::pin(async move {
						Event::ReadLocalHeartbeatResponse {
							op,
							result: read_heartbeat(
								store.as_ref(),
								worker_registration.role,
								&worker_registration.identity.worker_id,
							)
							.await,
						}
					}),
					timeout_event,
					&self.tasks,
					call.deadline_boot_ms,
				)
			}
			Effect::WriteLocalHeartbeatRequest {
				op,
				worker_registration,
				heartbeat,
				expected_etag,
			} => {
				let Some(call) = self.pending_call(op) else {
					return;
				};
				let timeout_event = Event::WriteLocalHeartbeatResponse {
					op,
					result: Write::Unknown,
				};
				(
					Box::pin(async move {
						{
							let key = WorkerHeartbeatKey::new(
								worker_registration.role,
								worker_registration.identity.worker_id.clone(),
							);

							let mode = expected_etag.map_or(PutMode::Create, |e_tag| {
								PutMode::Update(UpdateVersion {
									e_tag: Some(e_tag),
									version: None,
								})
							});

							let result = async {
								let bytes = key.serialize(heartbeat)?;
								Ok::<_, anyhow::Error>(
									store
										.put_opts(&key.pack(), bytes.into(), mode.into())
										.await?,
								)
							}
							.await;

							let result = match result {
								Ok(result) => result.e_tag.map_or(Write::Unknown, Write::Confirmed),
								Err(error) => {
									tracing::warn!(%error, "topology heartbeat write outcome unknown; it will be retried or read back");
									Write::Unknown
								}
							};
							Event::WriteLocalHeartbeatResponse { op, result }
						}
					}),
					timeout_event,
					&self.tasks,
					call.deadline_boot_ms,
				)
			}
			Effect::ListStorageWorkersRequest {
				op,
				read_timeout_ms,
				worker_registration,
				known,
			} => {
				let Some(call) = self.pending_call(op) else {
					return;
				};
				let timeout_event = Event::ListStorageWorkersResponse { op, result: None };
				let clock = ctx.env().clock();
				(
					Box::pin(async move {
						{
							let timer = crate::metrics::REFRESH_DURATION.start_timer();

							let result = crate::discovery::list_storage(
								store.as_ref(),
								worker_registration.as_ref(),
								&known,
								clock.as_ref(),
								read_timeout_ms,
							)
							.await;
							timer.observe_duration();
							crate::metrics::REFRESHES
								.with_label_values(&[match &result {
									Ok(listing) if listing.partial => "partial",
									Ok(_) => "success",
									Err(_) => "error",
								}])
								.inc();

							if let Err(error) = &result {
								tracing::warn!(%error, "topology discovery failed");
							}
							match result {
								Ok(listing) if listing.partial => {
									Event::ListStorageWorkersPartialResponse {
										op,
										entries: listing.entries,
									}
								}
								Ok(listing) => Event::ListStorageWorkersResponse {
									op,
									result: Some(listing.entries),
								},
								Err(_) => Event::ListStorageWorkersResponse { op, result: None },
							}
						}
					}),
					timeout_event,
					&self.tasks,
					call.deadline_boot_ms,
				)
			}
			Effect::LookupEndpointRequest {
				worker,
				generation,
				first_role,
			} => {
				let Some(call) = self
					.snapshot
					.borrow()
					.lookups
					.get(&worker)
					.filter(|call| call.op == generation)
					.copied()
				else {
					return;
				};
				let timeout_event = Event::LookupEndpointResponse {
					worker: worker.clone(),
					generation,
					result: Err(RequestError::Unavailable),
					heartbeat_modified_unix_ms: None,
				};
				(
					Box::pin(async move {
						{
							let result = match read_slots.try_acquire() {
								Ok(_permit) => {
									crate::discovery::lookup_worker(
										store.as_ref(),
										&worker,
										first_role,
									)
									.await
								}
								Err(_) => {
									crate::metrics::LOOKUP_REJECTED.inc();
									tracing::warn!("topology endpoint I/O capacity exhausted");
									Err(anyhow::anyhow!("endpoint I/O capacity exhausted"))
								}
							};

							if let Err(error) = &result {
								tracing::warn!(%error, "topology endpoint read failed");
							}
							let heartbeat_modified_unix_ms = result
								.as_ref()
								.ok()
								.and_then(|found| found.as_ref())
								.map(|(_, modified)| *modified);
							Event::LookupEndpointResponse {
								worker,
								generation,
								result: result
									.map(|found| found.map(|(worker, _)| worker))
									.map_err(|_| RequestError::Unavailable),
								heartbeat_modified_unix_ms,
							}
						}
					}),
					timeout_event,
					&self.lookups,
					call.deadline_boot_ms,
				)
			}
		};
		let Some(tasks) = tasks.upgrade() else {
			return;
		};
		let inbox = ctx.inbox().clone();
		let clock = ctx.env().clock();
		ctx.spawn_in(&tasks, async move {
			// Preserve the core's original deadline even when dispatch was delayed.
			let event = if clock
				.observe()
				.map_or(true, |now| now.boot_ms >= deadline_boot_ms)
			{
				timeout_event
			} else {
				rivet_env::timeout_at(clock.as_ref(), deadline_boot_ms, work)
					.await
					.unwrap_or(timeout_event)
			};
			let _ = inbox.send(event).await;
		});
	}
}

impl TopologyExecutor {
	/// Copies one ordinary I/O record from the latest snapshot. Only effects that start I/O
	/// read the snapshot, and the borrow ends before any future is built.
	fn pending_call(&self, op: OperationId) -> Option<IoCall<OperationId>> {
		self.snapshot
			.borrow()
			.calls
			.iter()
			.find(|call| call.op == op)
			.copied()
	}

	/// Deliver a response selected by the state machine; channels stay in the executor.
	fn respond(
		&self,
		ctx: EffectContext<TopologyStateMachine>,
		request_id: u64,
		send: impl FnOnce(ResponseSender) + Send + 'static,
	) {
		if let Some(tasks) = self.tasks.upgrade() {
			let responses = self.responses.clone();
			ctx.spawn_in(&tasks, async move {
				if let Some((sender, _permit)) = responses.lock().await.remove(&request_id) {
					send(sender);
				}
			});
		}
	}
}



/// Records one confirmed renewal. Logs only the end of a late episode, so a healthy lease
/// stays silent.
fn record_renewal(latency_ms: u64, remaining_lease_ms: u64, recovered: bool) {
	crate::metrics::HEARTBEAT_RENEWAL_DURATION.observe(latency_ms as f64 / 1000.0);
	crate::metrics::LEASE_REMAINING_MS.set(i64::try_from(remaining_lease_ms).unwrap_or(i64::MAX));

	if recovered {
		tracing::info!(
			latency_ms,
			remaining_lease_ms,
			"topology heartbeat renewal confirmed in time again"
		);
	}
}

/// Warns once per late episode, while the lease can still be saved, so a slow object store
/// shows up before the fence and exit it would otherwise cause.
fn record_renewal_late(report: &RenewalLateReport) {
	crate::metrics::LEASE_REMAINING_MS
		.set(i64::try_from(report.remaining_lease_ms).unwrap_or(i64::MAX));

	tracing::warn!(
		worker_id = %report.worker_id,
		since_confirmed_send_ms = report.since_confirmed_send_ms,
		pending_renewal_wait_ms = ?report.pending_renewal_wait_ms,
		unconfirmed_writes = report.unconfirmed_writes,
		remaining_lease_ms = report.remaining_lease_ms,
		"topology heartbeat renewal is late: half the lease serving window passed without a confirmed renewal, and the worker fences if none confirms in time. Renewals wait on the object store, so check object-store latency"
	);
}

/// Logs this worker's own fence once and counts it. The coordinator and other dependents
/// exit after topology fences, so this line names the cause their exit logs do not.
fn record_fence(report: &FenceReport) {
	let reason = report.reason.as_str();
	crate::metrics::FENCES.with_label_values(&[reason]).inc();
	crate::metrics::LEASE_REMAINING_MS.set(0);

	let worker_id = report.worker_id.as_ref().map(ToString::to_string);
	match report.reason {
		FenceReason::Expired => tracing::error!(
			?worker_id,
			reason,
			last_confirmed_issued_boot_ms = ?report.last_confirmed_issued_boot_ms,
			last_confirmed_latency_ms = ?report.last_confirmed_latency_ms,
			deadline_boot_ms = ?report.deadline_boot_ms,
			now_boot_ms = report.now_boot_ms,
			unconfirmed_write_age_ms = ?report.unconfirmed_write_age_ms,
			unconfirmed_writes = report.unconfirmed_writes,
			"topology fenced this worker: lease expired before a heartbeat renewal confirmed. Renewals waited on the object store, so check object-store latency"
		),
		FenceReason::MissingHeartbeat
		| FenceReason::ForeignHeartbeat
		| FenceReason::RegistrationFailed
		| FenceReason::DependentFailed
		| FenceReason::IncompatibleVersion => tracing::error!(
			?worker_id,
			reason,
			last_confirmed_issued_boot_ms = ?report.last_confirmed_issued_boot_ms,
			deadline_boot_ms = ?report.deadline_boot_ms,
			now_boot_ms = report.now_boot_ms,
			unconfirmed_write_age_ms = ?report.unconfirmed_write_age_ms,
			"topology fenced this worker's lease"
		),
	}
}
