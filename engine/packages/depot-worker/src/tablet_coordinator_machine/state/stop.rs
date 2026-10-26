//! Stop intents and permits share the manifest CAS with grants and reconfiguration.

use super::{GrantAuthority, Phase, State, grant_authorized, reconfiguration, reject, validate};
use crate::tablet_coordinator_machine::{Effects, effect::Effect, types::*};
use rivet_data::generated::{
	depot_storage_worker_protocol_v1::StopReadinessResponse,
	depot_tablet_coordinator_protocol_v1::{
		StopBlockReason as Reason, StopBlocker, StopPending,
		TabletCoordinatorRejectResponse as Reject, TabletCoordinatorRequest as Request,
		TabletCoordinatorResponse as Response,
	},
	depot_tablet_manifest_v1::{Tablet, TabletManifest, WorkerStop},
	shared_depot_types_v1::TabletPhase,
};
use std::collections::{BTreeMap, BTreeSet};

fn blocker(tablet: &Tablet, reason: Reason) -> StopBlocker {
	StopBlocker {
		tablet_id: tablet.id,
		reason,
		volumes_behind: 0,
		detail: None,
	}
}
fn affected(manifest: &TabletManifest, worker: &[u8; 16]) -> Vec<Tablet> {
	manifest
		.tablets
		.iter()
		.filter(|t| t.replica_set.disks.iter().any(|d| &d.worker_id == worker))
		.cloned()
		.collect()
}

