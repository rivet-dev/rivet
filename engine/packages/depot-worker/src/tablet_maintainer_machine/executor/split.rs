//! Guarded checkpoint retention and projected clones for one frozen parent.

use crate::tablet_maintainer_machine::state::split::SplitStep;
use anyhow::{Context, Result, ensure};
use depot_types::keys::{
	s3::TabletVolumePrefix,
	slatedb::{SplitSnapshotKey, split_snapshot_range, tablet_range},
};
use rivet_data::generated::depot_tablet_manifest_v1::Tablet;
use rivet_env::{
	MutationGuard,
	capabilities::slatedb::{ProjectedClone, SlateDbFactory},
};
use rivet_keys::keys::{KeyAddress, codec::FormalKey};
use std::sync::Arc;
use uuid::Uuid;

/// The split record fixes every identity before the first mutation. Retrying cannot
/// replace an initialized child or choose another snapshot after an uncertain result.
#[tracing::instrument(
	skip_all,
	err,
	fields(
		tablet_id = %Uuid::from_bytes(tablet.id),
		tablet_grant_epoch = tablet.tablet_grant_epoch,
		step = ?step,
	),
)]
pub(crate) async fn prepare(
	db: &slatedb::Db,
	factory: &dyn SlateDbFactory,
	tablet: &Tablet,
	step: SplitStep,
	guard: Arc<dyn MutationGuard>,
) -> Result<u64> {
	let split = tablet.split.as_ref().context("split record missing")?;
	ensure!(guard.may_send(), "split authority revoked");
	let checkpoint_id = Uuid::from_bytes(split.checkpoint_id);
	let (manifest_id, child, start, end) = match step {
		SplitStep::Checkpoint => {
			if crate::split_snapshot::enabled(tablet) {
				let address = SplitSnapshotKey::new(tablet.hash_range.start, checkpoint_id);
				let snapshot = crate::split_snapshot::from_parent(tablet)?;
				if let Some(existing) = db.get(address.pack()).await? {
					ensure!(
						address.deserialize(existing)? == snapshot,
						"split snapshot identity changed"
					);
				} else {
					db.put(address.pack(), address.serialize(snapshot)?).await?;
				}
			}
			let checkpoint = factory.final_checkpoint(db, checkpoint_id, guard).await?;
			ensure!(
				checkpoint.id == checkpoint_id,
				"checkpoint identity changed"
			);
			return Ok(checkpoint.manifest_id);
		}
		SplitStep::CloneLeft { manifest_id } => (
			manifest_id,
			split.left_tablet_id,
			tablet.hash_range.start,
			Some(split.split_key_hash),
		),
		SplitStep::CloneRight { manifest_id } => (
			manifest_id,
			split.right_tablet_id,
			split.split_key_hash,
			tablet.hash_range.end,
		),
		SplitStep::Report => anyhow::bail!("coordinator reports do not use the store"),
	};
	let range = if crate::split_snapshot::enabled(tablet) {
		// Validate the exact retained intent as well as the shared physical projection.
		// Otherwise a changed midpoint would look identical to the full snapshot clone.
		let address = SplitSnapshotKey::new(tablet.hash_range.start, checkpoint_id);
		let key = address.pack();
		let rows = factory
			.checkpoint_page(
				TabletVolumePrefix::new(Uuid::from_bytes(tablet.id)).pack(),
				checkpoint_id,
				manifest_id,
				(
					std::ops::Bound::Included(key.clone()),
					std::ops::Bound::Included(key.clone()),
				),
				1,
			)
			.await?;
		ensure!(
			rows.len() == 1
				&& rows[0].0 == key
				&& address.deserialize(rows[0].1.clone())?
					== crate::split_snapshot::from_parent(tablet)?,
			"split snapshot differs from retained intent"
		);
		split_snapshot_range()
	} else {
		tablet_range(depot_types::PartitionKeyHashRange { start, end })
	};

	factory
		.clone_projected(
			ProjectedClone {
				source: TabletVolumePrefix::new(Uuid::from_bytes(tablet.id)).pack(),
				destination: TabletVolumePrefix::new(Uuid::from_bytes(child)).pack(),
				checkpoint_id,
				manifest_id,
				// New snapshots share volume rows, pending work and timers. Legacy splits
				// retain their original projection and resumable bootstrap import.
				range,
			},
			guard,
		)
		.await?;
	Ok(manifest_id)
}


