//! Log shard disk framing. FORMAT binds a folder to its schema version, the fingerprint of the
//! exact schemas its records use, the largest batch any writer was allowed, its topology
//! worker UUID, and the highest version of each payload family any writer stored in it. Segments contain atomic batches, each behind a twelve-byte header (the
//! little-endian payload length, the CRC32C of the raw BARE payload, and a CRC32C of those
//! eight bytes) and followed by a nonzero trailer byte. There is no per-frame or per-segment
//! version header; the folder's FORMAT selects the batch schema and framing. A preallocated
//! segment starts with two header slots ([`SegmentHeader`]) before its first batch.
//!
//! The batch checksum decides at recovery whether a batch was written whole. A read of one
//! entry checks only that entry's frame, against a CRC32C the shard keeps in memory. It takes
//! that checksum from the bytes it wrote, or at recovery from bytes whose batch checksum it
//! has just verified, so every byte a read returns is still covered by a checksum taken from
//! verified bytes. [`FrameChecksum`] carries that checksum and where the frame lies.

use anyhow::{Result, ensure};
use rivet_data::{
	DEPOT_LOG_SHARD_VERSION,
	families::{VersionAboveRead, VersionFamily},
	generated::DEPOT_LOG_SHARD_SCHEMA_FINGERPRINT,
	generated::depot_log_shard_v1::DiskBatch,
	generated::shared_depot_types_v1::VersionMark,
	generated::topology_worker_heartbeat_v1::FamilyVersions,
};
use rivet_topology::worker::WorkerId;
use vbare::{Bytes, Encode, Message};

/// The fixed FORMAT prefix: version (2), worker UUID (16), disk incarnation (16), schema
/// fingerprint (32) and the largest batch payload limit any writer used (4). Version marks
/// follow it: a count (u16 LE), then that many (family u16 LE, version u16 LE) pairs, sorted by
/// family. A FORMAT that ends after the prefix has no marks, which reads as version 1 data.
pub const FORMAT_BYTES: usize = 70;

/// The payload families the binary's own writers store in log shards. A new FORMAT records
/// each at version 1, so the first payload of a family never rewrites FORMAT on the append
/// path; only a later version bump does, once per disk.
pub const LOG_PAYLOAD_FAMILIES: &[VersionFamily] = &[
	VersionFamily::DEPOT_FS_ENTRY,
	VersionFamily::DEPOT_MAINTAINER_LOG,
];

/// The marks a fresh FORMAT records: every [`LOG_PAYLOAD_FAMILIES`] entry at version 1.
pub fn initial_version_marks() -> Vec<VersionMark> {
	let mut marks = Vec::new();
	for family in LOG_PAYLOAD_FAMILIES {
		rivet_data::families::raise_mark(&mut marks, *family, 1);
	}
	marks
}

/// The FORMAT length binaries wrote before it carried a schema fingerprint.
const UNFINGERPRINTED_FORMAT_BYTES: usize = 34;

/// The batch header: payload length (4), payload CRC32C (4) and a CRC32C of those eight bytes
/// (4). The header's own checksum lets recovery trust a length before reading the payload, so a
/// damaged length is never mistaken for an incomplete final write.
pub const BATCH_HEADER_BYTES: usize = 12;

/// Ends every batch. Being nonzero, it makes an intact batch end at a nonzero byte, so the
/// last nonzero byte of a segment is never inside an intact batch. Recovery can then tell a
/// batch whose payload fails its checksum, which is damage, from a final write whose tail never
/// reached the disk, which ends in zeros or the end of the file before its trailer.
pub const BATCH_TRAILER: u8 = 0xa5;

/// Bytes a batch adds around its payload: the header and the trailer.
pub const BATCH_FRAMING_BYTES: usize = BATCH_HEADER_BYTES + 1;

