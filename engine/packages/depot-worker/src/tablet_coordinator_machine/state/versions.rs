//! Tablet version marks in the manifest. The coordinator merges marks its maintainers
//! report and never grants a tablet, nor places a replica of it, on a worker that cannot read
//! them, so a new owner never refuses its grant at open and the grant never churns.

use super::Reject;
use crate::tablet_coordinator_machine::types::StopTopology;
use rivet_data::{
	families::{VersionFamily, advertised, merge_marks, normalize_marks},
	generated::{
		depot_tablet_coordinator_protocol_v1::{
			StaleTabletGrantEpoch, VersionMarksReport, VersionUnreadable,
		},
		depot_tablet_manifest_v1::{Tablet, TabletManifest},
		topology_worker_heartbeat_v1::FamilyVersions,
	},
};

/// Merges a maintainer's reported marks into its tablet. Returns whether any rose, so a
/// report the manifest already covers writes nothing. Decoded marks may be in any order,
/// so both lists are normalized first.
pub(super) fn report(
	manifest: &mut TabletManifest,
	report: &VersionMarksReport,
) -> Result<bool, Reject> {
	let tablet = manifest
		.tablets
		.iter_mut()
		.find(|tablet| tablet.id == report.tablet_id)
		.ok_or(Reject::UnknownTablet)?;
	if tablet.tablet_grant_epoch != report.tablet_grant_epoch {
		return Err(Reject::StaleTabletGrantEpoch(StaleTabletGrantEpoch {
			tablet_grant_epoch: tablet.tablet_grant_epoch,
		}));
	}
	if tablet.maintainer != Some(report.maintainer) {
		return Err(Reject::InvalidCommand);
	}
	let normalized = normalize_marks(&mut tablet.version_marks);
	Ok(merge_marks(&mut tablet.version_marks, &report.version_marks) || normalized)
}

/// The first mark of `tablet` that a worker advertising `versions` cannot read. Unknown
/// versions read nothing, so with any mark they refuse.
fn unreadable(tablet: &Tablet, versions: Option<&[FamilyVersions]>) -> Option<VersionUnreadable> {
	tablet.version_marks.iter().find_map(|mark| {
		let max_read = versions
			.and_then(|versions| advertised(versions, VersionFamily(mark.family)))
			.map(|range| range.max_read);
		if max_read.is_some_and(|max_read| mark.version <= max_read) {
			return None;
		}
		Some(VersionUnreadable {
			family: mark.family,
			mark: mark.version,
			max_read,
		})
	})
}

/// Checks that `worker` can read `tablet`'s data, from the versions the discovery
/// observation lists for it. A tablet without marks accepts any worker; with marks, a worker
/// whose versions are unknown, because there is no observation or it is not listed, refuses.
pub(crate) fn check(
	tablet: &Tablet,
	worker: &[u8; 16],
	discovery: Option<&StopTopology>,
) -> Result<(), VersionUnreadable> {
	if tablet.version_marks.is_empty() {
		return Ok(());
	}
	let versions = discovery
		.and_then(|discovery| discovery.versions.get(worker))
		.map(|versions| &versions[..]);
	match unreadable(tablet, versions) {
		Some(unreadable) => Err(unreadable),
		None => Ok(()),
	}
}

/// `check` as a predicate, for automatic placement that skips workers instead of rejecting.
pub(crate) fn readable(
	tablet: &Tablet,
	worker: &[u8; 16],
	discovery: Option<&StopTopology>,
) -> bool {
	check(tablet, worker, discovery).is_ok()
}

/// The same check against versions placement already holds for a worker.
pub(crate) fn readable_with(tablet: &Tablet, versions: Option<&[FamilyVersions]>) -> bool {
	tablet.version_marks.is_empty() || unreadable(tablet, versions).is_none()
}

/// Select the new split layout only when its current producer advertises support.
/// The mark follows the children and excludes older maintainers from future placement.
pub(super) fn mark_split_snapshot(tablet: &mut Tablet, discovery: Option<&StopTopology>) {
	let family = VersionFamily::DEPOT_SPLIT_SNAPSHOT;
	let supported = tablet
		.maintainer
		.and_then(|owner| discovery?.versions.get(&owner))
		.and_then(|versions| advertised(versions, family))
		.is_some_and(|versions| {
			versions.min_read <= 1 && versions.max_read >= 1 && versions.max_write >= 1
		});
	if supported {
		rivet_data::families::raise_mark(&mut tablet.version_marks, family, 1);
	}
}