impl State {
	pub(super) fn request_stop(&mut self, command: Command, effects: &mut Effects) {
		let Phase::Ready(stored) = &self.phase else {
			return;
		};
		let stored = stored.clone();
		let Request::RequestStop(request) = &command.request else {
			unreachable!()
		};
		let Some(topology) = &command.topology else {
			reject(command.request_id, Reject::InvalidCommand, effects);
			return;
		};
		// Queueing cannot extend a discovery snapshot's monotonic lifetime.
		if effects.now_boot_ms() >= topology.valid_until_boot_ms {
			reject(command.request_id, Reject::Overloaded, effects);
			return;
		}
		if topology.workers.get(&request.worker_id) != Some(&request.instance_id) {
			reject(command.request_id, Reject::InvalidCommand, effects);
			return;
		}
		let mut manifest = stored.manifest.clone();
		manifest.stopping.retain(|entry| {
			// Discovery omissions are hints, not proof that this process stopped.
			// Only an observed replacement can discard its durable intent.
			topology
				.workers
				.get(&entry.worker_id)
				.is_none_or(|instance| instance == &entry.instance_id)
		});
		self.stop_rounds.retain(|worker, round| {
			let keep = manifest.stopping.iter().any(|entry| {
				entry.worker_id == *worker && entry.instance_id == round.check.request.instance_id
			});
			if !keep {
				effects.execute(Effect::CancelStopReadinessRequest { worker: *worker });
			}
			keep
		});
		if !manifest
			.stopping
			.iter()
			.any(|entry| entry.worker_id == request.worker_id)
		{
			manifest.stopping.push(WorkerStop {
				worker_id: request.worker_id,
				instance_id: request.instance_id,
				requested_at: command.requested_at,
				permitted_at: None,
			});
		}
		let already_permitted = manifest
			.stopping
			.iter()
			.any(|entry| entry.worker_id == request.worker_id && entry.permitted_at.is_some());
		let mut check = None;
		let response = if already_permitted {
			Response::StopPermitted
		} else {
			// A learner has never voted, so aborting its uncommitted replacement keeps every
			// committed entry on the old members.
			let missing = self.missing_workers(effects.now_boot_ms());
			let learning: Vec<_> = manifest
				.tablets
				.iter()
				.filter(|tablet| {
					stop_aborts_learning(tablet, &request.worker_id, &manifest, topology, &missing)
				})
				.map(|tablet| tablet.id)
				.collect();
			for id in learning {
				if let Err(reason) =
					reconfiguration::abort(&mut manifest, &id, command.requested_at)
				{
					reject(command.request_id, reason, effects);
					return;
				}
			}
			// One serialized manifest mutation moves all of this worker's grants. Later
			// stop requests cannot select it as a target, even while readiness is pending.
			let owned: Vec<_> = manifest
				.tablets
				.iter()
				.filter(|t| t.maintainer == Some(request.worker_id))
				// Keep the live owner (and its incomplete create authority) until
				// membership finishes. A successor can discover a minority left by
				// a raced create without authority to initialize the missing voters.
				// Progress checks below still drive this phase during the drain.
				.filter(|t| matches!(t.replica_set.phase, TabletPhase::Stable))
				.cloned()
				.collect();
			// Persist the intent before asking the old owner to close create admission.
			// A Ready from another grant, replica set, or peer process proves nothing
			// about an initializer this owner may still be completing.
			let handoff = StopCheck {
				purpose: StopCheckPurpose::Handoff {
					tablet_manifest_version: stored.manifest.tablet_manifest_version,
				},
				request: request.clone(),
				instances: owned
					.iter()
					.flat_map(|tablet| tablet.replica_set.disks.iter())
					.filter_map(|disk| {
						topology
							.workers
							.get(&disk.worker_id)
							.map(|instance| (disk.worker_id, *instance))
					})
					.collect(),
				tablets: owned.clone(),
			};
			let round = self
				.stop_rounds
				.get(&request.worker_id)
				.filter(|round| round.check == handoff);
			let handoff_ready =
				round
					.and_then(|round| round.results.as_ref())
					.is_some_and(|results| {
						owned.iter().all(|tablet| {
							results.get(&tablet.id) == Some(&StopReadinessResponse::Ready)
						})
					});
			let waiting_handoff = !owned.is_empty() && !handoff_ready;
			if waiting_handoff {
				if !round.is_some_and(|round| round.results.is_none()) {
					if self.stop_rounds.remove(&request.worker_id).is_some() {
						effects.execute(Effect::CancelStopReadinessRequest {
							worker: request.worker_id,
						});
					}
					check = Some(handoff);
				}
			} else if handoff_ready {
				self.stop_rounds.remove(&request.worker_id);
			}
			for owned in owned.into_iter().filter(|_| handoff_ready) {
				let id = owned.id;
				let tablet = manifest
					.tablets
					.iter()
					.find(|t| t.id == id)
					.expect("owned tablet");
				let target = super::regrant::least_loaded_target(
					&manifest,
					tablet,
					&request.worker_id,
					topology,
					None,
				);
				if let Some(target) = target {
					if grant_authorized(
						&mut manifest,
						id,
						target,
						command.requested_at,
						GrantAuthority::StoppedOwnerReady,
					)
					.is_err()
					{
						reject(command.request_id, Reject::InvalidCommand, effects);
						return;
					}
				}
			}
			// A stop never shrinks a replica set. A stopping worker may be restarting on its
			// disk, so a reduction here would turn every rolling restart into a leave and a
			// second stop before the first worker qualifies again into a single replica.
			// Sizes follow the balancer, which shrinks a set only for a member gone past
			// the missing-worker grace. A stop that would cost a majority waits instead.
			let tablets = affected(&manifest, &request.worker_id);
			let mut candidate = StopCheck {
				purpose: StopCheckPurpose::Permit,
				request: request.clone(),
				instances: tablets
					.iter()
					.flat_map(|tablet| tablet.replica_set.disks.iter())
					.filter_map(|disk| {
						topology
							.workers
							.get(&disk.worker_id)
							.map(|instance| (disk.worker_id, *instance))
					})
					.collect(),
				tablets: tablets.clone(),
			};
			let mut blockers = Vec::new();
			for tablet in &tablets {
				let reason = if matches!(&tablet.replica_set.phase,
					TabletPhase::Learning(phase)
					if phase.next.contains(&request.worker_id))
				{
					// Unlike its excluded leaving voter, a retained voter must stay
					// until promotion: its permit would block that very phase change.
					Some(Reason::MaintainerOpening)
				} else if tablet.maintainer.is_none()
					|| tablet.maintainer == Some(request.worker_id)
				{
					Some(Reason::GrantNotMoved)
				} else if tablet.replica_set.disks.iter().any(|disk| {
					disk.worker_id != request.worker_id
						&& manifest.stopping.iter().any(|entry| {
							entry.worker_id == disk.worker_id && entry.permitted_at.is_some()
						})
				}) {
					Some(Reason::PeerStopping)
				} else if depot_types::stop::required_voters(
					&tablet.replica_set,
					&request.worker_id,
				)
				.iter()
				.any(|worker| !topology.workers.contains_key(worker))
				{
					Some(Reason::PeerDown)
				} else if depot_types::stop::loses_majority(&tablet.replica_set, &request.worker_id)
				{
					// A two-member set, current or joint next, keeps no majority without this
					// worker. It waits until a replacement or a returning peer restores one.
					Some(Reason::PeerStopping)
				} else {
					None
				};
				if let Some(reason) = reason {
					blockers.push(blocker(tablet, reason));
				}
			}
			if blockers.is_empty() {
				let round = self
					.stop_rounds
					.get(&request.worker_id)
					.filter(|r| r.check == candidate);
				match round.and_then(|round| round.results.as_ref()) {
					Some(results) => {
						for tablet in &tablets {
							match results.get(&tablet.id) {
								Some(StopReadinessResponse::Ready) => {}
								Some(StopReadinessResponse::NotReady(behind))
									if behind.blocking_volumes > 0 =>
								{
									let mut samples = behind.sample.clone();
									samples.sort_by_key(|s| {
										(
											s.appended_seq.saturating_mul(100)
												/ s.committed_seq.max(1),
											s.appended_seq,
										)
									});
									blockers.push(StopBlocker {
										tablet_id: tablet.id,
										reason: Reason::ReplicaBehind,
										volumes_behind: behind.blocking_volumes,
										detail: samples.into_iter().next(),
									});
								}
								_ => blockers.push(blocker(tablet, Reason::MaintainerOpening)),
							}
						}
						self.stop_rounds.remove(&request.worker_id);
						if !blockers.is_empty() {
							check = Some(candidate);
						}
					}
					None if tablets.is_empty() => {}
					None => {
						if round.is_none() {
							check = Some(candidate);
						}
						blockers.extend(
							tablets
								.iter()
								.map(|tablet| blocker(tablet, Reason::MaintainerOpening)),
						);
					}
				}
			} else if !waiting_handoff {
				// One tablet's missing peer must not prevent other maintainers from
				// closing this worker's released volumes. These replies are progress
				// evidence only: clearing blockers changes the check's purpose and
				// requires fresh permission evidence even if every reply was Ready.
				candidate.purpose = StopCheckPurpose::Progress;
				// A process can hold a client without storing that volume's replicas.
				// Visit affected tablets first; only progress checks include the others.
				candidate.tablets.extend(
					manifest
						.tablets
						.iter()
						.filter(|tablet| {
							!tablet
								.replica_set
								.disks
								.iter()
								.any(|disk| disk.worker_id == request.worker_id)
						})
						.cloned(),
				);
				// A late listing can reveal more closed volumes without changing
				// placement. Only an in-flight progress round is reusable.
				let reusable = self
					.stop_rounds
					.get(&request.worker_id)
					.is_some_and(|round| {
						round.check.purpose == StopCheckPurpose::Progress
							&& round.check.request == candidate.request
							&& round.results.is_none()
					});
				if !reusable {
					if self.stop_rounds.remove(&request.worker_id).is_some() {
						// An outage or a changed grant/configuration invalidates even completed evidence.
						effects.execute(Effect::CancelStopReadinessRequest {
							worker: request.worker_id,
						});
					}
					check = Some(candidate);
				}
			}
			if blockers.is_empty() {
				manifest
					.stopping
					.iter_mut()
					.find(|entry| entry.worker_id == request.worker_id)
					.expect("stop intent")
					.permitted_at = Some(command.requested_at);
				Response::StopPermitted
			} else {
				Response::StopPending(StopPending {
					total_blocking_tablets: blockers.len() as u64,
					blockers,
				})
			}
		};
		let mut completion = StopCompletion {
			entries: manifest.stopping.len() as u64,
			new_permit: !already_permitted && response == Response::StopPermitted,
			response,
			check,
		};
		if manifest != stored.manifest {
			let Some(version) = manifest.tablet_manifest_version.checked_add(1) else {
				reject(command.request_id, Reject::InvalidCommand, effects);
				return;
			};
			manifest.tablet_manifest_version = version;
			if let Some(check) = &mut completion.check {
				if matches!(check.purpose, StopCheckPurpose::Handoff { .. }) {
					check.purpose = StopCheckPurpose::Handoff {
						tablet_manifest_version: version,
					};
				}
			}
			if validate(&manifest).is_err()
				|| !crate::tablet_coordinator_machine::store::has_completion_headroom(
					&manifest,
					&self.alarms,
				) {
				reject(command.request_id, Reject::InvalidCommand, effects);
				return;
			}
			self.write(
				ManifestWrite {
					manifest,
					expected_etag: Some(stored.etag),
					origin: WriteOrigin::Stop {
						request_id: command.request_id,
						completion,
					},
				},
				effects,
			);
		} else {
			self.stop_confirmed(command.request_id, completion, effects);
		}
	}

