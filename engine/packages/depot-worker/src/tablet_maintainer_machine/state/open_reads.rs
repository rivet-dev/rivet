//! Read-back of named keys in an open that found its volume existing with no log tail.
//!
//! An open of an existing volume may name keys of one KV file. When every member of the
//! tablet's replica set answered with the same appended position, and the store has applied
//! that position, nothing above it exists on any replica, so it is the position the opener's
//! recovery starts from. The values there are then exactly the values the opener would read
//! after recovering, and the open answers them in its own reply. In every other case the open
//! replies at once without them and the opener reads after recovery as usual.
//!
//! The read is opportunistic: it never waits for an apply, a read slot or a lagging member,
//! and a failed or slow read answers the open without values instead of failing it.

use super::{MaintainerState, Operation};
use crate::tablet_maintainer_machine::types::MAX_OPEN_READ_KEYS;
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{MaintainerTimer, OpId, RequestId},
};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_client_protocol_v1::{self as client, ClientResponse},
	shared_depot_types_v1::{VolumeLogPosition, VolumeLogReplicaStatus},
};
use serde::{Deserialize, Serialize};

/// An open whose reply waits for its read-back. `response` is the reply without values.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct OpenRead {
	pub(crate) response: client::VolumeOpenResponse,
}

/// The position every member of the replica set holds, when the store applied exactly it.
fn untailed_position(
	state: &MaintainerState,
	key: &VolumeLogKey,
	states: &[client::MemberReplicaState],
) -> Option<VolumeLogPosition> {
	let mut position = None::<VolumeLogPosition>;
	for member in &state.tablet.replica_set.members {
		let answered = states.iter().find(|answer| answer.worker_id == *member)?;
		if answered.state.status != VolumeLogReplicaStatus::InSync {
			return None;
		}
		match &position {
			Some(position) if *position != answered.state.appended => return None,
			Some(_) => {}
			None => position = Some(answered.state.appended.clone()),
		}
	}
	let position = position?;
	let applied = &state.filesystem.volumes.get(key)?.applied;
	(*applied == position).then_some(position)
}

/// Starts the read-back for an open of an existing volume, or returns the response to send
/// now when the open cannot be answered with values.
pub(crate) fn begin(
	state: &mut MaintainerState,
	request_id: RequestId,
	key: &VolumeLogKey,
	read: Option<client::VolumeReadKeys>,
	deadline_boot_ms: u64,
	response: client::VolumeOpenResponse,
	effects: &mut Effects,
) -> Option<ClientResponse> {
	// A reply without values ends any wait for the remaining members.
	effects.cancel_timer(MaintainerTimer::OpenRead(request_id));
	let reply_now = |response| Some(ClientResponse::VolumeOpenResponse(response));
	let Some(read) = read else {
		return reply_now(response);
	};
	if read.keys.is_empty()
		|| read.keys.len() > MAX_OPEN_READ_KEYS as usize
		|| response.created.is_some()
		|| super::store::StoreUsage::of(state).reads() >= state.config.max_cold_reads as usize
	{
		return reply_now(response);
	}
	let (Some(applied), Some(volume)) = (
		untailed_position(state, key, &response.states),
		state.filesystem.volumes.get(key),
	) else {
		return reply_now(response);
	};
	let record = volume.record.to_record();

	let budget_ms = budget_ms(
		deadline_boot_ms,
		effects.clocks().boot_ms,
		state.config.tuning().operation_timeout_ms,
	);
	let op_id = state.begin(
		Operation::ReadOpenKeys {
			request_id,
			volume_key: key.0.volume_key.clone(),
		},
		effects,
	);
	state.open_reads.insert(request_id, OpenRead { response });
	effects.arm_timer_after(MaintainerTimer::OpenRead(request_id), budget_ms);
	effects.execute(MaintainerEffect::ReadOpenKeysRequest {
		op_id,
		key: key.clone(),
		record,
		applied,
		read,
	});
	None
}