/// Describes the batch framing for the fingerprint. Change it whenever the framing changes, so
/// a folder framed another way refuses to open.
const BATCH_FRAMING: &[u8] =
	b"batch: u32le payload length, u32le crc32c(payload), u32le crc32c(length and payload crc), payload, trailer 0xa5";

/// The record fingerprint for each `depot.log_shard` version this binary reads. FORMAT's
/// version field is that family's mark, and its fingerprint must be the one known for it.
/// Until the first release that bumps the version, schemas change in place at v1 and the
/// fingerprint is what refuses a folder written under another layout.
pub fn known_fingerprint(log_shard_version: u16) -> Option<[u8; 32]> {
	match log_shard_version {
		DEPOT_LOG_SHARD_VERSION => Some(log_shard_fingerprint()),
		_ => None,
	}
}

/// The fingerprint FORMAT records: the log record schemas this binary reads and writes and
/// the batch framing around them.
pub fn log_shard_fingerprint() -> [u8; 32] {
	use sha2::{Digest, Sha256};

	let mut hasher = Sha256::new();
	hasher.update(DEPOT_LOG_SHARD_SCHEMA_FINGERPRINT);
	hasher.update(BATCH_FRAMING);
	hasher.finalize().into()
}

/// The default size of one checksummed disk batch. A coalesced request to a log shard is capped
/// below it, so the frames of every item in the request fit one batch unless a shard is
/// configured smaller.
pub const DEFAULT_MAX_BATCH_BYTES: u32 = 4 * 1024 * 1024;

/// The default number of volumes one batched forget or trim request may name. The shard
/// rejects a larger batch, so maintainers take their batch size from the shard's setting.
pub const DEFAULT_MAX_GRANT_BATCH_VOLUMES: u32 = 256;

/// FORMAT records the folder format, the schema its records were written with, and the
/// creating worker identity. Recovery refuses to start on any mismatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatFile {
	pub log_shard_format_version: u16,
	pub worker_id: WorkerId,
	/// Changes only when the disk folder is initialized, never on an intact restart.
	pub disk_incarnation: uuid::Uuid,
	pub schema_fingerprint: [u8; 32],
	/// The largest batch payload limit any writer of this folder used. Stored batches are
	/// validated against it, never against the live configuration, which may be lower.
	pub max_batch_bytes: u32,
	/// The highest version of each payload family stored in this folder, sorted by family.
	/// Raised before the first batch holding a higher version; never lowered.
	pub version_marks: Vec<VersionMark>,
}

/// Why a FORMAT file cannot be used. None of these may initialize a replacement folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatError {
	UnsupportedVersion {
		found: u16,
		supported: u16,
	},
	/// Written before FORMAT carried a schema fingerprint, so its records' layout is unknown.
	Unfingerprinted,
	Length {
		found: usize,
	},
	SchemaMismatch {
		found: [u8; 32],
		expected: [u8; 32],
	},
	/// The marks after the fixed prefix are truncated, unsorted or repeat a family.
	MalformedMarks,
	/// A writer stored a payload family at a version this binary does not read.
	VersionAboveRead(VersionAboveRead),
	ForeignWorker,
}

impl std::fmt::Display for FormatError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::UnsupportedVersion { found, supported } => write!(
				f,
				"log shard FORMAT version {found} is not supported by this binary, which reads \
				 version {supported}; run a binary that supports it"
			),
			Self::Unfingerprinted => write!(
				f,
				"log shard FORMAT has no schema fingerprint, so it was written by an older binary \
				 whose record layout is unknown; run the binary that wrote it or reinitialize the \
				 disk after its tablets are replaced"
			),
			Self::Length { found } => write!(
				f,
				"log shard FORMAT is {found} bytes, not {FORMAT_BYTES}; the file is damaged or \
				 was written by an incompatible binary"
			),
			Self::SchemaMismatch { found, expected } => write!(
				f,
				"log shard records were written with schema fingerprint {} but this binary uses \
				 {}; refusing to replay them. Run the binary that wrote this disk, or reinitialize \
				 the disk after its tablets are replaced",
				hex(found),
				hex(expected)
			),
			Self::MalformedMarks => write!(
				f,
				"log shard FORMAT version marks are malformed; the file is damaged or was \
				 written by an incompatible binary"
			),
			Self::VersionAboveRead(error) => write!(f, "log shard FORMAT refused: {error}"),
			Self::ForeignWorker => write!(f, "log shard belongs to another worker"),
		}
	}
}

