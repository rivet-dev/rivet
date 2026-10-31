//! Stabilize host page-cache bytes before interpreting them. A damaged suffix is
//! cut only if no valid batch follows it. Interior corruption requires disk
//! repair marking before quarantine; nothing from a corrupt image is served.
//!
//! Only damage the checksum catches is corruption. A batch whose checksum matches holds
//! exactly what a writer produced, so if it fails to decode or to replay, another schema or a
//! bug wrote it. Every member of a tablet runs the same binary and meets the same bytes, so
//! repair marking would empty the same replicas on every member at once and lose committed
//! entries. Recovery instead fails before changing anything on disk, and FORMAT's schema
//! fingerprint refuses a folder written under another schema before any batch is read.
//!
//! A segment's data ends at its last nonzero byte; a preallocated segment is zero-filled past
//! it. Every intact batch ends in a nonzero trailer and its header carries its own checksum,
//! so an intact batch always ends within the data, and only a header cut short by the end of
//! the data, or a checksum-valid header whose frame runs past it, is an incomplete final write,
//! the torn tail. A complete header that fails its checksum, or a batch ending within the data
//! that fails its payload checksum or trailer, is corruption, in either kind of segment.
//!
//! A preallocated segment's batches start after its header, whose newest slot records how
//! far the batches reached when its last write started, all of it synced by earlier writes.
//! Batches ending before that length mean the disk returned synced pages as zeros or lost
//! them, so the shard is corrupt and goes through repair marking; nothing is cut. In the
//! newest segment, batches ending between that length and the last write's own end are an
//! interrupted final write, cut like any torn tail, and recovery then records the cut length
//! as both synced and written. A segment is only ever left behind by a roll after its last
//! write synced, so in any older segment batches ending before the last write's end are
//! lost too.
//!
//! What remains undetected: a disk that returns the end of the newest segment's synced data as
//! zeros looks like an interrupted write. In a preallocated segment that is at most its final
//! synced write, which only a second sync per write could close; a plain segment, written when
//! no spare was ready, has no synced length, so it may be several writes. A disk that returns
//! a whole segment, header included, as zeros looks like a spare renamed into place before its
//! first write.

use super::{
	Location, LogShardExecutor, Segment, apply_locations,
	checkpoint::{decode_checkpoint, merge_checkpoints},
};
use crate::log_shard_machine::state::{
	disk_segments::DiskSegmentUsage,
	loaded_volume_replica::{LoadedVolumeReplica, ReplicaEntry},
	memory_batches::replay_state,
	recovery::{
		CorruptionFound, CorruptionKind, RecoveredReplica, RecoveredShard, RecoveryError,
		RecoveryOutcome, RepairMark,
	},
};
use anyhow::{Context, Result, ensure};
use depot_types::{
	VolumeLogKey,
	keys::fs::LogShardFile,
	log_shard::{
		BATCH_FRAMING_BYTES, BatchError, FormatError, FrameHeader, SEGMENT_HEADER_BYTES,
		SegmentHeader, decode_batch_checked, decode_format, encode_batch, frame_checksums,
		read_frame_header,
	},
};
use rivet_data::generated::{
	depot_log_shard_v1::{DiskBatch, Frame, LogShardCheckpoint, StateFrame},
	shared_depot_types_v1::VolumeLogReplicaStatus,
};
use rivet_env::fs::Fs;
use rivet_keys::keys::codec::KeyAddress;
use rivet_keys::keys::codec::TextComponent;
use rivet_topology::worker::WorkerId;
use std::{
	collections::{BTreeMap, BTreeSet},
	io::ErrorKind,
	path::PathBuf,
};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

type ReplicaStates = BTreeMap<VolumeLogKey, StateFrame>;

impl LogShardExecutor {
	/// Stabilizes and scans the disk image, then returns either recovered replicas
	/// or a repair marking plan that must finish before the shard serves requests.
	/// The returned executor is poisoned until a corrupt outcome has been marked repairing.
	/// Unknown FORMAT versions and foreign worker identities always fail stop.
	#[tracing::instrument(skip_all)]
	pub async fn open(
		fs: Fs,
		root: PathBuf,
		worker: &WorkerId,
		max_batch_bytes: usize,
		segment_bytes: u64,
	) -> Result<(Self, RecoveryOutcome)> {
		Self::open_with_payload_retention(
			fs,
			root,
			worker,
			max_batch_bytes,
			segment_bytes,
			u64::MAX,
			&|| {},
			&rivet_data::families::compiled_versions(),
		)
		.await
	}

