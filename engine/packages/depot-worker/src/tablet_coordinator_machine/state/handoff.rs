//! Live-owner grants retain one prepared manifest CAS until create admission is quiescent.
//!
//! Rollout requires enhanced readiness support on every possible old owner before
//! elective moves and graceful grant handoffs can complete. Legacy Stop/Permit/Progress
//! remain V1, but a legacy Ready cannot certify the V2 create barrier. Unsupported V2
//! checks fail closed; an abandoned ordinary check is released only by a manifest fence.

use super::{Phase, State, alarms, reject};
use crate::tablet_coordinator_machine::{Effects, effect::Effect, types::*};
use rivet_data::{
	generated::{
		depot_storage_worker_protocol_v1 as wire,
		depot_storage_worker_protocol_v1::StopReadinessResponse,
		depot_tablet_coordinator_protocol_v1::{
			RequestStop, TabletCoordinatorRejectResponse as Reject,
		},
	},
	versioned::handoff_proof_matches,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// No grant CAS has been sent. The original manifest/ETag also owns abandonment's
/// fencing touch; a newer coordinator always fences this version before serving.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PendingGrant {
	pub(crate) stored: StoredManifest,
	pub(crate) write: ManifestWrite,
	pub(crate) rollback_alarms: alarms::AlarmManifestState,
	pub(crate) checks: BTreeMap<[u8; 16], StopCheck>,
	pub(crate) ready: BTreeSet<[u8; 16]>,
	/// One absolute alarm bounds all readiness attempts. Retries never extend it.
	pub(crate) generation: u64,
}

fn request_ids(write: &ManifestWrite) -> Vec<u64> {
	match &write.origin {
		WriteOrigin::Commands { request_ids, .. } | WriteOrigin::HandoffAborted { request_ids } => {
			request_ids.clone()
		}
		WriteOrigin::Stop { request_id, .. } => vec![*request_id],
		WriteOrigin::Takeover => vec![],
	}
}

impl State {
	/// Every ordinary owner change, including balancing, passes through this gate.
	/// Missing-owner failover keeps its existing path and never pretends to have Ready.
	pub(super) fn prepare_grant_write(
		&mut self,
		stored: StoredManifest,
		write: ManifestWrite,
		rollback_alarms: alarms::AlarmManifestState,
		missing_owner_grants: BTreeSet<[u8; 16]>,
		effects: &mut Effects,
	) {
		let changed: Vec<_> = stored
			.manifest
			.tablets
			.iter()
			.filter(|old| {
				old.maintainer.is_some()
					&& write
						.manifest
						.tablets
						.iter()
						.any(|new| new.id == old.id && new.maintainer != old.maintainer)
			})
			.collect();
		if changed.is_empty() {
			self.write(write, effects);
			return;
		}
		let Some(topology) = self
			.discovery
			.as_ref()
			.filter(|topology| effects.now_boot_ms() < topology.valid_until_boot_ms)
		else {
			self.restore_handoff_alarms(rollback_alarms);
			for id in request_ids(&write) {
				reject(id, Reject::Overloaded, effects);
			}
			self.phase = Phase::Ready(stored);
			return;
		};
		let mut checks = BTreeMap::new();
		for tablet in changed {
			let worker = tablet.maintainer.expect("owned tablet");
			// An owner whose binary cannot read the tablet never opened it, so it holds no
			// admitted create to quiesce; failover away from it needs no readiness proof.
			if missing_owner_grants.contains(&tablet.id)
				&& super::versions::check(tablet, &worker, Some(topology)).is_err()
			{
				continue;
			}
			let Some(instance) = topology.workers.get(&worker) else {
				if missing_owner_grants.contains(&tablet.id) {
					continue;
				}
				self.restore_handoff_alarms(rollback_alarms);
				for id in request_ids(&write) {
					reject(id, Reject::Overloaded, effects);
				}
				self.phase = Phase::Ready(stored);
				return;
			};
			let check = checks.entry(worker).or_insert_with(|| StopCheck {
				purpose: StopCheckPurpose::Handoff {
					tablet_manifest_version: stored.manifest.tablet_manifest_version,
				},
				request: RequestStop {
					worker_id: worker,
					instance_id: *instance,
				},
				tablets: vec![],
				instances: BTreeMap::new(),
			});
			check.tablets.push(tablet.clone());
			let candidate = write
				.manifest
				.tablets
				.iter()
				.find(|new| new.id == tablet.id)
				.expect("changed assignment");
			for disk in tablet
				.replica_set
				.disks
				.iter()
				.chain(candidate.replica_set.disks.iter())
			{
				if let Some(instance) = topology.workers.get(&disk.worker_id) {
					check.instances.insert(disk.worker_id, *instance);
				}
			}
			check.instances.insert(worker, *instance);
		}
		if checks.is_empty() {
			self.write(write, effects);
			return;
		}
		let generation = self.operation(effects);
		let pending = PendingGrant {
			stored,
			write,
			rollback_alarms,
			checks,
			ready: BTreeSet::new(),
			generation,
		};
		self.check_handoff(pending, effects);
	}

	fn restore_handoff_alarms(&mut self, mut rollback: alarms::AlarmManifestState) {
		rollback.commands = std::mem::take(&mut self.alarms.commands);
		self.alarms = rollback;
	}

