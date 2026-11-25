//! Storage startup completion. The owner performs one finite recovery barrier, then
//! keeps its publication alive until storage stops. Probes perform no I/O and never
//! turn ongoing writes, new actor volumes or unrelated placement into startup work.

use super::super::tablet_coordinator_machine::{store, types::ReadResult};
use crate::{
	metrics,
	tablet_maintainer_machine::types::{
		STARTUP_READINESS_SAMPLE_LIMIT, STARTUP_READINESS_WARN_AFTER_MS,
		STARTUP_READINESS_WARN_INTERVAL_MS,
	},
};
use anyhow::{Result, bail, ensure};
use depot_log::log_shard_machine::{snapshot::LogShardSnapshot, state::LogShardPhase};
use depot_types::keys::pubsub::ReplicaReadinessSubject;
use futures_util::{StreamExt, stream};
use object_store::ObjectStore;
use parking_lot::RwLock;
use rivet_data::{
	generated::{
		depot_replica_readiness_v1::ReplicaReadiness,
		depot_storage_worker_protocol_v1::StopReadinessResponse,
		depot_tablet_manifest_v1::{Tablet, TabletManifest},
	},
	versioned::{DepotReplicaReadinessData, DepotStopReadinessResponseData},
};
use rivet_env::TimeoutError;
use rivet_topology::{Topology, worker::WorkerId};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::sync::watch;
use universalpubsub::{PubSub, RequestClass, SubjectCapacities};
use vbare::Versioned;

struct Completion {
	disk: [u8; 16],
	shard: watch::Receiver<LogShardSnapshot>,
}

/// The storage service owns publication. Construction starts no background work.
#[derive(Clone, Default)]
pub struct Readiness(Arc<RwLock<Option<Completion>>>);

impl Readiness {
	/// The caller also checks the latest topology lease and shutdown signal.
	pub fn is_ready(&self) -> bool {
		let completion = self.0.read();
		completion.as_ref().is_some_and(|completion| {
			let shard = completion.shard.borrow();
			shard.phase == LogShardPhase::Serving
				&& shard.disk_incarnation == completion.disk
				&& completion.shard.has_changed().is_ok()
		})
	}
	fn invalidate(&self) {
		*self.0.write() = None;
	}
}

struct Owner(Readiness);
impl Owner {
	fn clear_metrics() {
		metrics::STARTUP_READINESS_BLOCKING_TABLETS.set(0);
		metrics::STARTUP_READINESS_BLOCKING_VOLUMES.set(0);
		metrics::STARTUP_READINESS_WAIT_SECONDS.set(0);
	}
}

impl Drop for Owner {
	fn drop(&mut self) {
		self.0.invalidate();
		Self::clear_metrics();
	}
}

/// A completed replica barrier survives a remote owner or replica-set change. A
/// new local maintainer grant still has to open and answer its own startup check.
fn same_work(previous: &Tablet, current: &Tablet, worker: [u8; 16]) -> bool {
	previous.id == current.id
		&& previous.hash_range == current.hash_range
		&& previous.replica_set.members.contains(&worker)
		&& current.replica_set.members.contains(&worker)
		&& previous
			.replica_set
			.disks
			.iter()
			.find(|disk| disk.worker_id == worker)
			== current
				.replica_set
				.disks
				.iter()
				.find(|disk| disk.worker_id == worker)
		&& (current.maintainer != Some(worker)
			|| previous.maintainer == Some(worker)
				&& previous.tablet_grant_epoch == current.tablet_grant_epoch)
}

fn assignments(manifest: &TabletManifest, worker: [u8; 16]) -> impl Iterator<Item = &Tablet> {
	manifest.tablets.iter().filter(move |tablet| {
		tablet.maintainer == Some(worker)
			|| tablet
				.replica_set
				.disks
				.iter()
				.any(|disk| disk.worker_id == worker)
	})
}

