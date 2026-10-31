//! Spare segments: files zero-filled and synced before a segment roll needs them, so the
//! writes into a new segment overwrite allocated blocks and sync only their data instead of
//! journaling a size or extent change on every commit.
//!
//! A spare also holds an empty segment header in both slots, written and synced with its
//! zeros, so a header is durable before any batch reaches the segment.
//!
//! A spare is written under `seg-spare.tmp`, synced, renamed to `seg-spare`, and the shard
//! directory is synced. Only then is it ready, and only the executor touches it after that:
//! a roll renames it to the new segment's name and syncs the directory before any batch in
//! it is acknowledged. A spare therefore never holds an acknowledged batch, and recovery
//! deletes both names.

use super::LogShardExecutor;
use anyhow::{Context, Result};
use depot_types::{
	keys::fs::{LogShardFile, LogShardKey},
	log_shard::{SEGMENT_HEADER_BYTES, SEGMENT_HEADER_SLOT_BYTES, SegmentHeader},
};
use rivet_env::{TaskScope, fs::Fs};
use rivet_keys::keys::codec::KeyAddress;
use std::{
	path::{Path, PathBuf},
	sync::Weak,
};
use tokio::{
	io::{AsyncSeekExt, AsyncWriteExt},
	sync::oneshot,
};

/// Zero bytes written per call while filling a spare. It bounds the fill's buffer, not
/// its duration; the fill runs beside the write queue.
const ZERO_CHUNK_BYTES: usize = 1024 * 1024;

/// Whether a zero-filled spare is available for the next segment roll.
pub(crate) enum SpareSegment {
	/// The owner has not enabled preallocation, or a fill failed and the executor stopped
	/// preparing spares until restart. Rolls create empty segments.
	Disabled,

	/// No spare exists or is being prepared. The next successful write starts a fill when
	/// the owner supplied a task scope.
	Absent,

	/// A task owned by the shard's task scope is preparing the spare. It reports the spare's
	/// size once `seg-spare` and its directory entry are synced, or the error that ended it.
	Filling {
		done: oneshot::Receiver<Result<u64>>,
	},

	/// `seg-spare` is durably zero-filled to `bytes`, and no task touches it.
	Ready { bytes: u64 },
}

impl LogShardExecutor {
	/// Makes later segment rolls use zero-filled spares. With a task scope, spares are
	/// prepared in the background, starting now and again after each roll consumes one.
	/// Without one, only `prepare_spare_segment` prepares them. Segments too small to hold
	/// a header and a batch after it stay plain.
	pub fn enable_preallocation(&mut self, tasks: Option<Weak<TaskScope>>) {
		if self.segment_bytes <= SEGMENT_HEADER_BYTES {
			tracing::warn!(
				segment_bytes = self.segment_bytes,
				"log segments are too small to preallocate; raise storage.log_shard.segment_bytes"
			);
			return;
		}
		if matches!(self.spare, SpareSegment::Disabled) {
			self.spare = SpareSegment::Absent;
		}
		self.spare_tasks = tasks;
		self.start_spare_fill();
	}

	/// Prepares a spare in this call when preallocation is enabled and none exists. The
	/// write queue waits for the whole fill, so owners with a task scope rely on the
	/// background fill instead.
	pub async fn prepare_spare_segment(&mut self) -> Result<()> {
		if !matches!(self.spare, SpareSegment::Absent) {
			return Ok(());
		}
		let bytes = fill_spare(self.fs.clone(), self.root.clone(), self.segment_bytes).await?;
		self.spare = SpareSegment::Ready { bytes };
		Ok(())
	}

	/// Bytes the spare occupies or may occupy once its fill completes.
	pub(crate) fn spare_bytes(&self) -> u64 {
		match &self.spare {
			SpareSegment::Disabled | SpareSegment::Absent => 0,
			SpareSegment::Filling { .. } => self.segment_bytes,
			SpareSegment::Ready { bytes } => *bytes,
		}
	}

