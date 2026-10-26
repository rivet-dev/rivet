//! Canonical MVCC alarm values and a reconstructible, monotonic time-index projection.
use super::{
	session::ReadSession,
	store::{FilesystemStore, PlannedWrites},
};
use anyhow::{Context, Result, ensure};
use depot_fs_core::{
	keys::slatedb as fs_keys,
	store::{Row, RowKey},
};
use depot_types::{
	VolumeLogKey,
	alarm::{COLD_ALARM_KEY, index_deadline, ordered_deadline},
	keys::slatedb::{AlarmCurrentKey, AlarmTimeKey, AlarmTimePrefix},
};
use rivet_data::{
	generated::{
		depot_alarm_v1::{AlarmCurrent, AlarmRetryRecord, AlarmTimeRecord, AlarmValue},
		shared_depot_alarms_v1::AlarmSummary,
		shared_depot_fs_v1::{FsVolumeRecord, FsVolumeStatus},
		shared_depot_types_v1::VolumeLogPosition,
	},
	versioned::DepotVolumeAlarmData,
};
use rivet_keys::keys::{KeyAddress, PrefixAddress, codec::FormalKey};
use std::ops::Bound;
use uuid::Uuid;
use vbare::Versioned;

pub(crate) fn metadata_key() -> RowKey {
	RowKey::Kv(0, COLD_ALARM_KEY.to_vec())
}
fn current_key(key: &VolumeLogKey) -> AlarmCurrentKey {
	AlarmCurrentKey::new(
		depot_types::partition_key_hash(&key.0.volume_key),
		key.0.volume_key.name.clone().map(Into::into),
		Uuid::from_bytes(key.0.volume_incarnation),
	)
}
pub(crate) fn time_key(key: &VolumeLogKey, deadline: i64) -> AlarmTimeKey {
	AlarmTimeKey::new(
		ordered_deadline(deadline),
		depot_types::partition_key_hash(&key.0.volume_key),
		key.0.volume_key.name.clone().map(Into::into),
		Uuid::from_bytes(key.0.volume_incarnation),
	)
}

impl FilesystemStore {
	async fn alarm_value_at(
		&self,
		key: &VolumeLogKey,
		position: VolumeLogPosition,
	) -> Result<Option<AlarmValue>> {
		let session = ReadSession::internal(
			self.db.snapshot().await?,
			fs_keys::Scope::new(
				&key.0.volume_key,
				Uuid::from_bytes(key.0.volume_incarnation),
			),
			position,
			self.clock.clone(),
			self.guard.clone(),
			self.config.clone(),
		);
		match session.get(&metadata_key()).await? {
			Some(Row::Bytes(bytes)) => Ok(Some(
				DepotVolumeAlarmData::deserialize_with_embedded_version(&bytes)?,
			)),
			None => Ok(None),
			_ => anyhow::bail!("invalid volume alarm value"),
		}
	}
	pub(crate) async fn alarm_at(
		&self,
		key: &VolumeLogKey,
		position: VolumeLogPosition,
	) -> Result<AlarmSummary> {
		Ok(AlarmSummary {
			pending: self
				.alarm_value_at(key, position.clone())
				.await?
				.and_then(|v| v.pending),
			last_change_position: position,
		})
	}