	/// Like `open`, keeping only the newest `payload_bytes_per_volume` encoded bytes of
	/// each recovered replica's payloads in memory, and refusing FORMAT marks above what
	/// `read_versions`, this worker's advertised versions, reads. Calls `progress` after each
	/// segment the scan syncs, reads or truncates and each segment whose live entries it reads
	/// back, so the caller can bound the time without progress instead of the whole scan, which
	/// grows with the retained log.
	#[tracing::instrument(skip_all)]
	pub async fn open_with_payload_retention(
		fs: Fs,
		root: PathBuf,
		worker: &WorkerId,
		max_batch_bytes: usize,
		segment_bytes: u64,
		payload_bytes_per_volume: u64,
		progress: &(dyn Fn() + Send + Sync),
		read_versions: &[rivet_data::generated::topology_worker_heartbeat_v1::FamilyVersions],
	) -> Result<(Self, RecoveryOutcome)> {
		ensure!(segment_bytes > 0, "zero segment limit");

		let mut store = Self {
			disk_incarnation: uuid::Uuid::nil(),
			active_segment: None,
			fs,
			root,
			max_batch_bytes,
			stored_max_batch_bytes: max_batch_bytes,
			worker_id: worker.clone(),
			version_marks: Vec::new(),
			log_shard_format_version: rivet_data::DEPOT_LOG_SHARD_VERSION,
			schema_fingerprint: depot_types::log_shard::log_shard_fingerprint(),
			segment_bytes,
			disk_segments: BTreeMap::new(),
			spare: super::spare::SpareSegment::Disabled,
			spare_tasks: None,
			entry_locations: BTreeMap::new(),
			recovered_payload_bytes_per_volume: payload_bytes_per_volume,
			last_log_seq: 0,
			poisoned: true,
			checkpoint_seq: 0,
			checkpoint_base_bytes: 0,
			checkpoint_deltas: BTreeMap::new(),
			checkpoint_compaction: Default::default(),
			quarantined_bytes: 0,
		};

		// Verify the format, record schema and worker ownership before interpreting any shard
		// data.
		let format = store.fs.read(store.path(LogShardFile::Format)).await?;
		let format =
			decode_format(&format, worker, read_versions).map_err(|error| match error {
				FormatError::ForeignWorker => RecoveryError::ForeignFolder,
				FormatError::UnsupportedVersion { found, supported } => {
					RecoveryError::UnsupportedFormatVersion { found, supported }
				}
				FormatError::Unfingerprinted
				| FormatError::Length { .. }
				| FormatError::SchemaMismatch { .. }
				| FormatError::MalformedMarks
				| FormatError::VersionAboveRead(_) => RecoveryError::IncompatibleFormat {
					reason: error.to_string(),
				},
			})?;
		store.disk_incarnation = format.disk_incarnation;
		store.version_marks = format.version_marks;
		store.log_shard_format_version = format.log_shard_format_version;
		store.schema_fingerprint = format.schema_fingerprint;

		// Stored batches keep the limit they were written under even when the configuration
		// is lowered later.
		let recorded_max_batch_bytes = format.max_batch_bytes as usize;
		store.stored_max_batch_bytes = recorded_max_batch_bytes.max(max_batch_bytes);
		let max_batch_bytes = store.stored_max_batch_bytes;

		// The configured root exists before shard creation. Stabilize all shard
		// directories, FORMAT, checkpoint and segments before recovery decisions.
		store.sync_dir(&store.root).await?;
		store.sync_dir(&store.root.join("shards")).await?;
		store.sync_shard_dir().await?;
		store
			.fs
			.open(store.path(LogShardFile::Format))
			.await?
			.sync_all()
			.await?;

		// Stabilize normal segments for replay and retain quarantined bytes in the disk budget.
		let mut entries = store.fs.read_dir(store.root.join("shards/0")).await?;
		let mut files = BTreeMap::new();
		let mut delta_files = BTreeMap::new();
		let mut removed_spares = false;

		while let Some(entry) = entries.next_entry().await? {
			let path = entry.path();
			let name = path.file_name().and_then(|s| s.to_str());
			let file = match name.and_then(|name| LogShardFile::from_text(name).ok()) {
				Some(file) => file,
				None => {
					tracing::warn!(file = ?path.file_name(), "ignoring unrelated shard file");
					continue;
				}
			};
			match file {
				LogShardFile::Segment { first_log_seq } => {
					store.fs.open(&path).await?.sync_all().await?;
					progress();
					files.insert(first_log_seq, path);
				}
				LogShardFile::Checkpoint => {
					store.fs.open(path).await?.sync_all().await?;
				}
				LogShardFile::CheckpointDelta {
					last_included_log_seq,
				} => {
					store.fs.open(&path).await?.sync_all().await?;
					delta_files.insert(last_included_log_seq, path);
				}
				LogShardFile::CorruptSegment { .. } => {
					let bytes = store
						.fs
						.open(path)
						.await?
						.seek(std::io::SeekFrom::End(0))
						.await?;
					store.quarantined_bytes = store
						.quarantined_bytes
						.checked_add(bytes)
						.context("quarantined segment byte count overflow")?;
				}
				// A spare never holds an acknowledged batch: a roll syncs the rename that
				// makes it a segment before replying. Delete it so fills start clean.
				LogShardFile::SpareSegment | LogShardFile::SpareSegmentTemp => {
					store.fs.remove_file(&path).await?;
					removed_spares = true;
				}
				LogShardFile::Format
				| LogShardFile::FormatTemp
				| LogShardFile::CheckpointTemp
				| LogShardFile::CheckpointDeltaTemp => {}
			}
		}
		if removed_spares {
			store.sync_shard_dir().await?;
		}

		// An absent checkpoint allows full replay; an unreadable one cannot restore the
		// tablet fences, so recovery fails without touching the disk. The chain is the base
		// plus its deltas, and an unreadable delta or a gap in the chain loses fences the same
		// way.
		let mut corrupt = None;
		let mut corruption_kind = None;
		let base = match store.fs.read(store.path(LogShardFile::Checkpoint)).await {
			Ok(bytes) => {
				store.checkpoint_base_bytes = bytes.len() as u64;
				decode_checkpoint(&bytes).map(Some)
			}
			Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
			Err(e) => return Err(e.into()),
		};
		let mut deltas = Vec::with_capacity(delta_files.len());
		for (last_included_log_seq, path) in &delta_files {
			let bytes = store.fs.read(path).await?;
			store
				.checkpoint_deltas
				.insert(*last_included_log_seq, bytes.len() as u64);
			deltas.push(bytes);
		}
		let checkpoint = base.and_then(|base| {
			let deltas = deltas
				.iter()
				.map(|bytes| decode_checkpoint(bytes))
				.collect::<Result<Vec<_>>>()?;
			merge_checkpoints(base, deltas)
		});
		let checkpoint = checkpoint.map_err(|error| RecoveryError::UnreadableCheckpoint {
			reason: format!("{error:#}"),
		})?;

		// Scan in file order to distinguish an incomplete tail from interior data loss.
		let boundary = checkpoint.as_ref().map_or(0, |c| c.last_included_log_seq);
		let mut states: ReplicaStates = checkpoint.as_ref().map_or_else(BTreeMap::new, |c| {
			c.replicas
				.iter()
				.map(|r| (VolumeLogKey::from(r.volume_log_key.clone()), r.clone()))
				.collect()
		});
		let mut tablet_grants = BTreeMap::new();
		let mut retired_volumes =
			crate::log_shard_machine::state::retired_volumes::RetiredVolumes::default();
		let mut tablet_replica_sets = BTreeMap::new();
		if let Some(checkpoint) = &checkpoint {
			for set in &checkpoint.tablet_replica_sets {
				crate::log_shard_machine::state::tablet_replica_sets::replay(
					&mut tablet_replica_sets,
					set,
				);
			}
			for grant in &checkpoint.tablet_grants {
				crate::log_shard_machine::state::tablet_grants::replay(&mut tablet_grants, grant);
			}
			for retired in &checkpoint.retired_volumes {
				let grant = &retired.retirement;
				let covered = grant.tablet_grant_epoch > 0
					&& tablet_grants
						.get(&grant.tablet_id)
						.is_some_and(|epoch| *epoch >= grant.tablet_grant_epoch);
				if !covered {
					return Err(RecoveryError::UnreadableCheckpoint {
						reason: "retirement exceeds checkpoint grant".to_string(),
					}
					.into());
				}
				crate::log_shard_machine::state::retired_volumes::restore(
					&mut retired_volumes,
					&tablet_grants,
					retired,
				);
			}
		}
		let mut grant_fences_complete = true;
		let mut undecodable_bytes = false;
		let mut crossed_checkpoint = false;
		let mut bytes_since_checkpoint = 0;
		let mut last = 0;
		let mut cut = None;
		let mut facts = states.clone();
		let mut saw_replica_frame = !facts.is_empty();
		let mut changed_since_checkpoint = BTreeSet::new();
		let mut data_ends = BTreeMap::new();
		let mut zero_tails = BTreeMap::new();

		let newest_file = files.last_key_value().map(|(first, _)| *first);
		for (&first, path) in &files {
			let raw = store.fs.read(path).await?;
			progress();
			store.last_log_seq = store.last_log_seq.max(first);
			// A preallocated segment's batches start after its header.
			let header = SegmentHeader::read(&raw);
			let data_start = header.map_or(0, |_| SEGMENT_HEADER_BYTES as usize);
			let mut offset = data_start;

			// Bytes past the last nonzero byte are a preallocated tail, never a batch. Every
			// intact batch ends in its nonzero trailer, so the scan ends at the first batch
			// boundary at or past this point.
			let data_end = raw
				.get(data_start..)
				.and_then(|data| data.iter().rposition(|byte| *byte != 0))
				.map_or(data_start, |last| data_start + last + 1);
			// A file with a header, or exactly one preallocation long, is treated as
			// preallocated: zeros past its data are free space, not a sign of lost writes.
			let preallocated = header.is_some() || raw.len() as u64 == segment_bytes;

			while offset < data_end {
				match decode_batch_checked(&raw[offset..], max_batch_bytes) {
					// The bytes are what a writer produced, so this binary or its schema
					// differs from the writer's. Stop before anything is cut or marked.
					Err(BatchError::Undecodable { reason }) => {
						return Err(RecoveryError::UndecodableBatch {
							segment_first_log_seq: first,
							offset: offset as u64,
							reason,
						}
						.into());
					}
					Ok((batch, len)) => {
						// A checkpoint covers earlier grant frames, but no later fence may
						// disappear. Missing sequences and undecodable intervals above it
						// cannot be repaired from replica states.
						if undecodable_bytes {
							grant_fences_complete &= batch.log_seq <= boundary.saturating_add(1);
							undecodable_bytes = false;
						}
						if batch.log_seq > boundary
							&& batch.log_seq > last.max(boundary).saturating_add(1)
						{
							grant_fences_complete = false;
							corrupt.get_or_insert(first);
							corruption_kind.get_or_insert(CorruptionKind::DamagedBatch {
								log_seq: last.max(boundary).saturating_add(1),
							});
						}
						if cut.is_some()
							|| (offset == data_start && batch.log_seq != first)
							|| batch.log_seq <= last
						{
							corruption_kind.get_or_insert({
								if offset == data_start && batch.log_seq != first {
									CorruptionKind::SegmentNameMismatch {
										file_log_seq: first,
										first_batch_log_seq: batch.log_seq,
									}
								} else {
									CorruptionKind::DamagedBatch {
										log_seq: batch.log_seq,
									}
								}
							});
							corrupt = Some(
								corrupt
									.unwrap_or(first)
									.min(cut.map_or(first, |(s, _)| s))
									.min(first),
							);
						}

						last = last.max(batch.log_seq);
						store.last_log_seq = store.last_log_seq.max(batch.log_seq);
						for frame in &batch.frames {
							if let Frame::TabletReplicaSetFrame(set) = frame {
								crate::log_shard_machine::state::tablet_replica_sets::replay(
									&mut tablet_replica_sets,
									set,
								);
							}
							if let Frame::TabletGrantFrame(grant) = frame {
								crate::log_shard_machine::state::tablet_grants::replay(
									&mut tablet_grants,
									grant,
								);
							}
							// The checkpoint supersedes earlier membership and deletion facts.
							if batch.log_seq <= boundary {
								continue;
							}
							// The next checkpoint delta must cover every change replayed
							// above the installed chain.
							crate::log_shard_machine::state::retired_volumes::replay(
								&mut retired_volumes,
								&tablet_grants,
								frame,
								&mut changed_since_checkpoint,
							);
							if let Some(key) = frame_volume_log_key(frame) {
								changed_since_checkpoint.insert(VolumeLogKey::from(key.clone()));
							}
							match frame {
								Frame::StateFrame(frame) => {
									saw_replica_frame = true;
									let key = VolumeLogKey::from(frame.volume_log_key.clone());
									let mut saved = frame.clone();
									if let Some(old) = facts.get(&key) {
										if old.state.cold.volume_seq > saved.state.cold.volume_seq {
											saved.state.cold = old.state.cold.clone();
										}
										if old.state.status == VolumeLogReplicaStatus::Deleted {
											saved.state.status = VolumeLogReplicaStatus::Deleted;
										}
										// A stale frame decoded late must not lower the fence a
										// repair mark keeps, so the highest claim epoch wins.
										let old_epoch = old
											.state
											.claimed_client
											.as_ref()
											.map(|claimed| claimed.client_epoch);
										let saved_epoch = saved
											.state
											.claimed_client
											.as_ref()
											.map(|claimed| claimed.client_epoch);
										if old_epoch > saved_epoch {
											saved.state.claimed_client =
												old.state.claimed_client.clone();
										}
									}
									facts.insert(key, saved);
								}
								Frame::ForgetFrame(frame) => {
									saw_replica_frame = true;
									facts.remove(&VolumeLogKey::from(frame.volume_log_key.clone()));
								}
								Frame::EntryFrame(_) => {
									saw_replica_frame = true;
								}
								Frame::TabletGrantFrame(_) | Frame::TabletReplicaSetFrame(_) => {}
							}
						}

						if batch.log_seq > boundary {
							bytes_since_checkpoint += len as u64;
							if !crossed_checkpoint {
								retain_checkpoint_locations(&mut store.entry_locations, &states);
								crossed_checkpoint = true;
							}

							// After earlier damage, later batches may name replicas the damaged
							// bytes created, so a replay failure is expected and repair marking
							// already covers it. On an undamaged log it is a version or code
							// mismatch.
							if let Err(error) = replay_batch(&mut states, &batch) {
								if corrupt.is_none() && cut.is_none() {
									return Err(RecoveryError::InvalidBatch {
										segment_first_log_seq: first,
										offset: offset as u64,
										reason: format!("{error:#}"),
									}
									.into());
								}
								corrupt.get_or_insert(first);
							}
						}
						// Historical frames rebuild byte locations without overwriting checkpoint state.
						// Frame checksums come from bytes the batch checksum just verified.
						let frames = frame_checksums(&batch, &raw[offset..offset + len])?;
						apply_locations(
							&mut store.entry_locations,
							&batch,
							&frames,
							first,
							offset as u64,
						)?;
						offset += len;
					}

					Err(BatchError::Damaged(_)) => {
						cut.get_or_insert((first, offset));
						undecodable_bytes = true;

						// Only an incomplete final write may take the torn-tail path, and only a
						// header that passes its own checksum says how long the frame was meant
						// to be. A header cut short by the end of the data is torn: nothing was
						// written after it. A complete header that fails its checksum is
						// damage, because the length it would need to prove a torn write cannot
						// be trusted, so it is corruption wherever it lies. With a valid header,
						// a frame that runs past EOF, or whose expected end lies inside a zero
						// tail, is torn; one that ends within the data is corruption, even at
						// EOF. Every intact batch ends in its nonzero trailer, so one that ends
						// past the data's end never reached the disk whole, even when the file's
						// length reaches its end.
						let header = read_frame_header(&raw[offset..data_end]);
						let frame_end = match header {
							FrameHeader::Valid { payload_bytes, .. } => {
								offset.checked_add(BATCH_FRAMING_BYTES + payload_bytes)
							}
							FrameHeader::Incomplete | FrameHeader::Damaged => None,
						};
						let damaged = match header {
							FrameHeader::Incomplete => false,
							FrameHeader::Damaged => true,
							FrameHeader::Valid { .. } => {
								frame_end.is_some_and(|end| end <= data_end)
							}
						};
						if damaged {
							corrupt.get_or_insert(first);
							corruption_kind.get_or_insert(CorruptionKind::DamagedBatch {
								log_seq: last.saturating_add(1),
							});
						}

						// Follow framing only, and only through a header that passed its checksum;
						// never search arbitrary bytes for an accidental checksum match.
						let next = frame_end.filter(|next| *next < data_end);
						if let Some(next) = next {
							offset = next;
						} else {
							break;
						}
					}
				}
			}

			// A file with no data, including a spare renamed into place before its first
			// write was synced, is cut like an empty file.
			if data_end == data_start {
				cut.get_or_insert((first, 0));
			}

			// Batches that end before the length the header says earlier syncs covered were
			// acknowledged and are gone. Treat the shard as corrupt rather than cutting them.
			// An older segment's last write synced before the roll that left it behind.
			if let Some(header) = header {
				let found = match cut {
					Some((cut_first, cut_offset)) if cut_first == first => {
						cut_offset.max(data_start)
					}
					_ => offset,
				} as u64;
				let synced_bytes = if Some(first) == newest_file {
					header.synced_bytes
				} else {
					header.written_bytes
				};
				if found < synced_bytes {
					tracing::error!(
						first_log_seq = first,
						synced_bytes,
						found_bytes = found,
						"log segment lost batches an earlier sync made durable; the disk returned synced pages as zeros or lost writes, so the shard is treated as corrupt"
					);

					crate::metrics::RECOVERY_TAILS
						.with_label_values(&["lost_synced"])
						.inc();
					corrupt.get_or_insert(first);
					corruption_kind.get_or_insert(CorruptionKind::LostSyncedTail {
						first_log_seq: first,
						synced_bytes,
						found_bytes: found,
					});
				} else if found < header.written_bytes {
					tracing::warn!(
						first_log_seq = first,
						synced_bytes = header.synced_bytes,
						written_bytes = header.written_bytes,
						found_bytes = found,
						"log segment's final write did not fully reach the disk; expected after a crash or power loss"
					);
					crate::metrics::RECOVERY_TAILS
						.with_label_values(&["unsynced_write"])
						.inc();
				}
			}

			// The scan stops at a batch boundary at or past the data's end, or at damage
			// within it, so this is the logical length unless a cut below lowers it.
			store.disk_segments.insert(
				first,
				Segment {
					first,
					bytes: offset.max(data_end) as u64,
					allocated: raw.len() as u64,
					last_log_seq: last,
					header,
				},
			);
			data_ends.insert(first, data_end);
			if !preallocated && offset < raw.len() && cut.is_none() {
				zero_tails.insert(first, (offset, raw.len()));
			}
		}

		// Without a later decoded boundary, damaged complete bytes may hide a
		// grant above the checkpoint. A genuinely incomplete final append is cut below.
		if undecodable_bytes && corrupt.is_some() {
			grant_fences_complete = false;
		}

		// Checkpoint-only replicas must also be included if corruption forces repair_mark.
		store.last_log_seq = store.last_log_seq.max(boundary);

		// A tail cut is safe only when the scan found no valid batch after the damage.
		if corrupt.is_none()
			&& let Some((first, offset)) = cut
		{
			// There may be empty files after the torn tail. Delete all empty
			// later files too; no valid batch follows this cut.
			tracing::warn!(
				first_log_seq = first,
				offset,
				"log shard recovery cut an incomplete final batch; expected after a crash or power loss, but repeated cuts without one suggest the disk loses synced writes"
			);
			crate::metrics::RECOVERY_TAILS
				.with_label_values(&["torn_cut"])
				.inc();
			for (&seq, _) in files.range(first..) {
				if seq == first && offset > 0 {
					let data_end = data_ends.get(&seq).copied().context("missing segment")?;
					let mut file = store
						.fs
						.open_options()
						.write(true)
						.open(store.path(LogShardFile::Segment { first_log_seq: seq }))
						.await?;
					let segment = store
						.disk_segments
						.get_mut(&seq)
						.context("missing segment")?;
					if (data_end as u64) < segment.allocated {
						// Keep the preallocation: zero the torn bytes in place so the next
						// write lands on zeros and syncs only its data.
						zero_range(&mut file, offset as u64, data_end as u64).await?;
						file.flush().await?;
						file.sync_data().await?;
					} else {
						file.set_len(offset as u64).await?;
						file.sync_all().await?;
						segment.allocated = offset as u64;
					}
					segment.bytes = offset as u64;
				} else {
					store
						.fs
						.remove_file(store.path(LogShardFile::Segment { first_log_seq: seq }))
						.await?;
					store.disk_segments.remove(&seq);
				}
			}
			store.sync_shard_dir().await?;
			// File names reserve sequences only while their bytes still exist. Reuse
			// a torn roll's sequence so the next restart sees no artificial gap.
			store.last_log_seq = last.max(boundary);
		}

		// Zeros past the last batch of a file that was never preallocated can come only from a
		// disk that lost synced pages, which would silently drop acknowledged batches. Report
		// the newest segment's, since a crash that skipped a roll's truncation leaves the
		// same zeros on a sealed segment.
		if let Some((&first, &(logical, physical))) = zero_tails.last_key_value()
			&& files.last_key_value().map(|(seq, _)| *seq) == Some(first)
		{
			tracing::warn!(
				first_log_seq = first,
				logical_bytes = logical,
				file_bytes = physical,
				"newest log segment ends in zeros past its last batch although it is not preallocated; the disk may have lost synced writes"
			);
			crate::metrics::RECOVERY_TAILS
				.with_label_values(&["zero_tail"])
				.inc();
		}

		// A crash can leave a sealed segment's zero tail untruncated. Truncate every
		// segment but the newest to its logical length, and the newest to at most one
		// preallocation, so disk accounting can charge sealed segments by logical length.
		if corrupt.is_none() {
			store.trim_preallocated_tails(progress).await?;
			store.settle_newest_header().await?;
		}

		// Checksums alone cannot prove that retained entries form the checkpoint's suffix.
		let mut recovered = None;
		if corrupt.is_none() {
			if !crossed_checkpoint {
				retain_checkpoint_locations(&mut store.entry_locations, &states);
			}
			store.poisoned = false;
			store.checkpoint_seq = boundary;
			match store
				.materialize(
					states.into_values().collect(),
					retired_volumes.clone(),
					tablet_grants.clone(),
					tablet_replica_sets.clone(),
					bytes_since_checkpoint,
					changed_since_checkpoint,
					progress,
				)
				.await
			{
				Ok(image) => recovered = Some(image),
				Err(error) if error.downcast_ref::<std::io::Error>().is_some() => {
					return Err(error);
				}
				// Every byte replayed passed its checksum and no damage was found, so a failed
				// validation is a version or code mismatch, never disk damage.
				Err(error) => {
					store.poisoned = true;
					return Err(RecoveryError::InvalidImage {
						reason: format!("{error:#}"),
					}
					.into());
				}
			}
		}

		// Preserve known identities and checkpoint floors, but trust no damaged suffix.
		if let Some(first_bad) = corrupt {
			tracing::warn!(first_bad, kind = ?corruption_kind, "log shard corruption requires repair marking");

			// Payloads are untrusted, but checksum-valid lifecycle facts remain authoritative.
			let floors = facts;
			ensure!(
				!floors.is_empty()
					|| !grant_fences_complete
					|| saw_replica_frame
					|| checkpoint.is_some(),
				"corruption erased all recoverable log identities"
			);

			store
				.record_format_on_open(recorded_max_batch_bytes)
				.await?;
			let plan = CorruptionFound {
				retired_volumes,
				disk_incarnation: *store.disk_incarnation.as_bytes(),
				grant_fences_complete,
				kind: corruption_kind
					.unwrap_or(CorruptionKind::DamagedBatch { log_seq: first_bad }),
				replica_floors: floors.into_values().collect(),
				tablet_grants,
				tablet_replica_sets,
				quarantine_from_log_seq: first_bad,
				last_valid_log_seq: store.last_log_seq,
			};

			return Ok((store, RecoveryOutcome::Corrupt(plan)));
		}

		store
			.record_format_on_open(recorded_max_batch_bytes)
			.await?;

		// Only a complete, validated image can be exposed to a serving shard.
		store.poisoned = false;
		store.checkpoint_seq = boundary;
		let recovered = recovered.context("missing clean recovery image")?;
		tracing::info!(logs = recovered.replicas.len(), "recovered log shard");

		Ok((store, RecoveryOutcome::Clean(recovered)))
	}

