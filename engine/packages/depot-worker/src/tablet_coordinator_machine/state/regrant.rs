//! Automatic re-grant of tablets whose maintainer left storage discovery.
//!
//! This existing failover path is distinct from a voluntary owner handoff. Storage
//! discovery lists Active workers; Engine keeps a graceful owner Active until its
//! durable stop permit, joins its storage services, then publishes topology Draining.
//! This path retains its existing missing-from-Active policy; the hint is not itself
//! new proof of an expired lease. Voluntary commands cannot borrow that failover
//! authority just because their source is absent from a hint.
//! Replicas are never moved here.

use super::{GrantAuthority, grant_authorized, reconfiguration};
use crate::tablet_coordinator_machine::types::StopTopology;
use rivet_data::generated::depot_tablet_manifest_v1::{Tablet, TabletManifest};
use std::collections::{BTreeMap, BTreeSet};

/// Re-grants applied to one manifest copy.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Regrants {
	pub granted: u64,
	/// Tablets whose maintainer is gone and that have no eligible live member.
	pub no_member: BTreeSet<[u8; 16]>,
}

/// The live member of `tablet` that may maintain it, other than `leaving`, with the fewest
/// grants in `manifest`, then the lowest worker ID. Stopping workers are never chosen, nor
/// are workers whose advertised versions cannot read the tablet's data.
pub(super) fn least_loaded_target(
	manifest: &TabletManifest,
	tablet: &Tablet,
	leaving: &[u8; 16],
	discovery: &StopTopology,
	available: Option<&BTreeMap<[u8; 16], crate::tablet_coordinator_machine::placement::Worker>>,
) -> Option<[u8; 16]> {
	tablet
		.replica_set
		.members
		.iter()
		.filter(|worker| {
			*worker != leaving
				&& reconfiguration::may_maintain(tablet, worker)
				&& discovery.workers.contains_key(*worker)
				&& super::versions::readable(tablet, worker, Some(discovery))
				&& available.is_none_or(|available| {
					available.get(*worker).is_some_and(|observed| {
						tablet.replica_set.disks.iter().any(|disk| {
							disk.worker_id == **worker
								&& disk.disk_incarnation == observed.disk_incarnation
						})
					})
				}) && !manifest
				.stopping
				.iter()
				.any(|entry| &entry.worker_id == *worker)
		})
		.min_by_key(|worker| {
			let grants = manifest
				.tablets
				.iter()
				.filter(|tablet| tablet.maintainer == Some(**worker))
				.count();
			(grants, **worker)
		})
		.copied()
}

/// Moves every grant held by a worker missing from `discovery`, or by a listed worker that
/// cannot read the tablet's version marks, to its least-loaded live member that can. An
/// owner that cannot read its tablet refuses to open it, so leaving the grant would keep the
/// tablet down. Frozen parents retain their split identities across regrant. A retained stop
/// intent excludes destinations; it must not suppress failover away from an absent owner.
pub(super) fn regrant(
	manifest: &mut TabletManifest,
	discovery: &StopTopology,
	available: Option<&BTreeMap<[u8; 16], crate::tablet_coordinator_machine::placement::Worker>>,
	requested_at: i64,
) -> Regrants {
	let mut regrants = Regrants::default();
	let orphaned: Vec<([u8; 16], [u8; 16])> = manifest
		.tablets
		.iter()
		.filter_map(|tablet| {
			let maintainer = tablet.maintainer?;
			let orphaned = !discovery.workers.contains_key(&maintainer)
				|| !super::versions::readable(tablet, &maintainer, Some(discovery));
			match orphaned {
				true => Some((tablet.id, maintainer)),
				false => None,
			}
		})
		.collect();
	for (tablet_id, maintainer) in orphaned {
		let Some(tablet) = manifest
			.tablets
			.iter()
			.find(|tablet| tablet.id == tablet_id)
		else {
			continue;
		};
		let Some(target) = least_loaded_target(manifest, tablet, &maintainer, discovery, available)
		else {
			regrants.no_member.insert(tablet_id);
			continue;
		};
		match grant_authorized(
			manifest,
			tablet_id,
			target,
			requested_at,
			GrantAuthority::OwnerMissing,
		) {
			Ok(true) => regrants.granted += 1,
			Ok(false) | Err(_) => {
				regrants.no_member.insert(tablet_id);
			}
		}
	}
	regrants
}
