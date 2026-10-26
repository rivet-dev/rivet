//! Cold-store fencing precedes paginated discovery on a member quorum.

use super::{LoadedVolume, MaintainerPhase, MaintainerState, Operation, RecoveryFailure, fence};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::*};
use depot_types::{VolumeLogKey, listing_order, partition_key_hash};
use rivet_data::generated::{
	depot_storage_worker_protocol_v1::{self as wire, StorageWorkerRequest, StorageWorkerResponse},
	shared_depot_types_v1::VolumeLogReplicaStatus,
};
use rivet_topology::worker::WorkerId;

/// Opening establishes the object-store fence before any shard grant is raised.
pub(crate) fn start(state: &mut MaintainerState, effects: &mut Effects) {
	if state.phase != MaintainerPhase::Opening || !state.operations.is_empty() {
		return;
	}
	let op_id = state.begin(Operation::OpenCold, effects);
	effects.execute(MaintainerEffect::OpenColdRequest {
		op_id,
		tablet: state.tablet.clone(),
		rows_version: state.config.write_versions.fs_rows,
	});
}

/// Only the current store-open completion can begin member discovery.
pub(crate) fn opened(
	state: &mut MaintainerState,
	id: OpId,
	result: Result<
		Option<rivet_data::generated::depot_maintainer_log_v1::MaintainerLogRecord>,
		String,
	>,
	effects: &mut Effects,
) {
	if !matches!(state.operations.get(&id), Some(Operation::OpenCold)) {
		return;
	}
	state.finish(id, effects);
	let record = match result {
		Ok(record) => record,
		Err(error) => {
			tracing::warn!(%error, "tablet cold store open failed; fencing the maintainer");
			fence(state, effects);
			return;
		}
	};
	// The record holds the highest version of each family in the tablet's data. Opening
	// data this binary cannot read would misread it, so refuse before replay.
	if let Some(record) = &record
		&& let Err(error) = rivet_data::families::check_marks(&record.version_marks)
	{
		effects.execute(MaintainerEffect::RecordVersionRefused {
			family: error.family.0,
			mark: error.mark,
			max_read: error.max_read,
		});
		// Tell the coordinator first, so it stops granting the tablet to workers like this
		// one. The report is bounded by its operation timer and fences on any outcome.
		if !super::version_marks::report_refusal(state, &record.version_marks, effects) {
			fence(state, effects);
		}
		return;
	}
	super::maintainer_log::record_read(state, record);
	for worker in state.members() {
		list(state, worker, None, effects);
	}
}

/// Each page carries the same grant, range and retired parent as the admitted assignment.
pub(crate) fn list(
	state: &mut MaintainerState,
	worker: WorkerId,
	after: Option<VolumeLogKey>,
	effects: &mut Effects,
) {
	if matches!(
		state.phase,
		MaintainerPhase::Fenced | MaintainerPhase::Stopped | MaintainerPhase::Draining
	) || state.listed.contains(&worker)
		|| !state.members().contains(&worker)
	{
		return;
	}
	let request =
		StorageWorkerRequest::ListTabletReplicasRequest(wire::ListTabletReplicasRequest {
			tablet_replica_set_version: state.tablet.replica_set.tablet_replica_set_version,
			tablet_grant: state.grant(),
			hash_range: state.tablet.hash_range,
			retired_parent: state.tablet.parent_tablet_id,
			after: after.clone().map(Into::into),
			max_replicas: state.config.tuning().listing_page_size,
		});
	let op_id = state.begin(
		Operation::List {
			worker: worker.clone(),
			after,
		},
		effects,
	);
	effects.execute(MaintainerEffect::ReplicaRequest {
		op_id,
		worker_id: worker,
		request,
	});
}

