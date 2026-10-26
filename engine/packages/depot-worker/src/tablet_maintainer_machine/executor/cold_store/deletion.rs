//! Bounded cleanup of one deleted incarnation; discovery remains a maintainer decision.

use super::*;
use rivet_data::generated::shared_depot_fs_v1::FsVolumeStatus;

pub use crate::tablet_maintainer_machine::types::DeleteSweepResult;

impl ColdStore {
	/// A row update of one incarnation cannot move its cold position backwards, change its
	/// identity or ordering epoch, or resurrect its deletion. Each incarnation has its own row,
	/// so other incarnations of the name do not constrain it.
	pub(super) async fn validate_row_update(
		&self,
		snapshot: &slatedb::DbSnapshot,
		volume_key: &depot_types::VolumeKey,
		proposed: &FsVolumeRecord,
	) -> Result<()> {
		ensure!(
			proposed.volume_key() == *volume_key,
			"proposed volume record identity mismatch"
		);
		let key = VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
			volume_key: volume_key.clone(),
			volume_incarnation: proposed.volume_incarnation,
		});
		let address = super::incarnation_record_address(&key);
		let row = snapshot
			.get(address.pack())
			.await?
			.map(|value| address.deserialize(value))
			.transpose()?;

		if let Some(existing) = row {
			ensure!(
				existing.volume_key() == *volume_key
					&& existing.volume_incarnation == proposed.volume_incarnation,
				"stored volume record identity mismatch"
			);
			ensure!(
				existing.incarnation_epoch == proposed.incarnation_epoch,
				"incarnation epoch changed"
			);
			ensure!(
				proposed.cold.volume_seq >= existing.cold.volume_seq,
				"cold progress cannot move backwards"
			);
			ensure!(
				proposed.cold.volume_seq != existing.cold.volume_seq
					|| proposed.cold.client_epoch == existing.cold.client_epoch,
				"cold epoch changed at the same position"
			);
			ensure!(
				existing.status != FsVolumeStatus::Deleted
					|| proposed.status == FsVolumeStatus::Deleted,
				"cannot resurrect a deleted volume"
			);
		}
		Ok(())
	}

	/// Reads an incarnation's deletion marker from its own record row.
	#[tracing::instrument(skip_all)]
	pub async fn deleted_row(&self, key: &VolumeLogKey) -> Result<Option<FsVolumeRecord>> {
		Ok(self
			.incarnation_row(key)
			.await?
			.filter(|row| row.status == FsVolumeStatus::Deleted))
	}

	/// Releases a deleted incarnation's metadata at once and hands its data to the background
	/// sweep, so the volume key can be created again without waiting for its data to go.
	///
	/// The incarnation's own Deleted record, which an ordinary pass stored first, is the
	/// authority. One batch writes the incarnation's `PendingSweepKey`, deletes its pending
	/// delete and deletes the name's pointer when the pointer names this incarnation; a pointer
	/// naming a newer incarnation, of this volume key or another sharing its name, stays. The
	/// incarnation's own row stays until the pending sweep removes its data through
	/// `sweep_pending`, which respects read sessions and durable
	/// checkpoint and fork holds, so readers and forks of the old incarnation keep working and
	/// can still prove its final position. The batch is not flushed: the next ordinary pass
	/// makes it durable. `max_rows` must be positive.
	pub async fn sweep_deleted_volume(
		&self,
		key: &VolumeLogKey,
		max_rows: u32,
	) -> Result<DeleteSweepResult> {
		use depot_fs_core::keys::slatedb as keys;
		use rivet_keys::keys::PrefixAddress;
		ensure!(max_rows > 0, "sweep requires a nonzero row budget");
		let scope = keys::Scope::new(
			&key.0.volume_key,
			Uuid::from_bytes(key.0.volume_incarnation),
		);
		let fs = self.filesystem();
		// The volume's stripe covers the handoff, so lookups and session opens of the volume
		// never see it half done.
		let _ordered = fs
			.lock(
				crate::filesystem::locks::LockRequest::scope(&scope),
				crate::filesystem::store::LockOperation::Sweep,
			)
			.await;
		let own = self.incarnation_row(key).await?;
		// The pointer may name another volume key that shares this name, such as the database
		// of a recreated actor. It is then a newer incarnation of the name, which stays.
		let pointer = self.row(key.0.volume_key.clone()).await?;
		fs.admit()?;
		let pending = keys::PendingSweepKey::new(scope.hash, scope.name.clone(), scope.incarnation);
		let pending_exists = self.db.get(pending.pack()).await?.is_some();
		let Some(own) = own else {
			if pending_exists {
				fs.admit()?;
				return Ok(DeleteSweepResult::Complete);
			}
			let mut remaining = self
				.db
				.scan_prefix(
					keys::IncarnationPrefix::new(scope.hash, scope.name.clone(), scope.incarnation)
						.pack(),
					..,
				)
				.await?;
			// The terminal incarnation receipt deliberately survives its data.
			// At most that one key can be skipped; any other row still requires
			// a durable cleanup marker and must not be silently reclaimed.
			while let Some(row) = remaining.next().await? {
				let archive = keys::IncarnationRecordKey::unpack(&row.key).map_err(|_| {
					anyhow::anyhow!("deletion marker missing while filesystem data remains")
				})?;
				let retained = archive.deserialize(row.value)?;
				ensure!(
					retained.record.volume_key() == key.0.volume_key
						&& retained.record.volume_incarnation == key.0.volume_incarnation,
					"retained incarnation address mismatch"
				);
			}
			fs.admit()?;
			return Ok(DeleteSweepResult::Complete);
		};
		ensure!(
			own.status == FsVolumeStatus::Deleted,
			"cannot sweep a live incarnation"
		);

		let mut batch = WriteBatch::new();
		if !pending_exists {
			batch.put(
				pending.pack(),
				pending.serialize(rivet_keys::keys::codec::AtVersion::new(
					depot_fs_core::FsPendingSweep {
						incarnation: *scope.incarnation.as_bytes(),
						cursor: None,
					},
					fs.row_version(),
				))?,
			);
		}
		if pointer.as_ref().is_some_and(|pointer| {
			pointer.volume_incarnation == key.0.volume_incarnation
				&& pointer.volume_key() == key.0.volume_key
		}) {
			batch.delete(VolumeRecordKey::for_volume(&key.0.volume_key).pack());
		}
		batch.delete(
			keys::PendingDeleteKey::new(scope.hash, scope.name.clone(), scope.incarnation).pack(),
		);
		// The batch rides the tablet's next ordinary pass. The maintainer forgets the
		// tombstoned replicas, the delete's proof, only after a pass covered it.
		self.db.write(batch).await?;
		self.filesystem().admit()?;
		Ok(DeleteSweepResult::Complete)
	}
}


