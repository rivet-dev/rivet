//! Serial filesystem executor. The owner keeps one instance per shard, awaits an
//! operation in its I/O task and delivers its outcome to the state machine. It
//! must discard the instance after any failed or cancelled operation. Construction
//! starts no tasks; filesystem capabilities are injected by the owner from Env.

mod checkpoint;
mod recovery;
mod spare;

pub use checkpoint::CheckpointCompaction;

use super::{
	event::LogShardEvent,
	types::{DiskReadError, WriteToDiskError, WrittenToDisk},
};
use anyhow::{Context, Result, ensure};
use depot_types::{
	VolumeLogKey,
	keys::fs::{LogShardFile, LogShardKey},
	log_shard::{
		BATCH_FRAMING_BYTES, FrameChecksum, SEGMENT_HEADER_BYTES, SegmentHeader,
		encode_batch_with_frames_into, encode_format, framed_batch_bytes,
	},
};
use rivet_data::generated::{
	depot_log_shard_v1::{DiskBatch, Frame, VolumeLogEntry, VolumeLogEntryBody},
	shared_depot_types_v1::VersionMark,
};
use rivet_env::fs::Fs;
use rivet_keys::keys::codec::KeyAddress;
use rivet_topology::worker::WorkerId;
use std::{
	collections::BTreeMap,
	path::{Path, PathBuf},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use vbare::{Encode, Message};

/// Where an entry's frame lies on disk, with the checksum a read verifies it against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
	/// The atomic batch that holds the frame.
	pub log_seq: u64,
	pub segment: u64,
	/// Byte offset of the frame itself within the segment file.
	pub offset: u64,
	pub len: u32,
	/// CRC32C of the frame's bytes. It is taken from the bytes the shard wrote, or at
	/// recovery from bytes whose batch checksum was just verified, never from an unverified
	/// read, so a read that matches it returns bytes as they were written.
	pub crc32c: u32,
}

impl Location {
	fn end(&self) -> u64 {
		self.offset + u64::from(self.len)
	}
}

#[derive(Clone, Debug)]
pub(crate) struct Segment {
	pub first: u64,
	/// The logical length: the end of the last batch. Bytes past it are zeros.
	pub bytes: u64,
	/// The file's length. It exceeds `bytes` only for a preallocated segment, whose tail
	/// stays zero-filled until batches overwrite it.
	pub allocated: u64,
	pub last_log_seq: u64,
	/// The newest header slot of a preallocated segment, or `None` for a plain segment,
	/// whose first batch starts at offset zero.
	pub header: Option<SegmentHeader>,
}

/// The writable segment handle; absent before the first batch or after recovery. Its
/// cursor is always at `len_bytes`, where the next batch is written.
pub struct ActiveSegment {
	pub first_log_seq: u64,
	pub file: rivet_env::fs::File,
	pub len_bytes: u64,
	/// The file's length. A write ending within it changes no file size, so syncing its
	/// data alone makes it durable.
	pub allocated_bytes: u64,
	/// The header slot the last write recorded, for a preallocated segment. Each write
	/// records the next one in the same sync as its batches.
	pub header: Option<SegmentHeader>,
}

pub struct LogShardExecutor {
	pub(crate) disk_incarnation: uuid::Uuid,
	pub(crate) active_segment: Option<ActiveSegment>,
	pub(crate) fs: Fs,
	pub(crate) root: PathBuf,
	/// The largest batch payload this process writes: the live configuration.
	pub(crate) max_batch_bytes: usize,
	/// The largest batch payload a stored batch may have: the limit FORMAT records, the
	/// largest any writer of this folder used. Reads and recovery validate stored batches
	/// against it, so lowering the configuration never turns valid batches into corruption.
	pub(crate) stored_max_batch_bytes: usize,
	/// The worker that owns this folder, which every FORMAT rewrite records.
	pub(crate) worker_id: WorkerId,
	/// The version marks FORMAT records, sorted by family. A batch holding a payload above
	/// its family's mark rewrites FORMAT first, so a binary that cannot read the payload
	/// refuses the folder instead of misreading it.
	pub(crate) version_marks: Vec<VersionMark>,
	/// The folder's recorded `depot.log_shard` version and its fingerprint. FORMAT rewrites
	/// keep them, so raising a mark never relabels records written under an older version.
	pub(crate) log_shard_format_version: u16,
	pub(crate) schema_fingerprint: [u8; 32],
	pub(crate) segment_bytes: u64,
	pub(crate) disk_segments: BTreeMap<u64, Segment>,
	/// The zero-filled file the next roll may rename into place.
	pub(crate) spare: spare::SpareSegment,
	/// Where background spare fills run; `None` prepares spares only on request.
	pub(crate) spare_tasks: Option<std::sync::Weak<rivet_env::TaskScope>>,