fn complete(
	manifest: &TabletManifest,
	worker: [u8; 16],
	instance: [u8; 16],
	completed: &BTreeMap<[u8; 16], Tablet>,
) -> bool {
	!manifest
		.stopping
		.iter()
		.any(|stop| stop.worker_id == worker && stop.instance_id == instance)
		&& assignments(manifest, worker).all(|tablet| {
			completed
				.get(&tablet.id)
				.is_some_and(|previous| same_work(previous, tablet, worker))
		})
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run(
	readiness: Readiness,
	pubsub: PubSub,
	topology: Arc<Topology>,
	store: Arc<dyn ObjectStore>,
	mut shard: watch::Receiver<LogShardSnapshot>,
	mut manifests: watch::Receiver<Option<TabletManifest>>,
	capacities: SubjectCapacities,
	timeout: Duration,
	mut stop: watch::Receiver<bool>,
) -> Result<()> {
	let _owner = Owner(readiness.clone());
	Owner::clear_metrics();
	let started = metrics::telemetry_elapsed();
	let clock = pubsub.env().clock();
	let mut next_warning_at = clock
		.observe()
		.ok()
		.map(|now| now.boot_ms.saturating_add(STARTUP_READINESS_WARN_AFTER_MS));
	let worker = *pubsub.worker_id().as_bytes();
	let instance = *pubsub.instance_id().as_bytes();
	let mut rng = pubsub.env().rng("depot.startup_readiness");
	let mut completed = BTreeMap::new();
	loop {
		if *stop.borrow() {
			return Ok(());
		}
		let warn = clock.observe().ok().is_some_and(|now| {
			let next_warning_at = next_warning_at
				.get_or_insert_with(|| now.boot_ms.saturating_add(STARTUP_READINESS_WARN_AFTER_MS));
			if now.boot_ms < *next_warning_at {
				return false;
			}
			*next_warning_at = now
				.boot_ms
				.saturating_add(STARTUP_READINESS_WARN_INTERVAL_MS);
			true
		});
		let disk = shard.borrow().disk_incarnation;
		let round = async {
			ensure!(
				topology.may_admit()? && shard.borrow().phase == LogShardPhase::Serving,
				"storage is not serving"
			);
			let manifest = read_manifest(store.as_ref()).await?;
			let work = assignments(&manifest, worker)
				.filter(|tablet| {
					!completed
						.get(&tablet.id)
						.is_some_and(|previous| same_work(previous, tablet, worker))
				})
				.cloned()
				.collect::<Vec<_>>();
			let mut blocking_tablets = work.len();
			let mut reported_blocking_volumes = 0u64;
			let checks = stream::iter(work.into_iter().map(|tablet| {
				let pubsub = &pubsub;
				let topology = topology.as_ref();
				async move {
					let result = check(
						&pubsub, &topology, capacities, &tablet, worker, instance, disk, timeout,
					)
					.await;
					(tablet, result)
				}
			}))
			.buffer_unordered(8);
			tokio::pin!(checks);
			while let Some((tablet, result)) = checks.next().await {
				let label = check_result(&result);
				metrics::STARTUP_READINESS_CHECKS_TOTAL
					.with_label_values(&[label])
					.inc();
				if matches!(result, Ok(Some(StopReadinessResponse::Ready))) {
					blocking_tablets -= 1;
					completed.insert(tablet.id, tablet);
					continue;
				}
				let details = match &result {
					Ok(Some(StopReadinessResponse::NotReady(details))) => {
						reported_blocking_volumes =
							reported_blocking_volumes.saturating_add(details.blocking_volumes);
						Some(details)
					}
					_ => None,
				};
				if warn {
					tracing::warn!(
						tablet_id = %uuid::Uuid::from_bytes(tablet.id),
						maintainer = ?tablet.maintainer.map(WorkerId::from_bytes),
						joining_worker = %pubsub.worker_id(),
						result = label,
						blocking_volumes = details.map(|details| details.blocking_volumes),
						sample = ?details.map(|details| {
							&details.sample[..details.sample.len().min(STARTUP_READINESS_SAMPLE_LIMIT)]
						}),
						error = ?result.as_ref().err(),
						"storage startup is waiting for tablet readiness"
					);
				}
			}
			metrics::STARTUP_READINESS_BLOCKING_TABLETS
				.set(i64::try_from(blocking_tablets).unwrap_or(i64::MAX));
			metrics::STARTUP_READINESS_BLOCKING_VOLUMES
				.set(i64::try_from(reported_blocking_volumes).unwrap_or(i64::MAX));
			let current = read_manifest(store.as_ref()).await?;
			Ok::<_, anyhow::Error>(
				complete(&current, worker, instance, &completed)
					&& topology.may_admit()?
					&& shard.borrow().phase == LogShardPhase::Serving
					&& shard.borrow().disk_incarnation == disk,
			)
		};
		let result = rivet_env::select_biased!("storage shutdown precedes startup completion",
			changed = stop.changed() => { let _ = changed; return Ok(()); },
			result = rivet_env::timeout(clock.as_ref(), timeout, round) => result,
		);
		let (round_result, error): (_, Option<&dyn std::fmt::Display>) = match &result {
			Ok(Ok(true)) => ("ready", None),
			Ok(Ok(false)) => ("blocked", None),
			Ok(Err(error)) => ("error", Some(error)),
			Err(error @ TimeoutError::Elapsed) => ("timeout", Some(error)),
			Err(error @ TimeoutError::Clock(_)) => ("clock_error", Some(error)),
		};
		metrics::STARTUP_READINESS_ROUNDS_TOTAL
			.with_label_values(&[round_result])
			.inc();
		if warn && let Some(error) = error {
			tracing::warn!(result = round_result, %error, "storage startup readiness round failed");
		}
		let elapsed = metrics::telemetry_elapsed().saturating_sub(started);
		if matches!(result, Ok(Ok(true))) {
			Owner::clear_metrics();
			metrics::STARTUP_READINESS_DURATION_SECONDS.observe(elapsed.as_secs_f64());
			drop(completed);
			*readiness.0.write() = Some(Completion {
				disk,
				shard: shard.clone(),
			});
			// Completion belongs to this storage lifetime; it does not expire because
			// another worker moves a tablet or because a later status RPC is slow.
			loop {
				rivet_env::select_biased!("storage shutdown precedes observing a completed startup",
					changed = stop.changed() => { let _ = changed; return Ok(()); },
					changed = shard.changed() => { changed?; ensure!(shard.borrow().phase == LogShardPhase::Serving && shard.borrow().disk_incarnation == disk, "storage left its completed startup lifetime"); },
				);
			}
		}
		metrics::STARTUP_READINESS_WAIT_SECONDS
			.set(i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX));
		let next = clock
			.observe()?
			.boot_ms
			.saturating_add(rng.gen_range(2_500..5_000));
		rivet_env::select_biased!("shutdown and assignment changes precede another startup check",
			changed = stop.changed() => { let _ = changed; return Ok(()); },
			changed = manifests.changed() => { changed?; },
			result = clock.sleep_until(next) => result?,
		);
	}
}

