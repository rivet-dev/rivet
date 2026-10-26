//! Static tablet placement for a fresh cluster in one operator process.
//!
//! The hash space is split into equal tablets. Tablet `i` has up to three members in a rotating window
//! starting at `workers[i % n]`, and its first member
//! maintains it, so tablets, replicas and grants spread evenly over any number of storage workers.
//! Creates and grants are sent concurrently so the coordinator can batch them into few manifest
//! writes, and the result is checked against this plan before the command reports success.

use anyhow::{Context, Result, bail, ensure};
use futures_util::{StreamExt, TryStreamExt, stream};
use rivet_data::generated::{
	depot_tablet_coordinator_protocol_v1::{
		CreateTablet, GrantTablet, TabletCoordinatorRequest, TabletCoordinatorResponse,
	},
	depot_tablet_manifest_v1::TabletManifest,
	shared_depot_types_v1::{PartitionKeyHashRange, StorageWorkerDisk},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

/// One planned tablet: its hash range and the indices of its members, maintainer first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedTablet {
	pub start: u128,
	/// Exclusive; `None` is the end of the hash space.
	pub end: Option<u128>,
	pub members: Vec<usize>,
}

/// Equal hash ranges with rotating up-to-three-member windows over `workers` storage workers.
pub fn plan(tablets: usize, workers: usize) -> Result<Vec<PlannedTablet>> {
	ensure!(tablets > 0, "provision needs at least one partition");
	ensure!(
		workers > 0,
		"provision needs at least one storage worker, found {workers}"
	);
	let step = u128::MAX / tablets as u128;
	ensure!(step > 0, "too many partitions for the hash space");
	Ok((0..tablets)
		.map(|index| {
			// The last range ends at the end of the hash space and absorbs the remainder.
			let start = step * index as u128;
			let end = if index + 1 < tablets {
				Some(step * (index + 1) as u128)
			} else {
				None
			};
			PlannedTablet {
				start,
				end,
				members: (0..workers.min(3))
					.map(|offset| (index + offset) % workers)
					.collect(),
			}
		})
		.collect())
}

/// Maintained and replicated tablet counts per worker in a manifest.
pub fn placement(manifest: &TabletManifest) -> BTreeMap<Uuid, (usize, usize)> {
	let mut spread = BTreeMap::<Uuid, (usize, usize)>::new();
	for tablet in &manifest.tablets {
		for member in &tablet.replica_set.members {
			spread.entry(Uuid::from_bytes(*member)).or_default().1 += 1;
		}
		if let Some(maintainer) = tablet.maintainer {
			spread.entry(Uuid::from_bytes(maintainer)).or_default().0 += 1;
		}
	}
	spread
}

/// Checks that `manifest` holds exactly the planned tablets, members and maintainers.
pub fn verify(
	manifest: &TabletManifest,
	planned: &[PlannedTablet],
	workers: &[Uuid],
) -> Result<()> {
	ensure!(
		manifest.tablets.len() == planned.len(),
		"manifest has {} partitions, expected {}",
		manifest.tablets.len(),
		planned.len()
	);
	for want in planned {
		let tablet = manifest
			.tablets
			.iter()
			.find(|tablet| tablet.hash_range.start == want.start.to_be_bytes())
			.with_context(|| format!("no partition starts at {}", want.start))?;
		ensure!(
			tablet.hash_range.end == want.end.map(u128::to_be_bytes),
			"partition at {} has the wrong end",
			want.start
		);
		let members: Vec<[u8; 16]> = want
			.members
			.iter()
			.map(|index| *workers[*index].as_bytes())
			.collect();
		ensure!(
			tablet.replica_set.members == members,
			"partition at {} has unexpected members",
			want.start
		);
		ensure!(
			tablet.maintainer == Some(members[0]),
			"partition at {} is not maintained by its first member",
			want.start
		);
	}
	Ok(())
}

fn placement_json(manifest: &TabletManifest) -> Value {
	let spread = placement(manifest);
	let maintained = spread.values().map(|(maintained, _)| *maintained);
	let replicated = spread.values().map(|(_, replicated)| *replicated);
	json!({
		"partitions": manifest.tablets.len(),
		"workers": spread.len(),
		"maintained": { "min": maintained.clone().min(), "max": maintained.max() },
		"replicated": { "min": replicated.clone().min(), "max": replicated.max() },
		"perWorker": spread
			.iter()
			.map(|(worker, (maintained, replicated))| {
				(worker.to_string(), json!({ "maintained": maintained, "replicated": replicated }))
			})
			.collect::<serde_json::Map<_, _>>(),
	})
}

/// Sends coordinator requests with bounded concurrency and fails on the first rejection.
async fn send_all(
	requests: &depot_worker::coordinator_requests::CoordinatorRequests,
	batch: Vec<TabletCoordinatorRequest>,
	concurrency: usize,
) -> Result<()> {
	stream::iter(batch.into_iter().map(|request| async move {
		match requests.request(request).await? {
			TabletCoordinatorResponse::Accepted(_) => Ok(()),
			TabletCoordinatorResponse::TabletCoordinatorRejectResponse(reason) => {
				bail!("coordinator rejected a provisioning command: {reason:?}")
			}
			TabletCoordinatorResponse::StopPermitted
			| TabletCoordinatorResponse::StopPending(_) => {
				bail!("unexpected coordinator stop response to a provisioning command")
			}
		}
	}))
	.buffer_unordered(concurrency.max(1))
	.try_collect::<Vec<()>>()
	.await?;
	Ok(())
}