	pub(super) fn stop_confirmed(
		&mut self,
		request_id: u64,
		completion: StopCompletion,
		effects: &mut Effects,
	) {
		effects.execute(Effect::ObserveStops {
			entries: completion.entries,
			new_permit: completion.new_permit,
		});
		effects.execute(Effect::CommandResponse {
			request_id,
			response: completion.response,
		});
		if let Some(check) = completion.check {
			if self.stop_rounds.len() >= self.max_requests
				&& !self.stop_rounds.contains_key(&check.request.worker_id)
			{
				return;
			}
			self.next_operation = self
				.next_operation
				.checked_add(1)
				.expect("operation id exhausted");
			let operation = self.next_operation;
			self.stop_rounds.insert(
				check.request.worker_id,
				StopRound {
					operation,
					check: check.clone(),
					results: None,
				},
			);
			effects.execute(Effect::StopReadinessRequest { operation, check });
		}
	}

	pub(in crate::tablet_coordinator_machine) fn stop_readiness(
		&mut self,
		operation: u64,
		worker: [u8; 16],
		results: BTreeMap<[u8; 16], StopReadinessResponse>,
	) {
		if let Some(round) = self.stop_rounds.get_mut(&worker)
			&& round.operation == operation
			&& !matches!(round.check.purpose, StopCheckPurpose::Handoff { .. })
		{
			round.results = Some(results);
		}
	}
}