fn check_result(result: &Result<Option<StopReadinessResponse>>) -> &'static str {
	match result {
		Ok(Some(StopReadinessResponse::Ready)) => "ready",
		Ok(Some(StopReadinessResponse::NotReady(_))) => "not_ready",
		Ok(Some(StopReadinessResponse::NotServing)) => "not_serving",
		Ok(Some(StopReadinessResponse::Overloaded(_))) => "overloaded",
		Ok(Some(StopReadinessResponse::StaleTabletGrantEpoch(_))) => "stale_grant",
		Ok(Some(StopReadinessResponse::StaleTabletReplicaSet(_))) => "stale_replica_set",
		Ok(None) => "not_assigned",
		Err(_) => "error",
	}
}

async fn read_manifest(store: &dyn ObjectStore) -> Result<TabletManifest> {
	match store::read(store).await {
		ReadResult::Found(stored) => Ok(stored.manifest),
		ReadResult::Missing => Ok(TabletManifest {
			tablet_manifest_version: 0,
			tablets: vec![],
			stopping: vec![],
			retired_tablets: vec![],
			paused: vec![],
		}),
		_ => bail!("startup readiness could not read the tablet manifest"),
	}
}

#[allow(clippy::too_many_arguments)]
async fn check(
	pubsub: &PubSub,
	topology: &Topology,
	capacities: SubjectCapacities,
	tablet: &Tablet,
	worker: [u8; 16],
	instance: [u8; 16],
	disk: [u8; 16],
	timeout: Duration,
) -> Result<Option<StopReadinessResponse>> {
	if !tablet.replica_set.members.contains(&worker)
		|| !tablet
			.replica_set
			.disks
			.iter()
			.any(|entry| entry.worker_id == worker && entry.disk_incarnation == disk)
	{
		return Ok(None);
	}
	let maintainer = WorkerId::from_bytes(
		tablet
			.maintainer
			.ok_or_else(|| anyhow::anyhow!("tablet has no maintainer"))?,
	);
	let subject = ReplicaReadinessSubject::new(maintainer.clone());
	let _reservation =
		pubsub.reserve_request_subject(&subject, RequestClass::DepotStopReadiness, capacities)?;
	let request = ReplicaReadiness {
		tablet_id: tablet.id,
		tablet_grant_epoch: tablet.tablet_grant_epoch,
		tablet_replica_set_version: tablet.replica_set.tablet_replica_set_version,
		worker_id: worker,
		worker_incarnation: instance,
		disk_incarnation: disk,
	};
	let payload = DepotReplicaReadinessData::serialize_with_embedded_version(
		&request,
		rivet_data::generated::DEPOT_REPLICA_READINESS_VERSION,
	)?;
	let bytes = pubsub
		.request_with_timeout_guarded(
			RequestClass::DepotStopReadiness,
			&maintainer,
			subject,
			&payload,
			timeout,
			&|| topology.may_admit().unwrap_or(false),
		)
		.await?;
	Ok(Some(
		DepotStopReadinessResponseData::deserialize_with_embedded_version(&bytes)?,
	))
}