impl std::error::Error for FormatError {}

fn hex(bytes: &[u8]) -> String {
	bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Encodes the folder identity with this binary's schema fingerprint and the given version
/// marks, which must be sorted by family. A caller must durably create this before segments,
/// and rewrite it before writing a batch larger than the recorded `max_batch_bytes` or a
/// payload above its family's recorded mark.
pub fn encode_format(
	worker_id: &WorkerId,
	disk_incarnation: uuid::Uuid,
	max_batch_bytes: u32,
	version_marks: &[VersionMark],
) -> Vec<u8> {
	encode_format_at(
		DEPOT_LOG_SHARD_VERSION,
		&log_shard_fingerprint(),
		worker_id,
		disk_incarnation,
		max_batch_bytes,
		version_marks,
	)
}

/// Like [`encode_format`], but keeps a folder's recorded record version and fingerprint.
/// Rewrites of an existing FORMAT (a raised batch limit or version mark) use this, so they
/// never relabel records written under an older `depot.log_shard` version; only an explicit
/// migration step may move a folder's record version.
pub fn encode_format_at(
	log_shard_format_version: u16,
	schema_fingerprint: &[u8; 32],
	worker_id: &WorkerId,
	disk_incarnation: uuid::Uuid,
	max_batch_bytes: u32,
	version_marks: &[VersionMark],
) -> Vec<u8> {
	let mut bytes = vec![0; FORMAT_BYTES];
	bytes[..2].copy_from_slice(&log_shard_format_version.to_le_bytes());
	bytes[2..18].copy_from_slice(worker_id.as_bytes());
	bytes[18..34].copy_from_slice(disk_incarnation.as_bytes());
	bytes[34..66].copy_from_slice(schema_fingerprint);
	bytes[66..70].copy_from_slice(&max_batch_bytes.to_le_bytes());
	if !version_marks.is_empty() {
		let count = u16::try_from(version_marks.len()).unwrap_or(u16::MAX);
		bytes.extend_from_slice(&count.to_le_bytes());
		for mark in version_marks.iter().take(count as usize) {
			bytes.extend_from_slice(&mark.family.to_le_bytes());
			bytes.extend_from_slice(&mark.version.to_le_bytes());
		}
	}
	bytes
}

/// Decodes the marks after the fixed prefix. They must be complete and strictly sorted by
/// family, as [`encode_format`] writes them.
fn decode_marks(bytes: &[u8]) -> std::result::Result<Vec<VersionMark>, FormatError> {
	if bytes.is_empty() {
		return Ok(Vec::new());
	}
	let count = bytes
		.get(..2)
		.map(|count| u16::from_le_bytes([count[0], count[1]]) as usize)
		.ok_or(FormatError::MalformedMarks)?;
	let pairs = &bytes[2..];
	if pairs.len() != count * 4 {
		return Err(FormatError::MalformedMarks);
	}
	let marks: Vec<VersionMark> = pairs
		.chunks_exact(4)
		.map(|pair| VersionMark {
			family: u16::from_le_bytes([pair[0], pair[1]]),
			version: u16::from_le_bytes([pair[2], pair[3]]),
		})
		.collect();
	if !marks.windows(2).all(|pair| pair[0].family < pair[1].family) {
		return Err(FormatError::MalformedMarks);
	}
	Ok(marks)
}

/// Reads the identity before startup constructs worker-addressed clients. Unsupported
/// versions, another schema fingerprint, marks above what `read_versions` reads or malformed
/// bytes must never initialize a replacement folder. `read_versions` is what this worker
/// advertises in its topology heartbeat.
pub fn read_format(
	bytes: &[u8],
	read_versions: &[FamilyVersions],
) -> std::result::Result<FormatFile, FormatError> {
	let mut expected = None;
	if let Some(version) = bytes.get(..2) {
		let version = u16::from_le_bytes([version[0], version[1]]);
		expected = known_fingerprint(version);
		if expected.is_none() {
			return Err(FormatError::UnsupportedVersion {
				found: version,
				supported: DEPOT_LOG_SHARD_VERSION,
			});
		}
	}
	if bytes.len() == UNFINGERPRINTED_FORMAT_BYTES {
		return Err(FormatError::Unfingerprinted);
	}
	if bytes.len() < FORMAT_BYTES {
		return Err(FormatError::Length { found: bytes.len() });
	}
	let (prefix, marks) = bytes.split_at(FORMAT_BYTES);
	let version_marks = decode_marks(marks)?;
	let bytes: &[u8; FORMAT_BYTES] = prefix
		.try_into()
		.map_err(|_| FormatError::Length { found: bytes.len() })?;
	let expected = expected.ok_or(FormatError::Length { found: bytes.len() })?;

	let mut schema_fingerprint = [0; 32];
	schema_fingerprint.copy_from_slice(&bytes[34..66]);
	if schema_fingerprint != expected {
		return Err(FormatError::SchemaMismatch {
			found: schema_fingerprint,
			expected,
		});
	}

	rivet_data::families::readable_by(&version_marks, read_versions)
		.map_err(FormatError::VersionAboveRead)?;

	let mut worker_id = [0; 16];
	worker_id.copy_from_slice(&bytes[2..18]);
	let mut disk_incarnation = [0; 16];
	disk_incarnation.copy_from_slice(&bytes[18..34]);
	Ok(FormatFile {
		log_shard_format_version: u16::from_le_bytes([bytes[0], bytes[1]]),
		worker_id: WorkerId::from_bytes(worker_id),
		disk_incarnation: uuid::Uuid::from_bytes(disk_incarnation),
		schema_fingerprint,
		max_batch_bytes: u32::from_le_bytes([bytes[66], bytes[67], bytes[68], bytes[69]]),
		version_marks,
	})
}

/// Rejects every [`read_format`] failure and a different owner. Recovery refuses to start on
/// a mismatch; it must never initialize over it.
pub fn decode_format(
	bytes: &[u8],
	expected_worker: &WorkerId,
	read_versions: &[FamilyVersions],
) -> std::result::Result<FormatFile, FormatError> {
	let format = read_format(bytes, read_versions)?;
	if &format.worker_id != expected_worker {
		return Err(FormatError::ForeignWorker);
	}
	Ok(format)
}

/// Why a framed batch cannot be returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchError {
	/// The framing header, size bound or checksum failed. The bytes changed after they were
	/// written, or the final write was torn.
	Damaged(String),
	/// The checksum matched, so the bytes are exactly what a writer produced, but this binary
	/// cannot decode them. A different schema or a bug wrote them; it is never disk damage.
	Undecodable { reason: String },
}

