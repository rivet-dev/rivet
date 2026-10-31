//! Store reads and writes of operator repair calls: the audit rows each call's audit op writes,
//! and the probe a record rebuild plans with.

use super::{session::Result as SessionResult, store::FilesystemStore};
use crate::repair_types::{PrepareReads, RepairPrepared, RepairProbe};
use anyhow::{Result, ensure};
use bytes::Bytes;
use depot_fs_core::{FsVolumeRecord, ROOT_INO, keys::slatedb as keys, store::RowKey};
use rivet_data::generated::{
	depot_client_protocol_v1::RepairAuditRow, depot_maintainer_log_v1::RepairAuditOp,
};
use rivet_keys::keys::{KeyAddress, PrefixAddress, codec::FormalKey};
use slatedb::WriteBatch;
use std::ops::Bound;
use uuid::Uuid;

fn audit_key(volume_key: &depot_types::VolumeKey, operation_id: [u8; 16]) -> keys::RepairAuditKey {
	keys::RepairAuditKey::new(
		depot_types::partition_key_hash(volume_key),
		volume_key.name.clone().map(Into::into),
		Uuid::from_bytes(operation_id),
	)
}

impl FilesystemStore {
	/// Writes one repair call's audit row to the memtable: the op is durable in the maintainer
	/// log, and an ordinary pass makes the row cold. A later op of the same call replaces it.
	pub(super) async fn write_repair_audit(&self, op: RepairAuditOp) -> Result<()> {
		let envelope = &op.row.request.envelope;
		let key = audit_key(&envelope.volume_log_key.volume_key, envelope.operation_id);
		let mut batch = WriteBatch::new();
		batch.put(key.pack(), key.serialize(op)?);
		self.stage(batch).await?;
		self.admit()
	}

	/// The audit row of one repair call of `volume_key`, if one was written.
	pub async fn repair_audit(
		&self,
		volume_key: &depot_types::VolumeKey,
		operation_id: [u8; 16],
	) -> Result<Option<RepairAuditRow>> {
		self.admit()?;
		let key = audit_key(volume_key, operation_id);
		let row = self
			.db
			.get(key.pack())
			.await?
			.map(|value| key.deserialize(value))
			.transpose()?
			.map(|op| op.row);
		ensure!(
			row.as_ref().is_none_or(|row| {
				row.request.envelope.operation_id == operation_id
					&& row.request.envelope.volume_log_key.volume_key == *volume_key
			}),
			"repair audit row identity mismatch"
		);
		self.admit()?;
		Ok(row)
	}

	/// Up to `limit` audit rows of `volume_key`, ordered by operation id after `after`, and
	/// the operation id to continue after when more may follow.
	pub async fn repair_audits(
		&self,
		volume_key: &depot_types::VolumeKey,
		after: Option<[u8; 16]>,
		limit: usize,
	) -> Result<(Vec<RepairAuditRow>, Option<[u8; 16]>)> {
		self.admit()?;
		let prefix = keys::RepairAuditPrefix::new(
			depot_types::partition_key_hash(volume_key),
			volume_key.name.clone().map(Into::into),
		)
		.pack();
		let lower = match after {
			Some(after) => {
				let key = audit_key(volume_key, after).pack();
				Bound::Excluded(Bytes::copy_from_slice(&key[prefix.len()..]))
			}
			None => Bound::Unbounded,
		};
		let mut scan = self
			.db
			.scan_prefix(prefix, (lower, Bound::<Bytes>::Unbounded))
			.await?;
		let mut rows = Vec::new();
		while rows.len() < limit {
			let Some(found) = scan.next().await? else {
				return Ok((rows, None));
			};
			let key = keys::RepairAuditKey::unpack(&found.key)?;
			rows.push(key.deserialize(found.value)?.row);
		}
		self.admit()?;
		let next = rows
			.last()
			.map(|row: &RepairAuditRow| row.request.envelope.operation_id);
		Ok((rows, next))
	}

	/// Reads `operation_id`'s audit row of `key`'s name and, with `probe`, whether `key` has a
	/// format row and a root inode at `probe`'s cold position. The probe opens a one-shot
	/// session against `probe` itself, since a quarantined incarnation has no stored record.
	pub async fn repair_prepare(
		&self,
		key: &depot_types::VolumeLogKey,
		operation_id: [u8; 16],
		reads: PrepareReads,
	) -> Result<RepairPrepared> {
		let audit = self.repair_audit(&key.0.volume_key, operation_id).await?;
		let reads = async {
			let read = match reads {
				PrepareReads::None => (None, None),
				PrepareReads::Probe(record) => (
					Some(
						self.repair_probe(key, record)
							.await
							.map_err(|error| anyhow::anyhow!("repair probe failed: {error}"))?,
					),
					None,
				),
				PrepareReads::Rows {
					record,
					applied,
					keys,
				} => (
					None,
					Some((
						applied.clone(),
						self.repair_rows(key, record, applied, &keys)
							.await
							.map_err(|error| anyhow::anyhow!("repair row read failed: {error}"))?,
					)),
				),
			};
			anyhow::Ok(read)
		};
		let (probe, rows) = match reads.await {
			Ok(read) => read,
			// A recorded call answers from its audit row, so a plan read that fails now, after
			// a deletion or a loss, never hides the outcome a retry must see.
			Err(_) if audit.is_some() => (None, None),
			Err(error) => return Err(error),
		};
		Ok(RepairPrepared { audit, probe, rows })
	}

	/// The newest stored value bytes of each of `keys` at `applied`, read in one session.
	async fn repair_rows(
		&self,
		key: &depot_types::VolumeLogKey,
		record: FsVolumeRecord,
		applied: rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
		keys: &[rivet_data::generated::shared_depot_fs_read_v1::FsRowKey],
	) -> SessionResult<Vec<Option<Bytes>>> {
		let session = self
			.open_session(key, &record, applied.clone(), Some(applied.volume_seq))
			.await?;
		let result = async {
			let mut values = Vec::with_capacity(keys.len());
			for row in keys {
				let row = RowKey::from(row.clone());
				let newest = session.raw_versions(&row, 1).await?.into_iter().next();
				values.push(match newest {
					// A delete marker leaves the row absent.
					Some((_, _, Ok(None))) | None => None,
					Some((_, raw, Ok(Some(_)) | Err(_))) => Some(raw),
				});
			}
			Ok(values)
		}
		.await;
		self.close_session(session.id);
		result
	}

	async fn repair_probe(
		&self,
		key: &depot_types::VolumeLogKey,
		record: FsVolumeRecord,
	) -> SessionResult<RepairProbe> {
		let cold = record.cold.clone();
		// The incarnation has no stored record to check the planned one against, and nothing
		// writes its rows while it is quarantined, so the session opens from the planned
		// record under the volume's stripe.
		let session = self
			.open_ordered_session(key, &record, cold.clone(), Some(cold.volume_seq))
			.await?;
		let result = async {
			let format = session.version(&RowKey::Format).await?;
			let root = session.version(&RowKey::Inode(ROOT_INO)).await?;
			Ok(RepairProbe {
				record: record.clone(),
				format_row_present: matches!(format, Some((_, Some(_)))),
				root_inode_present: matches!(root, Some((_, Some(_)))),
			})
		}
		.await;
		self.close_session(session.id);
		result
	}
}
