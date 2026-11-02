//! Atomic disk-state checkpoints and exact segment liveness. Entries are live
//! by their current index location, so replaced/truncated entries cannot pin disk.
//!
//! The installed checkpoint is a chain: an optional base file holding every replica
//! state and retirement, plus delta files that each hold only the volume logs changed
//! above their predecessor. The state machine emits deltas, so its work per checkpoint
//! grows with the changed replicas, not the loaded ones. This executor folds the chain
//! into a new base once the deltas outgrow it, so the full rewrite happens outside the
//! deterministic core and costs at most a constant factor of the delta bytes.

use super::LogShardExecutor;
use crate::log_shard_machine::event::LogShardEvent;
use crate::log_shard_machine::types::WriteToDiskError;
use anyhow::{Context, Result, ensure};
use depot_types::{VolumeLogKey, keys::fs::LogShardFile};
use rivet_data::{
	DEPOT_LOG_SHARD_VERSION,
	generated::depot_log_shard_v1::{
		LogShardCheckpoint, LogShardCheckpointEnvelope, RetiredVolumeFrame, StateFrame,
	},
	versioned::{DepotLogShardCheckpointData, DepotLogShardCheckpointEnvelopeData},
};
use std::collections::BTreeMap;
use vbare::Versioned;

/// When the executor folds the checkpoint chain into a new base: once the delta bytes
/// exceed both the base's bytes and `min_delta_bytes`, or once more than `max_deltas`
/// delta files exist. The first bound keeps the amortized rewrite cost a constant factor
/// of the delta bytes; the second bounds the files recovery reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointCompaction {
	pub min_delta_bytes: u64,
	pub max_deltas: usize,
}

impl Default for CheckpointCompaction {
	fn default() -> Self {
		Self {
			min_delta_bytes: 1024 * 1024,
			max_deltas: 128,
		}
	}
}

impl LogShardExecutor {
	/// Returns the checkpoint's correlated completion after its atomic replacement is synced.
	pub async fn checkpoint_response(
		&mut self,
		checkpoint: &LogShardCheckpoint,
		max_checkpoint_bytes: u64,
		base_bytes: u64,
	) -> LogShardEvent {
		let result = self
			.checkpoint_bounded(checkpoint, max_checkpoint_bytes, base_bytes)
			.await
			.map_err(|e| WriteToDiskError::Write(format!("{e:#}")));
		LogShardEvent::WriteCheckpointResponse {
			last_included_log_seq: checkpoint.last_included_log_seq,
			checkpoint_bytes: self.checkpoint_disk_bytes(),
			result,
		}
	}

	/// Writes the fixed disk image selected by the core, even when newer batches exist.
	/// A base replaces the whole chain. A delta is appended to the chain when its
	/// predecessor boundary is at or below the chain's, then the chain is folded into a
	/// new base if the deltas have outgrown it.
	#[tracing::instrument(skip_all, fields(log_seq = checkpoint.last_included_log_seq))]
	pub async fn checkpoint(&mut self, checkpoint: &LogShardCheckpoint) -> Result<()> {
		self.checkpoint_bounded(checkpoint, u64::MAX, 0).await
	}

	/// Writes within the core's reserved checkpoint space, compacting directly when a
	/// further resident delta would leave insufficient room for the next replacement.
	async fn checkpoint_bounded(
		&mut self,
		checkpoint: &LogShardCheckpoint,
		max_checkpoint_bytes: u64,
		base_bytes: u64,
	) -> Result<()> {
		// Reject images beyond the completed disk write boundary.
		ensure!(!self.poisoned, "shard requires restart");
		ensure!(
			checkpoint.last_included_log_seq <= self.last_log_seq,
			"checkpoint exceeds disk boundary"
		);

		// Keep the executor poisoned until the replacement is durably installed.
		self.poisoned = true;
		match checkpoint.previous_log_seq {
			None => {
				let bytes = encode_checkpoint(checkpoint)?;
				ensure!(
					self.checkpoint_disk_bytes()
						.saturating_add(bytes.len() as u64)
						<= max_checkpoint_bytes,
					"checkpoint exceeds reserved disk space"
				);
				self.write_base(checkpoint).await?;
			}
			Some(previous) => {
				ensure!(
					previous <= self.checkpoint_seq
						&& self.checkpoint_seq <= checkpoint.last_included_log_seq,
					"checkpoint delta does not extend the installed chain"
				);
				let bytes = encode_checkpoint(checkpoint)?;
				if self
					.checkpoint_disk_bytes()
					.saturating_add(bytes.len() as u64)
					.saturating_add(base_bytes.saturating_mul(3))
					> max_checkpoint_bytes
				{
					let merged = self.merged_chain(Some(checkpoint.clone())).await?;
					let bytes = encode_checkpoint(&merged)?;
					ensure!(
						self.checkpoint_disk_bytes()
							.saturating_add(bytes.len() as u64)
							<= max_checkpoint_bytes,
						"checkpoint compaction exceeds reserved disk space"
					);
					self.write_base(&merged).await?;
					self.poisoned = false;
					return Ok(());
				}
				let file = LogShardFile::CheckpointDelta {
					last_included_log_seq: checkpoint.last_included_log_seq,
				};
				self.atomic_write(LogShardFile::CheckpointDeltaTemp, file, &bytes)
					.await?;
				self.checkpoint_deltas
					.insert(checkpoint.last_included_log_seq, bytes.len() as u64);
				self.checkpoint_seq = checkpoint.last_included_log_seq;

				if self.should_compact() {
					self.compact().await?;
				}
			}
		}
		self.poisoned = false;

		Ok(())
	}