impl std::fmt::Display for BatchError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Damaged(reason) => write!(f, "{reason}"),
			Self::Undecodable { reason } => {
				write!(f, "checksum-valid log batch does not decode: {reason}")
			}
		}
	}
}

impl std::error::Error for BatchError {}

/// Frames a whole disk batch. `max_batch_bytes` bounds the encoded payload; the
/// [`BATCH_FRAMING_BYTES`] of header and trailer are additional.
pub fn encode_batch(batch: &DiskBatch, max_batch_bytes: usize) -> Result<Vec<u8>> {
	let mut bytes = Vec::new();
	encode_batch_into(batch, max_batch_bytes, &mut bytes)?;
	Ok(bytes)
}

/// Where one frame lies inside its framed batch, and the CRC32C of exactly its bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameChecksum {
	/// Offset from the start of the batch's header.
	pub offset: u32,
	pub len: u32,
	pub crc32c: u32,
}

/// Appends one framed batch to `out`, encoding the batch directly behind its header so the
/// payload is written once and never copied. On error `out` is left unchanged.
pub fn encode_batch_into(
	batch: &DiskBatch,
	max_batch_bytes: usize,
	out: &mut Vec<u8>,
) -> Result<()> {
	let start = encode_unsummed(batch, max_batch_bytes, out)?;
	let payload_end = out.len() - 1;
	let checksum = crc32c::crc32c(&out[start + BATCH_HEADER_BYTES..payload_end]);
	out[start + 4..start + 8].copy_from_slice(&checksum.to_le_bytes());
	let header_checksum = crc32c::crc32c(&out[start..start + 8]);
	out[start + 8..start + BATCH_HEADER_BYTES].copy_from_slice(&header_checksum.to_le_bytes());
	Ok(())
}

