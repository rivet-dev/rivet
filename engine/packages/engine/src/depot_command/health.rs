//! Bounded, unfenced observations of assigned disks and tablet maintainer logs.

use anyhow::{Result, ensure};
use depot_volume::ups::UpsLogClient;
use futures_util::{StreamExt, stream};
use rivet_data::generated::{
	depot_storage_worker_protocol_v1::{
		ReadVolumeLogStatusRequest, StorageWorkerRequest, StorageWorkerResponse, VolumeLogStatus,
	},
	depot_tablet_manifest_v1::{Tablet, TabletManifest},
	shared_depot_types_v1::{TabletPhase, VolumeLogKey, VolumeLogReplicaStatus},
};
use rivet_engine_context::EngineContext;
use rivet_topology::worker::WorkerId;
use serde_json::{Value, json};
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
	time::Duration,
};
use universalpubsub::SubjectCapacities;
use uuid::Uuid;

const MAX_TABLETS: usize = 4096;
const MAX_WORKERS: usize = 256;
const STATUS_BATCH: usize = 64;
const CONCURRENCY: usize = 8;
const TOTAL_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Default)]
struct Observation {
	disk: Option<Result<[u8; 16], String>>,
	logs: BTreeMap<[u8; 16], Result<VolumeLogStatus, String>>,
}

pub(crate) async fn execute(ctx: &EngineContext, selected: Option<Uuid>) -> Result<Value> {
	// A failed or expired sample is never a healthy observation. No retries hide outages.
	rivet_env::timeout(
		ctx.env().clock().as_ref(),
		TOTAL_TIMEOUT,
		sample(ctx, selected),
	)
	.await?
}

async fn sample(ctx: &EngineContext, selected: Option<Uuid>) -> Result<Value> {
	let before = super::read_manifest(ctx).await?;
	let tablets: Vec<_> = before
		.tablets
		.iter()
		.filter(|tablet| selected.is_none_or(|id| id.as_bytes() == &tablet.id))
		.collect();
	ensure!(!tablets.is_empty(), "no matching partitions");
	ensure!(
		tablets.len() <= MAX_TABLETS,
		"too many partitions; select one partition per command"
	);
	let mut workers: BTreeMap<[u8; 16], Vec<([u8; 16], VolumeLogKey)>> = BTreeMap::new();
	for tablet in &tablets {
		let log = VolumeLogKey {
			volume_key: depot_worker::keys::partition::MaintainerLogVolume::new(
				tablet.id,
				tablet.hash_range.start,
			)
			.volume_key(),
			volume_incarnation: tablet.id,
		};
		for worker in destinations(tablet) {
			workers
				.entry(worker)
				.or_default()
				.push((tablet.id, log.clone()));
		}
	}
	ensure!(
		workers.len() <= MAX_WORKERS,
		"too many assigned workers; select one partition per command"
	);
	let topology = ctx.topology();
	let client = UpsLogClient::new(
		ctx.ups()?,
		SubjectCapacities {
			incoming: 64,
			outgoing: 64,
		},
		Duration::from_millis(ctx.config().depot().volume_request_timeout_ms())
			.min(Duration::from_secs(5)),
		Arc::new(move || topology.may_admit().unwrap_or(false)),
	)?;
	// UPS resolves only each exact worker ID, under its ordinary lease and reply checks.
	let observations: BTreeMap<_, _> = stream::iter(workers)
		.map(|(worker, logs)| {
			let client = client.clone();
			async move {
				let id = WorkerId::from_bytes(worker);
				let disk = match client
					.request(&id, StorageWorkerRequest::GetDiskIncarnationRequest)
					.await
				{
					Ok(StorageWorkerResponse::GetDiskIncarnationResponse(reply)) => {
						Ok(reply.disk_incarnation)
					}
					Ok(other) => Err(format!("{other:?}")),
					Err(error) => Err(format!("{error:#}")),
				};
				let mut observation = Observation {
					disk: Some(disk),
					..Default::default()
				};
				if matches!(observation.disk, Some(Ok(_))) {
					for batch in logs.chunks(STATUS_BATCH) {
						let result = client
							.request(
								&id,
								StorageWorkerRequest::ReadVolumeLogStatusRequest(
									ReadVolumeLogStatusRequest {
										volume_log_keys: batch
											.iter()
											.map(|(_, key)| key.clone())
											.collect(),
									},
								),
							)
							.await;
						let error = match result {
							Ok(StorageWorkerResponse::ReadVolumeLogStatusResponse(reply))
								if reply.statuses.len() == batch.len() =>
							{
								for ((tablet, _), status) in batch.iter().zip(reply.statuses) {
									observation.logs.insert(*tablet, Ok(status));
								}
								continue;
							}
							Ok(other) => format!("unexpected status response: {other:?}"),
							Err(error) => format!("{error:#}"),
						};
						for (tablet, _) in batch {
							observation.logs.insert(*tablet, Err(error.clone()));
						}
					}
					// Status replies do not carry disk identity. Bracket them so a replacement
					// disk cannot lend its new replica states to the old disk's observation.
					let after = match client
						.request(&id, StorageWorkerRequest::GetDiskIncarnationRequest)
						.await
					{
						Ok(StorageWorkerResponse::GetDiskIncarnationResponse(reply)) => {
							Ok(reply.disk_incarnation)
						}
						Ok(other) => Err(format!("{other:?}")),
						Err(error) => Err(format!("{error:#}")),
					};
					if observation.disk.as_ref() != Some(&after) {
						observation.disk = Some(Err(format!(
							"disk identity or serving state changed during status reads: {after:?}"
						)));
					}
				}
				(worker, observation)
			}
		})
		.buffer_unordered(CONCURRENCY)
		.collect()
		.await;
	let after = super::read_manifest(ctx).await?;
	Ok(report(&before, &after, &tablets, &observations))
}