	/// Replaces the checkpoint chain's compaction policy.
	pub fn set_checkpoint_compaction(&mut self, policy: CheckpointCompaction) {
		self.checkpoint_compaction = policy;
	}

	/// The delta checkpoint files above the base, by boundary.
	pub fn checkpoint_deltas(&self) -> impl Iterator<Item = u64> + '_ {
		self.checkpoint_deltas.keys().copied()
	}

	/// Installed checkpoint file lengths, independent of the segment index.
	pub(crate) fn checkpoint_disk_bytes(&self) -> u64 {
		self.checkpoint_deltas
			.values()
			.fold(self.checkpoint_base_bytes, |sum, bytes| {
				sum.saturating_add(*bytes)
			})
	}

	fn should_compact(&self) -> bool {
		let policy = self.checkpoint_compaction;
		let delta_bytes = self.checkpoint_deltas.values().sum::<u64>();
		delta_bytes > self.checkpoint_base_bytes.max(policy.min_delta_bytes)
			|| self.checkpoint_deltas.len() > policy.max_deltas
	}

	/// Folds the base and every delta into one base. A crash before the base rename
	/// keeps the old chain; after it, recovery skips the deltas the new base covers.
	async fn compact(&mut self) -> Result<()> {
		let merged = self.merged_chain(None).await?;
		ensure!(
			merged.last_included_log_seq == self.checkpoint_seq,
			"compacted chain ends below the installed boundary"
		);
		self.write_base(&merged).await
	}

	/// Decodes the installed chain, optionally applying a new delta without writing it first.
	async fn merged_chain(&self, next: Option<LogShardCheckpoint>) -> Result<LogShardCheckpoint> {
		let base = match self.fs.read(self.path(LogShardFile::Checkpoint)).await {
			Ok(bytes) => Some(decode_checkpoint(&bytes)?),
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
			Err(error) => return Err(error.into()),
		};
		let mut deltas = Vec::with_capacity(self.checkpoint_deltas.len());
		for last_included_log_seq in self.checkpoint_deltas.keys() {
			let path = self.path(LogShardFile::CheckpointDelta {
				last_included_log_seq: *last_included_log_seq,
			});
			deltas.push(decode_checkpoint(&self.fs.read(path).await?)?);
		}
		if let Some(next) = next {
			deltas.push(next);
		}
		merge_checkpoints(base, deltas)?.context("compacting an empty chain")
	}

	/// Atomically replaces the base, then removes every delta it covers. The caller
	/// holds the executor poisoned.
	pub(crate) async fn write_base(&mut self, image: &LogShardCheckpoint) -> Result<()> {
		ensure!(
			image.previous_log_seq.is_none() && image.changed_volume_log_keys.is_empty(),
			"base checkpoint carries delta fields"
		);
		ensure!(
			image.last_included_log_seq >= self.checkpoint_seq,
			"base checkpoint is below the installed chain"
		);
		let bytes = encode_checkpoint(image)?;
		self.atomic_write(
			LogShardFile::CheckpointTemp,
			LogShardFile::Checkpoint,
			&bytes,
		)
		.await?;
		self.checkpoint_base_bytes = bytes.len() as u64;
		self.checkpoint_seq = image.last_included_log_seq;

		// Every known delta is at or below the new base, so the chain is the base alone.
		let deltas = std::mem::take(&mut self.checkpoint_deltas);
		if !deltas.is_empty() {
			for last_included_log_seq in deltas.keys() {
				self.fs
					.remove_file(self.path(LogShardFile::CheckpointDelta {
						last_included_log_seq: *last_included_log_seq,
					}))
					.await?;
			}
			self.sync_shard_dir().await?;
		}
		Ok(())
	}

	/// Removes exactly the segments chosen by the state machine, then syncs their deletion.
	#[tracing::instrument(skip_all)]
	pub async fn remove_segments(&mut self, first_log_seqs: &[u64]) -> Result<()> {
		ensure!(!self.poisoned, "shard requires restart");
		let dead = first_log_seqs;

		if dead.is_empty() {
			return Ok(());
		}

		// Delete first, then sync the directory before dropping the in-memory accounting.
		self.poisoned = true;
		for first in dead {
			self.fs
				.remove_file(self.path(LogShardFile::Segment {
					first_log_seq: *first,
				}))
				.await?;
		}

		self.sync_shard_dir().await?;
		for first in dead {
			self.disk_segments.remove(first);
		}

		self.poisoned = false;
		tracing::debug!(disk_bytes = self.disk_bytes(), "collected log segments");
		Ok(())
	}
}