/// [`encode_batch_into`] that also returns every frame's checksum in frame order, for the
/// shard's entry index. The frame checksums are a second CRC32C pass over the frames rather
/// than parts combined into the batch checksum: combining costs a GF(2) matrix power per
/// frame, far more than rereading a small frame with the hardware instruction.
pub fn encode_batch_with_frames_into(
	batch: &DiskBatch,
	max_batch_bytes: usize,
	out: &mut Vec<u8>,
) -> Result<Vec<FrameChecksum>> {
	let start = out.len();
	encode_batch_into(batch, max_batch_bytes, out)?;
	match frame_checksums(batch, &out[start..]) {
		Ok(frames) => Ok(frames),
		Err(error) => {
			out.truncate(start);
			Err(error)
		}
	}
}

/// Appends the length, zero checksums, the payload and the trailer, returning where the batch
/// starts in `out`. The caller fills in the checksums.
fn encode_unsummed(batch: &DiskBatch, max_batch_bytes: usize, out: &mut Vec<u8>) -> Result<usize> {
	let payload_len = batch.encoded_len();
	ensure!(
		payload_len <= max_batch_bytes,
		"log batch exceeds maximum size"
	);
	let length = u32::try_from(payload_len)?;
	let start = out.len();
	out.reserve(BATCH_FRAMING_BYTES + payload_len);
	out.extend_from_slice(&length.to_le_bytes());
	out.extend_from_slice(&[0; 8]);
	batch.encode(out);
	out.push(BATCH_TRAILER);
	Ok(start)
}

/// Locates and checksums every frame of `batch` inside `framed`, the batch's framed bytes
/// from its header on. Frames are the batch's last field, so they fill the payload's tail in
/// order, just before the trailer, and each spans exactly its encoded length.
pub fn frame_checksums(batch: &DiskBatch, framed: &[u8]) -> Result<Vec<FrameChecksum>> {
	let frames_bytes: usize = batch.frames.iter().map(Encode::encoded_len).sum();
	let payload_len = batch.encoded_len();
	ensure!(
		framed.len() == BATCH_FRAMING_BYTES + payload_len,
		"framed batch length does not match its frames"
	);
	let mut offset = BATCH_HEADER_BYTES + payload_len - frames_bytes;
	let mut checksums = Vec::with_capacity(batch.frames.len());
	for frame in &batch.frames {
		let len = frame.encoded_len();
		checksums.push(FrameChecksum {
			offset: u32::try_from(offset)?,
			len: u32::try_from(len)?,
			crc32c: crc32c::crc32c(&framed[offset..offset + len]),
		});
		offset += len;
	}
	Ok(checksums)
}

