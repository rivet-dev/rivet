//! Bounded, resumable import runs before the child's opening response and shard listing.
//! New split snapshots inherit their indexes without importing rows. Older splits keep
//! the resumable import from their exact retained checkpoint.
use super::{shared_flush::FlushCause, store::FilesystemStore};
use crate::tablet_maintainer_machine::{event::MaintainerEvent, types::OpId};
use anyhow::{Context, Result, ensure};
use depot_types::{
	VolumeLogKey,
	keys::{
		s3::TabletVolumePrefix,
		slatedb::{AlarmBootstrapKey, AlarmTimePrefix, PendingImportKey, tablet_pending_range},
	},
};
use rivet_data::{
	generated::{depot_tablet_manifest_v1::Tablet, depot_tablet_manifest_v1::*},
	versioned::DepotAlarmTimeData,
};
use rivet_env::{Env, MutationGuard, capabilities::slatedb::SlateDbFactory};
use rivet_keys::keys::{KeyAddress, PrefixAddress, codec::FormalKey};
use std::{ops::Bound, sync::Arc, time::Duration};
use uuid::Uuid;
use vbare::Versioned;

// TODO: Remove this import. A split must copy zero rows: clone the parent checkpoint without a
// projection so the child carries the whole alarm index by reference, have the next-due scan skip
// and delete an `ALARM_TIME` row whose volume hash is outside this tablet's range, and drop the
// manifest alarm phase, the split holds and `ALARM_BOOTSTRAP`.
pub(crate) async fn initialize(
	fs: &FilesystemStore,
	factory: &dyn SlateDbFactory,
	tablet: &Tablet,
	initialization: &AlarmIndexInitialization,
	guard: Arc<dyn MutationGuard>,
	env: &dyn Env,
	inbox: &tokio::sync::mpsc::Sender<MaintainerEvent>,
	op_id: OpId,
) -> Result<()> {
	let Some(source) = &initialization.source else {
		ensure!(
			matches!(
				initialization.phase,
				AlarmIndexPhase::LegacyEmpty | AlarmIndexPhase::NativeEmpty
			),
			"missing alarm index source"
		);
		return Ok(());
	};
	ensure!(
		source.hash_range == tablet.hash_range && source.parent_id != tablet.id,
		"alarm source range changed"
	);
	let address = AlarmBootstrapKey::new();
	let mut progress = match fs.db.get(address.pack()).await? {
		Some(bytes) => address.deserialize(bytes)?,
		None => {
			ensure!(
				initialization.phase == AlarmIndexPhase::CloneNeedsBootstrap,
				"ready alarm index marker missing"
			);
			AlarmIndexProgress {
				source: source.clone(),
				after: None,
				ready: false,
			}
		}
	};
	ensure!(progress.source == *source, "alarm import source changed");
	if !progress.ready && crate::split_snapshot::enabled(tablet) {
		ensure!(guard.may_send(), "alarm bootstrap authority revoked");
		let parent_start = tablet
			.parent_range_start
			.context("split snapshot parent range missing")?;
		let marker = depot_types::keys::slatedb::SplitSnapshotKey::new(
			parent_start,
			Uuid::from_bytes(source.checkpoint_id),
		);
		let snapshot = marker.deserialize(
			fs.db
				.get(marker.pack())
				.await?
				.context("split snapshot marker missing")?,
		)?;
		crate::split_snapshot::validate_child(&snapshot, tablet, source)?;
		progress.ready = true;
		let mut batch = slatedb::WriteBatch::new();
		batch.put(address.pack(), address.serialize(progress.clone())?);
		fs.stage(batch).await?;
		fs.shared_flush.flush(&fs.db, FlushCause::Split).await?;
		ensure!(guard.may_send(), "alarm bootstrap authority revoked");
	}
	// The child serves only after the Ready marker, so until then nothing has consumed the
	// imported rows and a resumed import may repeat them from the same checkpoint. After the
	// marker, the child owns its pending work and must never import it again.
	if !progress.ready {
		import_pending_work(fs, factory, source, &guard, env, inbox, op_id).await?;
	}
	let prefix = AlarmTimePrefix::new().pack();
	let mut end = prefix.to_vec();
	*end.last_mut().context("empty alarm prefix")? += 1;
	while !progress.ready {
		ensure!(guard.may_send(), "alarm bootstrap authority revoked");
		inbox
			.send(MaintainerEvent::AlarmBootstrapProgress { op_id })
			.await?;
		let lower = progress
			.after
			.clone()
			.map_or(Bound::Included(prefix.clone()), |after| {
				Bound::Excluded(bytes::Bytes::copy_from_slice(&after))
			});
		let rows = rivet_env::timeout(
			env.clock().as_ref(),
			Duration::from_secs(3),
			factory.checkpoint_page(
				TabletVolumePrefix::new(Uuid::from_bytes(source.parent_id)).pack(),
				Uuid::from_bytes(source.checkpoint_id),
				source.manifest_id,
				(lower, Bound::Excluded(bytes::Bytes::copy_from_slice(&end))),
				128,
			),
		)
		.await??;
		// A byte-limited page may be short; only an empty page proves exhaustion.
		let complete = rows.is_empty();
		let mut batch = slatedb::WriteBatch::new();
		for (physical, value) in rows {
			ensure!(
				physical.starts_with(&prefix)
					&& progress
						.after
						.as_ref()
						.is_none_or(|a| a.as_ref() < physical.as_ref()),
				"alarm import cursor regressed"
			);
			let row = DepotAlarmTimeData::deserialize_with_embedded_version(&value)?;
			let key: VolumeLogKey = row.volume_log_key.clone().into();
			let _pending = row
				.summary
				.pending
				.as_ref()
				.context("cleared alarm in time index")?;
			ensure!(
				super::alarms::time_key(
					&key,
					depot_types::alarm::index_deadline(&row.summary)
						.context("missing alarm deadline")?
				)
				.pack() == physical,
				"alarm import key mismatch"
			);
			if source
				.hash_range
				.contains(depot_types::partition_key_hash(&key.0.volume_key))
			{
				batch.put(physical.clone(), value);
			}
			progress.after = Some(physical.into());
		}
		progress.ready = complete;
		batch.put(address.pack(), address.serialize(progress.clone())?);
		fs.stage(batch).await?;
		fs.shared_flush.flush(&fs.db, FlushCause::Split).await?;
	}
	Ok(())
}