	/// Records a raised batch limit in FORMAT before this process can write a batch above the
	/// recorded one, so a later restart under a lower limit still reads it, and records the
	/// initial version marks a folder written before marks lacks, so its first payloads never
	/// rewrite FORMAT on the append path. It runs only once recovery has decided to proceed,
	/// so a refused recovery leaves FORMAT as it was. The limit and marks only grow, and the
	/// replacement is atomic, so a crash leaves either FORMAT valid.
	async fn record_format_on_open(&mut self, recorded: usize) -> Result<()> {
		let mut marks = self.version_marks.clone();
		let marks_changed = rivet_data::families::merge_marks(
			&mut marks,
			&depot_types::log_shard::initial_version_marks(),
		);
		if self.max_batch_bytes <= recorded && !marks_changed {
			return Ok(());
		}
		self.record_version_marks(marks).await
	}

	/// Materializes only live payloads after a segment-at-a-time scan, calling `progress`
	/// each time the reads move on to another segment.
	#[allow(
		clippy::too_many_arguments,
		reason = "the recovered image's parts are handed over separately, as the scan built them"
	)]
	async fn materialize(
		&mut self,
		snapshots: Vec<StateFrame>,
		retired_volumes: crate::log_shard_machine::state::retired_volumes::RetiredVolumes,
		tablet_grants: BTreeMap<[u8; 16], u64>,
		tablet_replica_sets: BTreeMap<
			[u8; 16],
			rivet_data::generated::shared_depot_types_v1::TabletReplicaSet,
		>,
		bytes_since_checkpoint: u64,
		changed_since_checkpoint: BTreeSet<VolumeLogKey>,
		progress: &(dyn Fn() + Send + Sync),
	) -> Result<RecoveredShard> {
		// Visit retained locations in disk order, not volume order, so interleaved
		// volumes read each segment once, front to back, in runs of at most one batch.
		let mut locations: Vec<((VolumeLogKey, u64), Location)> = self
			.entry_locations
			.iter()
			.flat_map(|(key, entries)| {
				entries
					.iter()
					.map(move |(seq, location)| ((key.clone(), *seq), location.clone()))
			})
			.collect();
		locations.sort_by_key(|(_, location)| (location.segment, location.offset));

		let mut by_volume: BTreeMap<VolumeLogKey, RecoveredEntries> = BTreeMap::new();
		let max_span = self.stored_max_batch_bytes as u64;
		let mut next = 0;
		let mut segment = None;
		while next < locations.len() {
			if segment.replace(locations[next].1.segment) != Some(locations[next].1.segment) {
				progress();
			}
			let end = super::run_end(&locations, next, max_span, max_span, u64::MAX);
			let run = self
				.read_run_at(&locations[next].1, &locations[end - 1].1)
				.await?;
			for ((key, seq), location) in &locations[next..end] {
				self.poisoned = true;
				let entry = run
					.entry(location, key, *seq)
					.map_err(|error| anyhow::anyhow!("{error:?}"))?;
				self.poisoned = false;
				// Restore rebases the byte totals; each entry keeps its encoded size.
				by_volume.entry(key.clone()).or_default().push(
					ReplicaEntry::from_log_entry(entry, 0, Some(location.log_seq)),
					self.recovered_payload_bytes_per_volume,
				);
			}
			next = end;
		}
		let mut replicas = Vec::new();
		for snapshot in snapshots {
			let key = VolumeLogKey::from(snapshot.volume_log_key.clone());
			let entries = by_volume.remove(&key).unwrap_or_default().entries;

			// Validate the reconstructed suffix before exposing it to the core.
			LoadedVolumeReplica::restore(
				snapshot.volume_log_key.clone().into(),
				snapshot.state.clone(),
				entries.clone(),
			)?;
			replicas.push(RecoveredReplica {
				volume_log_key: snapshot.volume_log_key.into(),
				state: snapshot.state,
				entries,
			});
		}
		Ok(RecoveredShard {
			checkpoint_bytes: self.checkpoint_disk_bytes(),
			retired_volumes,
			disk_incarnation: *self.disk_incarnation.as_bytes(),
			replicas,
			tablet_grants,
			tablet_replica_sets,
			disk_segments: segment_usage(self),
			last_log_seq: self.last_log_seq,
			checkpoint_last_included_log_seq: self.checkpoint_seq,
			bytes_since_checkpoint,
			changed_since_checkpoint,
			quarantined_bytes: self.quarantined_bytes,
			stored_max_batch_bytes: u32::try_from(self.stored_max_batch_bytes)
				.context("stored max batch bytes exceed u32")?,
		})
	}

	/// Caller obtains new incarnations from operational entropy in the recovery
	/// transition. Sync all existing evidence first, then write and sync repairing states
	/// in a new segment, and only then rename damaged segments to `.corrupt`. Before
	/// the marks, a crash rediscovers the same damage; after them it recovers repairing
	/// replicas. Renaming first could erase the evidence before replacement states exist.
	/// New incarnations make pre-loss claim proofs and MarkInSync requests obsolete.
	///
	/// Quarantined `.corrupt` files stay in the disk budget and the quarantine metric.
	/// Operators may archive and remove those files after inspecting the corruption
	/// and repairing affected replicas. Stop the shard before removal, then restart
	/// it so the scan refreshes disk accounting. Never remove active `seg-*` files.
	#[tracing::instrument(skip_all)]
	pub async fn mark_repairing(
		&mut self,
		retired_volumes: &crate::log_shard_machine::state::retired_volumes::RetiredVolumes,
		tablet_grants: &BTreeMap<[u8; 16], u64>,
		tablet_replica_sets: &BTreeMap<
			[u8; 16],
			rivet_data::generated::shared_depot_types_v1::TabletReplicaSet,
		>,
		repair_marks: &[RepairMark],
		quarantine_from_log_seq: u64,
		first_log_seq: u64,
	) -> Result<RecoveredShard> {
		// RepairMark replaces each known replica with an empty repairing incarnation.
		ensure!(
			self.poisoned,
			"repair marking is only allowed during recovery"
		);
		let quarantine: Vec<_> = self
			.disk_segments
			.range(quarantine_from_log_seq..)
			.map(|(seq, _)| *seq)
			.collect();
		self.entry_locations.clear();
		self.last_log_seq = first_log_seq
			.checked_sub(1)
			.context("invalid repair marking sequence")?;
		let mut frames = vec![];
		for repair_mark in repair_marks {
			let mut state = LoadedVolumeReplica::new(
				repair_mark.volume_log_key.clone(),
				repair_mark.new_replica_incarnation,
				repair_mark.cold.clone(),
				repair_mark.status.clone(),
			)
			.state()
			.clone();
			state.learner_replica_set_version = repair_mark.learner_replica_set_version;
			// The damaged history may have held entries this replica acknowledged.
			state.lost_entries = repair_mark.status == VolumeLogReplicaStatus::Repairing;
			// The last recorded claim still fences the clients it fenced before the loss.
			state.claimed_client = repair_mark.claimed_client.clone();
			frames.push(Frame::StateFrame(StateFrame {
				volume_log_key: repair_mark.volume_log_key.clone().into(),
				state,
			}));
		}

		// Force each recovery batch to a new file, leaving old segments untouched.
		let mut marked_repairing_states = BTreeMap::new();
		let mut batch = DiskBatch {
			log_seq: self
				.last_log_seq
				.checked_add(1)
				.context("sequence exhausted")?,
			frames: vec![],
		};

		// Split the replacement states into bounded batches and durably append each one.
		for frame in frames {
			batch.frames.push(frame);
			if encode_batch(&batch, self.max_batch_bytes).is_err() {
				let frame = batch.frames.pop().context("missing frame")?;
				ensure!(
					!batch.frames.is_empty(),
					"repair marking frame exceeds batch limit"
				);

				self.poisoned = false;
				self.write_to_disk_with_roll(std::slice::from_ref(&batch), true)
					.await?;
				replay_batch(&mut marked_repairing_states, &batch)?;
				batch = DiskBatch {
					log_seq: self
						.last_log_seq
						.checked_add(1)
						.context("sequence exhausted")?,
					frames: vec![frame],
				};
			}
		}

		self.poisoned = false;
		self.write_to_disk_with_roll(std::slice::from_ref(&batch), true)
			.await?;
		replay_batch(&mut marked_repairing_states, &batch)?;

		// Replaces an unreadable checkpoint too. This image makes completed
		// repair marking independently recoverable after the last quarantine rename.
		let image = LogShardCheckpoint {
			retired_volumes: crate::log_shard_machine::state::retired_volumes::checkpoint(
				retired_volumes,
			),
			tablet_replica_sets: tablet_replica_sets
				.iter()
				.map(
					|(id, set)| rivet_data::generated::depot_log_shard_v1::TabletReplicaSetFrame {
						tablet_id: *id,
						replica_set: set.clone(),
					},
				)
				.collect(),
			tablet_grants: tablet_grants
				.iter()
				.map(|(tablet_id, epoch)| {
					rivet_data::generated::depot_log_shard_v1::TabletGrantFrame {
						tablet_id: *tablet_id,
						tablet_grant_epoch: *epoch,
					}
				})
				.collect(),
			last_included_log_seq: self.last_log_seq,
			replicas: marked_repairing_states.into_values().collect(),
			previous_log_seq: None,
			changed_volume_log_keys: Vec::new(),
		};

		// The base replaces the whole chain, including every delta found by the scan.
		self.poisoned = true;
		self.write_base(&image).await?;

		// Quarantine only after the new states and checkpoint can survive a restart.
		for first in &quarantine {
			self.fs
				.rename(
					self.path(LogShardFile::Segment {
						first_log_seq: *first,
					}),
					self.path(LogShardFile::CorruptSegment {
						first_log_seq: *first,
					}),
				)
				.await?;
			let segment = self
				.disk_segments
				.remove(first)
				.context("missing quarantined segment")?;
			// Quarantine keeps the whole file, including any preallocated tail.
			self.quarantined_bytes = self
				.quarantined_bytes
				.checked_add(segment.bytes.max(segment.allocated))
				.context("quarantined segment byte count overflow")?;
		}

		self.sync_shard_dir().await?;

		self.poisoned = false;
		tracing::warn!(logs = repair_marks.len(), quarantined_segments = ?quarantine, "durably marked_repairing log shard");

		self.materialize(
			image.replicas,
			retired_volumes.clone(),
			tablet_grants.clone(),
			tablet_replica_sets.clone(),
			0,
			BTreeSet::new(),
			&|| {},
		)
		.await
	}
}