/// Checks that every tablet in `manifest` is one of the planned tablets with its planned
/// members, so finishing the plan never builds on a layout someone else made.
pub fn check_partial(
	manifest: &TabletManifest,
	planned: &[PlannedTablet],
	workers: &[Uuid],
) -> Result<()> {
	for tablet in &manifest.tablets {
		let want = planned
			.iter()
			.find(|want| tablet.hash_range.start == want.start.to_be_bytes())
			.with_context(|| {
				format!(
					"partition {} is not part of the provisioning plan",
					Uuid::from_bytes(tablet.id)
				)
			})?;
		let members: Vec<[u8; 16]> = want
			.members
			.iter()
			.map(|index| *workers[*index].as_bytes())
			.collect();
		ensure!(
			tablet.hash_range.end == want.end.map(u128::to_be_bytes)
				&& tablet.replica_set.members == members,
			"partition {} differs from the provisioning plan",
			Uuid::from_bytes(tablet.id)
		);
	}
	Ok(())
}

/// Creates the planned tablets the manifest does not have yet. Safe to repeat after an
/// interruption: tablets already created are matched by range start and skipped.
pub async fn create_missing(
	ctx: &rivet_engine_context::EngineContext,
	requests: &depot_worker::coordinator_requests::CoordinatorRequests,
	disks: &[StorageWorkerDisk],
	planned: &[PlannedTablet],
	concurrency: usize,
) -> Result<()> {
	let workers = worker_ids(disks);
	let existing = super::read_manifest(ctx).await?;
	check_partial(&existing, planned, &workers)?;

	let creates = planned
		.iter()
		.filter(|tablet| {
			!existing
				.tablets
				.iter()
				.any(|existing| existing.hash_range.start == tablet.start.to_be_bytes())
		})
		.map(|tablet| {
			TabletCoordinatorRequest::CreateTablet(CreateTablet {
				hash_range: PartitionKeyHashRange {
					start: tablet.start.to_be_bytes(),
					end: tablet.end.map(u128::to_be_bytes),
				},
				members: tablet
					.members
					.iter()
					.map(|index| disks[*index].clone())
					.collect(),
			})
		})
		.collect();
	send_all(requests, creates, concurrency).await
}

/// Grants every planned tablet that has no maintainer to its first member. Safe to repeat:
/// granted tablets are skipped.
pub async fn grant_ungranted(
	ctx: &rivet_engine_context::EngineContext,
	requests: &depot_worker::coordinator_requests::CoordinatorRequests,
	disks: &[StorageWorkerDisk],
	planned: &[PlannedTablet],
	concurrency: usize,
) -> Result<()> {
	let workers = worker_ids(disks);
	let created = super::read_manifest(ctx).await?;
	check_partial(&created, planned, &workers)?;

	// Tablet IDs are drawn by the coordinator, so grants are matched to the plan by range start.
	let mut grants = Vec::with_capacity(planned.len());
	for want in planned {
		let tablet = created
			.tablets
			.iter()
			.find(|tablet| tablet.hash_range.start == want.start.to_be_bytes())
			.with_context(|| format!("created partition at {} is missing", want.start))?;
		if tablet.maintainer.is_some() {
			continue;
		}
		grants.push(TabletCoordinatorRequest::GrantTablet(GrantTablet {
			tablet_id: tablet.id,
			maintainer: *workers[want.members[0]].as_bytes(),
		}));
	}
	send_all(requests, grants, concurrency).await
}

/// Verifies the manifest matches the plan and summarizes its placement.
pub async fn verify_placement(
	ctx: &rivet_engine_context::EngineContext,
	disks: &[StorageWorkerDisk],
	planned: &[PlannedTablet],
) -> Result<Value> {
	let manifest = super::read_manifest(ctx).await?;
	verify(&manifest, planned, &worker_ids(disks))?;
	Ok(json!({
		"partitionManifestVersion": manifest.tablet_manifest_version,
		"placement": placement_json(&manifest),
	}))
}

fn worker_ids(disks: &[StorageWorkerDisk]) -> Vec<Uuid> {
	disks
		.iter()
		.map(|disk| Uuid::from_bytes(disk.worker_id))
		.collect()
}

/// Creates and grants every planned tablet of a fresh cluster, then verifies the manifest
/// matches the plan.
pub async fn provision(
	ctx: &rivet_engine_context::EngineContext,
	requests: &depot_worker::coordinator_requests::CoordinatorRequests,
	disks: Vec<StorageWorkerDisk>,
	tablets: usize,
	concurrency: usize,
) -> Result<Value> {
	let planned = plan(tablets, disks.len())?;

	// A previous provision against this prefix would make the grants land on other tablets.
	let existing = super::read_manifest(ctx).await?;
	ensure!(
		existing.tablets.is_empty(),
		"the manifest already has {} partitions; provision needs a fresh object-store prefix (deploy.sh reset)",
		existing.tablets.len()
	);

	create_missing(ctx, requests, &disks, &planned, concurrency).await?;
	grant_ungranted(ctx, requests, &disks, &planned, concurrency).await?;
	verify_placement(ctx, &disks, &planned).await
}
