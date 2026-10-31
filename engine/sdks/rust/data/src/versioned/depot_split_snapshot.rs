//! The immutable split identity carried by a shared cold-store snapshot.

pub enum DepotSplitSnapshotData {
	V1(crate::generated::depot_split_snapshot_v1::SplitSnapshot),
}
impl_versioned_v1!(
	DepotSplitSnapshotData,
	crate::generated::depot_split_snapshot_v1::SplitSnapshot
);