impl LogShardExecutor {
	/// Records the newest segment's recovered length as both synced and written when its
	/// header still describes a final write that did not fully reach the disk. Once the
	/// segment is left behind by a roll, recovery then holds it to that length.
	async fn settle_newest_header(&mut self) -> Result<()> {
		let Some((first, segment)) = self.disk_segments.last_key_value() else {
			return Ok(());
		};
		let Some(header) = segment.header else {
			return Ok(());
		};
		if header.written_bytes <= segment.bytes {
			return Ok(());
		}
		let settled = header.next(segment.bytes, segment.bytes);
		let first = *first;
		let mut file = self
			.fs
			.open_options()
			.write(true)
			.open(self.path(LogShardFile::Segment {
				first_log_seq: first,
			}))
			.await?;
		file.seek(std::io::SeekFrom::Start(settled.slot_offset()))
			.await?;
		file.write_all(&settled.encode()).await?;
		file.flush().await?;
		file.sync_data().await?;
		if let Some(segment) = self.disk_segments.get_mut(&first) {
			segment.header = Some(settled);
		}
		Ok(())
	}

	/// Truncates zero tails a clean scan found past each segment's logical length: all of
	/// it for sealed segments, and anything past one preallocation for the newest. Only
	/// zeros are removed, so the truncation needs no ordering against other writes.
	async fn trim_preallocated_tails(&mut self, progress: &(dyn Fn() + Send + Sync)) -> Result<()> {
		let newest = self.disk_segments.last_key_value().map(|(first, _)| *first);
		for (first, segment) in self.disk_segments.iter_mut() {
			let keep = if Some(*first) == newest {
				segment.bytes.max(self.segment_bytes)
			} else {
				segment.bytes
			};
			if segment.allocated <= keep {
				continue;
			}
			let file = self
				.fs
				.open_options()
				.write(true)
				.open(
					self.root.join(
						depot_types::keys::fs::LogShardKey::new(LogShardFile::Segment {
							first_log_seq: *first,
						})
						.pack(),
					),
				)
				.await?;
			file.set_len(keep).await?;
			file.sync_all().await?;
			segment.allocated = keep;
			progress();
		}
		Ok(())
	}
}