	/// Byte positions only; replica protocol state belongs to the state machine.
	pub(crate) entry_locations: BTreeMap<VolumeLogKey, BTreeMap<u64, Location>>,
	pub(crate) last_log_seq: u64,
	/// The boundary of the installed checkpoint chain: the base plus every delta.
	pub(crate) checkpoint_seq: u64,
	/// Encoded bytes of the base checkpoint file, or zero when there is none.
	pub(crate) checkpoint_base_bytes: u64,
	/// Encoded bytes of each delta checkpoint file above the base, by boundary.
	pub(crate) checkpoint_deltas: BTreeMap<u64, u64>,
	pub(crate) checkpoint_compaction: CheckpointCompaction,
	pub(crate) quarantined_bytes: u64,
	/// Encoded bytes of the newest payloads each recovered replica keeps in memory. Older
	/// recovered payloads stay only on disk, so a restart holds no more than steady state.
	pub(crate) recovered_payload_bytes_per_volume: u64,
	// Set before starting I/O; clearing happens only on full success. Cancellation
	// therefore poisons this executor just like an uncertain I/O outcome.
	pub(crate) poisoned: bool,
}

impl LogShardExecutor {
	pub const DEFAULT_SEGMENT_BYTES: u64 = 64 * 1024 * 1024;

	/// Creates an empty shard only. Existing folders must use the recovery path.
	/// Syncing every ancestor created here writes the folder entries to disk too.
	#[tracing::instrument(skip_all)]
	pub async fn create(
		fs: Fs,
		root: PathBuf,
		worker: &WorkerId,
		disk_incarnation: uuid::Uuid,
		max_batch_bytes: usize,
		segment_bytes: u64,
	) -> Result<Self> {
		// Refuse existing shard data so initialization cannot overwrite a recoverable log.
		ensure!(segment_bytes > 0, "zero segment limit");
		fs.read_dir(&root)
			.await
			.context("configured root must exist")?;
		fs.create_dir_all(root.join("shards/0")).await?;
		let mut dir = fs.read_dir(root.join("shards/0")).await?;
		while let Some(entry) = dir.next_entry().await? {
			ensure!(
				entry.path() == root.join(LogShardKey::new(LogShardFile::FormatTemp).pack()),
				"existing shard requires recovery"
			);
		}

		let storage = Self {
			disk_incarnation,
			active_segment: None,
			fs,
			root,
			max_batch_bytes,
			stored_max_batch_bytes: max_batch_bytes,
			worker_id: worker.clone(),
			version_marks: depot_types::log_shard::initial_version_marks(),
			log_shard_format_version: rivet_data::DEPOT_LOG_SHARD_VERSION,
			schema_fingerprint: depot_types::log_shard::log_shard_fingerprint(),
			segment_bytes,
			disk_segments: BTreeMap::new(),
			spare: spare::SpareSegment::Disabled,
			spare_tasks: None,
			entry_locations: BTreeMap::new(),
			recovered_payload_bytes_per_volume: u64::MAX,
			last_log_seq: 0,
			checkpoint_seq: 0,
			checkpoint_base_bytes: 0,
			checkpoint_deltas: BTreeMap::new(),
			checkpoint_compaction: Default::default(),
			quarantined_bytes: 0,
			poisoned: false,
		};

		// Install the worker's format marker before the shard can accept any batches.
		let recorded_max_batch_bytes =
			u32::try_from(max_batch_bytes).context("max batch bytes exceed u32")?;
		storage
			.atomic_write(
				LogShardFile::FormatTemp,
				LogShardFile::Format,
				&encode_format(
					worker,
					disk_incarnation,
					recorded_max_batch_bytes,
					&storage.version_marks,
				),
			)
			.await?;

		// Bootstrap passes an existing configured root; sync its newly-created descendants.
		storage.sync_dir(&storage.root.join("shards")).await?;
		storage.sync_dir(&storage.root).await?;

		tracing::info!("created log shard");
		Ok(storage)
	}

	pub(crate) fn path(&self, file: LogShardFile) -> PathBuf {
		LogShardKey::new(file).pack_under(&self.root)
	}

	/// Writes directory entry changes to disk independently of the files' contents.
	pub(crate) async fn sync_dir(&self, dir: &Path) -> Result<()> {
		self.fs.open(dir).await?.sync_all().await?;
		crate::metrics::DISK_SYNCS.with_label_values(&["dir"]).inc();
		Ok(())
	}

	/// Writes file creation, replacement, or removal to disk in the shard directory.
	pub(crate) async fn sync_shard_dir(&self) -> Result<()> {
		self.sync_dir(&self.root.join("shards/0")).await
	}

