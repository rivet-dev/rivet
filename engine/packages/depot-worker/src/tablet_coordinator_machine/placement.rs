//! Pure, incremental search over existing split, replacement and grant operations.

use crate::balancing_config::TabletBalancingConfig;
use num_bigint::BigInt;
use rivet_data::generated::{
	depot_tablet_manifest_v1::{Tablet, TabletManifest},
	shared_depot_types_v1::{PartitionKeyHashRange, TabletPhase},
};
use serde::{Deserialize, Serialize};
use std::{
	cmp::Ordering,
	collections::{BTreeMap, BTreeSet},
};

pub type Id = [u8; 16];

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Worker {
	pub capacity_millicores: u32,
	pub disk_incarnation: Id,
	/// The worker's advertised per-family version ranges, when discovery carried them.
	pub versions: Option<std::sync::Arc<[rivet_topology::versions::FamilyVersions]>>,
}

/// Observations are supplied by the coordinator. They never authorize storage mutations.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Input {
	/// Exact-process, exact-disk probes; existing assignments need no destination delay.
	pub available: BTreeMap<Id, Worker>,
	/// Freshly verified assigned disks, or workers past the destination stability window.
	pub destinations: BTreeSet<Id>,
	/// Assigned workers past the absence grace or already granted a durable stop permit.
	pub replaceable: BTreeSet<Id>,
	/// Assigned workers past the absence grace. Only these shrink a replica set; a stop
	/// intent or permit never does, because a stopping worker may be restarting on its disk.
	pub gone: BTreeSet<Id>,
	pub tablets_per_core: u32,
	/// Whether discretionary actions may be chosen: bootstrap, splits, rebalancing
	/// replacements and grant moves. Repair never depends on it: replacing a dead or stopping
	/// replica and growing a replica set below its target size run either way.
	pub discretionary: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
	Bootstrap {
		members: Vec<Id>,
		tablets: u64,
	},
	Grow {
		tablet: Id,
		joining: Id,
	},
	Split {
		tablet: Id,
		midpoint: Id,
	},
	Replace {
		tablet: Id,
		leaving: Id,
		joining: Id,
		maintainer: Id,
	},
	/// Removes a member gone past the absence grace that no destination can replace.
	Shrink {
		tablet: Id,
		leaving: Id,
	},
	Grant {
		tablet: Id,
		maintainer: Id,
	},
}

/// Signed change to sum(load²/capacity). Compare exactly, including the full 2^128 range.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Delta {
	numerator: BigInt,
	denominator: u64,
}
impl Delta {
	fn zero() -> Self {
		Self {
			numerator: BigInt::from(0),
			denominator: 1,
		}
	}
	fn new(numerator: BigInt, denominator: u64) -> Self {
		let (_, remainder) = (&numerator % denominator).to_u64_digits();
		let (mut a, mut b) = (denominator, remainder.first().copied().unwrap_or(0));
		while b != 0 {
			(a, b) = (b, a % b);
		}
		Self {
			numerator: numerator / a,
			denominator: denominator / a,
		}
	}
}
impl Ord for Delta {
	fn cmp(&self, other: &Self) -> Ordering {
		(&self.numerator * other.denominator).cmp(&(&other.numerator * self.denominator))
	}
}
impl PartialOrd for Delta {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Candidate {
	priority: u8,
	replicas: Delta,
	maintainers: Delta,
	key: [Id; 4],
	action: Action,
}
impl Candidate {
	fn compare(&self, other: &Self) -> Ordering {
		(self.priority, &self.replicas, &self.maintainers, self.key).cmp(&(
			other.priority,
			&other.replicas,
			&other.maintainers,
			other.key,
		))
	}
}

/// Retains a bounded search across turns. Any changed manifest or eligibility input restarts it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Search {
	pub manifest_version: u64,
	pub input: Input,
	pub desired_tablets: u64,
	pub projected_tablets: u64,
	/// Include busy ranges so their ready, smaller peers cannot consume the split target.
	widest_split_width: BigInt,
	/// Tablets an operator paused: never the subject of a discretionary action, and not a
	/// split target their smaller peers wait on. Their dead or stopping replicas are still
	/// replaced and their replica sets still grown to the target size.
	paused: BTreeSet<Id>,
	loads: BTreeMap<Id, (BigInt, BigInt)>,
	destinations: Vec<Id>,
	tablet: usize,
	candidate: usize,
	best: Option<Candidate>,
	pub complete: bool,
}