	/// Starts a background fill if a spare is wanted and the owner's task scope is alive.
	///
	/// The fill outlives this executor only until the scope is cancelled, which the runner does
	/// when its shard stops. A new shard in the same process could recover before an old fill
	/// notices that cancellation and rename `seg-spare.tmp` to `seg-spare`, possibly the new
	/// fill's file. Both fills write only zeros, and an executor uses a spare only after its
	/// own fill reported a completed rename, so any interleaving leaves a zero-filled spare
	/// or a failed fill that disables spares, never a file holding batches.
	pub(crate) fn start_spare_fill(&mut self) {
		if !matches!(self.spare, SpareSegment::Absent) {
			return;
		}
		let Some(tasks) = self.spare_tasks.as_ref().and_then(Weak::upgrade) else {
			return;
		};
		let (sender, done) = oneshot::channel();
		let fs = self.fs.clone();
		let root = self.root.clone();
		let bytes = self.segment_bytes;
		tasks.spawn(async move {
			let _ = sender.send(fill_spare(fs, root, bytes).await);
		});
		self.spare = SpareSegment::Filling { done };
	}

	/// Takes the ready spare's size for a roll, leaving no spare behind. A fill still
	/// running is left to finish for a later roll. A failed fill disables spares, so a disk
	/// that cannot hold one does not rewrite a spare after every write.
	pub(crate) fn take_ready_spare(&mut self) -> Option<u64> {
		if let SpareSegment::Filling { done } = &mut self.spare {
			match done.try_recv() {
				Ok(Ok(bytes)) => self.spare = SpareSegment::Ready { bytes },
				Ok(Err(error)) => {
					tracing::warn!(
						?error,
						"preparing a spare log segment failed, likely because the disk is full or failing; new segments are no longer preallocated until the shard restarts"
					);
					self.spare = SpareSegment::Disabled;
				}
				Err(oneshot::error::TryRecvError::Empty) => {}
				// The shard's task scope was cancelled, so the shard is stopping.
				Err(oneshot::error::TryRecvError::Closed) => self.spare = SpareSegment::Absent,
			}
		}
		match self.spare {
			SpareSegment::Ready { bytes } => {
				self.spare = SpareSegment::Absent;
				Some(bytes)
			}
			SpareSegment::Disabled | SpareSegment::Absent | SpareSegment::Filling { .. } => None,
		}
	}
}

/// Writes `bytes` zeros and an empty header to the temporary spare, syncs them, then renames the file to
/// `seg-spare` and syncs the directory. A failure removes the temporary file when it can;
/// recovery removes anything left behind.
async fn fill_spare(fs: Fs, root: PathBuf, bytes: u64) -> Result<u64> {
	let temp = root.join(LogShardKey::new(LogShardFile::SpareSegmentTemp).pack());
	let spare = root.join(LogShardKey::new(LogShardFile::SpareSegment).pack());
	let result = write_spare(&fs, &temp, &spare, bytes).await;
	if result.is_err() {
		let _ = fs.remove_file(&temp).await;
	}
	result.map(|()| bytes)
}

async fn write_spare(fs: &Fs, temp: &Path, spare: &Path, bytes: u64) -> Result<()> {
	let mut file = fs
		.open_options()
		.write(true)
		.create(true)
		.truncate(true)
		.open(temp)
		.await?;
	let zeros = vec![0; ZERO_CHUNK_BYTES];
	let mut written = 0;
	while written < bytes {
		let chunk = (bytes - written).min(ZERO_CHUNK_BYTES as u64) as usize;
		file.write_all(&zeros[..chunk]).await?;
		written += chunk as u64;
	}
	let header = SegmentHeader::empty().encode();
	for slot in 0..SEGMENT_HEADER_BYTES / SEGMENT_HEADER_SLOT_BYTES {
		file.seek(std::io::SeekFrom::Start(slot * SEGMENT_HEADER_SLOT_BYTES))
			.await?;
		file.write_all(&header).await?;
	}
	file.flush().await?;
	file.sync_all().await?;
	drop(file);
	crate::metrics::DISK_WRITTEN_BYTES
		.with_label_values(&["spare"])
		.inc_by(bytes);
	crate::metrics::DISK_SYNCS
		.with_label_values(&["spare"])
		.inc();

	fs.rename(temp, spare).await?;
	let dir = spare.parent().context("spare segment has no directory")?;
	fs.open(dir).await?.sync_all().await?;
	crate::metrics::DISK_SYNCS.with_label_values(&["dir"]).inc();
	Ok(())
}