	/// Replaces a shard control file only after its complete contents are synced, then
	/// syncs the rename so recovery sees either the old or the new file.
	pub(crate) async fn atomic_write(
		&self,
		temp: LogShardFile,
		target: LogShardFile,
		bytes: &[u8],
	) -> Result<()> {
		let mut file = self
			.fs
			.open_options()
			.write(true)
			.create(true)
			.truncate(true)
			.open(self.path(temp.clone()))
			.await?;
		file.write_all(bytes).await?;
		file.flush().await?;
		file.sync_all().await?;
		crate::metrics::DISK_WRITTEN_BYTES
			.with_label_values(&["control"])
			.inc_by(bytes.len() as u64);
		crate::metrics::DISK_SYNCS
			.with_label_values(&["control"])
			.inc();

		self.fs.rename(self.path(temp), self.path(target)).await?;
		self.sync_shard_dir().await
	}

	/// The marks FORMAT must record before `batches` are written, or `None` when every
	/// payload is at or below its family's mark. Only integer comparisons per entry; family 0
	/// payloads are untracked. The core refuses payloads this binary cannot read before they
	/// reach a batch, so a raised mark never makes FORMAT unopenable here.
	fn raised_version_marks(&self, batches: &[DiskBatch]) -> Option<Vec<VersionMark>> {
		let mut raised: Option<Vec<VersionMark>> = None;
		for batch in batches {
			for frame in &batch.frames {
				let Frame::EntryFrame(frame) = frame else {
					continue;
				};
				let VolumeLogEntryBody::PayloadWrite(write) = &frame.entry.body else {
					continue;
				};
				if write.payload_family == 0 {
					continue;
				}
				let marks = raised.as_ref().unwrap_or(&self.version_marks);
				let family = rivet_data::families::VersionFamily(write.payload_family);
				if rivet_data::families::mark_of(marks, family)
					.is_some_and(|mark| mark >= write.payload_version)
				{
					continue;
				}
				let marks = raised.get_or_insert_with(|| self.version_marks.clone());
				rivet_data::families::raise_mark(marks, family, write.payload_version);
			}
		}
		raised
	}

	/// Atomically rewrites FORMAT with `marks` and the recorded batch limit. Marks only
	/// rise, and the replacement is atomic, so a crash leaves either FORMAT valid.
	pub(crate) async fn record_version_marks(&mut self, marks: Vec<VersionMark>) -> Result<()> {
		let recorded_max_batch_bytes =
			u32::try_from(self.stored_max_batch_bytes).context("max batch bytes exceed u32")?;
		self.atomic_write(
			LogShardFile::FormatTemp,
			LogShardFile::Format,
			&depot_types::log_shard::encode_format_at(
				self.log_shard_format_version,
				&self.schema_fingerprint,
				&self.worker_id,
				self.disk_incarnation,
				recorded_max_batch_bytes,
				&marks,
			),
		)
		.await?;
		tracing::info!(?marks, "raised log shard version marks");
		self.version_marks = marks;
		Ok(())
	}

	/// Bytes the shard's segment files occupy, including preallocated zero tails and any
	/// spare, plus quarantined segments.
	pub fn disk_bytes(&self) -> u64 {
		let segments = self
			.disk_segments
			.values()
			.map(|s| s.bytes.max(s.allocated))
			.sum::<u64>();
		self.quarantined_bytes + segments + self.spare_bytes()
	}

	/// The outcome is suitable for direct delivery to the shard core. No client
	/// replies are emitted here, including after a successful but late completion.
	pub async fn write_to_disk_response(&mut self, batches: &[DiskBatch]) -> LogShardEvent {
		LogShardEvent::WriteToDiskResponse {
			last_log_seq: batches.last().map_or(0, |batch| batch.log_seq),
			result: self.write_to_disk(batches).await.map_err(|e| {
				e.downcast_ref::<WriteToDiskError>()
					.cloned()
					.unwrap_or_else(|| WriteToDiskError::Write(format!("{e:#}")))
			}),
		}
	}

	/// Appends all framed batches in one write and sync before advancing the disk index.
	/// Once I/O begins, cancellation or failure leaves this executor unusable until restart.
	#[tracing::instrument(level = "debug", skip_all, fields(batch_count = batches.len()))]
	pub async fn write_to_disk(&mut self, batches: &[DiskBatch]) -> Result<WrittenToDisk> {
		let written = self.write_to_disk_with_roll(batches, false).await?;
		// Replace a spare the write consumed. The fill runs beside later writes.
		self.start_spare_fill();
		Ok(written)
	}