pub fn width(range: &PartitionKeyHashRange) -> BigInt {
	let end = range.end.map_or_else(
		|| BigInt::from(1u8) << 128,
		|end| BigInt::from(u128::from_be_bytes(end)),
	);
	end - BigInt::from(u128::from_be_bytes(range.start))
}

impl Search {
	/// Capacity-normalized excess width in parts per million, for diagnostics only.
	pub fn imbalance_ppm(&self) -> (u64, u64) {
		let capacity: u64 = self
			.input
			.destinations
			.iter()
			.map(|id| self.capacity_millicores(id))
			.sum();
		let excess = |maintainer: bool| {
			let total: BigInt = self
				.input
				.destinations
				.iter()
				.map(|id| {
					self.loads
						.get(id)
						.map(|load| {
							if maintainer {
								load.1.clone()
							} else {
								load.0.clone()
							}
						})
						.unwrap_or_default()
				})
				.sum();
			if total == BigInt::from(0) {
				return 0;
			}
			self.input
				.destinations
				.iter()
				.map(|id| {
					let load = self
						.loads
						.get(id)
						.map(|load| {
							if maintainer {
								load.1.clone()
							} else {
								load.0.clone()
							}
						})
						.unwrap_or_default();
					let ratio =
						load * capacity * 1_000_000u64 / (&total * self.capacity_millicores(id));
					let (_, digits) = ratio.to_u64_digits();
					if digits.len() > 1 {
						u64::MAX
					} else {
						digits
							.first()
							.copied()
							.unwrap_or(0)
							.saturating_sub(1_000_000)
					}
				})
				.max()
				.unwrap_or(0)
		};
		(excess(false), excess(true))
	}
	pub fn new(manifest: &TabletManifest, input: Input) -> Self {
		let mut loads: BTreeMap<Id, (BigInt, BigInt)> = BTreeMap::new();
		let mut widest_split_width = BigInt::from(0);
		let paused: BTreeSet<Id> = manifest
			.paused
			.iter()
			.map(|pause| pause.tablet_id)
			.collect();
		for tablet in &manifest.tablets {
			let size = width(&tablet.hash_range);
			if tablet.hash_range.midpoint().is_some()
				&& !paused.contains(&tablet.id)
				&& size > widest_split_width
			{
				widest_split_width = size.clone();
			}
			let members = match &tablet.replica_set.phase {
				TabletPhase::Stable => &tablet.replica_set.members,
				TabletPhase::Learning(phase) => &phase.next,
				TabletPhase::Joint(phase) => &phase.next,
			};
			for worker in members {
				loads.entry(*worker).or_default().0 += &size;
			}
			if let Some(worker) = tablet.maintainer {
				loads.entry(worker).or_default().1 += &size;
			}
		}
		let capacity: u64 = input
			.destinations
			.iter()
			.filter_map(|id| input.available.get(id))
			.map(|w| u64::from(w.capacity_millicores))
			.sum();
		let desired_tablets = capacity
			.saturating_mul(u64::from(input.tablets_per_core))
			.div_ceil(u64::from(TabletBalancingConfig::MILLICORES_PER_CORE));
		let projected_tablets = manifest.tablets.len() as u64
			+ manifest
				.tablets
				.iter()
				.filter(|t| t.split.is_some())
				.count() as u64;
		Self {
			manifest_version: manifest.tablet_manifest_version,
			destinations: input.destinations.iter().copied().collect(),
			input,
			desired_tablets,
			projected_tablets,
			widest_split_width,
			paused,
			loads,
			tablet: 0,
			candidate: 0,
			best: None,
			complete: false,
		}
	}

	/// Placement weight of a worker, never zero so it is safe as a divisor. Workers missing
	/// from the observations weigh as one core.
	fn capacity_millicores(&self, id: &Id) -> u64 {
		let capacity = self
			.input
			.available
			.get(id)
			.map_or(TabletBalancingConfig::MILLICORES_PER_CORE, |worker| {
				worker.capacity_millicores.max(1)
			});
		u64::from(capacity)
	}

