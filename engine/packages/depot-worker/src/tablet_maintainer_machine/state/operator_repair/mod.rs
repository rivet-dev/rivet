//! Operator repair of one volume name: read-only inspection of what the maintainer and its
//! store hold, and mutating calls that run through one audited, fenced pipeline (`calls`).
//! Every request is answered only by a serving maintainer, sheds at admission with a typed
//! retryable refusal, and changes nothing it does not name.

mod activity;
pub(crate) mod calls;
pub(crate) mod claims;
pub(crate) mod forget;
pub(crate) mod inspect;
pub(crate) mod lifecycle;
pub(crate) mod quarantine;

use super::{MaintainerPhase, MaintainerState, Operation};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::{MaintainerEffect, RepairEffect},
	event::RepairEvent,
	types::{OpId, RequestId},
};
use rivet_data::generated::depot_client_protocol_v1::{ClientResponse, VolumeRejectResponse};
use serde::{Deserialize, Serialize};

/// Work an operator repair request keeps in flight.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum RepairOperation {
	Diagnostics {
		request_id: RequestId,
	},
	/// An inspection's store read of the name's records. It reads without a store footprint
	/// and holds one read slot until its response or its timeout.
	InspectRead {
		request_id: RequestId,
		volume_key: depot_types::VolumeKey,
	},
	/// A mutating call's read of its audit row and plan facts; see `calls`. It reads without
	/// a store footprint and holds one read slot.
	Prepare {
		operation_id: [u8; 16],
	},
	/// A read of a name's audit rows for an operator listing.
	AuditList {
		request_id: RequestId,
	},
	/// One repair claim's client, for the claim's lifetime; see `claims`. Its timer is the
	/// deadline of the claim's current phase. It holds no store work.
	Claim {
		volume_key: depot_types::VolumeKey,
	},
	/// The deadline of one mutating call from its intent to its outcome; see `calls`. It
	/// holds no store work.
	Call {
		operation_id: [u8; 16],
	},
	/// A repair forget's request to the worker; see `forget`. It holds no store work.
	Forget {
		operation_id: [u8; 16],
	},
}

/// The repair state of one maintainer lifetime.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Repairs {
	pub(crate) calls: calls::Calls,
	pub(crate) claims: claims::Claims,
}

/// Routes one repair event.
pub(crate) fn on_event(state: &mut MaintainerState, event: RepairEvent, fx: &mut Effects) {
	match event {
		RepairEvent::DiagnosticsRequest {
			request_id,
			mut request,
		} => {
			if let Err(reason) = admissible(state) {
				refuse(request_id, reason, fx);
			} else if !super::in_tablet(state, &request.volume_log_key.volume_key) {
				refuse(request_id, depot_types::client::not_maintainer(), fx);
			} else if request.kind
				== rivet_data::generated::depot_client_protocol_v1::RepairDiagnosticsKind::Activity
			{
				match activity::page(state, &request) {
					Ok(page) => fx.execute(MaintainerEffect::Reply {
						request_id,
						response: ClientResponse::RepairDiagnosticsPage(page),
					}),
					Err(_) => refuse(request_id, depot_types::client::not_maintainer(), fx),
				}
			} else if super::store::StoreUsage::of(state).reads()
				>= state.config.max_cold_reads as usize
			{
				refuse(request_id, depot_types::client::overloaded(), fx);
			} else {
				request.limit = request.limit.clamp(1, MAX_AUDIT_ROWS);
				let op_id = state.begin(
					Operation::Repair(RepairOperation::Diagnostics { request_id }),
					fx,
				);
				fx.execute(MaintainerEffect::Repair(RepairEffect::DiagnosticsRequest {
					op_id,
					request,
				}));
			}
		}
		RepairEvent::DiagnosticsResponse { op_id, result } => {
			let Some(Operation::Repair(RepairOperation::Diagnostics { request_id })) =
				state.finish(op_id, fx)
			else {
				return;
			};
			match result {
				Ok(page) => fx.execute(MaintainerEffect::Reply {
					request_id,
					response: ClientResponse::RepairDiagnosticsPage(page),
				}),
				Err(error) => {
					tracing::warn!(%error, "operator diagnostic read failed");
					refuse(request_id, depot_types::client::overloaded(), fx);
				}
			}
		}
		RepairEvent::InspectRequest {
			request_id,
			volume_key,
		} => inspect::request(state, request_id, volume_key, fx),
		RepairEvent::InspectReadResponse { op_id, result } => {
			inspect::completed(state, op_id, result, fx)
		}
		RepairEvent::Request {
			request_id,
			request,
		} => calls::request(state, request_id, request, fx),
		RepairEvent::PrepareResponse { op_id, result } => {
			let Some(Operation::Repair(RepairOperation::Prepare { operation_id })) =
				state.finish(op_id, fx)
			else {
				return;
			};
			calls::prepared(state, op_id, operation_id, result, fx)
		}
		RepairEvent::ClaimOpenResponse { op_id, result } => {
			claims::opened(state, op_id, result, fx)
		}
		RepairEvent::ClaimAppendResponse { op_id, result } => {
			claims::appended_entry(state, op_id, result, fx)
		}
		RepairEvent::ClaimClosedResponse { op_id } => claims::closed(state, op_id, fx),
		RepairEvent::AuditListRequest {
			request_id,
			request,
		} => audit_list(state, request_id, request, fx),
		RepairEvent::AuditListResponse { op_id, result } => {
			let Some(Operation::Repair(RepairOperation::AuditList { request_id })) =
				state.finish(op_id, fx)
			else {
				return;
			};
			match result {
				Ok(page) => fx.execute(MaintainerEffect::Reply {
					request_id,
					response: ClientResponse::RepairAuditPage(page),
				}),
				Err(error) => {
					tracing::warn!(%error, "an operator audit listing's store read failed");
					refuse(request_id, depot_types::client::overloaded(), fx);
				}
			}
		}
	}
}