/// The framed size of a batch, header included, computed without encoding it.
pub fn framed_batch_bytes(batch: &DiskBatch) -> usize {
	BATCH_FRAMING_BYTES + batch.encoded_len()
}

/// Decodes the next whole batch from the remaining segment bytes. Returns the
/// number of bytes consumed so the caller can continue at the next batch.
/// No frames are returned on any failure. The size limit is checked before BARE
/// decoding, and a valid checksum does not excuse a truncated or trailing payload.
///
/// The payload is copied, so decoded entries do not keep `segment` alive. Callers that hold
/// one batch in a `Bytes` and drop it soon use [`decode_batch_shared`].
pub fn decode_batch(segment: &[u8], max_batch_bytes: usize) -> Result<(DiskBatch, usize)> {
	Ok(decode_batch_checked(segment, max_batch_bytes)?)
}

/// [`decode_batch`] with the failure classified, so recovery can tell disk damage from bytes
/// a writer produced that this binary cannot read.
pub fn decode_batch_checked(
	segment: &[u8],
	max_batch_bytes: usize,
) -> std::result::Result<(DiskBatch, usize), BatchError> {
	let end = check_batch(segment, max_batch_bytes)?;
	// The decoder rejects trailing bytes after the batch.
	let batch = DiskBatch::decode(Bytes::copy_from_slice(
		&segment[BATCH_HEADER_BYTES..end - 1],
	))
	.map_err(|error| BatchError::Undecodable {
		reason: format!("{error:#}"),
	})?;
	Ok((batch, end))
}

/// [`decode_batch`] without copying: decoded payloads are slices of `segment` and keep it
/// alive.
pub fn decode_batch_shared(segment: &Bytes, max_batch_bytes: usize) -> Result<(DiskBatch, usize)> {
	let end = check_batch(segment, max_batch_bytes)?;
	let batch = DiskBatch::decode(segment.slice(BATCH_HEADER_BYTES..end - 1)).map_err(|error| {
		BatchError::Undecodable {
			reason: format!("{error:#}"),
		}
	})?;
	Ok((batch, end))
}

/// What the header at the start of `bytes` says about the batch behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameHeader {
	/// Fewer than [`BATCH_HEADER_BYTES`] bytes remain, so no header was completely written.
	Incomplete,
	/// The header's own checksum fails, so its length cannot be trusted. Bytes changed after
	/// they were written; an incomplete final write never leaves a complete header like this
	/// unless later bytes of the same write landed while the header's did not.
	Damaged,
	/// The header is exactly what a writer produced.
	Valid {
		payload_bytes: usize,
		payload_checksum: u32,
	},
}

/// Reads and verifies the header at the start of `bytes`.
pub fn read_frame_header(bytes: &[u8]) -> FrameHeader {
	let Some(header) = bytes.get(..BATCH_HEADER_BYTES) else {
		return FrameHeader::Incomplete;
	};
	let word = |at: usize| {
		u32::from_le_bytes([header[at], header[at + 1], header[at + 2], header[at + 3]])
	};
	if crc32c::crc32c(&header[..8]) != word(8) {
		return FrameHeader::Damaged;
	}
	FrameHeader::Valid {
		payload_bytes: word(0) as usize,
		payload_checksum: word(4),
	}
}