	fn available(&self, tablet: &Tablet, id: &Id) -> bool {
		self.input.available.get(id).is_some_and(|worker| {
			worker.capacity_millicores > 0
				&& tablet.replica_set.disks.iter().any(|disk| {
					disk.worker_id == *id && disk.disk_incarnation == worker.disk_incarnation
				})
		})
	}
	/// Whether `id` can read every version mark of `tablet`'s data, so a grant to it opens and
	/// a replica on it accepts the tablet's appends. With any mark, a worker whose versions are
	/// unknown is skipped.
	fn reads_marks(&self, tablet: &Tablet, id: &Id) -> bool {
		let versions = self
			.input
			.available
			.get(id)
			.and_then(|worker| worker.versions.as_deref());
		super::state::versions::readable_with(tablet, versions)
	}
	fn delta(&self, leaving: Id, joining: Id, size: &BigInt, maintainer: bool) -> Delta {
		if leaving == joining {
			return Delta::zero();
		}
		let load = |id: &Id| {
			self.loads
				.get(id)
				.map(|l| if maintainer { l.1.clone() } else { l.0.clone() })
				.unwrap_or_default()
		};
		let (a, b) = (
			self.capacity_millicores(&leaving),
			self.capacity_millicores(&joining),
		);
		Delta::new(
			size * ((load(&joining) * 2 + size) * a - (load(&leaving) * 2 - size) * b),
			a * b,
		)
	}
	fn offer(&mut self, candidate: Candidate) {
		if self
			.best
			.as_ref()
			.is_none_or(|best| candidate.compare(best).is_lt())
		{
			self.best = Some(candidate);
		}
	}
	fn ordinary(&mut self, manifest: &TabletManifest, tablet: &Tablet) {
		if tablet.split.is_some()
			|| tablet.replica_set.phase != TabletPhase::Stable
			|| self.paused.contains(&tablet.id)
			|| !self.input.discretionary
		{
			return;
		}
		let Some(owner) = tablet.maintainer.filter(|id| self.available(tablet, id)) else {
			return;
		};
		// Only RequestStop's checked handoff may move this live owner's grant.
		if manifest
			.stopping
			.iter()
			.any(|entry| entry.worker_id == owner)
		{
			return;
		}
		if !tablet
			.replica_set
			.members
			.iter()
			.all(|id| self.available(tablet, id))
		{
			return;
		}
		let size = width(&tablet.hash_range);
		if !manifest
			.stopping
			.iter()
			.any(|entry| tablet.replica_set.members.contains(&entry.worker_id))
			&& !self.destinations.is_empty()
			&& self.projected_tablets < self.desired_tablets
			&& self.projected_tablets == manifest.tablets.len() as u64
			&& size == self.widest_split_width
			&& let Some(midpoint) = tablet.hash_range.midpoint()
		{
			self.offer(Candidate {
				priority: 1,
				replicas: Delta {
					numerator: -&size,
					denominator: 1,
				},
				maintainers: Delta::zero(),
				key: [tablet.id, [0; 16], [0; 16], [0; 16]],
				action: Action::Split {
					tablet: tablet.id,
					midpoint,
				},
			});
		}
		for worker in &tablet.replica_set.members {
			if *worker == owner
				|| !self.input.destinations.contains(worker)
				|| !self.reads_marks(tablet, worker)
			{
				continue;
			}
			let delta = self.delta(owner, *worker, &size, true);
			if delta < Delta::zero() {
				self.offer(Candidate {
					priority: 2,
					replicas: Delta::zero(),
					maintainers: delta,
					key: [tablet.id, [0; 16], [0; 16], *worker],
					action: Action::Grant {
						tablet: tablet.id,
						maintainer: *worker,
					},
				});
			}
		}
	}
	/// Offers to remove a member gone past the absence grace when no destination outside the
	/// set can replace it, so the set follows the workers that still exist. The survivors must
	/// be a live majority of the current set, as the Learning and Joint phases require, and
	/// the grant holder must be one of them.
	fn reduction(&mut self, tablet: &Tablet) {
		let members = &tablet.replica_set.members;
		if tablet.split.is_some() || tablet.replica_set.phase != TabletPhase::Stable {
			return;
		}
		let replaceable_elsewhere = self
			.destinations
			.iter()
			.any(|id| !members.contains(id) && self.reads_marks(tablet, id));
		// A worker outside the set that is still serving its admission window becomes a
		// destination soon, and replacing onto it costs one copy instead of a shrink and a
		// later grow.
		let admitting = self
			.input
			.available
			.keys()
			.any(|id| !members.contains(id) && !self.input.destinations.contains(id));
		if replaceable_elsewhere || admitting {
			return;
		}
		let Some(leaving) = members
			.iter()
			.copied()
			.find(|id| self.input.gone.contains(id) && !self.available(tablet, id))
		else {
			return;
		};
		let survivors = members.len() - 1;
		let all_survivors_live = members
			.iter()
			.filter(|id| **id != leaving)
			.all(|id| self.available(tablet, id));
		if survivors == 0 || !all_survivors_live || survivors * 2 <= members.len() {
			return;
		}
		if tablet
			.maintainer
			.is_none_or(|owner| owner == leaving || !self.available(tablet, &owner))
		{
			return;
		}
		self.offer(Candidate {
			priority: 0,
			replicas: Delta::zero(),
			maintainers: Delta::zero(),
			key: [tablet.id, leaving, [0; 16], [0; 16]],
			action: Action::Shrink {
				tablet: tablet.id,
				leaving,
			},
		});
	}