/// Complete listing pages, rather than individual keys, establish opening quorum evidence.
pub(crate) fn listed(
	state: &mut MaintainerState,
	worker: WorkerId,
	after: Option<VolumeLogKey>,
	result: Result<StorageWorkerResponse, String>,
	effects: &mut Effects,
) {
	// A reply to discovery started before an assignment update cannot restore
	// a removed member or begin another page under the newer configuration.
	if !state.members().contains(&worker) {
		return;
	}
	// A transport error, a refusal and NotServing, which a healthy shard answers while it
	// waits for this phase's manifest, all leave the member's coverage unknown. Retry the
	// page; a member that never answers holds Learning until the coordinator's Learning
	// deadline aborts the change, never until its coverage is waived.
	let Ok(StorageWorkerResponse::ListTabletReplicasResponse(page)) = result else {
		state.listing_cursors.insert(worker.clone(), after);
		effects.arm_timer_after(
			MaintainerTimer::ListRetry(worker),
			state.config.tuning().retry_ms,
		);
		return;
	};
	// A replacement disk is lost evidence, never an authoritative empty listing.
	// Keep retrying until the manifest replaces this member or Learning is aborted.
	if state.disk_incarnation(&worker) != Some(page.disk_incarnation)
		|| page.tablet_replica_set_version != state.tablet.replica_set.tablet_replica_set_version
	{
		state.listing_cursors.insert(worker.clone(), after);
		effects.arm_timer_after(
			MaintainerTimer::ListRetry(worker),
			state.config.tuning().retry_ms,
		);
		return;
	}
	let count = page.replicas.len();
	if count > state.config.tuning().listing_page_size as usize {
		fence(state, effects);
		return;
	}
	let mut cursor = after;
	for frame in page.replicas {
		let observed = frame.state.clone();
		let key = VolumeLogKey::from(frame.volume_log_key);
		let hash = partition_key_hash(&key.0.volume_key);
		if cursor
			.as_ref()
			.is_some_and(|previous| listing_order(&key) <= listing_order(previous))
			|| !state.tablet.hash_range.contains(hash)
			|| key.0.volume_key.byte_len() > state.config.tuning().max_volume_key_bytes as usize
		{
			fence(state, effects);
			return;
		}
		// A maintainer log is recovered by its own module, never loaded as a volume.
		if super::maintainer_log::listed(state, &worker, &key, &frame.state) {
			cursor = Some(key);
			continue;
		}
		if !state.loaded_volumes.contains_key(&key)
			&& state.reserves_at_least(state.config.max_loaded_volumes as usize)
		{
			state.recovery_failure = Some(RecoveryFailure::CapacityExceeded {
				max_loaded_volumes: state.config.max_loaded_volumes,
				required_capacity_at_least: state
					.reserved_volumes()
					.saturating_add(1)
					.try_into()
					.unwrap_or(u32::MAX),
			});
			fence(state, effects);
			return;
		}
		state
			.loaded_volumes
			.entry(key.clone())
			.or_insert_with(|| LoadedVolume {
				members: Default::default(),
				last_activity: state.now_boot_ms,
				creation: None,
				unacknowledged_cold_open: None,
				stored_row: None,
				status: Default::default(),
			})
			.members
			.get_or_insert(worker.clone(), frame.state);
		// Learning re-lists existing replicas to establish current-phase coverage.
		// Joint has its own post-barrier refresh in promotion::completed.
		if matches!(
			state.tablet.replica_set.phase,
			rivet_data::generated::shared_depot_types_v1::TabletPhase::Learning(_)
		) {
			super::cold::observe(state, &key, worker.clone(), observed);
		}
		super::track_loaded(state, &key);
		super::deletion::observe_loaded(state, &key);
		super::filesystem::member_observed(state, &key);
		cursor = Some(key);
	}
	if count == state.config.tuning().listing_page_size as usize {
		list(state, worker, cursor, effects);
		return;
	}
	tracing::info!(
		tablet_id = %uuid::Uuid::from_bytes(state.tablet.id),
		grant_epoch = state.tablet.tablet_grant_epoch,
		member = %worker,
		loaded_volumes = state.loaded_volumes.len(),
		"tablet maintainer listed a member's replicas"
	);
	state.listed.insert(worker);
	finish(state, effects);
	collect_orphans(state, "listing", OrphanScan::Every, effects);
}