/// Answers an open with the values its read returned, or without them when it failed.
pub(crate) fn completed(
	state: &mut MaintainerState,
	op_id: OpId,
	result: Result<Vec<Option<vbare::Bytes>>, String>,
	effects: &mut Effects,
) {
	let Some(Operation::ReadOpenKeys { request_id, .. }) = state.operations.get(&op_id).cloned()
	else {
		return;
	};
	state.finish(op_id, effects);
	let Some(open) = state.open_reads.remove(&request_id) else {
		return;
	};
	effects.cancel_timer(MaintainerTimer::OpenRead(request_id));
	let mut response = open.response;
	match result {
		Ok(values) => response.read = Some(values),
		Err(error) => {
			tracing::debug!(%error, "open read-back failed; the opener reads after recovery");
		}
	}
	effects.execute(MaintainerEffect::Reply {
		request_id,
		response: ClientResponse::VolumeOpenResponse(response),
	});
}

/// Answers an open without values when its wait for members or its read did not finish within
/// its budget. A read in flight keeps its operation until it completes or times out.
pub(crate) fn expired(state: &mut MaintainerState, request_id: RequestId, effects: &mut Effects) {
	// An open still waiting for its remaining members replies from the quorum it has.
	let waiting = state.pending.iter_mut().find_map(|(_, pending)| {
		let super::OpenStep::Loading(loading) = &pending.step else {
			return None;
		};
		(pending.caller.request_id == request_id && loading.read_wait).then(|| {
			pending.caller.read = None;
			loading.key.clone()
		})
	});
	if let Some(key) = waiting {
		super::lifecycle::complete_load(state, key, false, effects);
		return;
	}
	let Some(open) = state.open_reads.remove(&request_id) else {
		return;
	};
	effects.execute(MaintainerEffect::Reply {
		request_id,
		response: ClientResponse::VolumeOpenResponse(open.response),
	});
}

/// Releases a read whose operation timed out. Its open, if still waiting, is answered by its
/// own timer, which is never later than the operation timeout.
pub(crate) fn timed_out(state: &mut MaintainerState, op_id: OpId, effects: &mut Effects) {
	let Some(Operation::ReadOpenKeys { request_id, .. }) = state.finish(op_id, effects) else {
		return;
	};
	expired(state, request_id, effects);
}

/// Whether the open of `volume_key` should keep waiting for its remaining loads before it
/// replies, so it can answer its named keys. Arms the read timer on the first wait.
pub(crate) fn waits_for_members(
	state: &mut MaintainerState,
	volume_key: &depot_types::VolumeKey,
	effects: &mut Effects,
) -> bool {
	let max_keys = MAX_OPEN_READ_KEYS as usize;
	let operation_timeout_ms = state.config.tuning().operation_timeout_ms;
	let Some(pending) = state.pending.get_mut(volume_key) else {
		return false;
	};
	let wants_read = pending
		.caller
		.read
		.as_ref()
		.is_some_and(|read| !read.keys.is_empty() && read.keys.len() <= max_keys);
	let request_id = pending.caller.request_id;
	let deadline_boot_ms = pending.caller.deadline_boot_ms;
	let super::OpenStep::Loading(loading) = &mut pending.step else {
		return false;
	};
	// A volume this open creates has nothing to read back.
	if !wants_read || loading.unacknowledged_create {
		return false;
	}
	if !loading.read_wait {
		loading.read_wait = true;
		effects.arm_timer_after(
			MaintainerTimer::OpenRead(request_id),
			budget_ms(
				deadline_boot_ms,
				effects.clocks().boot_ms,
				operation_timeout_ms,
			),
		);
	}
	true
}

/// How long an open may hold its reply for a read-back: half the caller's remaining time,
/// bounded by the operation timeout, so a slow read still leaves the caller time to use the
/// plain answer.
fn budget_ms(deadline_boot_ms: u64, now_boot_ms: u64, operation_timeout_ms: u64) -> u64 {
	(deadline_boot_ms.saturating_sub(now_boot_ms) / 2)
		.min(operation_timeout_ms)
		.max(1)
}