/// Encloses the versioned checkpoint in length-delimited bytes with an integrity check.
fn encode_checkpoint(checkpoint: &LogShardCheckpoint) -> Result<Vec<u8>> {
	let payload = DepotLogShardCheckpointData::serialize_with_embedded_version(
		checkpoint,
		DEPOT_LOG_SHARD_VERSION,
	)?;
	DepotLogShardCheckpointEnvelopeData::serialize_with_embedded_version(
		&LogShardCheckpointEnvelope {
			payload_checksum: crc32c::crc32c(&payload),
			payload: payload.into(),
		},
		DEPOT_LOG_SHARD_VERSION,
	)
}

/// Checks the complete envelope before interpreting any checkpoint durability evidence.
pub(crate) fn decode_checkpoint(bytes: &[u8]) -> Result<LogShardCheckpoint> {
	let envelope = DepotLogShardCheckpointEnvelopeData::deserialize_with_embedded_version(bytes)?;
	ensure!(
		crc32c::crc32c(&envelope.payload) == envelope.payload_checksum,
		"checkpoint payload checksum mismatch"
	);
	DepotLogShardCheckpointData::deserialize_with_embedded_version(&envelope.payload)
}

/// Folds a checkpoint chain into one base image, or returns `None` for an empty chain.
/// `deltas` must be in boundary order. A delta at or below the chain's boundary is
/// already covered and skipped; one whose predecessor lies above the boundary means a
/// lost delta and is an error, as is a base among the deltas.
pub(crate) fn merge_checkpoints(
	base: Option<LogShardCheckpoint>,
	deltas: impl IntoIterator<Item = LogShardCheckpoint>,
) -> Result<Option<LogShardCheckpoint>> {
	let mut deltas = deltas.into_iter().peekable();
	if base.is_none() && deltas.peek().is_none() {
		return Ok(None);
	}
	let mut merged = match base {
		Some(base) => {
			ensure!(
				base.previous_log_seq.is_none(),
				"base checkpoint names a predecessor"
			);
			base
		}
		None => LogShardCheckpoint {
			retired_volumes: Vec::new(),
			last_included_log_seq: 0,
			replicas: Vec::new(),
			tablet_grants: Vec::new(),
			tablet_replica_sets: Vec::new(),
			previous_log_seq: None,
			changed_volume_log_keys: Vec::new(),
		},
	};
	let mut replicas: BTreeMap<VolumeLogKey, StateFrame> = std::mem::take(&mut merged.replicas)
		.into_iter()
		.map(|frame| (VolumeLogKey::from(frame.volume_log_key.clone()), frame))
		.collect();
	let mut retired: BTreeMap<[u8; 16], RetiredVolumeFrame> =
		std::mem::take(&mut merged.retired_volumes)
			.into_iter()
			.map(|frame| (frame.volume_log_key_digest, frame))
			.collect();

	for delta in deltas {
		let previous = delta
			.previous_log_seq
			.context("base checkpoint found among deltas")?;
		if delta.last_included_log_seq <= merged.last_included_log_seq {
			continue;
		}
		ensure!(
			previous <= merged.last_included_log_seq,
			"checkpoint chain is missing a delta"
		);
		for key in delta.changed_volume_log_keys {
			let key = VolumeLogKey::from(key);
			retired.remove(&key.digest());
			replicas.remove(&key);
		}
		for frame in delta.replicas {
			replicas.insert(VolumeLogKey::from(frame.volume_log_key.clone()), frame);
		}
		for frame in delta.retired_volumes {
			retired.insert(frame.volume_log_key_digest, frame);
		}
		merged.tablet_grants = delta.tablet_grants;
		merged.tablet_replica_sets = delta.tablet_replica_sets;
		merged.last_included_log_seq = delta.last_included_log_seq;
	}

	merged.replicas = replicas.into_values().collect();
	// A delta does not list the fences its grants released, so drop them here.
	let grants: BTreeMap<[u8; 16], u64> = merged
		.tablet_grants
		.iter()
		.map(|grant| (grant.tablet_id, grant.tablet_grant_epoch))
		.collect();
	retired.retain(|_, frame| {
		grants
			.get(&frame.retirement.tablet_id)
			.is_none_or(|epoch| *epoch <= frame.retirement.tablet_grant_epoch)
	});
	merged.retired_volumes = retired.into_values().collect();
	Ok(Some(merged))
}