	/// Recovery can force a new segment without modifying the configured limit. A forced
	/// roll writes repair marks into a plain new file and leaves the segments it follows
	/// untouched, so it neither consumes a spare nor truncates the previous segment.
	pub(crate) async fn write_to_disk_with_roll(
		&mut self,
		batches: &[DiskBatch],
		force_roll: bool,
	) -> Result<WrittenToDisk> {
		ensure!(!self.poisoned, "shard requires restart");
		let first_batch = batches.first().context("empty disk write")?;
		let last_batch = batches.last().context("empty disk write")?;
		ensure!(
			first_batch.log_seq > self.last_log_seq,
			"non-increasing log sequence"
		);
		ensure!(
			batches
				.windows(2)
				.all(|pair| pair[0].log_seq.checked_add(1) == Some(pair[1].log_seq)),
			"non-consecutive disk batches"
		);

		// Raise FORMAT's marks before any byte of a higher-version payload reaches a segment,
		// so a crash between the two leaves a raised mark over older data, which still opens.
		let raised_marks = self.raised_version_marks(batches);

		// Frame each batch independently into one buffer sized for the whole append, so every
		// frame is encoded once and never copied.
		let total_bytes = batches.iter().map(framed_batch_bytes).sum();
		let mut bytes = Vec::with_capacity(total_bytes);
		let mut framed = Vec::with_capacity(batches.len());
		for batch in batches {
			let relative_offset = bytes.len() as u64;
			let frames = encode_batch_with_frames_into(batch, self.max_batch_bytes, &mut bytes)?;
			framed.push((relative_offset, frames));
		}

		let previous = self.disk_segments.last_key_value().map(|(_, s)| s.clone());
		let active = previous.as_ref();
		let roll = force_roll
			|| active
				.as_ref()
				.is_none_or(|s| s.bytes.saturating_add(bytes.len() as u64) > self.segment_bytes);
		let first = if roll {
			first_batch.log_seq
		} else {
			active.context("missing active segment")?.first
		};

		// Any interruption from here leaves the write outcome uncertain.
		self.poisoned = true;
		if let Some(marks) = raised_marks {
			self.record_version_marks(marks)
				.await
				.map_err(|e| WriteToDiskError::Write(format!("{e:#}")))?;
		}
		if roll {
			self.active_segment = None;
			if !force_roll && let Some(previous) = &previous {
				self.seal_segment(previous)
					.await
					.map_err(|e| WriteToDiskError::SegmentRoll(format!("{e:#}")))?;
			}
		}

		if self.active_segment.is_none() {
			let opened = if roll {
				self.create_segment(first, !force_roll).await
			} else {
				let previous = previous.as_ref().context("missing active segment")?;
				self.reopen_segment(previous).await
			};
			let segment = opened.map_err(|e| WriteToDiskError::SegmentRoll(format!("{e:#}")))?;
			self.active_segment = Some(segment);
		}

		// Write the complete frame before syncing its bytes and any new directory entry. The
		// batches start at the segment's logical end, after the header of a preallocated one.
		let active = self
			.active_segment
			.as_mut()
			.context("missing active segment")?;
		let offset = active.len_bytes;
		let end = offset + bytes.len() as u64;
		let write_started = crate::metrics::phase_timer();
		active
			.file
			.write_all(&bytes)
			.await
			.map_err(|e| WriteToDiskError::Write(e.to_string()))?;

		// A preallocated segment also records, in the same sync, that everything before
		// `offset` was already synced, so recovery can tell batches the disk lost from free
		// space. The cursor returns to the logical end for the next write.
		let header = active.header.map(|header| header.next(offset, end));
		if let Some(header) = &header {
			let slot = header.encode();
			active
				.file
				.seek(std::io::SeekFrom::Start(header.slot_offset()))
				.await
				.map_err(|e| WriteToDiskError::Write(e.to_string()))?;
			active
				.file
				.write_all(&slot)
				.await
				.map_err(|e| WriteToDiskError::Write(e.to_string()))?;
			active
				.file
				.seek(std::io::SeekFrom::Start(end))
				.await
				.map_err(|e| WriteToDiskError::Write(e.to_string()))?;
			crate::metrics::DISK_WRITTEN_BYTES
				.with_label_values(&["segment_header"])
				.inc_by(slot.len() as u64);
		}
		crate::metrics::observe_disk_write_phase(
			"write",
			crate::metrics::elapsed_seconds(write_started),
		);

		// Tokio writes may still be pending on a blocking task; both syncs wait for them.
		// A write inside the preallocated length changes no metadata a read needs, so
		// syncing its data is enough. A write past it also syncs the file's new length.
		let sync_started = crate::metrics::phase_timer();
		// A sync does not report an error from a write still in flight on Tokio's blocking
		// task; flushing first does, so a failed write is never acknowledged.
		active
			.file
			.flush()
			.await
			.map_err(|e| WriteToDiskError::Write(e.to_string()))?;
		let synced = if end <= active.allocated_bytes {
			active.file.sync_data().await
		} else {
			active.file.sync_all().await
		};
		synced.map_err(|e| WriteToDiskError::Sync(e.to_string()))?;
		crate::metrics::observe_disk_write_phase(
			"sync",
			crate::metrics::elapsed_seconds(sync_started),
		);
		crate::metrics::DISK_WRITTEN_BYTES
			.with_label_values(&["segment"])
			.inc_by(bytes.len() as u64);
		crate::metrics::DISK_SYNCS
			.with_label_values(&["segment"])
			.inc();
		crate::metrics::DISK_BATCHES.inc_by(batches.len() as u64);
		crate::metrics::record_disk_frames(batches);
		active.len_bytes = end;
		active.allocated_bytes = active.allocated_bytes.max(end);
		active.header = header;
		let allocated = active.allocated_bytes;
		if roll {
			self.sync_shard_dir()
				.await
				.map_err(|e| WriteToDiskError::Sync(e.to_string()))?;
			tracing::debug!(first, "rolled log segment");
		}

		// Publish locations and the disk sequence only after both content and name are synced.
		self.disk_segments.insert(
			first,
			Segment {
				first,
				bytes: end,
				allocated,
				last_log_seq: last_batch.log_seq,
				header,
			},
		);

		for (batch, (relative_offset, frames)) in batches.iter().zip(framed) {
			apply_locations(
				&mut self.entry_locations,
				batch,
				&frames,
				first,
				offset + relative_offset,
			)?;
		}
		self.last_log_seq = last_batch.log_seq;

		self.poisoned = false;
		Ok(WrittenToDisk {
			segment_first_log_seq: first,
			segment_len_bytes: end,
		})
	}