/// Whether `worker`'s stop aborts `tablet`'s Learning phase. As the learner, its permit would
/// prevent promotion, so its Learning is always aborted. As a retained voter its stop waits
/// for promotion, so it aborts a rebalance, which replaces a member that is live and neither
/// stopping nor missing, and any change whose learner the balancer has seen missing for its
/// replacement grace. Placement proposes a rebalance only when no member's stop is pending,
/// so such an abort is not proposed again while this worker drains. A grow, or a replacement
/// of a stopping, absent or missing member, is what placement proposes during a drain, so it
/// keeps its learner and finishes first.
fn stop_aborts_learning(
	tablet: &Tablet,
	worker: &[u8; 16],
	manifest: &TabletManifest,
	topology: &StopTopology,
	missing: &BTreeSet<[u8; 16]>,
) -> bool {
	let TabletPhase::Learning(phase) = &tablet.replica_set.phase else {
		return false;
	};
	let members = &tablet.replica_set.members;
	if !phase.next.contains(worker) {
		return false;
	}
	if !members.contains(worker) {
		return true;
	}
	let leaving: Vec<_> = members
		.iter()
		.filter(|id| !phase.next.contains(id))
		.collect();
	let rebalance = !leaving.is_empty()
		&& leaving.iter().all(|id| {
			let stopping = manifest
				.stopping
				.iter()
				.any(|entry| entry.worker_id == **id);
			topology.workers.contains_key(*id) && !stopping && !missing.contains(*id)
		});
	let learner_missing = phase
		.next
		.iter()
		.filter(|id| !members.contains(id))
		.any(|id| missing.contains(id));
	rebalance || learner_missing
}

/// Earlier coordinators could permit a retained voter or learner while Learning
/// still depended on it. Repair that pre-Joint phase in the takeover's existing
/// barrier CAS, preserving the permit and all old voting authority.
pub(super) fn abort_permitted_learning(
	manifest: &mut TabletManifest,
	now: i64,
) -> Result<(), Reject> {
	let permitted: BTreeSet<_> = manifest
		.stopping
		.iter()
		.filter(|entry| entry.permitted_at.is_some())
		.map(|entry| entry.worker_id)
		.collect();
	let stranded: Vec<_> = manifest
		.tablets
		.iter()
		.filter_map(|tablet| {
			let TabletPhase::Learning(phase) = &tablet.replica_set.phase else {
				return None;
			};
			phase
				.next
				.iter()
				.any(|worker| permitted.contains(worker))
				.then_some(tablet.id)
		})
		.collect();
	for id in stranded {
		reconfiguration::abort(manifest, &id, now)?;
	}
	Ok(())
}

/// A phase change must not depend on a permitted disk unless it removes that disk.
pub(super) fn blocks_reconfiguration(
	manifest: &TabletManifest,
	tablet: &Tablet,
	leaving: Option<&[u8; 16]>,
) -> bool {
	manifest.stopping.iter().any(|entry| {
		entry.permitted_at.is_some()
			&& Some(&entry.worker_id) != leaving
			&& tablet
				.replica_set
				.disks
				.iter()
				.any(|disk| disk.worker_id == entry.worker_id)
	})
}