/// Audit rows one listing returns at most.
const MAX_AUDIT_ROWS: u32 = 100;

/// Admits a listing of a name's audit rows and starts its store read.
fn audit_list(
	state: &mut MaintainerState,
	request_id: RequestId,
	request: rivet_data::generated::depot_client_protocol_v1::RepairAuditListRequest,
	fx: &mut Effects,
) {
	if let Err(reason) = admissible(state) {
		refuse(request_id, reason, fx);
		return;
	}
	if super::store::StoreUsage::of(state).reads() >= state.config.max_cold_reads as usize {
		refuse(request_id, depot_types::client::overloaded(), fx);
		return;
	}
	let op_id = state.begin(
		Operation::Repair(RepairOperation::AuditList { request_id }),
		fx,
	);
	fx.execute(MaintainerEffect::Repair(RepairEffect::AuditListRequest {
		op_id,
		volume_key: request.volume_key,
		after: request.after,
		limit: request.limit.clamp(1, MAX_AUDIT_ROWS),
	}));
}

/// A repair operation outlived its deadline.
pub(crate) fn timed_out(state: &mut MaintainerState, op_id: OpId, fx: &mut Effects) {
	match state.operations.get(&op_id) {
		Some(super::Operation::Repair(RepairOperation::InspectRead { .. })) => {
			inspect::timed_out(state, op_id, fx)
		}
		Some(super::Operation::Repair(RepairOperation::Prepare { operation_id })) => {
			let operation_id = *operation_id;
			state.finish(op_id, fx);
			calls::prepare_timed_out(state, op_id, operation_id, fx)
		}
		Some(super::Operation::Repair(RepairOperation::Claim { .. })) => {
			claims::timed_out(state, op_id, fx)
		}
		Some(super::Operation::Repair(RepairOperation::Call { operation_id })) => {
			let operation_id = *operation_id;
			calls::call_timed_out(state, op_id, operation_id, fx)
		}
		Some(super::Operation::Repair(RepairOperation::Forget { .. })) => {
			forget::timed_out(state, op_id, fx)
		}
		Some(super::Operation::Repair(RepairOperation::Diagnostics { request_id }))
		| Some(super::Operation::Repair(RepairOperation::AuditList { request_id })) => {
			let request_id = *request_id;
			state.finish(op_id, fx);
			refuse(request_id, depot_types::client::overloaded(), fx);
		}
		Some(_) | None => {}
	}
}

/// Repair requests run only while the maintainer serves, within the caller budget.
pub(crate) fn admissible(state: &MaintainerState) -> Result<(), VolumeRejectResponse> {
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

/// Answers `request_id` with a refusal.
pub(crate) fn refuse(request_id: RequestId, reason: VolumeRejectResponse, fx: &mut Effects) {
	fx.execute(MaintainerEffect::Reply {
		request_id,
		response: ClientResponse::VolumeRejectResponse(reason),
	});
}