	/// Truncates a segment the roll leaves behind to its logical length, freeing its unused
	/// preallocation. The truncation is not synced: it removes only zeros past the last
	/// batch, and recovery truncates a non-final segment's zero tail again after a crash.
	async fn seal_segment(&mut self, segment: &Segment) -> Result<()> {
		if segment.allocated <= segment.bytes {
			return Ok(());
		}
		let file = self
			.fs
			.open_options()
			.write(true)
			.open(self.path(LogShardFile::Segment {
				first_log_seq: segment.first,
			}))
			.await?;
		file.set_len(segment.bytes).await?;
		if let Some(sealed) = self.disk_segments.get_mut(&segment.first) {
			sealed.allocated = segment.bytes;
		}
		Ok(())
	}

	/// Creates the file for a new segment, renaming a ready spare into place when allowed
	/// and one exists. A spare already holds an empty header, so its batches start after
	/// it. The caller syncs the directory entry after the first write. Like `create_new`,
	/// this refuses to replace an existing segment file.
	async fn create_segment(&mut self, first: u64, use_spare: bool) -> Result<ActiveSegment> {
		let path = self.path(LogShardFile::Segment {
			first_log_seq: first,
		});
		let spare = if use_spare {
			self.take_ready_spare()
		} else {
			None
		};
		let Some(allocated_bytes) = spare else {
			let file = self
				.fs
				.open_options()
				.write(true)
				.create_new(true)
				.open(&path)
				.await?;
			return Ok(ActiveSegment {
				first_log_seq: first,
				file,
				len_bytes: 0,
				allocated_bytes: 0,
				header: None,
			});
		};

		match self.fs.open(&path).await {
			Ok(_) => anyhow::bail!("segment {first} already exists"),
			Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
			Err(e) => return Err(e.into()),
		}
		self.fs
			.rename(self.path(LogShardFile::SpareSegment), &path)
			.await?;
		let mut file = self.fs.open_options().write(true).open(&path).await?;
		file.seek(std::io::SeekFrom::Start(SEGMENT_HEADER_BYTES))
			.await?;
		Ok(ActiveSegment {
			first_log_seq: first,
			file,
			len_bytes: SEGMENT_HEADER_BYTES,
			allocated_bytes,
			header: Some(SegmentHeader::empty()),
		})
	}

	/// Opens the newest segment after recovery, positioned at its logical end, which in a
	/// preallocated segment lies before the file's end.
	async fn reopen_segment(&self, segment: &Segment) -> Result<ActiveSegment> {
		let mut file = self
			.fs
			.open_options()
			.write(true)
			.open(self.path(LogShardFile::Segment {
				first_log_seq: segment.first,
			}))
			.await?;
		file.seek(std::io::SeekFrom::Start(segment.bytes)).await?;
		Ok(ActiveSegment {
			first_log_seq: segment.first,
			file,
			len_bytes: segment.bytes,
			allocated_bytes: segment.allocated,
			header: segment.header,
		})
	}

	/// Reads one captured disk image. The runner orders this operation before any
	/// later mutation, so the index and returned state describe the same suffix.
	/// Corrupt bytes poison the executor, so the next write fails the shard.
	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn read_entries(
		&mut self,
		key: VolumeLogKey,
		state: rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState,
		first: u64,
		max_bytes: u32,
	) -> Result<rivet_data::generated::depot_storage_worker_protocol_v1::StorageWorkerResponse> {
		let plan = self.plan_read(&key, &state, first)?;
		match plan.read(key, state, max_bytes).await {
			Ok(response) => Ok(response),
			Err(DiskReadError::Unavailable(error)) => anyhow::bail!(error),
			Err(DiskReadError::Corrupt(error)) => {
				self.poisoned = true;
				anyhow::bail!(error)
			}
		}
	}