	fn replacement(
		&mut self,
		manifest: &TabletManifest,
		tablet: &Tablet,
		leaving: Id,
		joining: Id,
	) {
		if tablet.split.is_some()
			|| tablet.replica_set.phase != TabletPhase::Stable
			|| tablet.replica_set.members.contains(&joining)
			|| !self.reads_marks(tablet, &joining)
		{
			return;
		}
		if (tablet.replica_set.members.len() < self.destinations.len().min(3)
			|| (tablet.replica_set.members.len() == 1
				&& !self
					.input
					.destinations
					.contains(&tablet.replica_set.members[0])))
			&& tablet
				.maintainer
				.is_some_and(|id| self.available(tablet, &id))
			&& tablet
				.replica_set
				.members
				.iter()
				.all(|id| self.available(tablet, id))
		{
			let size = width(&tablet.hash_range);
			let load = self
				.loads
				.get(&joining)
				.map(|l| l.0.clone())
				.unwrap_or_default();
			self.offer(Candidate {
				priority: 0,
				replicas: Delta::new(
					&size * (load * 2 + &size),
					self.capacity_millicores(&joining),
				),
				maintainers: Delta::zero(),
				key: [tablet.id, joining, [0; 16], [0; 16]],
				action: Action::Grow {
					tablet: tablet.id,
					joining,
				},
			});
			return;
		}
		let required =
			self.input.replaceable.contains(&leaving) && !self.available(tablet, &leaving);
		if !required
			&& !tablet
				.replica_set
				.members
				.iter()
				.all(|id| self.available(tablet, id))
		{
			return;
		}
		if required
			&& tablet
				.replica_set
				.members
				.iter()
				.filter(|id| self.available(tablet, id))
				.count() <= tablet.replica_set.members.len() / 2
		{
			return;
		}
		let Some(owner) = tablet.maintainer else {
			return;
		};
		let owner_waits_for_handoff = self.available(tablet, &owner)
			&& manifest
				.stopping
				.iter()
				.any(|entry| entry.worker_id == owner && entry.permitted_at.is_none());
		if owner_waits_for_handoff && leaving == owner {
			return;
		}
		let size = width(&tablet.hash_range);
		let replicas = self.delta(leaving, joining, &size, false);
		let draining = manifest
			.stopping
			.iter()
			.any(|entry| entry.worker_id == leaving && entry.permitted_at.is_none());
		if !required && !draining && replicas >= Delta::zero() {
			return;
		}
		// A rebalancing replacement is discretionary: a pause or the switch holds it. A
		// required or draining one is repair and runs either way.
		if !required && !draining && (self.paused.contains(&tablet.id) || !self.input.discretionary)
		{
			return;
		}
		// A stop aborts a rebalance that keeps its worker, so proposing one here would loop
		// with that worker's stop polls and starve its grant handoff. Rebalance after the
		// drain. A required or draining replacement, like a grow, keeps stopping members and
		// finishes; the stop does not abort those.
		if !required && !draining && retains_stopping(manifest, tablet) {
			return;
		}
		// A draining replica remains available for quorum, but cannot take a grant.
		// Filter it here so the final grant check does not reject the best candidate
		// while a different retained voter could carry out the required repair.
		let may_maintain = |id: &Id| {
			self.available(tablet, id)
				&& !manifest.stopping.iter().any(|entry| entry.worker_id == *id)
		};
		let Some(maintainer) = tablet
			.replica_set
			.members
			.iter()
			.copied()
			.filter(|id| *id != leaving && may_maintain(id) && self.reads_marks(tablet, id))
			.min_by(|a, b| {
				self.delta(owner, *a, &size, true)
					.cmp(&self.delta(owner, *b, &size, true))
					.then(a.cmp(b))
			})
			.or_else(|| {
				// Keeping an existing grant is not a new allocation. If every retained
				// voter is draining, its live owner can introduce the outside survivor
				// before a grant can move there. A permitted owner cannot do this.
				(owner != leaving
					&& self.available(tablet, &owner)
					&& manifest
						.stopping
						.iter()
						.any(|entry| entry.worker_id == owner && entry.permitted_at.is_none()))
				.then_some(owner)
			})
		else {
			return;
		};
		// Retain an eligible owner unless replacement removes it. A prerequisite grant
		// and its replacement are applied, in that order, to one manifest CAS candidate.
		let maintainer = if owner != leaving && (may_maintain(&owner) || owner_waits_for_handoff) {
			owner
		} else {
			maintainer
		};
		self.offer(Candidate {
			priority: if required || draining { 0 } else { 2 },
			replicas,
			maintainers: self.delta(owner, maintainer, &size, true),
			key: [tablet.id, leaving, joining, maintainer],
			action: Action::Replace {
				tablet: tablet.id,
				leaving,
				joining,
				maintainer,
			},
		});
	}

