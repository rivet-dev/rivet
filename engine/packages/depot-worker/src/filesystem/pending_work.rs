//! Bounded resumable reclamation driven by the maintainer's existing store queue.
use super::store::{FilesystemStore, LockOperation, SweepOutcome};
use anyhow::{Context, Result, ensure};
use bytes::Bytes;
use depot_fs_core::{
	keys::slatedb as keys,
	work::{PendingSweepProgress, PendingVolume, PendingWorkPage},
};
use rivet_data::generated::shared_depot_fs_v1::FsVolumeStatus;
use rivet_keys::keys::{
	KeyAddress, PrefixAddress,
	codec::{AtVersion, FormalKey},
};
use slatedb::WriteBatch;
use std::ops::Bound;

impl FilesystemStore {
	/// One page of the pass over every pending row, after `after`.
	pub async fn pending_work(
		&self,
		after: Option<Vec<u8>>,
		limit: u32,
	) -> Result<PendingWorkPage> {
		self.admit()?;
		ensure!(limit > 0, "empty sweep discovery budget");
		let prefix = keys::PendingWorkPrefix::new().pack();
		let after = after.map(Bytes::from);
		ensure!(
			after.as_ref().is_none_or(|a| a.starts_with(&prefix)),
			"foreign sweep discovery cursor"
		);
		let (start, end) =
			depot_types::keys::slatedb::tablet_pending_range(self.hash_range.clone());
		ensure!(
			after
				.as_ref()
				.is_none_or(|a| std::ops::RangeBounds::contains(&(start.clone(), end.clone()), a)),
			"sweep discovery cursor outside tablet"
		);
		let lower = after.map_or(start, Bound::Excluded);
		// One snapshot serves the whole page, so the rows and the records they name
		// agree. The maintainer core runs no writer of pending rows beside the scan.
		let snapshot = self.db.snapshot().await?;
		let mut scan = snapshot.scan((lower, end)).await?;
		let mut page = empty_page();
		for _ in 0..limit.min(self.batch_rows as u32) {
			let Some(row) = scan.next().await? else {
				page.after = None;
				self.admit()?;
				return Ok(page);
			};
			page.after = Some(row.key.to_vec());
			classify(&snapshot, &row.key, row.value, &mut page).await?;
		}
		self.admit()?;
		Ok(page)
	}

	/// One page of `volume`'s pending rows after `after`, without its fork checks. A store
	/// write that touched a volume key's pending rows is rediscovered here, so the cost
	/// follows that volume's own rows, not the tablet's. Fork checks are skipped because
	/// the maintainer arms each one's deadline when it learns of it, and a parent with many
	/// forks in their grace period would otherwise be read in full on every write.
	pub async fn pending_work_of(
		&self,
		volume: &PendingVolume,
		after: Option<Vec<u8>>,
		limit: u32,
	) -> Result<PendingWorkPage> {
		self.admit()?;
		ensure!(limit > 0, "empty sweep discovery budget");
		if !self.hash_range.contains(volume.hash) {
			return Ok(empty_page());
		}
		let prefix = keys::PendingVolumePrefix::new(volume.hash, volume.name.clone()).pack();
		let checks = keys::PendingForkChecksPrefix::new(volume.hash, volume.name.clone()).pack();
		let checks_suffix = checks.slice(prefix.len()..);
		let past_checks = Bound::Included(
			successor_prefix(&checks_suffix).context("fork check prefix has no successor")?,
		);
		let after = after.map(Bytes::from);
		ensure!(
			after.as_ref().is_none_or(|a| a.starts_with(&prefix)),
			"foreign volume discovery cursor"
		);
		let mut lower = after.as_ref().map_or(Bound::Unbounded, |a| {
			Bound::Excluded(a.slice(prefix.len()..))
		});
		let snapshot = self.db.snapshot().await?;
		let mut scan = snapshot
			.scan_prefix(prefix.clone(), (lower.clone(), Bound::<Bytes>::Unbounded))
			.await?;
		let mut page = empty_page();
		let mut skipped_checks = false;
		let mut read = 0;
		while read < limit.min(self.batch_rows as u32) {
			let Some(row) = scan.next().await? else {
				page.after = None;
				self.admit()?;
				return Ok(page);
			};
			if row.key.starts_with(&checks) {
				// Seek past every fork check at once; a second check row means the seek
				// did not move, which the key layout rules out.
				ensure!(
					!skipped_checks,
					"volume discovery did not move past fork checks"
				);
				skipped_checks = true;
				lower = past_checks.clone();
				scan = snapshot
					.scan_prefix(prefix.clone(), (lower.clone(), Bound::<Bytes>::Unbounded))
					.await?;
				continue;
			}
			read += 1;
			page.after = Some(row.key.to_vec());
			classify(&snapshot, &row.key, row.value, &mut page).await?;
		}
		self.admit()?;
		Ok(page)
	}