	/// Resolves where the entries of a read from `first` live, from the index as it stands
	/// now. Synced batches are never moved or rewritten, so the plan stays valid while later
	/// writes append, and a disk reader can run it beside the write queue. At most
	/// `MAX_READ_PLAN_ENTRIES` entries are planned; the read then returns a short page,
	/// which callers already handle.
	pub fn plan_read(
		&self,
		key: &VolumeLogKey,
		state: &rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState,
		first: u64,
	) -> Result<ReadPlan> {
		ensure!(!self.poisoned, "shard requires restart");
		let indexed = self.entry_locations.get(key);
		let mut entries = Vec::new();
		let mut missing_from = None;
		for seq in first..=state.appended.volume_seq {
			if entries.len() >= MAX_READ_PLAN_ENTRIES {
				break;
			}
			match indexed.and_then(|entries| entries.get(&seq)) {
				Some(location) => entries.push((seq, location.clone())),
				None => {
					missing_from = Some(seq);
					break;
				}
			}
		}
		Ok(ReadPlan {
			fs: self.fs.clone(),
			root: self.root.clone(),
			max_batch_bytes: self.stored_max_batch_bytes,
			entries,
			missing_from,
		})
	}

	/// Reads one indexed disk entry, verifying only its own frame. A checksum or index
	/// failure poisons the shard; only restart can repair it.
	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn read_entry(
		&mut self,
		volume_log_key: &VolumeLogKey,
		seq: u64,
	) -> Result<Option<VolumeLogEntry>> {
		// A missing index entry is an ordinary miss and requires no disk access.
		ensure!(!self.poisoned, "shard requires restart");
		let Some(location) = self
			.entry_locations
			.get(volume_log_key)
			.and_then(|entries| entries.get(&seq))
			.cloned()
		else {
			return Ok(None);
		};

		let run = self.read_run_at(&location, &location).await?;
		self.poisoned = true;
		let entry = run
			.entry(&location, volume_log_key, seq)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		self.poisoned = false;
		Ok(Some(entry))
	}

	/// Reads the bytes from `first`'s frame through `last`'s, which share a segment. Any
	/// read failure leaves the executor poisoned.
	pub(crate) async fn read_run_at(&mut self, first: &Location, last: &Location) -> Result<Run> {
		ensure!(!self.poisoned, "shard requires restart");
		self.poisoned = true;
		let mut reader = FrameReader::new(&self.fs, &self.root);
		let run = reader
			.read_run(first, last)
			.await
			.map_err(|error| match error {
				DiskReadError::Unavailable(error) | DiskReadError::Corrupt(error) => {
					anyhow::anyhow!(error)
				}
			})?;
		self.poisoned = false;
		Ok(run)
	}
}

/// Most entries one disk read resolves. It bounds the plan a queued read holds; a
/// longer range is returned as a short page.
pub const MAX_READ_PLAN_ENTRIES: usize = 2048;

/// The indexed locations of one read's entries, resolved by the write queue in effect order
/// and executed by a disk reader without the executor.
#[derive(Clone)]
pub struct ReadPlan {
	fs: Fs,
	root: PathBuf,
	/// The folder's recorded batch limit, which bounds both one stored batch and the scan.
	max_batch_bytes: usize,
	/// The planned entries in sequence order with their locations.
	entries: Vec<(u64, Location)>,
	/// The first sequence the index did not hold, if planning stopped there. Reaching it
	/// means the index and the captured state disagree.
	missing_from: Option<u64>,
}

