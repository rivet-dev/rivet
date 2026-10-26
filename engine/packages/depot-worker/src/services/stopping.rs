//! Permit waiting leaves UPS and topology admission Active until coordinated teardown.

use crate::{coordinator_requests::CoordinatorRequests, metrics};
use anyhow::Result;
use futures_util::FutureExt;
use rivet_config::Config;
use rivet_data::generated::depot_tablet_coordinator_protocol_v1::{
	RequestStop, StopPending, TabletCoordinatorRequest, TabletCoordinatorResponse,
};
use rivet_env::capabilities::shutdown::Shutdown;
use rivet_singleton_coordinator::{
	singleton_coordinator_machine::types::Config as SingletonConfig, store::ObjectLeaseStore,
};
use rivet_topology::{Topology, config::LeaseTiming};
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
	time::Duration,
};
use universalpubsub::{PubSub, SubjectCapacities};

/// Volatile process phase. The durable intent and permit belong to the coordinator.
struct Stopping {
	timer: Option<rivet_metrics::prometheus::HistogramTimer>,
	result: &'static str,
	progress_at: u64,
	warned_at: u64,
	pending: Option<StopPending>,
}
impl Stopping {
	fn new(now: u64) -> Self {
		Self {
			timer: Some(
				metrics::STOP_DURATION
					.with_label_values(&["aborted"])
					.start_timer(),
			),
			result: "aborted",
			progress_at: now,
			warned_at: now,
			pending: None,
		}
	}
	fn observe(&mut self, pending: StopPending, now: u64) {
		let progressed = self
			.pending
			.as_ref()
			.is_none_or(|old| progress(old, &pending));
		if progressed {
			self.progress_at = now;
			self.warned_at = now;
		}
		metrics::STOP_BLOCKING.set(pending.total_blocking_tablets.min(i64::MAX as u64) as i64);
		metrics::STOP_NO_PROGRESS.set(now.saturating_sub(self.progress_at) as f64 / 1000.0);
		for blocker in pending.blockers.iter().take(16) {
			let percentage = blocker.detail.as_ref().map(|detail| {
				detail.appended_seq.saturating_mul(100) / detail.committed_seq.max(1)
			});
			tracing::info!(tablet = %uuid::Uuid::from_bytes(blocker.tablet_id), reason = ?blocker.reason, volumes_behind = blocker.volumes_behind, detail = ?blocker.detail, percentage, total_blocking_tablets = pending.total_blocking_tablets, "stop pending");
		}
		self.pending = Some(pending);
		self.warn(now);
	}
	fn warn(&mut self, now: u64) {
		metrics::STOP_NO_PROGRESS.set(now.saturating_sub(self.progress_at) as f64 / 1000.0);
		if now.saturating_sub(self.warned_at) >= 60_000 {
			tracing::warn!(seconds_without_progress = now.saturating_sub(self.progress_at) / 1000, blockers = ?self.pending.as_ref().map(|p| p.blockers.iter().take(16).collect::<Vec<_>>()), "storage stop has made no progress");
			self.warned_at = now;
		}
	}
}
impl Drop for Stopping {
	fn drop(&mut self) {
		let elapsed = self.timer.take().expect("stop timer").stop_and_discard();
		metrics::STOP_DURATION
			.with_label_values(&[self.result])
			.observe(elapsed);
		metrics::STOP_BLOCKING.set(0);
		metrics::STOP_NO_PROGRESS.set(0.0);
	}
}

fn progress(old: &StopPending, new: &StopPending) -> bool {
	let before: BTreeSet<_> = old.blockers.iter().map(|b| b.tablet_id).collect();
	let after: BTreeSet<_> = new.blockers.iter().map(|b| b.tablet_id).collect();
	let seqs: BTreeMap<_, _> = old
		.blockers
		.iter()
		.filter_map(|b| {
			b.detail.as_ref().map(|d| {
				(
					(
						b.tablet_id,
						d.member,
						depot_types::VolumeLogKey(d.volume_log_key.clone()),
					),
					d.appended_seq,
				)
			})
		})
		.collect();
	(after.len() < before.len() && after.is_subset(&before))
		|| new.blockers.iter().any(|b| {
			b.detail.as_ref().is_some_and(|d| {
				seqs.get(&(
					b.tablet_id,
					d.member,
					depot_types::VolumeLogKey(d.volume_log_key.clone()),
				))
				.is_some_and(|seq| d.appended_seq > *seq)
			})
		})
}

