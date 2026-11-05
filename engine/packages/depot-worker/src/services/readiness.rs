//! Storage startup completion. The owner performs one finite recovery barrier, then
//! keeps its publication alive until storage stops. Probes perform no I/O and never
//! turn ongoing writes, new actor volumes or unrelated placement into startup work.

use super::super::tablet_coordinator_machine::{store, types::ReadResult};
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
impl Drop for Owner {
	fn drop(&mut self) {
		self.0.invalidate();
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
	let clock = pubsub.env().clock();
	let worker = *pubsub.worker_id().as_bytes();
	let instance = *pubsub.instance_id().as_bytes();
	let mut rng = pubsub.env().rng("depot.startup_readiness");
	let mut completed = BTreeMap::new();
	loop {
		if *stop.borrow() {
			return Ok(());
		}
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
			let checks = stream::iter(work.into_iter().map(|tablet| {
				let pubsub = &pubsub;
				let topology = topology.as_ref();
				async move {
					let ready = check(
						&pubsub, &topology, capacities, &tablet, worker, instance, disk, timeout,
					)
					.await;
					(tablet, ready)
				}
			}))
			.buffer_unordered(8);
			tokio::pin!(checks);
			while let Some((tablet, result)) = checks.next().await {
				if result.unwrap_or(false) {
					completed.insert(tablet.id, tablet);
				}
			}
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
		if matches!(result, Ok(Ok(true))) {
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
) -> Result<bool> {
	if !tablet.replica_set.members.contains(&worker)
		|| !tablet
			.replica_set
			.disks
			.iter()
			.any(|entry| entry.worker_id == worker && entry.disk_incarnation == disk)
	{
		return Ok(false);
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
	Ok(
		DepotStopReadinessResponseData::deserialize_with_embedded_version(&bytes)?
			== StopReadinessResponse::Ready,
	)
}