/// Checks the framing header, size limit and checksum, returning the batch's end offset.
fn check_batch(segment: &[u8], max_batch_bytes: usize) -> std::result::Result<usize, BatchError> {
	let damaged = |reason: &str| BatchError::Damaged(reason.to_string());
	let (length, checksum) = match read_frame_header(segment) {
		FrameHeader::Incomplete => return Err(damaged("truncated log batch header")),
		FrameHeader::Damaged => return Err(damaged("log batch header checksum mismatch")),
		FrameHeader::Valid {
			payload_bytes,
			payload_checksum,
		} => (payload_bytes, payload_checksum),
	};
	// A header that passes its checksum is what a writer produced. Every batch encodes its log
	// sequence, and FORMAT records the largest limit any writer used, so neither an empty nor
	// an oversized payload can be disk damage.
	if length == 0 {
		return Err(BatchError::Undecodable {
			reason: "checksum-valid header names an empty payload".into(),
		});
	}
	if length > max_batch_bytes {
		return Err(BatchError::Undecodable {
			reason: format!(
				"checksum-valid header names a {length} byte payload, above the recorded limit of {max_batch_bytes}"
			),
		});
	}
	let end = BATCH_FRAMING_BYTES
		.checked_add(length)
		.ok_or_else(|| damaged("log batch length overflow"))?;
	if end > segment.len() {
		return Err(damaged("log batch runs past the end of the segment"));
	}
	if crc32c::crc32c(&segment[BATCH_HEADER_BYTES..end - 1]) != checksum {
		return Err(damaged("log batch checksum mismatch"));
	}
	if segment[end - 1] != BATCH_TRAILER {
		return Err(damaged("log batch trailer mismatch"));
	}
	Ok(end)
}

/// Exact BARE size without allocating or copying block payloads. The folder's FORMAT pins v1.
pub fn entry_bytes(entry: &rivet_data::generated::depot_log_shard_v1::VolumeLogEntry) -> u64 {
	use rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody;
	fn uint_bytes(value: usize) -> u64 {
		((usize::BITS - value.leading_zeros()).max(1) as u64).div_ceil(7)
	}
	24 + 1
		+ match &entry.body {
			VolumeLogEntryBody::ClaimEntry => 0,
			// Payload family and version, identity and digest, the codec's one-byte tag, then
			// the length-prefixed payload.
			VolumeLogEntryBody::PayloadWrite(write) => {
				2 + 2
					+ 8 + 16 + 16 + 1
					+ uint_bytes(write.payload.len())
					+ write.payload.len() as u64
			}
		}
}

/// Bytes of one header slot of a preallocated log segment. Each slot fills its own 4 KiB
/// page, so a torn write of one slot never damages the other.
pub const SEGMENT_HEADER_SLOT_BYTES: u64 = 4096;

/// Bytes at the front of a preallocated log segment that hold its two header slots. Its
/// first batch starts here.
pub const SEGMENT_HEADER_BYTES: u64 = 2 * SEGMENT_HEADER_SLOT_BYTES;

/// Starts every encoded header slot. Its first four bytes read as a batch length above any
/// accepted batch, so a plain segment, which starts with a batch, never reads as one.
const SEGMENT_HEADER_MAGIC: [u8; 8] = [0xff, 0xff, 0xff, 0xff, b'D', b'L', b'S', b'H'];

/// Encoded bytes of one slot: the magic, three counters and a CRC32C over everything
/// before it. The rest of the slot's page stays zero.
pub const SEGMENT_HEADER_ENCODED_BYTES: usize = 8 + 8 + 8 + 8 + 4;

/// What one write recorded at the front of a preallocated log segment.
///
/// A preallocated segment is zero-filled past its last batch, so zeros there read as free
/// space. A disk that returns synced pages as zeros would otherwise make acknowledged
/// batches look unwritten. Each write therefore also rewrites a header slot, inside the same
/// data sync, recording how far the segment's batches reached when the write started. That
/// length was made durable by an earlier, completed sync, so recovery that finds the
/// batches ending before it knows the disk lost synced data. The write's own end is recorded
/// too, but the write and its slot reach the disk in either order before the sync returns,
/// so that end only tells recovery that a shorter tail may be an interrupted final write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentHeader {
	/// Writes recorded since the segment was preallocated. The slot holding the larger
	/// count is the newer one, and write `n` uses slot `n % 2`.
	pub write_count: u64,
	/// The segment's logical length when this write started, all of it synced by earlier
	/// writes.
	pub synced_bytes: u64,
	/// The segment's logical length after this write, which may not have reached the disk.
	pub written_bytes: u64,
}

