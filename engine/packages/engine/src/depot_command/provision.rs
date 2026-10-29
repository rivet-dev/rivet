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

/// Grants every planned tablet to its first member. Safe to repeat after an accepted grant:
/// tablets already maintained by their planned member are skipped.
pub async fn grant_planned(
	ctx: &rivet_engine_context::EngineContext,
	requests: &depot_worker::coordinator_requests::CoordinatorRequests,
	disks: &[StorageWorkerDisk],
	planned: &[PlannedTablet],
	concurrency: usize,
) -> Result<()> {
	let workers = worker_ids(disks);
	let created = super::read_manifest(ctx).await?;
	let grants = grants_needed(&created, planned, &workers)?;
	send_all(requests, grants, concurrency).await
}

/// The planned grants still missing from an otherwise unchanged layout.
fn grants_needed(
	manifest: &TabletManifest,
	planned: &[PlannedTablet],
	workers: &[Uuid],
) -> Result<Vec<TabletCoordinatorRequest>> {
	check_partial(manifest, planned, workers)?;
	// Tablet IDs are drawn by the coordinator, so grants are matched to the plan by range start.
	let mut grants = Vec::with_capacity(planned.len());
	for want in planned {
		let tablet = manifest
			.tablets
			.iter()
			.find(|tablet| tablet.hash_range.start == want.start.to_be_bytes())
			.with_context(|| format!("created partition at {} is missing", want.start))?;
		if tablet.maintainer == Some(*workers[want.members[0]].as_bytes()) {
			continue;
		}
		grants.push(TabletCoordinatorRequest::GrantTablet(GrantTablet {
			tablet_id: tablet.id,
			maintainer: *workers[want.members[0]].as_bytes(),
		}));
	}
	Ok(grants)
}

/// Reconciles the static placement within one injected-clock deadline. A successful grant
/// may already have been replaced by automatic failover using a startup discovery snapshot.
async fn reconcile_grants<Read, ReadFuture, Send, SendFuture>(
	clock: &dyn rivet_env::Clock,
	deadline_boot_ms: u64,
	planned: &[PlannedTablet],
	workers: &[Uuid],
	mut read: Read,
	mut send: Send,
) -> Result<TabletManifest>
where
	Read: FnMut() -> ReadFuture,
	ReadFuture: std::future::Future<Output = Result<TabletManifest>>,
	Send: FnMut(Vec<TabletCoordinatorRequest>) -> SendFuture,
	SendFuture: std::future::Future<Output = Result<()>>,
{
	rivet_env::timeout_at(clock, deadline_boot_ms, async {
		let mut delay_ms = 20;
		loop {
			let manifest = read().await?;
			let grants = grants_needed(&manifest, planned, workers)?;
			if grants.is_empty() {
				verify(&manifest, planned, workers)?;
				return Ok(manifest);
			}
			// A transport error has an unknown outcome: return it rather than retrying.
			// Only accepted grants may be reconciled against another manifest snapshot.
			send(grants).await?;
			rivet_env::sleep(clock, std::time::Duration::from_millis(delay_ms)).await?;
			delay_ms = (delay_ms * 2).min(1_000);
		}
	})
	.await
	.context("provisioning placement did not converge before the deadline")?
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
	let clock = ctx.env().clock();
	let deadline = rivet_env::deadline_after(
		clock.as_ref(),
		std::time::Duration::from_millis(ctx.config().depot().volume_request_timeout_ms()),
	)?;
	let manifest = reconcile_grants(
		clock.as_ref(),
		deadline,
		&planned,
		&worker_ids(&disks),
		|| super::read_manifest(ctx),
		|grants| send_all(requests, grants, concurrency),
	)
	.await?;
	Ok(
		json!({ "partitionManifestVersion": manifest.tablet_manifest_version, "placement": placement_json(&manifest) }),
	)
}