	/// Each replacement candidate costs one unit; no turn evaluates an unbounded cross product.
	pub fn step(&mut self, manifest: &TabletManifest, mut budget: usize) -> Option<Action> {
		if self.complete {
			return None;
		}
		// Bootstrap on the destinations that qualify now, even when there are fewer than three.
		// Local development runs a single storage worker, so waiting for a full replica set
		// would leave it with no tablets at all.
		if manifest.tablets.is_empty() && !self.destinations.is_empty() && self.input.discretionary
		{
			self.complete = true;
			return Some(Action::Bootstrap {
				members: self.destinations.iter().take(3).copied().collect(),
				tablets: self.desired_tablets.max(1),
			});
		}
		while self.tablet < manifest.tablets.len() && budget > 0 {
			let tablet = &manifest.tablets[self.tablet];
			if self.candidate == 0 {
				self.ordinary(manifest, tablet);
				self.reduction(tablet);
			}
			let possibilities = self.destinations.len() * tablet.replica_set.members.len();
			if self.candidate < possibilities {
				let leaving = tablet.replica_set.members[self.candidate / self.destinations.len()];
				let joining = self.destinations[self.candidate % self.destinations.len()];
				self.replacement(manifest, tablet, leaving, joining);
				self.candidate += 1;
			} else {
				self.tablet += 1;
				self.candidate = 0;
			}
			budget -= 1;
		}
		if self.tablet == manifest.tablets.len() {
			self.complete = true;
			self.best.take().map(|candidate| candidate.action)
		} else {
			None
		}
	}
}

/// Whether a member of `tablet` has a stop request that is not yet permitted. A stop aborts
/// a rebalancing Learning phase that keeps such a member, so placement must not propose one.
fn retains_stopping(manifest: &TabletManifest, tablet: &Tablet) -> bool {
	tablet.replica_set.members.iter().any(|id| {
		manifest
			.stopping
			.iter()
			.any(|entry| entry.worker_id == *id && entry.permitted_at.is_none())
	})
}