	/// One fresh projection carries both delivery eligibility and advisory retry progress.
	pub(crate) async fn current_alarm(&self, key: &VolumeLogKey) -> Result<Option<AlarmCurrent>> {
		let address = current_key(key);
		self.db
			.get(address.pack())
			.await?
			.map(|bytes| address.deserialize(bytes))
			.transpose()
	}
	/// Complete replay advances the projection monotonically; older replay cannot restore a clear.
	pub(crate) async fn update_alarm_index(
		&self,
		key: &VolumeLogKey,
		position: VolumeLogPosition,
	) -> Result<Option<AlarmSummary>> {
		let old = self.current_alarm(key).await?;
		if old
			.as_ref()
			.is_some_and(|old| old.summary.last_change_position.volume_seq >= position.volume_seq)
		{
			return Ok(None);
		}
		let value = self.alarm_value_at(key, position.clone()).await?;
		if value.is_none() && old.is_none() {
			return Ok(None);
		}
		let summary = AlarmSummary {
			last_change_position: position,
			pending: value.and_then(|value| value.pending),
		};
		let changed = old
			.as_ref()
			.is_none_or(|old| old.summary.pending != summary.pending);
		let current = AlarmCurrent {
			summary: summary.clone(),
			retry: if changed {
				None
			} else {
				old.as_ref().and_then(|old| old.retry.clone())
			},
		};
		let mut planned = PlannedWrites::default();
		if changed {
			self.put_alarm_projection(key, old.as_ref(), &current, &mut planned)?;
		} else {
			// A completed batch can contain Set then Clear. Certify its final position
			// even when its postimage matches, without resetting the arm's cooldown.
			let address = current_key(key);
			planned.put(address.pack(), address.serialize(current)?);
		}
		let mut batch = slatedb::WriteBatch::new();
		planned.apply(&mut batch);
		self.stage(batch).await?;
		if changed { Ok(Some(summary)) } else { Ok(None) }
	}
	pub(crate) fn put_alarm_projection(
		&self,
		key: &VolumeLogKey,
		old: Option<&AlarmCurrent>,
		current: &AlarmCurrent,
		planned: &mut PlannedWrites,
	) -> Result<()> {
		if let Some(at) = old.and_then(|old| index_deadline(&old.summary)) {
			planned.delete(time_key(key, at).pack());
		}
		let address = current_key(key);
		planned.put(address.pack(), address.serialize(current.clone())?);
		if let Some(at) = index_deadline(&current.summary) {
			let address = time_key(key, at);
			planned.put(
				address.pack(),
				address.serialize(AlarmTimeRecord {
					volume_log_key: key.0.clone(),
					summary: current.summary.clone(),
					retry: current.retry.clone(),
				})?,
			);
		}
		Ok(())
	}
	/// Retirement deletes current eligibility in the same batch as the volume record.
	pub(crate) async fn plan_alarm_index(
		&self,
		record: &FsVolumeRecord,
		planned: &mut PlannedWrites,
	) -> Result<()> {
		if record.status != FsVolumeStatus::Deleted {
			return Ok(());
		}
		let key = VolumeLogKey::from(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
			volume_key: record.volume_key(),
			volume_incarnation: record.volume_incarnation,
		});
		if let Some(old) = self.current_alarm(&key).await? {
			if let Some(at) = index_deadline(&old.summary) {
				planned.delete(time_key(&key, at).pack());
			}
		}
		planned.delete(current_key(&key).pack());
		Ok(())
	}
	/// Bounded ordered discovery reads the next page once, including future deadlines.
	pub(crate) async fn alarm_page(
		&self,
		snapshot: &slatedb::DbSnapshot,
		after: Option<bytes::Bytes>,
		limit: usize,
	) -> Result<(Vec<(bytes::Bytes, AlarmTimeRecord)>, bool)> {
		let prefix = AlarmTimePrefix::new().pack();
		ensure!(
			limit > 0
				&& after
					.as_ref()
					.is_none_or(|after| after.starts_with(&prefix)),
			"invalid alarm page"
		);
		let lower = after.map_or(Bound::Included(prefix.clone()), Bound::Excluded);
		let mut end = prefix.to_vec();
		*end.last_mut().context("empty alarm prefix")? += 1;
		let mut scan = snapshot
			.scan((lower, Bound::Excluded(bytes::Bytes::from(end))))
			.await?;
		let mut rows = Vec::new();
		for _ in 0..limit {
			let Some(row) = scan.next().await? else {
				return Ok((rows, true));
			};
			let value =
				rivet_data::versioned::DepotAlarmTimeData::deserialize_with_embedded_version(
					&row.value,
				)?;
			rows.push((row.key, value));
		}
		Ok((rows, false))
	}
	/// The core orders this write against replay and retirement of the same volume.
	/// A cancelled stale retry cannot replace a newer revision's eligibility.
	pub(crate) async fn stage_alarm_retry(
		&self,
		key: &VolumeLogKey,
		summary: AlarmSummary,
		next_unix_ms: i64,
		attempt: u32,
		owner_id: [u8; 16],
		next_boot_ms: u64,
	) -> Result<Option<AlarmCurrent>> {
		let Some(old) = self.current_alarm(key).await? else {
			return Ok(None);
		};
		let expected = depot_types::alarm::revision(key.0.volume_incarnation, &summary);
		if expected.is_none()
			|| expected != depot_types::alarm::revision(key.0.volume_incarnation, &old.summary)
		{
			return Ok(Some(old));
		}
		let current = AlarmCurrent {
			summary: old.summary.clone(),
			retry: Some(AlarmRetryRecord {
				revision: expected.context("retry without an alarm")?,
				next_unix_ms,
				next_boot_ms,
				attempt,
				owner_id,
			}),
		};
		let mut planned = PlannedWrites::default();
		self.put_alarm_projection(key, Some(&old), &current, &mut planned)?;
		let mut batch = slatedb::WriteBatch::new();
		planned.apply(&mut batch);
		self.stage(batch).await?;
		Ok(Some(current))
	}
}


