//! The manifest is the whole control-plane state; queued commands never mutate it before CAS.

pub(crate) mod alarms;
pub mod balancing;
pub mod handoff;
mod reconfiguration;
mod regrant;
pub(super) mod split;
mod stop;
pub(crate) mod versions;

use super::{Effects, effect::Effect, types::*};
pub use reconfiguration::DEFAULT_LEARNING_DEADLINE_MS;
use rivet_data::generated::{
	depot_tablet_coordinator_protocol_v1::{
		Accepted, TabletCoordinatorRejectResponse as Reject, TabletCoordinatorRequest as Request,
		TabletCoordinatorResponse as Response,
	},
	depot_tablet_manifest_v1::{Tablet, TabletManifest, TabletPause},
	shared_depot_types_v1::{TabletPhase, TabletReplicaSet},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReadContext {
	/// Read after election, then invalidate the previous owner’s captured ETag with a CAS.
	Takeover,
	/// Reconcile this exact attempt without rebasing its command onto a newer manifest.
	Resolve(ManifestWrite),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
	/// One object-store read is outstanding; only its operation ID may complete it.
	Reading {
		operation: u64,
		context: ReadContext,
	},
	/// A failed I/O attempt retains its read/readback purpose through bounded backoff.
	Waiting { context: ReadContext },
	/// A single immutable CAS is outstanding; no command is acknowledged yet.
	Writing {
		operation: u64,
		write: ManifestWrite,
	},
	/// Owner RPCs protect this exact prepared write; its single absolute alarm is unchanged.
	CheckingHandoff {
		operation: u64,
		pending: handoff::PendingGrant,
	},
	/// No RPC is in flight; bounded backoff retains the same write and create barriers.
	WaitingHandoff { pending: handoff::PendingGrant },
	/// A takeover or command write is confirmed; queued commands may inspect this value.
	Ready(StoredManifest),
	/// Authority or manifest validation failed; all future I/O completions are ignored.
	Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct State {
	pub balancing: Option<balancing::Balancing>,
	pub(crate) alarms: alarms::AlarmManifestState,
	pub stop_rounds: std::collections::BTreeMap<[u8; 16], StopRound>,
	pub phase: Phase,
	pub queued: VecDeque<Command>,
	pub max_requests: usize,
	pub operation_timeout_ms: u64,
	pub retry_ms: u64,
	/// How long a replica-set change may stay in Learning before it is aborted.
	pub learning_deadline_ms: u64,
	/// The manifest version whose Learning abort was last dropped, so it is reported once.
	dropped_learning_abort: Option<u64>,
	next_operation: u64,
	/// Latest storage discovery observation from the runner; in memory only.
	pub discovery: Option<StopTopology>,
	/// Tablets already reported as having no live member to re-grant to, so each is
	/// reported once until it is placed again.
	pub unplaceable: BTreeSet<[u8; 16]>,
}

impl Default for State {
	fn default() -> Self {
		Self {
			balancing: None,
			alarms: Default::default(),
			phase: Phase::Waiting {
				context: ReadContext::Takeover,
			},
			queued: VecDeque::new(),
			max_requests: 128,
			stop_rounds: Default::default(),
			operation_timeout_ms: 5_000,
			retry_ms: 100,
			learning_deadline_ms: reconfiguration::DEFAULT_LEARNING_DEADLINE_MS,
			dropped_learning_abort: None,
			next_operation: 0,
			discovery: None,
			unplaceable: BTreeSet::new(),
		}
	}
}

impl State {
	pub(super) fn read(&mut self, context: ReadContext, effects: &mut Effects) {
		let operation = self.operation(effects);
		self.phase = Phase::Reading { operation, context };
		effects.execute(Effect::ReadManifestRequest { operation });
	}

	fn write(&mut self, write: ManifestWrite, effects: &mut Effects) {
		let operation = self.operation(effects);
		self.phase = Phase::Writing {
			operation,
			write: write.clone(),
		};
		if self.alarms.enabled {
			let envelope = self.alarms.envelope(write.manifest.clone());
			effects.execute(Effect::WriteAlarmManifestRequest {
				operation,
				write,
				envelope,
			});
		} else {
			effects.execute(Effect::WriteManifestRequest { operation, write });
		}
	}

	fn operation(&mut self, effects: &mut Effects) -> u64 {
		self.next_operation = self
			.next_operation
			.checked_add(1)
			.expect("operation id exhausted");
		effects.arm_timer_after(
			Timer::Operation(self.next_operation),
			self.operation_timeout_ms,
		);
		self.next_operation
	}

	/// Authority loss rejects callers without treating cancellation as rollback of a sent CAS.
	pub(super) fn stop(&mut self, effects: &mut Effects) {
		let previous = std::mem::replace(&mut self.phase, Phase::Stopped);
		self.stop_rounds.clear();
		match previous {
			Phase::Reading { operation, context } => {
				effects.cancel_timer(Timer::Operation(operation));
				if let ReadContext::Resolve(write) = context {
					reject_write(write, effects);
				}
			}
			Phase::Waiting { context } => {
				if let ReadContext::Resolve(write) = context {
					reject_write(write, effects);
				}
			}
			Phase::Writing { operation, write } => {
				effects.cancel_timer(Timer::Operation(operation));
				reject_write(write, effects);
			}
			Phase::CheckingHandoff { pending, .. } | Phase::WaitingHandoff { pending } => {
				effects.cancel_timer(Timer::Operation(pending.generation));
				effects.cancel_timer(Timer::HandoffRetry(pending.generation));
				for worker in pending.checks.keys() {
					effects.execute(Effect::CancelStopReadinessRequest { worker: *worker });
				}
				reject_write(pending.write, effects);
			}
			Phase::Ready(_) | Phase::Stopped => {}
		}
		effects.cancel_timer(Timer::Retry);
		effects.cancel_timer(Timer::LearningDeadline);
		if let Some(balance) = self.balancing.as_mut() {
			balance.stop(effects);
		}
		for command in self.queued.drain(..) {
			reject(command.request_id, Reject::NotCoordinator, effects);
		}
	}

	pub(super) fn read_response(
		&mut self,
		operation: u64,
		result: ReadResult,
		effects: &mut Effects,
	) {
		let Phase::Reading {
			operation: current,
			context,
		} = &self.phase
		else {
			return;
		};
		if *current != operation {
			return;
		}
		let context = context.clone();
		let result = match result {
			ReadResult::FoundAlarm { envelope, etag } => {
				if alarms::validate(&envelope).is_err() {
					self.stop(effects);
					return;
				}
				if matches!(context, ReadContext::Takeover) {
					self.alarms.install(&envelope);
				} else if let ReadContext::Resolve(write) = &context {
					if write.manifest == envelope.manifest
						&& self.alarms.envelope(write.manifest.clone()) != envelope
					{
						self.stop(effects);
						return;
					}
				}
				ReadResult::Found(StoredManifest {
					manifest: envelope.manifest,
					etag,
				})
			}
			result => result,
		};
		effects.cancel_timer(Timer::Operation(operation));

		// Never confuse failed reads with an absent control object.
		let observed = match result {
			ReadResult::Found(stored) => {
				if validate(&stored.manifest).is_err() {
					self.stop(effects);
					return;
				}
				Some(stored)
			}
			ReadResult::Missing => None,
			ReadResult::Unknown(_) => {
				self.phase = Phase::Waiting { context };
				effects.arm_timer_after(Timer::Retry, self.retry_ms);
				return;
			}
			ReadResult::FoundAlarm { .. } => unreachable!("normalized alarm manifest"),
			ReadResult::Invalid(_) => {
				self.stop(effects);
				return;
			}
		};

		match context {
			ReadContext::Takeover => {
				let (mut manifest, expected_etag) = observed.map_or_else(
					|| {
						(
							TabletManifest {
								stopping: vec![],
								tablet_manifest_version: 0,
								tablets: vec![],
								retired_tablets: vec![],
								paused: vec![],
							},
							None,
						)
					},
					|stored| (stored.manifest, Some(stored.etag)),
				);
				let Some(version) = manifest.tablet_manifest_version.checked_add(1) else {
					self.stop(effects);
					return;
				};
				manifest.tablet_manifest_version = version;
				if stop::abort_permitted_learning(&mut manifest, effects.clocks().unix_ms).is_err()
				{
					self.stop(effects);
					return;
				}
				// A successful takeover CAS invalidates all preconditions captured by the prior singleton.
				self.write(
					ManifestWrite {
						origin: WriteOrigin::Takeover,
						manifest,
						expected_etag,
					},
					effects,
				);
			}
			ReadContext::Resolve(write) => {
				if let Some(stored) = &observed
					&& stored.manifest == write.manifest
				{
					self.confirm(write, stored.etag.clone(), effects);
					return;
				}
				let etag = observed.as_ref().map(|stored| &stored.etag);
				if etag == write.expected_etag.as_ref() {
					// The old value can race a sent write. Reissue only that exact conditional attempt.
					self.write(write, effects);
				} else if matches!(write.origin, WriteOrigin::Takeover) {
					// A predecessor's last write may beat the takeover barrier. Read its
					// result and fence that ETag before serving; no caller command is rebased.
					self.read(ReadContext::Takeover, effects);
				} else {
					// Another owner advanced the object. Do not apply this command atop its newer precondition.
					self.stop(effects);
				}
			}
		}
	}

	pub(super) fn write_response(
		&mut self,
		operation: u64,
		result: WriteResult,
		effects: &mut Effects,
	) {
		let Phase::Writing {
			operation: current,
			write,
		} = &self.phase
		else {
			return;
		};
		if *current != operation {
			return;
		}
		let write = write.clone();
		effects.cancel_timer(Timer::Operation(operation));
		match result {
			WriteResult::Confirmed { etag } => self.confirm(write, etag, effects),
			WriteResult::Invalid(_) => self.stop(effects),
			WriteResult::Precondition | WriteResult::Unknown(_) => {
				// A fast refusal followed by an unchanged read can otherwise cycle
				// GET/PUT without yielding. Retain this exact CAS, including its
				// precondition, while the existing retry timer delays readback.
				self.phase = Phase::Waiting {
					context: ReadContext::Resolve(write),
				};
				effects.arm_timer_after(Timer::Retry, self.retry_ms);
			}
		}
	}

	fn confirm(&mut self, write: ManifestWrite, etag: String, effects: &mut Effects) {
		match write.origin {
			WriteOrigin::Takeover => {}
			WriteOrigin::HandoffAborted { request_ids } => {
				for request_id in request_ids {
					reject(request_id, Reject::Overloaded, effects);
				}
			}
			WriteOrigin::Commands {
				request_ids,
				regranted,
			} => {
				for request_id in request_ids {
					accepted(request_id, write.manifest.tablet_manifest_version, effects);
				}
				if regranted > 0 {
					effects.execute(Effect::ObserveRegrants {
						granted: regranted,
						no_member: vec![],
					});
				}
			}
			WriteOrigin::Stop {
				request_id,
				completion,
			} => self.stop_confirmed(request_id, completion, effects),
		}
		self.phase = Phase::Ready(StoredManifest {
			manifest: write.manifest,
			etag,
		});
	}

	pub(super) fn timeout(&mut self, operation: u64, effects: &mut Effects) {
		match &self.phase {
			Phase::CheckingHandoff { pending, .. } | Phase::WaitingHandoff { pending }
				if pending.generation == operation =>
			{
				self.abort_handoff(effects)
			}
			Phase::Reading {
				operation: current,
				context,
			} if *current == operation => {
				self.phase = Phase::Waiting {
					context: context.clone(),
				};
				effects.arm_timer_after(Timer::Retry, self.retry_ms);
			}
			Phase::Writing {
				operation: current,
				write,
			} if *current == operation => self.read(ReadContext::Resolve(write.clone()), effects),
			Phase::Reading { .. }
			| Phase::CheckingHandoff { .. }
			| Phase::WaitingHandoff { .. }
			| Phase::Writing { .. }
			| Phase::Waiting { .. }
			| Phase::Ready(_)
			| Phase::Stopped => {}
		}
	}

	/// Commands before the next stop or voluntary grant share one CAS. A grant
	/// retains its immutable candidate while its old owner completes admitted creates.
	/// Stop requests keep their own durable intent and readiness rounds.
	pub(super) fn drive(&mut self, effects: &mut Effects, entropy: &rivet_state_machine::Entropy) {
		while let Phase::Ready(stored) = &self.phase {
			if let Some(front) = self.queued.front()
				&& matches!(front.request, Request::RequestStop(_))
			{
				let command = self.queued.pop_front().expect("front command");
				self.request_stop(command, effects);
				continue;
			}
			let stored = stored.clone();
			let stop_next = self.write_batch(stored, effects, entropy);
			if !stop_next && !(matches!(self.phase, Phase::Ready(_)) && !self.queued.is_empty()) {
				return;
			}
		}
	}

	/// Applies every queued command before the next stop request to one copy of `stored`,
	/// then re-grants tablets whose maintainer left an unexpired discovery observation.
	///
	/// Each command is applied and validated on its own, so a failing command is rejected
	/// alone and the others still apply in order. Commands that changed nothing are answered
	/// at once only while nothing earlier in the batch changed the manifest; otherwise their
	/// acceptance waits for the batch write, since their result may depend on it. Returns
	/// whether a stop request is next and nothing was written.
	fn write_batch(
		&mut self,
		stored: StoredManifest,
		effects: &mut Effects,
		entropy: &rivet_state_machine::Entropy,
	) -> bool {
		let rollback_alarms = self.alarms.clone();
		let mut manifest = stored.manifest.clone();
		let mut request_ids = Vec::new();
		let mut voluntary_grants = BTreeSet::new();
		let mut changed = self.cleanup_stops(&mut manifest, effects);
		while let Some(front) = self.queued.front() {
			if matches!(front.request, Request::RequestStop(_))
				|| (matches!(
					front.request,
					Request::GrantTablet(_) | Request::MoveTablet(_)
				) && changed)
			{
				break;
			}
			let command = self.queued.pop_front().expect("front command");
			let grant_command = matches!(
				command.request,
				Request::GrantTablet(_) | Request::MoveTablet(_)
			);
			if let Request::SplitTablet(request) = &command.request
				&& manifest.tablets.iter().any(|t| t.split.is_some())
				&& !manifest
					.tablets
					.iter()
					.any(|t| t.id == request.tablet_id && t.split.is_some())
			{
				reject(command.request_id, Reject::SplitInProgress, effects);
				continue;
			}
			let mut candidate = manifest.clone();
			let special = self.alarms.commands.remove(&command.request_id);
			let mut alarm_candidate = self.alarms.clone();
			let applied = if self.alarms.enabled {
				alarm_candidate.apply(&mut candidate, &command, special, self.discovery.as_ref())
			} else {
				apply(&mut candidate, &command, self.discovery.as_ref())
			};
			match applied {
				Err(reason) => reject(command.request_id, reason, effects),
				Ok(false) if !changed => accepted(
					command.request_id,
					manifest.tablet_manifest_version,
					effects,
				),
				Ok(false) => request_ids.push(command.request_id),
				Ok(true) => {
					if validate(&candidate).is_err()
						|| !super::store::has_completion_headroom(&candidate, &alarm_candidate)
						|| (candidate
							.tablets
							.iter()
							.filter(|t| t.split.is_some())
							.count() > 1 && candidate
							.tablets
							.iter()
							.filter(|t| t.split.is_some())
							.count() > manifest
							.tablets
							.iter()
							.filter(|t| t.split.is_some())
							.count()) || (alarm_candidate.enabled
						&& alarms::validate(&alarm_candidate.envelope(candidate.clone())).is_err())
					{
						reject(command.request_id, Reject::InvalidCommand, effects);
						continue;
					}
					if let Request::GrantTablet(request) = &command.request {
						voluntary_grants.insert(request.tablet_id);
					}
					if let Request::MoveTablet(request) = &command.request {
						voluntary_grants.insert(request.tablet_id);
					}
					manifest = candidate;
					alarm_candidate.commands = std::mem::take(&mut self.alarms.commands);
					self.alarms = alarm_candidate;
					changed = true;
					request_ids.push(command.request_id);
				}
			}
			if grant_command && changed {
				break;
			}
		}
		let stop_next = !self.queued.is_empty();

		self.prepare_balancing(&manifest, effects);
		changed |= self.expire_learning(&mut manifest, effects);
		let (regranted, mut missing_owner_grants) = self.regrant(&mut manifest, effects);
		// An operator's voluntary change cannot silently acquire failover authority
		// merely because the same batch also runs automatic missing-owner repair.
		missing_owner_grants.retain(|id| !voluntary_grants.contains(id));
		changed |= regranted > 0;
		if !changed && !stop_next {
			changed |= self.balance(&mut manifest, effects, entropy);
			self.publish_balancing(&stored.manifest, effects);
		}
		if !changed {
			return stop_next;
		}
		let Some(version) = manifest.tablet_manifest_version.checked_add(1) else {
			for request_id in request_ids {
				reject(request_id, Reject::InvalidCommand, effects);
			}
			return false;
		};
		manifest.tablet_manifest_version = version;
		self.prepare_grant_write(
			stored.clone(),
			ManifestWrite {
				origin: WriteOrigin::Commands {
					request_ids,
					regranted,
				},
				manifest,
				expected_etag: Some(stored.etag.clone()),
			},
			rollback_alarms,
			missing_owner_grants,
			effects,
		);
		false
	}

	/// Assigned workers the balancer has seen missing for its replacement grace. The evidence
	/// comes from the discovery snapshot, which queueing cannot extend past its monotonic
	/// lifetime, so a stale snapshot proves nothing missing.
	pub(super) fn missing_workers(&self, now_boot_ms: u64) -> BTreeSet<[u8; 16]> {
		match (&self.balancing, &self.discovery) {
			(Some(balancing), Some(discovery)) if now_boot_ms < discovery.valid_until_boot_ms => {
				balancing.missing(now_boot_ms)
			}
			_ => BTreeSet::new(),
		}
	}

	/// Aborts Learning phases past their deadline, or whose learner the balancer has seen
	/// missing for its replacement grace, on `manifest`, and arms the timer for the next
	/// deadline. The deadline counts from the phase's persisted start, so a new coordinator
	/// re-arms it from the manifest it reads. Returns whether any phase was aborted; an abort
	/// that would leave the manifest invalid is dropped until a later drive, which runs on the
	/// next manifest change or coordinator event.
	fn expire_learning(&mut self, manifest: &mut TabletManifest, effects: &mut Effects) -> bool {
		if !manifest
			.tablets
			.iter()
			.any(|tablet| matches!(tablet.replica_set.phase, TabletPhase::Learning(_)))
		{
			effects.cancel_timer(Timer::LearningDeadline);
			return false;
		}
		let missing = self.missing_workers(effects.now_boot_ms());
		let mut candidate = manifest.clone();
		let (expired, next) = reconfiguration::expire_learning(
			&mut candidate,
			self.learning_deadline_ms,
			&missing,
			effects.clocks().unix_ms,
		);
		match next {
			Some(delay_ms) => effects.arm_timer_after(Timer::LearningDeadline, delay_ms),
			None => effects.cancel_timer(Timer::LearningDeadline),
		}
		if expired.is_empty() {
			return false;
		}
		if validate(&candidate).is_err()
			|| !super::store::has_completion_headroom(&candidate, &self.alarms)
		{
			// The next manifest change or coordinator event tries again. Report the
			// dropped abort once per manifest version, not on every drive.
			if self.dropped_learning_abort != Some(manifest.tablet_manifest_version) {
				self.dropped_learning_abort = Some(manifest.tablet_manifest_version);
				tracing::warn!(
					tablets = expired.len(),
					tablet_manifest_version = manifest.tablet_manifest_version,
					"dropped a Learning abort that would leave the manifest invalid or without completion headroom; it is tried again when the manifest changes"
				);
			}
			return false;
		}
		*manifest = candidate;
		true
	}

	/// Re-grants orphaned tablets on `manifest` when the discovery observation is unexpired,
	/// and reports newly unplaceable tablets once. Returns how many tablets were re-granted;
	/// a re-grant that would leave the manifest invalid is dropped entirely.
	fn regrant(
		&mut self,
		manifest: &mut TabletManifest,
		effects: &mut Effects,
	) -> (u64, BTreeSet<[u8; 16]>) {
		let Some(discovery) = &self.discovery else {
			return (0, BTreeSet::new());
		};
		// Queueing cannot extend a discovery snapshot's monotonic lifetime.
		if effects.now_boot_ms() >= discovery.valid_until_boot_ms {
			return (0, BTreeSet::new());
		}
		let mut candidate = manifest.clone();
		let available = self
			.balancing
			.as_ref()
			.and_then(|b| b.search.as_ref())
			.map(|search| &search.input.available);
		let regrants = regrant::regrant(
			&mut candidate,
			discovery,
			available,
			effects.clocks().unix_ms,
		);
		let newly_unplaceable: Vec<[u8; 16]> = regrants
			.no_member
			.difference(&self.unplaceable)
			.copied()
			.collect();
		self.unplaceable = regrants.no_member;
		if !newly_unplaceable.is_empty() {
			effects.execute(Effect::ObserveRegrants {
				granted: 0,
				no_member: newly_unplaceable,
			});
		}
		if regrants.granted == 0
			|| validate(&candidate).is_err()
			|| !super::store::has_completion_headroom(&candidate, &self.alarms)
		{
			return (0, BTreeSet::new());
		}
		let missing_owner_grants = manifest
			.tablets
			.iter()
			.filter(|old| {
				candidate
					.tablets
					.iter()
					.any(|new| new.id == old.id && new.maintainer != old.maintainer)
			})
			.map(|tablet| tablet.id)
			.collect();
		*manifest = candidate;
		(regrants.granted, missing_owner_grants)
	}
}

fn reject_write(write: ManifestWrite, effects: &mut Effects) {
	match write.origin {
		WriteOrigin::Takeover => {}
		WriteOrigin::Commands { request_ids, .. } | WriteOrigin::HandoffAborted { request_ids } => {
			for request_id in request_ids {
				reject(request_id, Reject::NotCoordinator, effects);
			}
		}
		WriteOrigin::Stop { request_id, .. } => reject(request_id, Reject::NotCoordinator, effects),
	}
}

pub(super) fn reject(request_id: u64, reason: Reject, effects: &mut Effects) {
	effects.execute(Effect::CommandResponse {
		request_id,
		response: Response::TabletCoordinatorRejectResponse(reason),
	});
}

fn accepted(request_id: u64, version: u64, effects: &mut Effects) {
	effects.execute(Effect::CommandResponse {
		request_id,
		response: Response::Accepted(Accepted {
			tablet_manifest_version: version,
		}),
	});
}

/// Validate identity, range and grant invariants before making a stored manifest authoritative.
pub fn validate(manifest: &TabletManifest) -> anyhow::Result<()> {
	split::validate(manifest)?;
	let mut stopping = BTreeSet::new();
	for entry in &manifest.stopping {
		anyhow::ensure!(stopping.insert(entry.worker_id), "duplicate stop intent");
	}
	for tablet in &manifest.tablets {
		anyhow::ensure!(
			manifest
				.stopping
				.iter()
				.filter(|entry| entry.permitted_at.is_some()
					&& tablet
						.replica_set
						.disks
						.iter()
						.any(|disk| disk.worker_id == entry.worker_id))
				.count() <= 1,
			"multiple permits for one tablet"
		);
	}
	let mut paused = BTreeSet::new();
	for pause in &manifest.paused {
		anyhow::ensure!(paused.insert(pause.tablet_id), "duplicate tablet pause");
		anyhow::ensure!(
			manifest
				.tablets
				.iter()
				.any(|tablet| tablet.id == pause.tablet_id),
			"pause names no live tablet"
		);
	}
	let mut ids = BTreeSet::new();
	let mut ranges = vec![];
	for tablet in &manifest.tablets {
		anyhow::ensure!(ids.insert(tablet.id), "duplicate tablet identity");
		anyhow::ensure!(tablet.hash_range.is_valid(), "empty tablet range");
		reconfiguration::validate(&tablet.replica_set)?;
		let mut disk_members: BTreeSet<_> = tablet.replica_set.members.iter().copied().collect();
		match &tablet.replica_set.phase {
			TabletPhase::Stable => {}
			TabletPhase::Learning(phase) => disk_members.extend(&phase.next),
			TabletPhase::Joint(phase) => disk_members.extend(&phase.next),
		}
		anyhow::ensure!(
			tablet.replica_set.disks.len() == disk_members.len()
				&& tablet
					.replica_set
					.disks
					.iter()
					.map(|disk| disk.worker_id)
					.collect::<BTreeSet<_>>()
					== disk_members
				&& tablet
					.replica_set
					.disks
					.iter()
					.all(|disk| disk.disk_incarnation != [0; 16]),
			"tablet requires an admitted disk for every member and learner"
		);

		anyhow::ensure!(
			tablet
				.maintainer
				.is_none_or(|id| reconfiguration::may_maintain(tablet, &id))
				&& (tablet.maintainer.is_none() || tablet.tablet_grant_epoch > 0),
			"invalid tablet grant"
		);
		ranges.push((tablet.hash_range.start, tablet.hash_range.end));
	}
	ranges.sort_unstable();
	anyhow::ensure!(
		ranges
			.windows(2)
			.all(|pair| pair[0].1.is_some_and(|end| end <= pair[1].0)),
		"overlapping tablet ranges"
	);
	Ok(())
}

/// Grants a fresh epoch for every named grant that is still its tablet's current grant and
/// still held by the requesting owner. A replay or a late request cannot replace a newer grant
/// because the epoch must still match; such a grant is skipped, so one stale entry never
/// holds back the rest of a restarted worker's tablets. Fails only when no grant changed, with
/// the first grant's rejection.
fn recover(
	manifest: &mut TabletManifest,
	request: &rivet_data::generated::depot_tablet_coordinator_protocol_v1::RecoverTablets,
	discovery: Option<&StopTopology>,
	requested_at: i64,
) -> Result<bool, Reject> {
	// A stopping worker drains its tablets; recovery must not hand one back.
	if manifest
		.stopping
		.iter()
		.any(|entry| entry.worker_id == request.maintainer)
	{
		return Err(Reject::InvalidCommand);
	}
	let mut changed = false;
	let mut first_rejection = None;
	for grant in &request.grants {
		match recover_one(manifest, request.maintainer, grant, discovery, requested_at) {
			Ok(()) => changed = true,
			Err(reject) => {
				first_rejection.get_or_insert(reject);
			}
		}
	}
	match (changed, first_rejection) {
		(false, Some(reject)) => Err(reject),
		(true, _) | (false, None) => Ok(changed),
	}
}

/// Grants a fresh epoch for one grant of `recover`.
fn recover_one(
	manifest: &mut TabletManifest,
	maintainer: [u8; 16],
	grant: &rivet_data::generated::depot_tablet_coordinator_protocol_v1::SpentTabletGrant,
	discovery: Option<&StopTopology>,
	requested_at: i64,
) -> Result<(), Reject> {
	let tablet = manifest
		.tablets
		.iter_mut()
		.find(|tablet| tablet.id == grant.tablet_id)
		.ok_or(Reject::UnknownTablet)?;
	if tablet.tablet_grant_epoch != grant.tablet_grant_epoch {
		return Err(Reject::StaleTabletGrantEpoch(
			rivet_data::generated::depot_tablet_coordinator_protocol_v1::StaleTabletGrantEpoch {
				tablet_grant_epoch: tablet.tablet_grant_epoch,
			},
		));
	}
	if tablet.maintainer != Some(maintainer) {
		return Err(Reject::InvalidCommand);
	}
	// An owner listed with versions that cannot read the tablet would refuse it again at
	// open. Regrant moves the tablet to a member that can. An unlisted owner is not refused
	// here: recovery is for the owner that already holds the grant, and its open still checks.
	if discovery.is_some_and(|discovery| discovery.versions.contains_key(&maintainer)) {
		versions::check(tablet, &maintainer, discovery).map_err(Reject::VersionUnreadable)?;
	}
	tablet.tablet_grant_epoch = tablet
		.tablet_grant_epoch
		.checked_add(1)
		.filter(|epoch| *epoch < u64::MAX && (tablet.split.is_none() || *epoch < u64::MAX - 1))
		.ok_or(Reject::InvalidCommand)?;
	tablet.granted_at = requested_at;
	Ok(())
}

/// Apply manual placement without consulting topology or performing storage I/O.
fn apply(
	manifest: &mut TabletManifest,
	command: &Command,
	discovery: Option<&StopTopology>,
) -> Result<bool, Reject> {
	match &command.request {
		Request::RequestStop(_) => Err(Reject::InvalidCommand),
		Request::RecoverTablets(request) => {
			recover(manifest, request, discovery, command.requested_at)
		}
		Request::CreateTablet(request) => {
			if request.members.iter().any(|disk| {
				manifest
					.stopping
					.iter()
					.any(|entry| entry.worker_id == disk.worker_id && entry.permitted_at.is_some())
			}) || !request.hash_range.is_valid()
				|| !(1..=3).contains(&request.members.len())
				|| request
					.members
					.iter()
					.map(|disk| disk.worker_id)
					.collect::<BTreeSet<_>>()
					.len() != request.members.len()
				|| request
					.members
					.iter()
					.any(|disk| disk.disk_incarnation == [0; 16])
			{
				return Err(Reject::InvalidCommand);
			}
			for tablet in &manifest.tablets {
				if tablet.hash_range.start == request.hash_range.start
					&& tablet.hash_range.end == request.hash_range.end
					&& tablet
						.replica_set
						.disks
						.iter()
						.map(|disk| (disk.worker_id, disk.disk_incarnation))
						.collect::<BTreeSet<_>>()
						== request
							.members
							.iter()
							.map(|disk| (disk.worker_id, disk.disk_incarnation))
							.collect()
				{
					return Ok(false);
				}
				if tablet.hash_range.overlaps(&request.hash_range) {
					return Err(Reject::InvalidCommand);
				}
			}
			if manifest
				.tablets
				.iter()
				.any(|tablet| tablet.id == command.tablet_id)
			{
				return Err(Reject::InvalidCommand);
			}
			manifest.tablets.push(Tablet {
				id: command.tablet_id,
				hash_range: request.hash_range,
				maintainer: None,
				tablet_grant_epoch: 0,
				granted_at: 0,
				replica_set: TabletReplicaSet {
					disks: request.members.clone(),
					tablet_replica_set_version: 1,
					members: request.members.iter().map(|disk| disk.worker_id).collect(),
					phase: TabletPhase::Stable,
				},
				phase_started_at: command.requested_at,
				split: None,
				parent_tablet_id: None,
				parent_range_start: None,
				version_marks: Vec::new(),
			});
			Ok(true)
		}
		Request::GrantTablet(request) => {
			readable_grant(manifest, request.tablet_id, request.maintainer, discovery)?;
			grant(
				manifest,
				request.tablet_id,
				request.maintainer,
				command.requested_at,
			)
		}
		Request::MoveTablet(request) => {
			readable_grant(manifest, request.tablet_id, request.maintainer, discovery)?;
			grant(
				manifest,
				request.tablet_id,
				request.maintainer,
				command.requested_at,
			)
		}
		Request::VersionMarksReport(report) => versions::report(manifest, report),
		Request::ReplaceReplica(request) => {
			reconfiguration::replace(manifest, request, command.requested_at)
		}
		Request::AbortReconfiguration(request) => {
			reconfiguration::abort(manifest, &request.tablet_id, command.requested_at)
		}
		Request::CatchUpReport(report) => {
			reconfiguration::report(manifest, report, command.requested_at)
		}
		Request::SplitTablet(request) => {
			let changed = split::start(
				manifest,
				request,
				command.split.as_ref().ok_or(Reject::InvalidCommand)?,
				command.requested_at,
			)?;
			if changed {
				let tablet = manifest
					.tablets
					.iter_mut()
					.find(|tablet| tablet.id == request.tablet_id)
					.ok_or(Reject::UnknownTablet)?;
				versions::mark_split_snapshot(tablet, discovery);
			}
			Ok(changed)
		}
		Request::PauseTablet(request) => pause(manifest, request.tablet_id, command.requested_at),
		Request::ResumeTablet(request) => resume(manifest, request.tablet_id),
		Request::SplitPrepared(report) => split::prepared(manifest, report, command.requested_at),
		Request::RetiredTabletDeleted(report) => split::deleted(manifest, report),
		Request::ParentLogImported(report) => split::imported(manifest, report),
		// Only the alarm-index readiness report travels as `Repair`, and the alarm state
		// applies it before this path.
		Request::Repair(_) => Err(Reject::InvalidCommand),
	}
}

/// Holds the balancer off a live tablet. Repeating a pause changes nothing; the first
/// pause's time stays.
fn pause(manifest: &mut TabletManifest, id: [u8; 16], now: i64) -> Result<bool, Reject> {
	if !manifest.tablets.iter().any(|tablet| tablet.id == id) {
		return Err(Reject::UnknownTablet);
	}
	if manifest.paused.iter().any(|pause| pause.tablet_id == id) {
		return Ok(false);
	}
	manifest.paused.push(TabletPause {
		tablet_id: id,
		paused_at: now,
	});
	Ok(true)
}

/// Lifts a pause. A tablet that is not paused is accepted unchanged, so a repeated resume
/// is idempotent; an unknown tablet is refused.
fn resume(manifest: &mut TabletManifest, id: [u8; 16]) -> Result<bool, Reject> {
	if !manifest.tablets.iter().any(|tablet| tablet.id == id) {
		return Err(Reject::UnknownTablet);
	}
	let before = manifest.paused.len();
	manifest.paused.retain(|pause| pause.tablet_id != id);
	Ok(manifest.paused.len() != before)
}

/// Refuses an operator grant to a worker that cannot read the tablet's data.
fn readable_grant(
	manifest: &TabletManifest,
	id: [u8; 16],
	maintainer: [u8; 16],
	discovery: Option<&StopTopology>,
) -> Result<(), Reject> {
	let tablet = manifest
		.tablets
		.iter()
		.find(|tablet| tablet.id == id)
		.ok_or(Reject::UnknownTablet)?;
	versions::check(tablet, &maintainer, discovery).map_err(Reject::VersionUnreadable)?;
	Ok(())
}

fn grant(
	manifest: &mut TabletManifest,
	id: [u8; 16],
	maintainer: [u8; 16],
	now: i64,
) -> Result<bool, Reject> {
	grant_authorized(manifest, id, maintainer, now, GrantAuthority::Ordinary)
}

/// Voluntary changes require correlated owner readiness. The existing missing-owner
/// failover policy has separate authority; an Active-discovery omission alone does
/// not become a new lease-fencing proof for an operator's command.
enum GrantAuthority {
	Ordinary,
	StoppedOwnerReady,
	OwnerMissing,
}

fn grant_authorized(
	manifest: &mut TabletManifest,
	id: [u8; 16],
	maintainer: [u8; 16],
	now: i64,
	authority: GrantAuthority,
) -> Result<bool, Reject> {
	if manifest
		.stopping
		.iter()
		.any(|entry| entry.worker_id == maintainer)
	{
		return Err(Reject::InvalidCommand);
	}
	if matches!(authority, GrantAuthority::Ordinary)
		&& manifest
			.tablets
			.iter()
			.find(|tablet| tablet.id == id)
			.and_then(|tablet| tablet.maintainer)
			.is_some_and(|owner| {
				manifest
					.stopping
					.iter()
					.any(|entry| entry.worker_id == owner)
			}) {
		return Err(Reject::InvalidCommand);
	}
	let tablet = manifest
		.tablets
		.iter_mut()
		.find(|tablet| tablet.id == id)
		.ok_or(Reject::UnknownTablet)?;
	if !reconfiguration::may_maintain(tablet, &maintainer) {
		return Err(Reject::InvalidCommand);
	}
	if tablet.maintainer == Some(maintainer) {
		return Ok(false);
	}
	let epoch = tablet
		.tablet_grant_epoch
		.checked_add(1)
		.filter(|epoch| *epoch < u64::MAX && (tablet.split.is_none() || *epoch < u64::MAX - 1))
		.ok_or(Reject::InvalidCommand)?;
	tablet.maintainer = Some(maintainer);
	tablet.tablet_grant_epoch = epoch;
	tablet.granted_at = now;
	Ok(true)
}

impl rivet_state_machine::StateMachineState for State {}
