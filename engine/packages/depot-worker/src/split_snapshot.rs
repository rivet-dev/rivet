//! Snapshot layout capability and immutable parent/child split identity.

use anyhow::{Context, Result, ensure};
use rivet_data::{
	families::VersionFamily,
	generated::{
		depot_split_snapshot_v1::SplitSnapshot,
		depot_tablet_manifest_v1::{AlarmIndexSource, Tablet},
	},
};

/// The coordinator records this mark before the frozen parent writes the new layout.
/// Older coordinators leave it absent, so upgraded maintainers retain the legacy path.
pub(crate) fn enabled(tablet: &Tablet) -> bool {
	tablet
		.version_marks
		.iter()
		.any(|mark| mark.family == VersionFamily::DEPOT_SPLIT_SNAPSHOT.0 && mark.version >= 1)
}

pub(crate) fn from_parent(tablet: &Tablet) -> Result<SplitSnapshot> {
	Ok(SplitSnapshot {
		parent_id: tablet.id,
		hash_range: tablet.hash_range.clone(),
		split: tablet.split.clone().context("split record missing")?,
	})
}

/// A full shared snapshot still belongs to exactly the child named in its frozen split.
pub(crate) fn validate_child(
	snapshot: &SplitSnapshot,
	child: &Tablet,
	source: &AlarmIndexSource,
) -> Result<()> {
	ensure!(
		snapshot.parent_id == source.parent_id
			&& child.parent_tablet_id == Some(snapshot.parent_id)
			&& snapshot.split.checkpoint_id == source.checkpoint_id,
		"split snapshot source changed"
	);
	ensure!(
		child.parent_range_start == Some(snapshot.hash_range.start),
		"split snapshot parent range changed"
	);
	let expected = if child.id == snapshot.split.left_tablet_id {
		rivet_data::generated::shared_depot_types_v1::PartitionKeyHashRange {
			start: snapshot.hash_range.start,
			end: Some(snapshot.split.split_key_hash),
		}
	} else {
		ensure!(
			child.id == snapshot.split.right_tablet_id,
			"split snapshot child changed"
		);
		rivet_data::generated::shared_depot_types_v1::PartitionKeyHashRange {
			start: snapshot.split.split_key_hash,
			end: snapshot.hash_range.end,
		}
	};
	ensure!(
		expected == child.hash_range && source.hash_range == expected,
		"split snapshot child range changed"
	);
	Ok(())
}