impl ReadPlan {
	/// Reads and verifies the planned batches, each once, and returns the page under the
	/// same scan and reply limits as the serial path. Only reads segment bytes, so it is
	/// safe beside writes that append to the active segment through another descriptor.
	pub async fn read(
		self,
		key: VolumeLogKey,
		state: rivet_data::generated::depot_log_shard_v1::VolumeLogReplicaState,
		max_bytes: u32,
	) -> Result<
		rivet_data::generated::depot_storage_worker_protocol_v1::StorageWorkerResponse,
		DiskReadError,
	> {
		use rivet_data::generated::depot_storage_worker_protocol_v1::{
			ReadVolumeLogEntriesResponse, RejectResponse, RequestTooLarge, StorageWorkerResponse,
		};
		let mut entries = Vec::new();
		let mut reader = FrameReader::new(&self.fs, &self.root);
		let mut scanned_bytes = 0usize;
		let mut bytes = 0;
		let mut page_full = false;
		let mut next = 0;
		'runs: while next < self.entries.len() {
			// Bound physical scan bytes, including the other volumes' frames a run reads
			// between this volume's. One final run can cross the threshold, keeping the scan
			// below two maximum batches.
			if scanned_bytes >= self.max_batch_bytes + BATCH_FRAMING_BYTES {
				page_full = true;
				break;
			}
			let remaining_reply = u64::from(max_bytes).saturating_sub(bytes);
			let end = run_end(
				&self.entries,
				next,
				READ_COALESCE_GAP_BYTES,
				self.max_batch_bytes as u64,
				remaining_reply,
			);
			let run = reader
				.read_run(&self.entries[next].1, &self.entries[end - 1].1)
				.await?;
			scanned_bytes = scanned_bytes.saturating_add(run.bytes.len());

			for (seq, location) in &self.entries[next..end] {
				let entry = run.entry(location, &key, *seq)?;
				let length = entry.encoded_len() as u64;
				if bytes + length > u64::from(max_bytes) {
					if entries.is_empty() {
						return Ok(StorageWorkerResponse::RejectResponse(
							RejectResponse::RequestTooLarge(RequestTooLarge { max_bytes }),
						));
					}
					page_full = true;
					break 'runs;
				}
				bytes += length;
				entries.push(entry);
			}
			next = end;
		}
		if !page_full && self.missing_from.is_some() {
			return Err(DiskReadError::Corrupt("disk replica entry missing".into()));
		}
		crate::metrics::READ_BYTES
			.with_label_values(&["scanned"])
			.inc_by(scanned_bytes as u64);
		crate::metrics::READ_BYTES
			.with_label_values(&["returned"])
			.inc_by(bytes);
		Ok(StorageWorkerResponse::ReadVolumeLogEntriesResponse(
			ReadVolumeLogEntriesResponse { state, entries },
		))
	}
}

/// Largest gap of other frames a disk read spans to reach the next planned frame in the same
/// read. A wider gap starts a new read, so a read of one volume's entries never scans much
/// more than those entries, however many volumes share their batches.
pub const READ_COALESCE_GAP_BYTES: u64 = 4 * 1024;

/// Returns the end of the run of `entries` from `start` that one disk read covers: frames in
/// one segment, in file order, separated by at most `max_gap` bytes, spanning at most
/// `max_span` bytes, and whose frames fit `frame_budget`. A run always holds at least one
/// entry.
pub(crate) fn run_end<T>(
	entries: &[(T, Location)],
	start: usize,
	max_gap: u64,
	max_span: u64,
	frame_budget: u64,
) -> usize {
	let first = &entries[start].1;
	let mut frame_bytes = u64::from(first.len);
	let mut end = start + 1;
	while let Some((_, next)) = entries.get(end) {
		let previous = &entries[end - 1].1;
		let joins = next.segment == first.segment
			&& next.offset >= previous.end()
			&& next.offset - previous.end() <= max_gap
			&& next.end() - first.offset <= max_span
			&& frame_bytes + u64::from(next.len) <= frame_budget;
		if !joins {
			break;
		}
		frame_bytes += u64::from(next.len);
		end += 1;
	}
	end
}

/// Reads frame runs through one descriptor per segment, reused while a read stays in it.
pub(crate) struct FrameReader<'a> {
	fs: &'a Fs,
	root: &'a Path,
	open: Option<(u64, rivet_env::fs::File)>,
}

impl<'a> FrameReader<'a> {
	pub(crate) fn new(fs: &'a Fs, root: &'a Path) -> Self {
		Self {
			fs,
			root,
			open: None,
		}
	}

	/// Reads the bytes from `first`'s frame through the end of `last`'s. File access
	/// failures are `Unavailable`; a file shorter than the index claims is `Corrupt`.
	pub(crate) async fn read_run(
		&mut self,
		first: &Location,
		last: &Location,
	) -> std::result::Result<Run, DiskReadError> {
		let unavailable = |error: std::io::Error| DiskReadError::Unavailable(error.to_string());
		if first.segment != last.segment || last.end() < first.offset {
			return Err(DiskReadError::Corrupt("invalid planned read run".into()));
		}
		if self
			.open
			.as_ref()
			.is_none_or(|(segment, _)| *segment != first.segment)
		{
			let path = LogShardKey::new(LogShardFile::Segment {
				first_log_seq: first.segment,
			})
			.pack_under(self.root);
			let file = self.fs.open(path).await.map_err(unavailable)?;
			self.open = Some((first.segment, file));
		}
		let Some((_, file)) = self.open.as_mut() else {
			return Err(DiskReadError::Unavailable("segment not open".into()));
		};
		file.seek(std::io::SeekFrom::Start(first.offset))
			.await
			.map_err(unavailable)?;
		let mut bytes = vec![0; (last.end() - first.offset) as usize];
		file.read_exact(&mut bytes).await.map_err(|error| {
			// Indexed frames were synced before they were indexed, so a segment that ends
			// before them has lost data.
			if error.kind() == std::io::ErrorKind::UnexpectedEof {
				DiskReadError::Corrupt("segment ends before an indexed frame".into())
			} else {
				unavailable(error)
			}
		})?;
		Ok(Run {
			start: first.offset,
			bytes: vbare::Bytes::from(bytes),
		})
	}
}