	/// Both data and cursor writes are idempotent. The marker is deleted only
	/// after the final data page is durable; interruption repeats the old page.
	pub async fn sweep_pending(&self, scope: &keys::Scope) -> Result<PendingSweepProgress> {
		let _ordered = self
			.lock(
				super::locks::LockRequest::scope(scope),
				LockOperation::Other,
			)
			.await;
		self.admit()?;
		let key = keys::PendingSweepKey::new(scope.hash, scope.name.clone(), scope.incarnation);
		let Some(value) = self.db.get(key.pack()).await? else {
			self.admit()?;
			return Ok(PendingSweepProgress::Complete);
		};
		let mut pending = key.deserialize(value)?;
		ensure!(
			pending.incarnation == *scope.incarnation.as_bytes(),
			"pending sweep identity mismatch"
		);
		let result = self
			.sweep_locked(
				scope,
				pending.cursor.map(Bytes::from),
				self.batch_rows as u32,
			)
			.await?;
		let more = matches!(result, SweepOutcome::More(_));
		let mut batch = WriteBatch::new();
		match result {
			SweepOutcome::Held(hold) => return Ok(PendingSweepProgress::Held(hold)),
			SweepOutcome::More(after) => {
				pending.cursor = Some(after);
				batch.put(
					key.pack(),
					key.serialize(AtVersion::new(pending, self.row_version()))?,
				);
			}
			SweepOutcome::Complete => {
				batch.delete(key.pack());
			}
		}
		// The cursor rides the tablet's ordinary pass with the deletes staged before it.
		self.stage(batch).await?;
		self.admit()?;
		Ok(if more {
			PendingSweepProgress::More
		} else {
			PendingSweepProgress::Complete
		})
	}
}

fn empty_page() -> PendingWorkPage {
	PendingWorkPage {
		materializations: Vec::new(),
		cleanup: Vec::new(),
		forks: Vec::new(),
		scopes: Vec::new(),
		deletes: Vec::new(),
		after: None,
	}
}

/// The smallest key above every key that starts with `prefix`, or `None` for a prefix of
/// only 0xff bytes.
fn successor_prefix(prefix: &[u8]) -> Option<Bytes> {
	let mut bytes = prefix.to_vec();
	while let Some(last) = bytes.pop() {
		if last < u8::MAX {
			bytes.push(last + 1);
			return Some(Bytes::from(bytes));
		}
	}
	None
}

/// Adds one pending row to `page` by its kind. A deletion is reported only while its
/// incarnation's own record in `snapshot` is Deleted.
async fn classify(
	snapshot: &slatedb::DbSnapshot,
	key: &Bytes,
	value: Bytes,
	page: &mut PendingWorkPage,
) -> Result<()> {
	if let Ok(key) = keys::PendingDeleteKey::unpack(key) {
		// The marker shares the record's batches, so the record is the deletion's
		// authority.
		let record_key =
			keys::VolumeIncarnationRecordKey::new(key.hash, key.name.clone(), key.incarnation);
		if let Some(value) = snapshot.get(record_key.pack()).await? {
			let record = record_key.deserialize(value)?;
			if record.status == FsVolumeStatus::Deleted {
				page.deletes.push((record.volume_key(), record));
			}
		}
	} else if let Ok(key) = keys::PendingSweepKey::unpack(key) {
		page.scopes.push(key.scope());
	} else if let Ok(key) = keys::PendingMaterializeKey::unpack(key) {
		page.materializations.push(key.scope());
	} else if let Ok(key) = keys::PendingForkCheckKey::unpack(key) {
		page.cleanup.push(depot_fs_core::forks::cleanup::Work {
			scope: key.scope(),
			kind: depot_fs_core::forks::cleanup::Kind::Check,
		});
	} else if let Ok(key) = keys::PendingForkReleaseKey::unpack(key) {
		page.cleanup.push(depot_fs_core::forks::cleanup::Work {
			scope: key.scope(),
			kind: depot_fs_core::forks::cleanup::Kind::Release,
		});
	} else if let Ok(address) = keys::PendingForkKey::unpack(key) {
		let pending = address.deserialize(value)?;
		ensure!(
			keys::PendingForkKey::new(
				depot_types::partition_key_hash(&pending.spec.destination_volume_key),
				pending
					.spec
					.destination_volume_key
					.name
					.clone()
					.map(Into::into)
			)
			.pack() == *key,
			"fork discovery address mismatch"
		);
		page.forks.push(pending.spec);
	}
	Ok(())
}
