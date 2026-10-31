//! Inspection of one volume name: the maintainer's memory of every loaded incarnation beside the
//! records its store holds. An inspection reads; it never loads, registers, unloads or fences.

use super::{RepairOperation, admissible, refuse};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::{MaintainerEffect, RepairEffect},
	state::{MaintainerState, Operation},
	types::{OpId, RepairRecords, RequestId},
};
use depot_types::VolumeLogKey;
use rivet_data::generated::depot_client_protocol_v1::{
	ClientResponse, RepairIncarnationView, RepairInspection, RepairMemberView,
};

/// Admits an inspection of `volume_key` and starts its store read.
pub(crate) fn request(
	state: &mut MaintainerState,
	request_id: RequestId,
	volume_key: depot_types::VolumeKey,
	fx: &mut Effects,
) {
	if let Err(reason) = admissible(state) {
		refuse(request_id, reason, fx);
		return;
	}
	// The store read takes a read slot, so a full read budget refuses before any work.
	if super::super::store::StoreUsage::of(state).reads() >= state.config.max_cold_reads as usize {
		refuse(request_id, depot_types::client::overloaded(), fx);
		return;
	}
	let incarnations = state
		.loaded_volumes
		.range(super::super::incarnations(&volume_key))
		.map(|(key, _)| key.0.volume_incarnation)
		.collect::<Vec<_>>();
	let op_id = state.begin(
		Operation::Repair(RepairOperation::InspectRead {
			request_id,
			volume_key: volume_key.clone(),
		}),
		fx,
	);
	fx.execute(MaintainerEffect::Repair(RepairEffect::InspectReadRequest {
		op_id,
		volume_key,
		incarnations,
	}));
}

/// The store read answered: reply with the records beside the maintainer's memory now.
pub(crate) fn completed(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<RepairRecords, String>,
	fx: &mut Effects,
) {
	if !matches!(
		state.operations.get(&op_id),
		Some(Operation::Repair(RepairOperation::InspectRead { .. }))
	) {
		return;
	}
	let Some(Operation::Repair(RepairOperation::InspectRead {
		request_id,
		volume_key,
	})) = state.finish(op_id, fx)
	else {
		unreachable!("checked above")
	};
	let records = match result {
		Ok(records) => records,
		Err(error) => {
			tracing::warn!(%error, ?volume_key, "an operator inspection's store read failed");
			refuse(request_id, depot_types::client::overloaded(), fx);
			return;
		}
	};
	let inspection = view(state, &volume_key, records);
	fx.execute(MaintainerEffect::Reply {
		request_id,
		response: ClientResponse::RepairInspection(inspection),
	});
}

/// The store read outlived its deadline. It holds no footprint, so its operation ends here
/// and its late response is dropped; the caller is told to retry.
pub(crate) fn timed_out(state: &mut MaintainerState, op_id: OpId, fx: &mut Effects) {
	let Some(Operation::Repair(RepairOperation::InspectRead { request_id, .. })) =
		state.finish(op_id, fx)
	else {
		return;
	};
	refuse(request_id, depot_types::client::overloaded(), fx);
}

/// What the maintainer holds for `volume_key`, with the store's records.
pub(crate) fn view(
	state: &MaintainerState,
	volume_key: &depot_types::VolumeKey,
	records: RepairRecords,
) -> RepairInspection {
	let stored = |incarnation: &[u8; 16]| {
		records
			.incarnations
			.iter()
			.find(|(asked, _)| asked == incarnation)
			.and_then(|(_, record)| record.clone())
	};
	let incarnations = state
		.loaded_volumes
		.range(super::super::incarnations(volume_key))
		.map(|(key, volume)| {
			incarnation_view(state, key, volume, stored(&key.0.volume_incarnation))
		})
		.collect();
	RepairInspection {
		tablet_id: state.tablet.id,
		tablet_grant_epoch: state.tablet.tablet_grant_epoch,
		replica_set: state.tablet.replica_set.clone(),
		pointer: records.pointer,
		pending_fork: records.pending_fork,
		pending_open: state.pending.contains_key(volume_key),
		incarnations,
	}
}

fn incarnation_view(
	state: &MaintainerState,
	key: &VolumeLogKey,
	volume: &super::super::LoadedVolume,
	stored: Option<rivet_data::generated::shared_depot_fs_v1::FsVolumeRecord>,
) -> RepairIncarnationView {
	let registered = state.filesystem.volumes.get(key);
	RepairIncarnationView {
		volume_log_key: key.0.clone(),
		stored,
		known: volume.stored_row.as_deref().cloned(),
		evidence: volume.row(key),
		registered: registered.is_some(),
		applied: registered.map(|volume| volume.applied.clone()),
		members: volume
			.members
			.iter()
			.map(|(worker, replica)| RepairMemberView {
				worker_id: *worker.as_bytes(),
				state: replica.clone(),
				listed: state.listed.contains(worker),
			})
			.collect(),
		record_missing: super::super::filesystem::is_record_missing(state, key),
		deleting: state.deleting.contains_key(key),
		unloading: state.unloading.contains_key(key),
		awaiting_replay: state.filesystem.awaiting_replay.contains(key),
		holder: super::super::holders::holder(volume),
		idle_ms: state.now_boot_ms.saturating_sub(volume.last_activity),
	}
}


