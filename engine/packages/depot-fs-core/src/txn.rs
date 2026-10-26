//! Shared transaction visibility and footprint validation. The resolver retains
//! only identities and inode sets; payloads remain in the replicated log.

use crate::*;
use rivet_data::{
	generated::{
		depot_log_shard_v1::{VolumeLogEntry, VolumeLogEntryBody},
		shared_depot_types_v1::VolumeLogPosition,
	},
	versioned::DepotFsEntryData,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use vbare::Versioned;

pub const MAX_ID_BYTES: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpenTxn {
	/// Where a replay must start to see this transaction whole: the resolver's cold
	/// position when its first entry arrived. That is the position just before the entry,
	/// or the start of an older transaction still open then, because a replay starting
	/// inside the older one would see its commit without its pending entries.
	pub before: VolumeLogPosition,
	pub first: VolumeLogPosition,
	pub footprint: BTreeSet<u64>,
	pub bytes: u64,
}

/// A commit references a bounded replay of its pending entries. Consumers may
/// cache those entries, but must not charge the whole transaction to a read page.
#[derive(Debug)]
pub enum Resolution {
	Standalone(FsEntry),
	Pending,
	Commit {
		id: Vec<u8>,
		open: OpenTxn,
		entry: FsEntry,
	},
	Discarded,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Resolver {
	position: VolumeLogPosition,
	open: BTreeMap<Vec<u8>, OpenTxn>,
}

impl Resolver {
	pub fn new(position: VolumeLogPosition) -> Self {
		Self {
			position,
			open: BTreeMap::new(),
		}
	}
	pub fn position(&self) -> &VolumeLogPosition {
		&self.position
	}
	/// The highest position a replay may start from and still see every transaction after it
	/// whole. Each open transaction records the cold position when it opened, so the oldest
	/// one holds the lowest.
	pub fn cold(&self) -> &VolumeLogPosition {
		self.open
			.values()
			.min_by_key(|txn| txn.first.volume_seq)
			.map_or(&self.position, |txn| &txn.before)
	}
	pub fn open(&self) -> &BTreeMap<Vec<u8>, OpenTxn> {
		&self.open
	}

	/// Validate before changing resolver state. A malformed entry leaves the
	/// prior visibility boundary intact, allowing its owner to fence the volume.
	pub fn push(&mut self, log: &VolumeLogEntry) -> Result<Resolution> {
		let at = self.next_position(log.volume_seq, log.client_epoch)?;
		self.abandon_older(&at);
		let result = match &log.body {
			VolumeLogEntryBody::ClaimEntry => {
				self.open.clear();
				Resolution::Discarded
			}
			VolumeLogEntryBody::PayloadWrite(payload) => {
				// Decoding shares an unencoded payload's buffer, so its pages are not copied.
				let raw = depot_types::payload_codec::decode_shared(payload)
					.map_err(|_| Error::Corrupt)?;
				let bytes = raw.len() as u64;
				let entry = DepotFsEntryData::decode(raw).map_err(|_| Error::Corrupt)?;
				self.resolve(&at, entry, bytes)?
			}
		};
		self.position = at;
		Ok(result)
	}

	/// [`push`](Self::push) for a payload write whose entry the caller already holds decoded,
	/// such as the writer that just encoded it. `payload_bytes` is the encoded payload length.
	/// The result is the same as pushing the encoded entry.
	pub fn push_entry(
		&mut self,
		volume_seq: u64,
		client_epoch: u64,
		entry: FsEntry,
		payload_bytes: u64,
	) -> Result<Resolution> {
		let at = self.next_position(volume_seq, client_epoch)?;
		self.abandon_older(&at);
		let result = self.resolve(&at, entry, payload_bytes)?;
		self.position = at;
		Ok(result)
	}

	/// Discards every transaction a writer at an older claim epoch left open. A transaction
	/// belongs to one writer, and an entry at a newer epoch proves that writer was fenced, so
	/// its open transactions can never close. A settled claim writes no claim entry, so
	/// the new writer's first entry is where they end.
	fn abandon_older(&mut self, at: &VolumeLogPosition) {
		if at.client_epoch > self.position.client_epoch {
			self.open.clear();
		}
	}

	/// Whether every open transaction was left by a writer at an epoch below `client_epoch`,
	/// so the first entry at `client_epoch` discards them.
	pub fn only_older_open(&self, client_epoch: u64) -> bool {
		self.open
			.values()
			.all(|txn| txn.first.client_epoch < client_epoch)
	}

	/// The position of the next entry, rejecting a gap or an older client epoch.
	fn next_position(&self, volume_seq: u64, client_epoch: u64) -> Result<VolumeLogPosition> {
		if self.position.volume_seq.checked_add(1) != Some(volume_seq)
			|| client_epoch < self.position.client_epoch
		{
			return Err(Error::Corrupt);
		}
		Ok(VolumeLogPosition {
			volume_seq,
			client_epoch,
		})
	}

	fn resolve(
		&mut self,
		at: &VolumeLogPosition,
		entry: FsEntry,
		bytes: u64,
	) -> Result<Resolution> {
		validate(&entry)?;
		let inodes = footprint(at.volume_seq, &entry.ops);
		let id = entry.txn.as_ref().map(id);
		if matches!(
			entry.txn,
			Some(FsTxn::FsTxnCommit(_) | FsTxn::FsTxnAbort(_))
		) && !self.open.contains_key(id.unwrap())
		{
			return Ok(Resolution::Discarded);
		}
		if !matches!(entry.txn, Some(FsTxn::FsTxnAbort(_)))
			&& self.open.iter().any(|(other, txn)| {
				Some(other.as_slice()) != id && !txn.footprint.is_disjoint(&inodes)
			}) {
			return Err(Error::Corrupt);
		}
		if id
			.and_then(|id| self.open.get(id))
			.is_some_and(|txn| txn.first.client_epoch != at.client_epoch)
		{
			return Err(Error::Corrupt);
		}
		match &entry.txn {
			None => Ok(Resolution::Standalone(entry)),
			Some(FsTxn::FsTxnPending(v)) => {
				let before = self.cold().clone();
				let pending = self.open.entry(v.id.to_vec()).or_insert_with(|| OpenTxn {
					before,
					first: at.clone(),
					footprint: BTreeSet::new(),
					bytes: 0,
				});
				pending.footprint.extend(inodes);
				pending.bytes = pending.bytes.saturating_add(bytes);
				Ok(Resolution::Pending)
			}
			Some(FsTxn::FsTxnCommit(v)) => {
				let id = v.id.clone();
				let open = self.open.remove(&id[..]).ok_or(Error::Corrupt)?;
				Ok(Resolution::Commit {
					id: id.to_vec(),
					open,
					entry,
				})
			}
			Some(FsTxn::FsTxnAbort(v)) => {
				self.open.remove(&v.id[..]);
				Ok(Resolution::Discarded)
			}
		}
	}
}

pub fn id(txn: &FsTxn) -> &[u8] {
	match txn {
		FsTxn::FsTxnPending(v) => &v.id,
		FsTxn::FsTxnCommit(v) => &v.id,
		FsTxn::FsTxnAbort(v) => &v.id,
	}
}

pub fn creates(op: &FsOp) -> bool {
	matches!(
		op,
		FsOp::FsRoot(_)
			| FsOp::FsCreate(_)
			| FsOp::FsMkdir(_)
			| FsOp::FsSymlink(_)
			| FsOp::FsMknod(_)
	)
}

/// Whether an entry creates too many inodes. A created inode is numbered by its entry's
/// sequence, so an entry holds at most one such create. The root has a fixed number, so an
/// entry may also hold one root, which lets an empty volume format in its first write.
pub fn too_many_creates(ops: &[FsOp]) -> bool {
	let roots = ops
		.iter()
		.filter(|op| matches!(op, FsOp::FsRoot(_)))
		.count();
	let numbered = ops
		.iter()
		.filter(|op| creates(op) && !matches!(op, FsOp::FsRoot(_)))
		.count();
	roots > 1 || numbered > 1
}

pub fn validate(entry: &FsEntry) -> Result<()> {
	use vbare::Versioned;
	for op in &entry.ops {
		match op {
			FsOp::FsKvPut(v) if v.ino == 0 => {
				if v.key.as_ref() != depot_types::alarm::COLD_ALARM_KEY
					|| rivet_data::versioned::DepotVolumeAlarmData::deserialize_with_embedded_version(
						&v.value,
					)
					.is_err()
				{
					return Err(Error::Invalid);
				}
			}
			FsOp::FsKvDelete(v) if v.ino == 0 => return Err(Error::Invalid),
			FsOp::FsKvDeleteRange(v) if v.ino == 0 => return Err(Error::Invalid),
			FsOp::FsReclaim(v) if v.ino == 0 => return Err(Error::Invalid),
			_ => {}
		}
	}

	if entry
		.txn
		.as_ref()
		.is_some_and(|txn| id(txn).len() > MAX_ID_BYTES)
		|| (matches!(entry.txn, Some(FsTxn::FsTxnAbort(_))) && !entry.ops.is_empty())
		|| (matches!(entry.txn, Some(FsTxn::FsTxnPending(_))) && entry.ops.iter().any(creates))
		|| too_many_creates(&entry.ops)
		|| (entry.ops.iter().any(administrative) && (entry.txn.is_some() || entry.ops.len() != 1))
		|| entry.ops.iter().any(|op| !valid_administrative(op))
	{
		return Err(Error::Corrupt);
	}
	Ok(())
}

/// Every inode referenced by an operation participates in isolation, including
/// parents, replaced rename targets, raw KV operations and the allocated inode.
pub fn footprint(seq: u64, ops: &[FsOp]) -> BTreeSet<u64> {
	let mut result = BTreeSet::new();
	for op in ops {
		match op {
			FsOp::FsRoot(_) => {
				result.insert(ROOT_INO);
			}
			FsOp::FsCreate(v) => {
				result.extend([v.parent.ino, seq]);
			}
			FsOp::FsMkdir(v) => {
				result.extend([v.parent.ino, seq]);
			}
			FsOp::FsSymlink(v) => {
				result.extend([v.parent.ino, seq]);
			}
			FsOp::FsMknod(v) => {
				result.extend([v.parent.ino, seq]);
			}
			FsOp::FsLink(v) => {
				result.extend([v.parent.ino, v.target.ino]);
			}
			FsOp::FsUnlink(v) => {
				result.extend([v.parent.ino, v.target.ino]);
			}
			FsOp::FsRename(v) => {
				result.extend([v.from_parent.ino, v.to_parent.ino, v.target.ino]);
				if let Some(v) = &v.replaced {
					result.insert(v.ino);
				}
			}
			FsOp::FsSetAttr(v) => {
				result.insert(v.target.ino);
			}
			FsOp::FsWrite(v) => {
				result.insert(v.target.ino);
			}
			FsOp::FsSetXattr(v) => {
				result.insert(v.target.ino);
			}
			FsOp::FsRemoveXattr(v) => {
				result.insert(v.target.ino);
			}
			FsOp::FsReclaim(v) => {
				result.insert(v.ino);
			}
			FsOp::FsKvPut(v) => {
				result.insert(v.ino);
			}
			FsOp::FsKvDelete(v) => {
				result.insert(v.ino);
			}
			FsOp::FsKvDeleteRange(v) => {
				result.insert(v.ino);
			}
			FsOp::FsCheckpoint(_)
			| FsOp::FsCheckpointRelease(_)
			| FsOp::FsRetentionSet(_)
			| FsOp::FsForkHoldRelease(_)
			| FsOp::FsForkCheckDone(_) => {}
		}
	}
	result
}

/// Rejects an administrative op whose arguments no apply could act on. Every other op is
/// valid here; its own checks run at apply.
fn valid_administrative(op: &FsOp) -> bool {
	match op {
		FsOp::FsCheckpointRelease(v) => {
			!v.name.is_empty() && v.name.len() <= crate::checkpoints::MAX_NAME_BYTES
		}
		FsOp::FsRetentionSet(v) => crate::checkpoints::valid_policy(v.policy.as_ref()),
		FsOp::FsForkHoldRelease(_)
		| FsOp::FsForkCheckDone(_)
		| FsOp::FsRoot(_)
		| FsOp::FsCreate(_)
		| FsOp::FsMkdir(_)
		| FsOp::FsSymlink(_)
		| FsOp::FsMknod(_)
		| FsOp::FsLink(_)
		| FsOp::FsUnlink(_)
		| FsOp::FsRename(_)
		| FsOp::FsSetAttr(_)
		| FsOp::FsWrite(_)
		| FsOp::FsSetXattr(_)
		| FsOp::FsRemoveXattr(_)
		| FsOp::FsReclaim(_)
		| FsOp::FsKvPut(_)
		| FsOp::FsKvDelete(_)
		| FsOp::FsKvDeleteRange(_)
		| FsOp::FsCheckpoint(_) => true,
	}
}

/// Whether `op` is an administrative op: one applied by the store in log order rather than
/// written as rows, which must stand alone in its entry, outside any transaction.
pub fn administrative(op: &FsOp) -> bool {
	match op {
		FsOp::FsCheckpointRelease(_)
		| FsOp::FsRetentionSet(_)
		| FsOp::FsForkHoldRelease(_)
		| FsOp::FsForkCheckDone(_) => true,
		FsOp::FsRoot(_)
		| FsOp::FsCreate(_)
		| FsOp::FsMkdir(_)
		| FsOp::FsSymlink(_)
		| FsOp::FsMknod(_)
		| FsOp::FsLink(_)
		| FsOp::FsUnlink(_)
		| FsOp::FsRename(_)
		| FsOp::FsSetAttr(_)
		| FsOp::FsWrite(_)
		| FsOp::FsSetXattr(_)
		| FsOp::FsRemoveXattr(_)
		| FsOp::FsReclaim(_)
		| FsOp::FsKvPut(_)
		| FsOp::FsKvDelete(_)
		| FsOp::FsKvDeleteRange(_)
		| FsOp::FsCheckpoint(_) => false,
	}
}