/// The share of the force-shutdown budget a graceful stop reserves for handing the singleton
/// tenure over, or `None` when the budget cannot hold the whole stop wait, a whole step-down
/// and the teardown. A step-down serves until shortly before its published deadline, so one
/// that does not fit would run past the budget and the forced exit would fence this worker.
/// Without the reserve the worker skips the step-down and closing its storage releases the
/// tenure, which a candidate can claim at once.
pub fn singleton_handover_reserve_ms(config: &Config, lease: &LeaseTiming) -> Option<u64> {
	let settings = config.depot();
	let step_down_ms = SingletonConfig::from_topology_timing(lease).step_down_ms;
	let needed_ms = settings
		.stop_timeout_ms()
		.saturating_add(step_down_ms)
		.saturating_add(settings.stop_teardown_ms());
	let budget_ms =
		u64::try_from(config.runtime.force_shutdown_duration().as_millis()).unwrap_or(u64::MAX);
	if budget_ms >= needed_ms {
		Some(step_down_ms)
	} else {
		None
	}
}

/// Called by before_drain after closing volume clients, before the singleton handover. No
/// service or topology admission state changes until this returns. Every retry is explicitly
/// owned here.
pub async fn wait_for_permit(
	config: &Config,
	pubsub: PubSub,
	topology: Arc<Topology>,
	store: Arc<dyn object_store::ObjectStore>,
	shutdown: &Shutdown,
) -> Result<()> {
	let settings = config.depot();
	let mut signal = shutdown.clone();
	if signal.recv().now_or_never().transpose()?.is_none() {
		// Service failure without a shutdown signal already revokes serving authority.
		return Ok(());
	}
	let stop_deadline = shutdown.deadline(Duration::from_millis(settings.stop_timeout_ms()))?;
	// The singleton handover runs after this wait and serves until shortly before its
	// step-down deadline, so the teardown reserve must also leave room for a whole step-down.
	let handover_reserve_ms =
		singleton_handover_reserve_ms(config, &topology.config().lease).unwrap_or(0);
	let deadline = stop_deadline.min(
		shutdown
			.deadline(config.runtime.force_shutdown_duration())?
			.saturating_sub(settings.stop_teardown_ms())
			.saturating_sub(handover_reserve_ms),
	);
	let clock = pubsub.env().clock();
	let mut phase = Stopping::new(clock.observe()?.boot_ms);
	let request = RequestStop {
		worker_id: *pubsub.worker_id().as_bytes(),
		instance_id: *pubsub.instance_id().as_bytes(),
	};
	let capacity = settings.coordinator_request_capacity();
	let requests = CoordinatorRequests::new(
		pubsub,
		topology,
		Arc::new(ObjectLeaseStore::new(store)),
		SubjectCapacities {
			incoming: capacity,
			outgoing: capacity,
		},
		Duration::from_millis(settings.volume_request_timeout_ms()),
	)?;
	tracing::info!(
		deadline_boot_ms = deadline,
		"storage worker waiting for stop permit; admission remains active"
	);
	loop {
		let now = clock.observe()?.boot_ms;
		if now >= deadline {
			break;
		}
		let response = rivet_env::timeout_at(
			clock.as_ref(),
			deadline,
			requests.request(TabletCoordinatorRequest::RequestStop(request.clone())),
		)
		.await;
		match response {
			Ok(Ok(TabletCoordinatorResponse::StopPermitted)) => {
				phase.result = "permitted";
				tracing::info!("storage worker received durable stop permit");
				return Ok(());
			}
			Ok(Ok(TabletCoordinatorResponse::StopPending(pending))) => {
				phase.observe(pending, clock.observe()?.boot_ms)
			}
			other => tracing::warn!(result = ?other, "stop request failed; retrying while serving"),
		}
		phase.warn(clock.observe()?.boot_ms);
		let _ = rivet_env::timeout_at(
			clock.as_ref(),
			deadline,
			rivet_env::sleep(
				clock.as_ref(),
				Duration::from_millis(settings.stop_poll_ms()),
			),
		)
		.await;
	}
	phase.result = "timeout";
	if let Some(pending) = &phase.pending {
		for blocker in &pending.blockers {
			tracing::error!(tablet = %uuid::Uuid::from_bytes(blocker.tablet_id), reason = ?blocker.reason, detail = ?blocker.detail, "stop timeout: tearing down without quorum protection");
		}
	} else {
		tracing::error!(
			"stop timeout without coordinator evidence: tearing down without quorum protection"
		);
	}
	Ok(())
}