// TODO: Remove this import. A split must copy zero rows: with an unprojected clone the child
// already holds the parent's `FS_PENDING` rows by reference, and bounding its pending scan to its
// own hash range ignores the sibling's rows, so `FS_PENDING_IMPORT` and this copy go away.
/// Copies every pending work row of `source.hash_range` from the parent checkpoint. Each
/// page is flushed with its cursor, so a failed open resumes where it stopped. The final
/// Ready cursor is only staged: the alarm import's flush follows it, and that import's Ready
/// marker can never be durable without it.
async fn import_pending_work(
	fs: &FilesystemStore,
	factory: &dyn SlateDbFactory,
	source: &AlarmIndexSource,
	guard: &Arc<dyn MutationGuard>,
	env: &dyn Env,
	inbox: &tokio::sync::mpsc::Sender<MaintainerEvent>,
	op_id: OpId,
) -> Result<()> {
	let address = PendingImportKey::new();
	let mut progress = match fs.db.get(address.pack()).await? {
		Some(bytes) => address.deserialize(bytes)?,
		None => AlarmIndexProgress {
			source: source.clone(),
			after: None,
			ready: false,
		},
	};
	ensure!(
		progress.source == *source,
		"pending work import source changed"
	);
	let (start, end) = tablet_pending_range(depot_types::PartitionKeyHashRange {
		start: source.hash_range.start,
		end: source.hash_range.end,
	});
	while !progress.ready {
		ensure!(guard.may_send(), "pending work import authority revoked");
		inbox
			.send(MaintainerEvent::AlarmBootstrapProgress { op_id })
			.await?;
		let lower = match &progress.after {
			Some(after) => Bound::Excluded(bytes::Bytes::copy_from_slice(after)),
			None => start.clone(),
		};
		let rows = rivet_env::timeout(
			env.clock().as_ref(),
			Duration::from_secs(3),
			factory.checkpoint_page(
				TabletVolumePrefix::new(Uuid::from_bytes(source.parent_id)).pack(),
				Uuid::from_bytes(source.checkpoint_id),
				source.manifest_id,
				(lower, end.clone()),
				128,
			),
		)
		.await??;
		// A byte-limited page may be short; only an empty page proves exhaustion.
		let complete = rows.is_empty();
		let mut batch = slatedb::WriteBatch::new();
		for (key, value) in rows {
			ensure!(
				progress
					.after
					.as_ref()
					.is_none_or(|after| after.as_ref() < key.as_ref())
					&& std::ops::RangeBounds::contains(&(start.clone(), end.clone()), &key),
				"pending work import cursor left its range"
			);
			batch.put(key.clone(), value);
			progress.after = Some(key.to_vec().into());
		}
		progress.ready = complete;
		batch.put(address.pack(), address.serialize(progress.clone())?);
		fs.stage(batch).await?;
		if !complete {
			fs.shared_flush.flush(&fs.db, FlushCause::Split).await?;
		}
	}
	Ok(())
}
