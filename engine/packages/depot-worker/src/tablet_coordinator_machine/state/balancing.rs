//! Placement observations and deadlines live in the existing coordinator core.

use super::{State, grant, reconfiguration};
use crate::balancing_config::TabletBalancingConfig;
use crate::tablet_coordinator_machine::{
	Effects,
	effect::Effect,
	placement::{self, Id},
	types::Timer,
};
use rivet_data::generated::{
	depot_tablet_coordinator_protocol_v1::{
		CreateTablet, ReplaceReplica, SplitTablet, TabletCoordinatorRequest,
	},
	depot_tablet_manifest_v1::{TabletManifest, TabletSplit},
	shared_depot_types_v1::StorageWorkerDisk,
};
use rivet_state_machine::Entropy;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockReason {
	WaitingForWorkers,
	ReplacementGrace,
	NoDestinationOrQuorum,
	SplitBlocked,
	ManifestHeadroom,
	/// Automatic placement is switched off by configuration.
	Disabled,
}
impl BlockReason {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::WaitingForWorkers => "workers",
			Self::ReplacementGrace => "grace",
			Self::NoDestinationOrQuorum => "destination_or_quorum",
			Self::SplitBlocked => "split",
			Self::ManifestHeadroom => "manifest_headroom",
			Self::Disabled => "disabled",
		}
	}
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Status {
	pub desired_tablets: u64,
	pub projected_tablets: u64,
	pub admitted_workers: u64,
	pub pending_splits: u64,
	pub pending_replacements: u64,
	/// Tablets an operator paused, which the balancer leaves alone.
	pub paused_tablets: u64,
	pub search_complete: bool,
	pub replica_imbalance_ppm: u64,
	pub maintainer_imbalance_ppm: u64,
	pub blocked: Option<BlockReason>,
	pub disk_probes_in_flight: u32,
	pub waiting_disk_probes: usize,
	pub disk_probe_capacity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Settings {
	pub tablets_per_core: u32,
	pub destination_stability_ms: u64,
	pub missing_worker_ms: u64,
	pub max_disk_probes: u32,
}
impl Default for Settings {
	fn default() -> Self {
		Self {
			tablets_per_core: 2,
			destination_stability_ms: 60_000,
			missing_worker_ms: 120_000,
			max_disk_probes: 4,
		}
	}
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Probe {
	Waiting { at_boot_ms: u64 },
	Running { operation: u64 },
	Verified { disk: Id },
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Observation {
	instance: Id,
	capacity: u32,
	/// Advertised version ranges, fixed for the observed process.
	versions: Option<std::sync::Arc<[rivet_topology::versions::FamilyVersions]>>,
	first_seen_boot_ms: u64,
	probe: Probe,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Balancing {
	/// Whether the balancer may choose discretionary actions: bootstrapping tablets,
	/// splits, rebalancing replacements and grant moves. Off, it still observes workers and
	/// repairs, replacing dead or stopping replicas and restoring a replica set below its
	/// target size, since durability must not depend on the switch.
	enabled: bool,
	last_rejection: Option<BlockReason>,
	reported: Option<Status>,
	pub settings: Settings,
	workers: BTreeMap<Id, Observation>,
	absent_since_boot_ms: BTreeMap<Id, u64>,
	pub search: Option<placement::Search>,
	armed_at: Option<u64>,
}
impl Balancing {
	pub fn new(settings: Settings) -> Self {
		Self {
			enabled: true,
			last_rejection: None,
			reported: None,
			settings,
			workers: BTreeMap::new(),
			absent_since_boot_ms: BTreeMap::new(),
			search: None,
			armed_at: None,
		}
	}
	/// Sets the initial switch position before the balancer has observed anything.
	pub fn with_enabled(mut self, enabled: bool) -> Self {
		self.enabled = enabled;
		self
	}
	pub fn enabled(&self) -> bool {
		self.enabled
	}
	/// Applies a change of the switch. The next drive restarts the search with the new
	/// position, which is part of its input. Returns whether the position changed.
	pub(in crate::tablet_coordinator_machine) fn set_enabled(
		&mut self,
		enabled: bool,
		effects: &mut Effects,
	) -> bool {
		if self.enabled == enabled {
			return false;
		}
		self.enabled = enabled;
		self.search = None;
		self.last_rejection = None;
		effects.cancel_timer(Timer::Balance);
		self.armed_at = None;
		true
	}
	pub fn status(&self, manifest: &TabletManifest) -> Option<Status> {
		let pending_splits = manifest
			.tablets
			.iter()
			.filter(|t| t.split.is_some())
			.count() as u64;
		let pending_replacements = manifest
			.tablets
			.iter()
			.filter(|t| {
				t.replica_set.phase
					!= rivet_data::generated::shared_depot_types_v1::TabletPhase::Stable
			})
			.count() as u64;
		let paused_tablets = manifest.paused.len() as u64;
		let search = self.search.as_ref()?;
		let unavailable: Vec<_> = manifest
			.tablets
			.iter()
			.flat_map(|t| &t.replica_set.disks)
			.filter(|disk| {
				search
					.input
					.available
					.get(&disk.worker_id)
					.is_none_or(|worker| worker.disk_incarnation != disk.disk_incarnation)
			})
			.collect();
		let blocked = self.last_rejection.clone().or_else(|| {
			if search.input.destinations.is_empty() {
				Some(BlockReason::WaitingForWorkers)
			} else if !unavailable.is_empty() {
				Some(
					if unavailable
						.iter()
						.any(|disk| search.input.replaceable.contains(&disk.worker_id))
					{
						BlockReason::NoDestinationOrQuorum
					} else {
						BlockReason::ReplacementGrace
					},
				)
			} else if !self.enabled {
				// Repair still runs; only the discretionary actions below are held.
				Some(BlockReason::Disabled)
			} else if search.complete && search.projected_tablets < search.desired_tablets {
				Some(BlockReason::SplitBlocked)
			} else {
				None
			}
		});
		let (replica_imbalance_ppm, maintainer_imbalance_ppm) = search.imbalance_ppm();
		Some(Status {
			desired_tablets: search.desired_tablets,
			projected_tablets: search.projected_tablets,
			admitted_workers: search.input.destinations.len() as u64,
			pending_splits,
			pending_replacements,
			paused_tablets,
			search_complete: search.complete,
			replica_imbalance_ppm,
			maintainer_imbalance_ppm,
			blocked,
			disk_probes_in_flight: self
				.workers
				.values()
				.filter(|w| matches!(w.probe, Probe::Running { .. }))
				.count() as u32,
			waiting_disk_probes: self
				.workers
				.values()
				.filter(|w| matches!(w.probe, Probe::Waiting { .. }))
				.count(),
			disk_probe_capacity: self.settings.max_disk_probes,
		})
	}
	pub(in crate::tablet_coordinator_machine) fn stop(&mut self, effects: &mut Effects) {
		self.clear(effects);
		if let Some(mut status) = self.reported.take() {
			status.disk_probes_in_flight = 0;
			status.waiting_disk_probes = 0;
			effects.execute(Effect::ObservePlacement {
				status,
				probe_limit_reached: false,
			});
		}
	}
	/// Drops every observation and cancels the probes and deadlines armed for them.
	fn clear(&mut self, effects: &mut Effects) {
		for (worker, observation) in &self.workers {
			if let Probe::Running { operation } = observation.probe {
				effects.cancel_timer(Timer::DiskProbe(*worker, operation));
			}
		}
		effects.cancel_timer(Timer::Balance);
		self.workers.clear();
		self.absent_since_boot_ms.clear();
		self.search = None;
		self.last_rejection = None;
		self.armed_at = None;
	}
	pub(in crate::tablet_coordinator_machine) fn observed(
		&mut self,
		topology: &super::StopTopology,
		effects: &mut Effects,
	) {
		let now = effects.now_boot_ms();
		if now >= topology.valid_until_boot_ms {
			return;
		}
		self.workers.retain(|id, observation| {
			let same = topology.workers.get(id) == Some(&observation.instance)
				&& topology
					.capacities
					.get(id)
					.copied()
					.unwrap_or(TabletBalancingConfig::MILLICORES_PER_CORE)
					== observation.capacity;
			if !same {
				self.absent_since_boot_ms.entry(*id).or_insert(now);
				if let Probe::Running { operation } = observation.probe {
					effects.cancel_timer(Timer::DiskProbe(*id, operation));
				}
			}
			same
		});
		for (id, instance) in &topology.workers {
			let capacity = topology
				.capacities
				.get(id)
				.copied()
				.unwrap_or(TabletBalancingConfig::MILLICORES_PER_CORE);
			if capacity == 0 {
				continue;
			}
			self.workers.entry(*id).or_insert(Observation {
				instance: *instance,
				capacity,
				versions: topology.versions.get(id).cloned(),
				first_seen_boot_ms: now,
				probe: Probe::Waiting { at_boot_ms: now },
			});
		}
	}
	pub(in crate::tablet_coordinator_machine) fn probe_response(
		&mut self,
		worker: Id,
		operation: u64,
		result: Result<Id, String>,
		effects: &mut Effects,
	) {
		let Some(observation) = self.workers.get_mut(&worker) else {
			return;
		};
		if observation.probe != (Probe::Running { operation }) {
			return;
		}
		effects.cancel_timer(Timer::DiskProbe(worker, operation));
		observation.probe = match result {
			Ok(disk) if disk != [0; 16] => Probe::Verified { disk },
			_ => Probe::Waiting {
				at_boot_ms: effects.now_boot_ms().saturating_add(1000),
			},
		};
	}
	/// Assigned workers that storage discovery has not shown on their assigned disk for at
	/// least the replacement grace, the same evidence that lets the balancer replace them.
	pub(in crate::tablet_coordinator_machine) fn missing(&self, now: u64) -> BTreeSet<Id> {
		self.absent_since_boot_ms
			.iter()
			.filter(|(_, since)| now.saturating_sub(**since) >= self.settings.missing_worker_ms)
			.map(|(id, _)| *id)
			.collect()
	}

	pub(in crate::tablet_coordinator_machine) fn timer(&mut self) {
		self.armed_at = None;
	}
	fn input(&mut self, manifest: &TabletManifest, now: u64) -> placement::Input {
		let assigned: BTreeMap<Id, BTreeSet<Id>> = manifest
			.tablets
			.iter()
			.flat_map(|t| &t.replica_set.disks)
			.fold(BTreeMap::new(), |mut assigned, d| {
				assigned
					.entry(d.worker_id)
					.or_default()
					.insert(d.disk_incarnation);
				assigned
			});
		self.absent_since_boot_ms
			.retain(|id, _| assigned.contains_key(id));
		for (id, disks) in &assigned {
			match self.workers.get(id).map(|w| &w.probe) {
				Some(Probe::Verified { disk }) if disks.iter().all(|assigned| assigned == disk) => {
					self.absent_since_boot_ms.remove(id);
				}
				None | Some(Probe::Verified { .. }) => {
					self.absent_since_boot_ms.entry(*id).or_insert(now);
				}
				_ => {}
			}
		}
		let available: BTreeMap<Id, placement::Worker> = self
			.workers
			.iter()
			.filter(|(id, _)| {
				!manifest
					.stopping
					.iter()
					.any(|s| s.worker_id == **id && s.permitted_at.is_some())
			})
			.filter_map(|(id, w)| {
				if let Probe::Verified { disk } = w.probe {
					Some((
						*id,
						placement::Worker {
							capacity_millicores: w.capacity,
							disk_incarnation: disk,
							versions: w.versions.clone(),
						},
					))
				} else {
					None
				}
			})
			.collect();
		let destinations = available
			.keys()
			.copied()
			.filter(|id| {
				!manifest.stopping.iter().any(|s| s.worker_id == *id)
					&& self.workers.get(id).is_some_and(|worker| {
						// A coordinator takeover loses its local observation age, not the
						// disk's durable assignments. The available map still requires a
						// fresh probe of this exact process; every assigned incarnation
						// must agree before using that disk without a new admission delay.
						assigned.get(id).is_some_and(|disks| {
							disks
								.iter()
								.all(|disk| *disk == available[id].disk_incarnation)
						}) || now.saturating_sub(worker.first_seen_boot_ms)
							>= self.settings.destination_stability_ms
					})
			})
			.collect();
		let gone = self.missing(now);
		let replaceable = gone
			.iter()
			.copied()
			// A durable stop permit has already withdrawn this process from
			// quorum availability. Waiting for failure-detection grace here can
			// strand the next drain behind the already permitted replica.
			.chain(
				manifest
					.stopping
					.iter()
					.filter_map(|entry| entry.permitted_at.map(|_| entry.worker_id)),
			)
			.collect();
		placement::Input {
			available,
			destinations,
			replaceable,
			gone,
			tablets_per_core: self.settings.tablets_per_core,
			discretionary: self.enabled,
		}
	}
	fn arm(&mut self, now: u64, effects: &mut Effects) {
		let next = self
			.workers
			.values()
			.flat_map(|w| {
				let admission = w
					.first_seen_boot_ms
					.saturating_add(self.settings.destination_stability_ms);
				let probe = if let Probe::Waiting { at_boot_ms } = w.probe {
					Some(at_boot_ms)
				} else {
					None
				};
				[Some(admission), probe]
			})
			.flatten()
			.chain(
				self.absent_since_boot_ms
					.values()
					.map(|since| since.saturating_add(self.settings.missing_worker_ms)),
			)
			.filter(|deadline| *deadline > now)
			.min();
		let next = if self.search.as_ref().is_some_and(|s| !s.complete) {
			Some(next.map_or(now.saturating_add(1), |at| at.min(now.saturating_add(1))))
		} else {
			next
		};
		if next != self.armed_at {
			effects.cancel_timer(Timer::Balance);
			if let Some(at) = next {
				effects.arm_timer_at(Timer::Balance, at);
			}
			self.armed_at = next;
		}
	}
}

impl State {
	pub(in crate::tablet_coordinator_machine) fn cleanup_stops(
		&mut self,
		manifest: &mut TabletManifest,
		effects: &mut Effects,
	) -> bool {
		let Some(topology) = &self.discovery else {
			return false;
		};
		if effects.now_boot_ms() >= topology.valid_until_boot_ms {
			return false;
		}
		let before = manifest.stopping.len();
		manifest.stopping.retain(|s| {
			// A lagging discovery listing must not readmit the same stopping process.
			topology
				.workers
				.get(&s.worker_id)
				.is_none_or(|instance| instance == &s.instance_id)
		});
		self.stop_rounds.retain(|worker, round| {
			let keep = manifest.stopping.iter().any(|s| {
				s.worker_id == *worker && s.instance_id == round.check.request.instance_id
			});
			if !keep {
				effects.execute(Effect::CancelStopReadinessRequest { worker: *worker });
			}
			keep
		});
		before != manifest.stopping.len()
	}
	pub(in crate::tablet_coordinator_machine) fn prepare_balancing(
		&mut self,
		manifest: &TabletManifest,
		effects: &mut Effects,
	) {
		let (Some(balance), Some(topology)) = (self.balancing.as_mut(), &self.discovery) else {
			return;
		};
		if effects.now_boot_ms() >= topology.valid_until_boot_ms {
			return;
		}
		balance.observed(topology, effects);
		let mut slots = (balance.settings.max_disk_probes as usize).saturating_sub(
			balance
				.workers
				.values()
				.filter(|w| matches!(w.probe, Probe::Running { .. }))
				.count(),
		);
		for (worker, observation) in &mut balance.workers {
			if slots == 0 {
				break;
			}
			if manifest
				.stopping
				.iter()
				.any(|s| s.worker_id == *worker && s.permitted_at.is_some())
			{
				continue;
			}
			if let Probe::Waiting { at_boot_ms } = observation.probe
				&& at_boot_ms <= effects.now_boot_ms()
			{
				// Operation ids live only in memory, start at zero for each coordinator
				// lifetime and grow by one per issued request, so u64 cannot overflow.
				self.next_operation = self
					.next_operation
					.checked_add(1)
					.expect("operation id exhausted");
				let operation = self.next_operation;
				observation.probe = Probe::Running { operation };
				slots -= 1;
				effects.arm_timer_after(
					Timer::DiskProbe(*worker, operation),
					self.operation_timeout_ms,
				);
				effects.execute(Effect::DiskProbeRequest {
					worker: *worker,
					instance: observation.instance,
					capacity: observation.capacity,
					operation,
				});
			}
		}
		// This also arms grace/admission deadlines while a manual write takes precedence.
		let input = balance.input(manifest, effects.now_boot_ms());
		if balance.search.as_ref().is_none_or(|search| {
			search.manifest_version != manifest.tablet_manifest_version || search.input != input
		}) {
			balance.last_rejection = None;
			balance.search = Some(placement::Search::new(manifest, input));
		}
		balance.arm(effects.now_boot_ms(), effects);
	}
	pub(in crate::tablet_coordinator_machine) fn balance(
		&mut self,
		manifest: &mut TabletManifest,
		effects: &mut Effects,
		entropy: &Entropy,
	) -> bool {
		let (Some(balance), Some(topology)) = (self.balancing.as_mut(), &self.discovery) else {
			return false;
		};
		if effects.now_boot_ms() >= topology.valid_until_boot_ms {
			return false;
		}
		let action = balance
			.search
			.as_mut()
			.and_then(|search| search.step(manifest, 1024));
		balance.arm(effects.now_boot_ms(), effects);
		let (Some(action), Some(search)) = (action, balance.search.as_ref()) else {
			return false;
		};

		// Actions name workers and tablets from the search's observations. Check each
		// reference against them and reject the action instead of trusting it.
		let available = &search.input.available;
		let disk_incarnation = |id: &Id| {
			available
				.get(id)
				.map(|worker| worker.disk_incarnation)
				.ok_or(super::Reject::InvalidCommand)
		};
		let mut candidate = manifest.clone();
		let mut alarm_candidate = self.alarms.clone();
		let now = effects.clocks().unix_ms;
		let changed = (|| match action {
			placement::Action::Bootstrap { members, tablets } => {
				// A fresh manifest has no database to checkpoint. Provision the initial
				// ranges directly, as the operator's provision command does, rather than
				// freezing a parent whose first cold open has not happened yet.
				// Bound work in this transition; oversized startup targets require the
				// operator's bounded provisioning command instead.
				if tablets > 1024 || members.is_empty() {
					return Err(super::Reject::InvalidCommand);
				}
				let disks = members
					.iter()
					.map(|id| {
						Ok(StorageWorkerDisk {
							worker_id: *id,
							disk_incarnation: disk_incarnation(id)?,
						})
					})
					.collect::<Result<Vec<_>, super::Reject>>()?;
				let mut rng = entropy.clone().rng();
				let step = u128::MAX / u128::from(tablets);
				for index in 0..tablets {
					let id = *rng.uuid_v4().as_bytes();
					let command = super::Command {
						request_id: 0,
						topology: None,
						tablet_id: id,
						split: None,
						requested_at: now,
						request: TabletCoordinatorRequest::CreateTablet(CreateTablet {
							hash_range: depot_types::PartitionKeyHashRange {
								start: (step * u128::from(index)).to_be_bytes(),
								end: (index + 1 < tablets)
									.then(|| (step * u128::from(index + 1)).to_be_bytes()),
							},
							members: disks.clone(),
						}),
					};
					if alarm_candidate.enabled {
						alarm_candidate.apply(&mut candidate, &command, None, None)?;
					} else {
						super::apply(&mut candidate, &command, None)?;
					}
					grant(
						&mut candidate,
						id,
						members[index as usize % members.len()],
						now,
					)?;
				}
				Ok(true)
			}
			placement::Action::Grow { tablet, joining } => {
				let mut next = candidate
					.tablets
					.iter()
					.find(|t| t.id == tablet)
					.ok_or(super::Reject::UnknownTablet)?
					.replica_set
					.members
					.clone();
				next.push(joining);
				let disk = StorageWorkerDisk {
					worker_id: joining,
					disk_incarnation: disk_incarnation(&joining)?,
				};
				reconfiguration::resize(&mut candidate, &tablet, next, Some(disk), now)
			}
			placement::Action::Grant { tablet, maintainer } => {
				grant(&mut candidate, tablet, maintainer, now)
			}
			placement::Action::Shrink { tablet, leaving } => {
				let next = candidate
					.tablets
					.iter()
					.find(|t| t.id == tablet)
					.ok_or(super::Reject::UnknownTablet)?
					.replica_set
					.members
					.iter()
					.copied()
					.filter(|id| *id != leaving)
					.collect();
				reconfiguration::resize(&mut candidate, &tablet, next, None, now)
			}
			placement::Action::Replace {
				tablet,
				leaving,
				joining,
				maintainer,
			} => {
				let disk = disk_incarnation(&joining)?;
				// A draining owner may retain its current grant to bring in a survivor.
				// Only an actual grant change goes through the new-allocation check.
				if candidate
					.tablets
					.iter()
					.find(|t| t.id == tablet)
					.and_then(|t| t.maintainer)
					!= Some(maintainer)
				{
					grant(&mut candidate, tablet, maintainer, now)?;
				}
				reconfiguration::replace(
					&mut candidate,
					&ReplaceReplica {
						tablet_id: tablet,
						leaving,
						joining: StorageWorkerDisk {
							worker_id: joining,
							disk_incarnation: disk,
						},
					},
					now,
				)
			}
			placement::Action::Split { tablet, midpoint } => {
				let mut rng = entropy.clone().rng();
				let proposal = TabletSplit {
					split_key_hash: midpoint,
					left_tablet_id: *rng.uuid_v4().as_bytes(),
					right_tablet_id: *rng.uuid_v4().as_bytes(),
					checkpoint_id: *rng.uuid_v4().as_bytes(),
				};
				let command = super::Command {
					request_id: 0,
					topology: None,
					tablet_id: tablet,
					split: Some(proposal),
					requested_at: now,
					request: TabletCoordinatorRequest::SplitTablet(SplitTablet {
						tablet_id: tablet,
						split_key_hash: midpoint,
					}),
				};
				if alarm_candidate.enabled {
					alarm_candidate.apply(&mut candidate, &command, None, Some(topology))
				} else {
					super::apply(&mut candidate, &command, Some(topology))
				}
			}
		})();
		if changed != Ok(true)
			|| super::validate(&candidate).is_err()
			|| (alarm_candidate.enabled
				&& super::alarms::validate(&alarm_candidate.envelope(candidate.clone())).is_err())
		{
			balance.last_rejection = Some(BlockReason::NoDestinationOrQuorum);
			return false;
		}
		if !crate::tablet_coordinator_machine::store::has_completion_headroom(
			&candidate,
			&alarm_candidate,
		) {
			balance.last_rejection = Some(BlockReason::ManifestHeadroom);
			return false;
		}
		self.alarms = alarm_candidate;
		*manifest = candidate;
		true
	}
	pub(in crate::tablet_coordinator_machine) fn publish_balancing(
		&mut self,
		manifest: &TabletManifest,
		effects: &mut Effects,
	) {
		if let Some(balance) = self.balancing.as_mut()
			&& let Some(status) = balance.status(manifest)
			&& balance.reported.as_ref() != Some(&status)
		{
			let probe_limit_reached = status.waiting_disk_probes > 0
				&& status.disk_probes_in_flight >= status.disk_probe_capacity
				&& balance.reported.as_ref().is_none_or(|previous| {
					previous.waiting_disk_probes == 0
						|| previous.disk_probes_in_flight < previous.disk_probe_capacity
				});
			balance.reported = Some(status.clone());
			effects.execute(Effect::ObservePlacement {
				status,
				probe_limit_reached,
			});
		}
	}
}


