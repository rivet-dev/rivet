//! Operator requests that make a tablet's data cold on demand: tier its applied data now, and
//! unload its quiet volumes. Both act on the tablet's own bookkeeping, never on other tablets,
//! and reply with what they started rather than waiting for the work to finish.

use super::{MaintainerPhase, MaintainerState};
use crate::tablet_maintainer_machine::{Effects, effect::MaintainerEffect, types::RequestId};
use rivet_data::generated::depot_client_protocol_v1::{
	ClientResponse, OperatorTabletReport, VolumeRejectResponse,
};

/// Loaded volumes one unload request may visit per volume it is allowed to begin, so a request
/// whose front volumes are all busy still returns promptly.
const VISITS_PER_UNLOAD: usize = 8;

/// Most unloads one request may begin, so one event's work stays bounded however many volumes
/// the tablet holds. Repeating the request unloads the rest.
const MAX_UNLOADS_PER_REQUEST: usize = 1024;

/// Asks every volume with applied data above cold for a prompt cold write and queues a cold
/// pass for pending rows. The work runs through the store queue's usual budgets.
pub(crate) fn tier(state: &mut MaintainerState, request_id: RequestId, fx: &mut Effects) {
	if let Err(reason) = admissible(state) {
		fx.execute(MaintainerEffect::Reply {
			request_id,
			response: ClientResponse::VolumeRejectResponse(reason),
		});
		return;
	}

	// The unflushed index names exactly the volumes with applied bytes above cold, so the
	// request costs what is dirty, not what is registered. A retiring volume takes no flush
	// request and is not counted, so the count reaches zero once the rest are cold.
	let keys = state
		.filesystem
		.unflushed_keys()
		.into_iter()
		.filter(|key| !super::filesystem::retiring(state, key))
		.collect::<Vec<_>>();
	for key in &keys {
		super::filesystem::request_flush(state, key, fx);
	}
	let generation = super::poll::refresh_all(state, fx);
	super::store::queue_pass(state, Some(generation));

	let report = report(state, keys.len() as u64, 0);
	fx.execute(MaintainerEffect::Reply {
		request_id,
		response: ClientResponse::OperatorTabletReport(report),
	});
}

/// Begins the unload of at most `limit` loaded volumes quiet for `quiet_ms`. Each unload checks
/// the volume is still quiet before its row read, and waits for ordinary tiering like a close.
///
/// Quiet means no request reached this maintainer; commits go to the replicas and never count.
/// An open client keeps its volume loaded only through the holder check, so `quiet_ms` is
/// raised to the holder check interval: below it a volume with a live writer would look quiet
/// and its unload would fence that writer.
pub(crate) fn unload_quiet(
	state: &mut MaintainerState,
	request_id: RequestId,
	quiet_ms: u64,
	limit: u32,
	fx: &mut Effects,
) {
	if let Err(reason) = admissible(state) {
		fx.execute(MaintainerEffect::Reply {
			request_id,
			response: ClientResponse::VolumeRejectResponse(reason),
		});
		return;
	}

	let limit = (limit as usize).min(MAX_UNLOADS_PER_REQUEST);
	let quiet_ms = quiet_ms.max(state.config.tuning().holder_check_after_ms);
	let now = state.now_boot_ms;
	// Idle unloads collect a provable orphan minority instead of unloading it, so this leaves
	// those to the idle timer rather than starting an unload that cannot finish.
	let orphans_provable = super::opening::orphans_provable(state);
	let mut selected = Vec::new();
	for (key, volume) in state
		.loaded_volumes
		.iter()
		.take(limit.saturating_mul(VISITS_PER_UNLOAD))
	{
		if selected.len() >= limit {
			break;
		}
		let quiet = !volume.is_deleted()
			&& volume.idle(now, quiet_ms)
			&& !state.pending.contains_key(&key.0.volume_key)
			&& !state.unloading.contains_key(key)
			&& !super::filesystem::is_record_missing(state, key)
			&& !(orphans_provable && super::opening::is_orphan(state, key, volume));
		if quiet {
			selected.push(key.clone());
		}
	}
	for key in &selected {
		super::unload::begin(
			state,
			key.clone(),
			super::unload::UnloadReason::Quiet { quiet_ms },
		);
	}

	let report = report(state, 0, selected.len() as u64);
	fx.execute(MaintainerEffect::Reply {
		request_id,
		response: ClientResponse::OperatorTabletReport(report),
	});
}

/// Operator requests run only while the maintainer serves, within the caller budget.
fn admissible(state: &MaintainerState) -> Result<(), VolumeRejectResponse> {
	match state.phase {
		MaintainerPhase::Frozen => Err(VolumeRejectResponse::TabletSplitting),
		MaintainerPhase::Serving if state.request_count() >= state.config.max_requests as usize => {
			Err(depot_types::client::overloaded())
		}
		MaintainerPhase::Serving => Ok(()),
		MaintainerPhase::Opening
		| MaintainerPhase::Draining
		| MaintainerPhase::Stopped
		| MaintainerPhase::Fenced => Err(depot_types::client::not_maintainer()),
	}
}

fn report(
	state: &MaintainerState,
	tier_requested: u64,
	unloads_started: u64,
) -> OperatorTabletReport {
	OperatorTabletReport {
		tablet_id: state.tablet.id,
		tier_requested,
		unloads_started,
		loaded_volumes: state.loaded_volumes.len() as u64,
		unloading_volumes: state.unloading.len() as u64,
	}
}