/// Overwrites `[from, to)` of an open file with zeros, leaving its cursor at `to`.
async fn zero_range(file: &mut rivet_env::fs::File, from: u64, to: u64) -> Result<()> {
	const CHUNK_BYTES: u64 = 1024 * 1024;
	file.seek(std::io::SeekFrom::Start(from)).await?;
	let zeros = vec![0; to.saturating_sub(from).min(CHUNK_BYTES) as usize];
	let mut position = from;
	while position < to {
		let chunk = (to - position).min(CHUNK_BYTES) as usize;
		file.write_all(&zeros[..chunk]).await?;
		position += chunk as u64;
	}
	Ok(())
}

/// The volume log a frame changes, or `None` for a tablet frame.
fn frame_volume_log_key(
	frame: &Frame,
) -> Option<&rivet_data::generated::shared_depot_types_v1::VolumeLogKey> {
	match frame {
		Frame::EntryFrame(f) => Some(&f.volume_log_key),
		Frame::StateFrame(f) => Some(&f.volume_log_key),
		Frame::ForgetFrame(f) => Some(&f.volume_log_key),
		Frame::TabletGrantFrame(_) | Frame::TabletReplicaSetFrame(_) => None,
	}
}

/// Uses the same frame reducer as live disk-state installation.
fn replay_batch(states: &mut ReplicaStates, batch: &DiskBatch) -> Result<()> {
	for frame in &batch.frames {
		let volume_log_key = match frame {
			Frame::EntryFrame(f) => &f.volume_log_key,
			Frame::StateFrame(f) => &f.volume_log_key,
			Frame::ForgetFrame(f) => &f.volume_log_key,
			// Tablet grants are collected separately from volume replica state.
			Frame::TabletGrantFrame(_) | Frame::TabletReplicaSetFrame(_) => continue,
		};

		let key = VolumeLogKey::from(volume_log_key.clone());
		let mut state = states.get(&key).map(|r| r.state.clone());
		replay_state(&mut state, frame)?;
		let Some(state) = state else {
			states.remove(&key);
			continue;
		};
		states.insert(
			key,
			StateFrame {
				volume_log_key: volume_log_key.clone(),
				state,
			},
		);
	}
	Ok(())
}