/// Contiguous segment bytes holding one or more planned frames.
pub(crate) struct Run {
	start: u64,
	bytes: vbare::Bytes,
}

impl Run {
	/// Verifies one frame's checksum and then its index target before exposing the entry.
	/// The entry shares the run's buffer; a read reply holds it only until it is encoded.
	pub(crate) fn entry(
		&self,
		location: &Location,
		key: &VolumeLogKey,
		seq: u64,
	) -> std::result::Result<VolumeLogEntry, DiskReadError> {
		let corrupt = |reason: &str| DiskReadError::Corrupt(reason.into());
		let start = location
			.offset
			.checked_sub(self.start)
			.ok_or_else(|| corrupt("frame before its read run"))? as usize;
		let end = start + location.len as usize;
		if end > self.bytes.len() {
			return Err(corrupt("frame past its read run"));
		}
		let bytes = self.bytes.slice(start..end);
		if crc32c::crc32c(&bytes) != location.crc32c {
			return Err(corrupt("log entry checksum mismatch"));
		}

		let frame = Frame::decode(bytes)
			.map_err(|error| DiskReadError::Corrupt(format!("undecodable log entry: {error}")))?;
		let Frame::EntryFrame(frame) = frame else {
			return Err(corrupt("index points to a frame that is not an entry"));
		};
		if frame.volume_log_key != key.0 {
			return Err(corrupt("index points to another log"));
		}
		if frame.entry.volume_seq != seq {
			return Err(corrupt("index points to another entry"));
		}
		Ok(frame.entry)
	}
}

/// Updates only byte locations, never the replica's protocol state or rules.
/// `frames` holds each frame's checksum, in frame order, and `offset` is where the batch's
/// header starts in the segment.
pub(crate) fn apply_locations(
	entry_locations: &mut BTreeMap<VolumeLogKey, BTreeMap<u64, Location>>,
	batch: &DiskBatch,
	frames: &[FrameChecksum],
	segment: u64,
	offset: u64,
) -> Result<()> {
	ensure!(
		batch.frames.len() == frames.len(),
		"frame checksums do not match the batch's frames"
	);
	for (frame, checksum) in batch.frames.iter().zip(frames) {
		let volume_log_key = match frame {
			Frame::EntryFrame(f) => VolumeLogKey::from(f.volume_log_key.clone()),
			Frame::StateFrame(f) => VolumeLogKey::from(f.volume_log_key.clone()),
			Frame::ForgetFrame(f) => {
				entry_locations.remove(&VolumeLogKey::from(f.volume_log_key.clone()));
				continue;
			}
			// Tablet grant frames have no volume entry locations.
			Frame::TabletGrantFrame(_) | Frame::TabletReplicaSetFrame(_) => continue,
		};

		let entries = entry_locations.entry(volume_log_key).or_default();
		match frame {
			// Grant and forget frames add no payload locations.
			Frame::TabletGrantFrame(_)
			| Frame::TabletReplicaSetFrame(_)
			| Frame::ForgetFrame(_) => {}
			Frame::StateFrame(f) => {
				// A deleted replica keeps its tail above cold until a trim releases it.
				retain_between(
					entries,
					f.state.cold.volume_seq,
					f.state.appended.volume_seq,
				);
			}
			Frame::EntryFrame(f) => {
				entries.insert(
					f.entry.volume_seq,
					Location {
						log_seq: batch.log_seq,
						segment,
						offset: offset + u64::from(checksum.offset),
						len: checksum.len,
						crc32c: checksum.crc32c,
					},
				);
			}
		}
	}
	Ok(())
}

/// Keeps the locations above `cold` and at or below `appended`. Most state frames move
/// neither end, so this costs the two end lookups rather than a walk over the volume's tail;
/// a trim or truncate pays only for the locations it drops.
fn retain_between(entries: &mut BTreeMap<u64, Location>, cold: u64, appended: u64) {
	if appended <= cold {
		entries.clear();
		return;
	}
	if entries
		.first_key_value()
		.is_some_and(|(seq, _)| *seq <= cold)
	{
		*entries = entries.split_off(&(cold + 1));
	}
	if entries
		.last_key_value()
		.is_some_and(|(seq, _)| *seq > appended)
	{
		entries.split_off(&(appended + 1));
	}
}