	pub(in crate::tablet_coordinator_machine) fn check_handoff(
		&mut self,
		mut pending: PendingGrant,
		effects: &mut Effects,
	) {
		pending.ready.clear();
		self.next_operation = self
			.next_operation
			.checked_add(1)
			.expect("operation id exhausted");
		let operation = self.next_operation;
		for check in pending.checks.values() {
			effects.execute(Effect::StopReadinessRequest {
				operation,
				check: check.clone(),
			});
		}
		self.phase = Phase::CheckingHandoff { operation, pending };
	}

	pub(super) fn abort_handoff(&mut self, effects: &mut Effects) {
		let (Phase::CheckingHandoff { pending, .. } | Phase::WaitingHandoff { pending }) =
			&self.phase
		else {
			return;
		};
		let pending = pending.clone();
		self.cancel_handoff_checks(&pending, effects);
		self.restore_handoff_alarms(pending.rollback_alarms);
		// The prepared CAS has not been sent. This exact competing precondition must
		// confirm before the old owner can release its barrier at the next version.
		let mut manifest = pending.stored.manifest;
		manifest.tablet_manifest_version = pending.write.manifest.tablet_manifest_version;
		self.write(
			ManifestWrite {
				manifest,
				expected_etag: Some(pending.stored.etag),
				origin: WriteOrigin::HandoffAborted {
					request_ids: request_ids(&pending.write),
				},
			},
			effects,
		);
	}

	fn cancel_handoff_checks(&self, pending: &PendingGrant, effects: &mut Effects) {
		effects.cancel_timer(Timer::Operation(pending.generation));
		effects.cancel_timer(Timer::HandoffRetry(pending.generation));
		for worker in pending.checks.keys() {
			effects.execute(Effect::CancelStopReadinessRequest { worker: *worker });
		}
	}

	pub(in crate::tablet_coordinator_machine) fn cancel_command(
		&mut self,
		id: u64,
		effects: &mut Effects,
	) {
		self.queued.retain(|command| command.request_id != id);
		self.alarms.commands.remove(&id);
		if matches!(&self.phase, Phase::CheckingHandoff { pending, .. } | Phase::WaitingHandoff { pending } if request_ids(&pending.write).contains(&id))
		{
			self.abort_handoff(effects);
		}
		// Writing/Resolve retain their exact CAS and conditional readback obligation.
	}

	/// Exact decoded V2 evidence is the only route to enhanced readiness.
	pub(in crate::tablet_coordinator_machine) fn handoff_readiness(
		&mut self,
		operation: u64,
		worker: [u8; 16],
		results: BTreeMap<[u8; 16], Option<wire::HandoffReadinessResponse>>,
		effects: &mut Effects,
	) {
		if let Phase::CheckingHandoff {
			operation: current,
			pending,
		} = &self.phase
			&& *current == operation
		{
			let Some(check) = pending.checks.get(&worker) else {
				return;
			};
			let fresh = self.discovery.as_ref().is_some_and(|topology| {
				effects.now_boot_ms() < topology.valid_until_boot_ms
					&& pending.checks.values().all(|check| {
						check.instances.iter().all(|(worker, instance)| {
							topology.workers.get(worker) == Some(instance)
						})
					})
			});
			if !fresh {
				self.abort_handoff(effects);
				return;
			}
			let ready = proofs_ready(check, &results);
			let mut pending = pending.clone();
			if ready {
				pending.ready.insert(worker);
				if pending.ready.len() == pending.checks.len() {
					self.cancel_handoff_checks(&pending, effects);
					self.write(pending.write, effects);
				} else {
					self.phase = Phase::CheckingHandoff { operation, pending };
				}
			} else {
				for worker in pending.checks.keys() {
					effects.execute(Effect::CancelStopReadinessRequest { worker: *worker });
				}
				effects.arm_timer_after(Timer::HandoffRetry(pending.generation), self.retry_ms);
				self.phase = Phase::WaitingHandoff { pending };
			}
			return;
		}
		// Stop rounds carry a version as well: a legacy Ready cannot populate this cache.
		if let Some(round) = self.stop_rounds.get_mut(&worker) {
			if round.operation != operation
				|| !matches!(round.check.purpose, StopCheckPurpose::Handoff { .. })
			{
				return;
			}
			let ready = proofs_ready(&round.check, &results);
			round.results = Some(
				round
					.check
					.tablets
					.iter()
					.map(|tablet| {
						(
							tablet.id,
							if ready {
								StopReadinessResponse::Ready
							} else {
								StopReadinessResponse::NotServing
							},
						)
					})
					.collect(),
			);
		}
	}
}

fn proofs_ready(
	check: &StopCheck,
	results: &BTreeMap<[u8; 16], Option<wire::HandoffReadinessResponse>>,
) -> bool {
	check.tablets.iter().all(|tablet| {
		let Some(request) = check.handoff_request(tablet) else {
			return false;
		};
		results
			.get(&tablet.id)
			.and_then(Option::as_ref)
			.is_some_and(|proof| {
				handoff_proof_matches(&request, proof)
					&& proof.result == wire::HandoffReadinessResult::HandoffReady
			})
	})
}