fn destinations(tablet: &Tablet) -> BTreeSet<[u8; 16]> {
	let next: &[[u8; 16]] = match &tablet.replica_set.phase {
		TabletPhase::Stable => &[],
		TabletPhase::Learning(phase) => &phase.next,
		TabletPhase::Joint(phase) => &phase.next,
	};
	tablet
		.replica_set
		.members
		.iter()
		.chain(next)
		.copied()
		.collect()
}

fn report(
	before: &TabletManifest,
	after: &TabletManifest,
	tablets: &[&Tablet],
	observations: &BTreeMap<[u8; 16], Observation>,
) -> Value {
	let coherent = before == after;
	let mut healthy = coherent && !tablets.is_empty();
	let tablets: Vec<_> = tablets.iter().map(|tablet| {
		let mut available = BTreeSet::new();
		let mut in_sync = BTreeSet::new();
		let workers: Vec<_> = destinations(tablet).into_iter().map(|worker| {
			let expected: Vec<_> = tablet.replica_set.disks.iter().filter(|disk| disk.worker_id == worker).collect();
			let observation = observations.get(&worker);
			let observed = observation.and_then(|o| o.disk.as_ref()).and_then(|disk| disk.as_ref().ok());
			let disk_matches = expected.len() == 1 && expected[0].disk_incarnation != [0; 16]
				&& observed == Some(&expected[0].disk_incarnation);
			let status = observation.and_then(|o| o.logs.get(&tablet.id));
			let synced = disk_matches && matches!(status, Some(Ok(VolumeLogStatus::BelowCold(reply)))
				if reply.state.status == VolumeLogReplicaStatus::InSync && !reply.state.lost_entries);
			if disk_matches { available.insert(worker); }
			if synced { in_sync.insert(worker); }
			json!({
				"workerId": Uuid::from_bytes(worker),
				"expectedDiskIncarnations": expected.iter().map(|disk| Uuid::from_bytes(disk.disk_incarnation)).collect::<Vec<_>>(),
				"observedDiskIncarnation": observed.map(|disk| Uuid::from_bytes(*disk)),
				"membershipAvailable": disk_matches,
				"maintainerLogInSync": synced,
				"diskError": observation.and_then(|o| o.disk.as_ref()).and_then(|disk| disk.as_ref().err()),
				"maintainerLogStatus": match status {
					Some(Ok(status)) => json!(status),
					Some(Err(error)) => json!({"Unknown": error}),
					None => json!({"Unknown": "no status observation"}),
				},
			})
		}).collect();
		let count = |members: &[[u8; 16]]| {
			let members: BTreeSet<_> = members.iter().copied().collect();
			json!({"assigned": members.len(), "membershipAvailable": members.intersection(&available).count(),
				"maintainerLogInSync": members.intersection(&in_sync).count()})
		};
		let members = count(&tablet.replica_set.members);
		let next = match &tablet.replica_set.phase {
			TabletPhase::Stable => Value::Null,
			TabletPhase::Learning(phase) => count(&phase.next),
			TabletPhase::Joint(phase) => count(&phase.next),
		};
		let stable = matches!(tablet.replica_set.phase, TabletPhase::Stable) && tablet.split.is_none();
		let tablet_healthy = coherent && stable && members["membershipAvailable"].as_u64().unwrap_or(0) >= 3
			&& members["maintainerLogInSync"].as_u64().unwrap_or(0) >= 3;
		healthy &= tablet_healthy;
		json!({"partitionId": Uuid::from_bytes(tablet.id), "partitionReplicaSetVersion": tablet.replica_set.tablet_replica_set_version,
			"phase": tablet.replica_set.phase, "stable": stable, "healthy": tablet_healthy,
			"members": members, "next": next, "workers": workers})
	}).collect();
	json!({"scope": "assigned disks and maintainer logs; not all application volumes", "healthy": healthy,
		"manifestCoherent": coherent, "partitionManifestVersion": before.tablet_manifest_version,
		"afterPartitionManifestVersion": after.tablet_manifest_version, "partitions": tablets})
}