/// Rechecks the opening barriers after the maintainer log recovers.
pub(crate) fn log_recovered(state: &mut MaintainerState, effects: &mut Effects) {
	finish(state, effects);
	collect_orphans(state, "opened", OrphanScan::Every, effects);
}

/// Whether a minority can be proven orphan now: every member of a stable replica set
/// answered the listing. A missing current quorum during replacement does not prove
/// that an older configuration never acknowledged an incarnation.
pub(crate) fn orphans_provable(state: &MaintainerState) -> bool {
	state.phase == MaintainerPhase::Serving
		&& matches!(
			state.tablet.replica_set.phase,
			rivet_data::generated::shared_depot_types_v1::TabletPhase::Stable
		) && state.listed.len() == state.members().len()
}

/// Whether an idle loaded volume is an orphaned minority that `collect_orphans` forgets.
/// Deletion and pending opens of its key take precedence. Another incarnation of the
/// key is found through the key's contiguous range, not a scan of every loaded volume.
pub(crate) fn is_orphan(
	state: &MaintainerState,
	key: &VolumeLogKey,
	volume: &LoadedVolume,
) -> bool {
	!super::membership::quorum(&state.tablet.replica_set, |worker| {
		volume.members.contains_key(worker)
	}) && volume.idle(state.now_boot_ms, state.config.idle_unload_after_ms)
		&& volume.creation.is_none()
		&& !state.pending.contains_key(&key.0.volume_key)
		&& !state.unloading.contains_key(key)
		&& !super::filesystem::is_record_missing(state, key)
		&& !super::deletion::holds_key(state, &key.0.volume_key)
		&& !state
			.loaded_volumes
			.range(super::incarnations(&key.0.volume_key))
			.any(|(other, candidate)| {
				candidate.row(other).is_some_and(|row| {
					row.status == rivet_data::generated::shared_depot_fs_v1::FsVolumeStatus::Deleted
				})
			}) && !volume
		.members
		.values()
		.any(|r| r.status == VolumeLogReplicaStatus::Deleted)
}

/// Which volumes one orphan collection examines.
pub(crate) enum OrphanScan {
	/// Every loaded volume. Used only when the proof itself became possible: listing
	/// completion and the replica set returning to `Stable`. The `OrphanSweep` retry examines
	/// only `orphan_backlog`, where a partial load, a failed forget or an abandoned create
	/// queues its volume.
	Every,
	/// Only these volumes, for events that change whether one volume is an orphan.
	Volumes(Vec<VolumeLogKey>),
}