/// A checkpoint contains no payloads. Keep only its live historical locations before newer replay.
fn retain_checkpoint_locations(
	entry_locations: &mut BTreeMap<VolumeLogKey, BTreeMap<u64, Location>>,
	states: &ReplicaStates,
) {
	entry_locations.retain(|volume_log_key, entries| {
		let Some(saved) = states.get(volume_log_key) else {
			return false;
		};
		// A deleted replica keeps its tail above cold until a trim releases it.
		entries.retain(|seq, _| {
			*seq > saved.state.cold.volume_seq && *seq <= saved.state.appended.volume_seq
		});
		true
	});
}

/// Copies recovered file lengths and batch boundaries into core-owned accounting.
fn segment_usage(store: &LogShardExecutor) -> BTreeMap<u64, DiskSegmentUsage> {
	store
		.disk_segments
		.iter()
		.map(|(first, s)| {
			(
				*first,
				DiskSegmentUsage {
					last_log_seq: s.last_log_seq,
					len_bytes: s.bytes,
				},
			)
		})
		.collect()
}

/// One volume's recovered entries in log order, releasing the oldest payloads as newer
/// entries arrive so at most the per-volume limit of payload bytes is held at once.
#[derive(Default)]
struct RecoveredEntries {
	entries: Vec<ReplicaEntry>,
	/// Entries before this index have released their payloads.
	released: usize,
	/// Encoded bytes of the entries from `released` on.
	held_bytes: u64,
}

impl RecoveredEntries {
	fn push(&mut self, entry: ReplicaEntry, keep_bytes: u64) {
		self.held_bytes += entry.encoded_bytes();
		self.entries.push(entry);
		while self.held_bytes > keep_bytes && self.released < self.entries.len() {
			let oldest = &mut self.entries[self.released];
			self.held_bytes -= oldest.encoded_bytes();
			oldest.release_payload();
			self.released += 1;
		}
	}
}