impl SegmentHeader {
	/// The header a spare is filled with: no writes, and no batches.
	pub fn empty() -> Self {
		Self {
			write_count: 0,
			synced_bytes: SEGMENT_HEADER_BYTES,
			written_bytes: SEGMENT_HEADER_BYTES,
		}
	}

	/// The header the next write records after this one, for a write from `synced_bytes`,
	/// the current logical length, to `written_bytes`.
	pub fn next(&self, synced_bytes: u64, written_bytes: u64) -> Self {
		Self {
			write_count: self.write_count + 1,
			synced_bytes,
			written_bytes,
		}
	}

	/// File offset of the slot this header is written to.
	pub fn slot_offset(&self) -> u64 {
		(self.write_count % 2) * SEGMENT_HEADER_SLOT_BYTES
	}

	pub fn encode(&self) -> [u8; SEGMENT_HEADER_ENCODED_BYTES] {
		let mut out = [0; SEGMENT_HEADER_ENCODED_BYTES];
		out[..8].copy_from_slice(&SEGMENT_HEADER_MAGIC);
		out[8..16].copy_from_slice(&self.write_count.to_le_bytes());
		out[16..24].copy_from_slice(&self.synced_bytes.to_le_bytes());
		out[24..32].copy_from_slice(&self.written_bytes.to_le_bytes());
		let checksum = crc32c::crc32c(&out[..32]);
		out[32..].copy_from_slice(&checksum.to_le_bytes());
		out
	}

	/// Decodes one slot, or `None` when its magic or checksum does not match or its lengths
	/// are impossible.
	fn decode_slot(slot: &[u8]) -> Option<Self> {
		let encoded = slot.get(..SEGMENT_HEADER_ENCODED_BYTES)?;
		if encoded[..8] != SEGMENT_HEADER_MAGIC {
			return None;
		}
		let checksum = u32::from_le_bytes(encoded[32..36].try_into().ok()?);
		if crc32c::crc32c(&encoded[..32]) != checksum {
			return None;
		}
		let field =
			|at: usize| u64::from_le_bytes(encoded[at..at + 8].try_into().unwrap_or([0; 8]));
		let header = Self {
			write_count: field(8),
			synced_bytes: field(16),
			written_bytes: field(24),
		};
		let valid = header.synced_bytes >= SEGMENT_HEADER_BYTES
			&& header.synced_bytes <= header.written_bytes;
		if valid { Some(header) } else { None }
	}

	/// Reads the newest valid header slot at the front of a segment image, or `None` for a
	/// segment without a header.
	///
	/// A plain segment starts with a batch, whose first four bytes are a length no larger
	/// than `max_batch_bytes`, so it never starts with the magic. Every slot write rewrites
	/// the magic with the same bytes, so a header segment keeps it at offset zero even when
	/// the first slot's write tears. The second slot is consulted only behind that magic,
	/// so batch bytes are never searched for an accidental match. A header segment that
	/// lost its first page reads as a plain segment whose first frame is damaged, which
	/// recovery reports as corruption once any batch follows it.
	pub fn read(segment: &[u8]) -> Option<Self> {
		if segment.get(..SEGMENT_HEADER_MAGIC.len()) != Some(&SEGMENT_HEADER_MAGIC[..]) {
			return None;
		}
		let first = segment
			.get(..SEGMENT_HEADER_SLOT_BYTES as usize)
			.and_then(Self::decode_slot);
		let second = segment
			.get(SEGMENT_HEADER_SLOT_BYTES as usize..SEGMENT_HEADER_BYTES as usize)
			.and_then(Self::decode_slot);
		match (first, second) {
			(Some(a), Some(b)) => Some(if b.write_count > a.write_count { b } else { a }),
			(Some(header), None) | (None, Some(header)) => Some(header),
			(None, None) => None,
		}
	}
}