/// Forgets minority replicas that can never become a quorum.
///
/// A minority is proven orphan only after every member answered. Deletion and
/// newly admitted creates take precedence over this cleanup. Whether one volume is an
/// orphan depends on that volume and on the tablet-wide proof, so a full scan runs only
/// when the proof becomes possible and on the failure retry. The per-event triggers name
/// their volumes: an expired creation, an idle deadline, an abandoned create and a
/// finished forget. Orphans the in-flight cap defers wait in `orphan_backlog`, which every
/// collection examines too. Nothing runs it periodically.
pub(crate) fn collect_orphans(
	state: &mut MaintainerState,
	trigger: &'static str,
	scan: OrphanScan,
	effects: &mut Effects,
) {
	// TODO: Revisit retained incarnations once assignment updates are wired. They
	// consume loaded-volume slots and can block the key until replacement finishes.
	if !orphans_provable(state) {
		return;
	}
	crate::metrics::record_recovery_scan("orphan_sweep", trigger);
	let mut candidates = std::mem::take(&mut state.orphan_backlog);
	match scan {
		OrphanScan::Every => candidates.extend(state.loaded_volumes.keys().cloned()),
		OrphanScan::Volumes(keys) => candidates.extend(keys),
	}
	let orphans: Vec<_> = candidates
		.into_iter()
		.filter_map(|key| {
			let volume = state.loaded_volumes.get(&key)?;
			if !is_orphan(state, &key, volume) {
				return None;
			}
			let members = volume.members.keys().cloned().collect::<Vec<_>>();
			Some((key, members))
		})
		.collect();

	let counts = state.operations.index().counts;
	let mut active = (counts.forget_orphan + counts.forget_volume) as usize;
	let mut orphans = orphans.into_iter();
	while let Some((key, members)) = orphans.next() {
		if members.is_empty() {
			super::filesystem::forget(state, &key, effects);
			state.remove_loaded_volume(&key);
			continue;
		}
		for worker in members {
			if active >= state.config.tuning().max_forgets_in_flight as usize {
				// A finished forget frees a slot and collects the deferred volumes.
				state.orphan_backlog.insert(key);
				state.orphan_backlog.extend(orphans.map(|(key, _)| key));
				return;
			}
			if forgetting_orphan(state, &key, &worker) {
				continue;
			}
			let request = StorageWorkerRequest::ForgetVolumeRequest(wire::ForgetVolumeRequest {
				observation: None,
				header: wire::ReplicaRequestHeader::MaintainerRequestHeader(
					wire::MaintainerRequestHeader {
						volume_log_key: key.0.clone(),
						tablet_grant: state.grant(),
					},
				),
				expected_appended: None,
				expected_claim: None,
			});
			active += 1;
			let op_id = state.begin(
				Operation::ForgetOrphan {
					worker: worker.clone(),
					key: key.clone(),
				},
				effects,
			);
			effects.execute(MaintainerEffect::ReplicaRequest {
				op_id,
				worker_id: worker,
				request,
			});
		}
	}
}

/// Whether a forget of `key` on `worker` is already in flight.
fn forgetting_orphan(state: &MaintainerState, key: &VolumeLogKey, worker: &WorkerId) -> bool {
	let Some(ids) = state.operations.index().forget_orphans.get(key) else {
		return false;
	};
	ids.iter().any(|id| {
		matches!(
			state.operations.get(id),
			Some(Operation::ForgetOrphan { worker: w, .. }) if w == worker
		)
	})
}

/// Both the durable imported index and a hot listing quorum precede Ready publication.
pub(crate) fn finish(state: &mut MaintainerState, effects: &mut Effects) {
	if !super::maintainer_log::ready(state) {
		return;
	}
	if super::membership::quorum(&state.tablet.replica_set, |w| state.listed.contains(w))
		&& state.phase == MaintainerPhase::Opening
	{
		if let Some(source) = state.alarms.initialization.clone() {
			if !state.alarms.reporting_ready {
				state.alarms.reporting_ready = true;
				effects.execute(MaintainerEffect::ReportAlarmIndexReady {
					request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::AlarmIndexReady {
						tablet_id: state.tablet.id,
						tablet_grant_epoch: state.tablet.tablet_grant_epoch,
						maintainer: state.tablet.maintainer.expect("assigned alarm child"),
						source,
					}
				});
			}
			return;
		}
		state.phase = if state.tablet.split.is_some() {
			MaintainerPhase::Frozen
		} else {
			MaintainerPhase::Serving
		};
		if state.phase == MaintainerPhase::Serving {
			if matches!(
				state.tablet.replica_set.phase,
				rivet_data::generated::shared_depot_types_v1::TabletPhase::Learning(_)
					| rivet_data::generated::shared_depot_types_v1::TabletPhase::Joint(_)
			) {
				effects.arm_timer_after(MaintainerTimer::CatchUp, state.config.tuning().retry_ms);
			}
			if !state.pending_rows.is_empty() {
				effects.arm_timer_after(
					MaintainerTimer::WriteToCold,
					state.config.write_to_cold_after_ms,
				);
			}
			// Volumes listed while opening become eligible for filesystem apply now.
			let keys = state.loaded_volumes.keys().cloned().collect::<Vec<_>>();
			for key in keys {
				super::filesystem::offer(state, &key);
			}
		}
	}
}
