use std::time::Duration;

use rivet_envoy_protocol as ep;

use super::{metrics, utils::EntryValidationErrorKind};

const SQLITE_PREFIX: u8 = 0x08;
const SQLITE_SCHEMA_VERSION: u8 = 0x01;
const SQLITE_META_PREFIX: u8 = 0x00;
const SQLITE_CHUNK_PREFIX: u8 = 0x01;
pub const PATH_GENERIC: &str = "generic";
pub const PATH_FAST_PATH: &str = "fast_path";
const OP_READ: &str = "read";
const OP_WRITE: &str = "write";
const OP_TRUNCATE: &str = "truncate";
const ENTRY_PAGE: &str = "page";
const ENTRY_METADATA: &str = "metadata";
const BYTE_REQUEST: &str = "request";
const BYTE_RESPONSE: &str = "response";
const BYTE_PAYLOAD: &str = "payload";
const PHASE_ESTIMATE_KV_SIZE: &str = "estimate_kv_size";
const PHASE_CLEAR_AND_REWRITE: &str = "clear_and_rewrite";
const VALIDATION_OK: &str = "ok";
const VALIDATION_LENGTH_MISMATCH: &str = "length_mismatch";
const VALIDATION_TOO_MANY_ENTRIES: &str = "too_many_entries";
const VALIDATION_PAYLOAD_TOO_LARGE: &str = "payload_too_large";
const VALIDATION_STORAGE_QUOTA_EXCEEDED: &str = "storage_quota_exceeded";
const VALIDATION_KEY_TOO_LARGE: &str = "key_too_large";
const VALIDATION_VALUE_TOO_LARGE: &str = "value_too_large";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SqliteOpSummary {
	matched: bool,
	page_count: u64,
	metadata_count: u64,
	request_bytes: u64,
	payload_bytes: u64,
}

impl SqliteOpSummary {
	pub fn matched(&self) -> bool {
		self.matched
	}

	pub fn entry_count(&self) -> u64 {
		self.page_count + self.metadata_count
	}
}

pub fn summarize_get(keys: &[ep::KvKey]) -> Option<SqliteOpSummary> {
	let mut summary = SqliteOpSummary::default();

	for key in keys {
		match classify_key(key) {
			Some(EntryKind::Page) => {
				summary.matched = true;
				summary.page_count += 1;
				summary.request_bytes += key.len() as u64;
			}
			Some(EntryKind::Metadata) => {
				summary.matched = true;
				summary.metadata_count += 1;
				summary.request_bytes += key.len() as u64;
			}
			None => {}
		}
	}

	summary.matched.then_some(summary)
}

pub fn summarize_response(keys: &[ep::KvKey], values: &[ep::KvValue]) -> u64 {
	keys.iter()
		.zip(values.iter())
		.filter_map(|(key, value)| classify_key(key).map(|_| value.len() as u64))
		.sum()
}

pub fn summarize_put(keys: &[ep::KvKey], values: &[ep::KvValue]) -> Option<SqliteOpSummary> {
	let mut summary = SqliteOpSummary::default();

	for (key, value) in keys.iter().zip(values.iter()) {
		match classify_key(key) {
			Some(EntryKind::Page) => {
				summary.matched = true;
				summary.page_count += 1;
				summary.request_bytes += (key.len() + value.len()) as u64;
				summary.payload_bytes += value.len() as u64;
			}
			Some(EntryKind::Metadata) => {
				summary.matched = true;
				summary.metadata_count += 1;
				summary.request_bytes += (key.len() + value.len()) as u64;
				summary.payload_bytes += value.len() as u64;
			}
			None => {}
		}
	}

	summary.matched.then_some(summary)
}

pub fn summarize_delete_range(start: &ep::KvKey, end: &ep::KvKey) -> Option<SqliteOpSummary> {
	let start_chunk = parse_chunk_key(start)?;
	let end_kind = parse_delete_range_end(end)?;
	let file_tag = start_chunk.file_tag;
	let matched = match end_kind {
		DeleteRangeEnd::Chunk(end_chunk) => end_chunk.file_tag == file_tag,
		DeleteRangeEnd::ChunkRangeEnd(end_file_tag) => end_file_tag == file_tag + 1,
	};

	if !matched {
		return None;
	}

	let page_count = match end_kind {
		DeleteRangeEnd::Chunk(end_chunk) if end_chunk.chunk_index >= start_chunk.chunk_index => {
			(end_chunk.chunk_index - start_chunk.chunk_index) as u64
		}
		_ => 0,
	};

	Some(SqliteOpSummary {
		matched: true,
		page_count,
		metadata_count: 0,
		request_bytes: (start.len() + end.len()) as u64,
		payload_bytes: 0,
	})
}

pub fn summarize_write_batch(
	file_tag: u8,
	meta_value: &[u8],
	page_updates: &[ep::SqlitePageUpdate],
) -> SqliteOpSummary {
	let page_request_bytes = page_updates.iter().fold(0_u64, |acc, update| {
		acc + sqlite_chunk_key(file_tag, update.chunk_index).len() as u64 + update.data.len() as u64
	});
	let payload_bytes = meta_value.len() as u64
		+ page_updates
			.iter()
			.map(|update| update.data.len() as u64)
			.sum::<u64>();

	SqliteOpSummary {
		matched: true,
		page_count: page_updates.len() as u64,
		metadata_count: 1,
		request_bytes: sqlite_meta_key(file_tag).len() as u64
			+ meta_value.len() as u64
			+ page_request_bytes,
		payload_bytes,
	}
}

pub fn summarize_truncate(
	file_tag: u8,
	meta_value: &[u8],
	delete_chunks_from: u32,
	tail_chunk: Option<&ep::SqlitePageUpdate>,
) -> SqliteOpSummary {
	let tail_request_bytes = tail_chunk.map_or(0_u64, |tail| {
		sqlite_chunk_key(file_tag, tail.chunk_index).len() as u64 + tail.data.len() as u64
	});
	let tail_payload_bytes = tail_chunk.map_or(0_u64, |tail| tail.data.len() as u64);

	SqliteOpSummary {
		matched: true,
		page_count: tail_chunk.map_or(0, |_| 1),
		metadata_count: 1,
		request_bytes: sqlite_chunk_key(file_tag, delete_chunks_from).len() as u64
			+ sqlite_chunk_range_end(file_tag).len() as u64
			+ sqlite_meta_key(file_tag).len() as u64
			+ meta_value.len() as u64
			+ tail_request_bytes,
		payload_bytes: meta_value.len() as u64 + tail_payload_bytes,
	}
}

pub fn record_operation(op: OperationKind, summary: SqliteOpSummary, duration: Duration) {
	record_operation_for_path(PATH_GENERIC, op, summary, duration);
}

pub fn record_operation_for_path(
	path: &'static str,
	op: OperationKind,
	summary: SqliteOpSummary,
	duration: Duration,
) {
	if !summary.matched() {
		return;
	}

	let op = op.as_str();
	metrics::ACTOR_KV_SQLITE_STORAGE_REQUEST_TOTAL
		.with_label_values(&[path, op])
		.inc();
	if summary.page_count > 0 {
		metrics::ACTOR_KV_SQLITE_STORAGE_ENTRY_TOTAL
			.with_label_values(&[path, op, ENTRY_PAGE])
			.inc_by(summary.page_count);
	}
	if summary.metadata_count > 0 {
		metrics::ACTOR_KV_SQLITE_STORAGE_ENTRY_TOTAL
			.with_label_values(&[path, op, ENTRY_METADATA])
			.inc_by(summary.metadata_count);
	}
	if summary.request_bytes > 0 {
		metrics::ACTOR_KV_SQLITE_STORAGE_BYTES_TOTAL
			.with_label_values(&[path, op, BYTE_REQUEST])
			.inc_by(summary.request_bytes);
	}
	if summary.payload_bytes > 0 {
		metrics::ACTOR_KV_SQLITE_STORAGE_BYTES_TOTAL
			.with_label_values(&[path, op, BYTE_PAYLOAD])
			.inc_by(summary.payload_bytes);
	}
	metrics::ACTOR_KV_SQLITE_STORAGE_DURATION_SECONDS_TOTAL
		.with_label_values(&[path, op])
		.inc_by(duration.as_secs_f64());
}

pub fn record_response_bytes(bytes: u64) {
	if bytes == 0 {
		return;
	}

	metrics::ACTOR_KV_SQLITE_STORAGE_BYTES_TOTAL
		.with_label_values(&[PATH_GENERIC, OP_READ, BYTE_RESPONSE])
		.inc_by(bytes);
}

pub fn record_phase_duration(phase: PhaseKind, duration: Duration) {
	record_phase_duration_for_path(PATH_GENERIC, phase, duration);
}

pub fn record_phase_duration_for_path(path: &'static str, phase: PhaseKind, duration: Duration) {
	metrics::ACTOR_KV_SQLITE_STORAGE_PHASE_DURATION_SECONDS_TOTAL
		.with_label_values(&[path, phase.as_str()])
		.inc_by(duration.as_secs_f64());
}

pub fn record_clear_subspace(count: u64) {
	record_clear_subspace_for_path(PATH_GENERIC, count);
}

pub fn record_clear_subspace_for_path(path: &'static str, count: u64) {
	if count == 0 {
		return;
	}

	metrics::ACTOR_KV_SQLITE_STORAGE_CLEAR_SUBSPACE_TOTAL
		.with_label_values(&[path])
		.inc_by(count);
}

pub fn record_validation(kind: Option<EntryValidationErrorKind>) {
	record_validation_for_path(PATH_GENERIC, kind);
}

pub fn record_validation_for_path(path: &'static str, kind: Option<EntryValidationErrorKind>) {
	let result = match kind {
		None => VALIDATION_OK,
		Some(EntryValidationErrorKind::LengthMismatch) => VALIDATION_LENGTH_MISMATCH,
		Some(EntryValidationErrorKind::TooManyEntries) => VALIDATION_TOO_MANY_ENTRIES,
		Some(EntryValidationErrorKind::PayloadTooLarge) => VALIDATION_PAYLOAD_TOO_LARGE,
		Some(EntryValidationErrorKind::StorageQuotaExceeded) => VALIDATION_STORAGE_QUOTA_EXCEEDED,
		Some(EntryValidationErrorKind::KeyTooLarge) => VALIDATION_KEY_TOO_LARGE,
		Some(EntryValidationErrorKind::ValueTooLarge) => VALIDATION_VALUE_TOO_LARGE,
	};

	metrics::ACTOR_KV_SQLITE_STORAGE_VALIDATION_TOTAL
		.with_label_values(&[path, result])
		.inc();
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationKind {
	Read,
	Write,
	Truncate,
}

impl OperationKind {
	fn as_str(&self) -> &'static str {
		match self {
			OperationKind::Read => OP_READ,
			OperationKind::Write => OP_WRITE,
			OperationKind::Truncate => OP_TRUNCATE,
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhaseKind {
	EstimateKvSize,
	ClearAndRewrite,
}

impl PhaseKind {
	fn as_str(&self) -> &'static str {
		match self {
			PhaseKind::EstimateKvSize => PHASE_ESTIMATE_KV_SIZE,
			PhaseKind::ClearAndRewrite => PHASE_CLEAR_AND_REWRITE,
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntryKind {
	Page,
	Metadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ChunkKey {
	file_tag: u8,
	chunk_index: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeleteRangeEnd {
	Chunk(ChunkKey),
	ChunkRangeEnd(u8),
}

pub fn sqlite_file_tag_for_key(key: &[u8]) -> Option<u8> {
	if key.len() == 8
		&& key[0] == SQLITE_PREFIX
		&& key[1] == SQLITE_SCHEMA_VERSION
		&& key[2] == SQLITE_CHUNK_PREFIX
	{
		return Some(key[3]);
	}

	if key.len() == 4
		&& key[0] == SQLITE_PREFIX
		&& key[1] == SQLITE_SCHEMA_VERSION
		&& key[2] == SQLITE_META_PREFIX
	{
		return Some(key[3]);
	}

	None
}

pub fn sqlite_file_tag_for_delete_range(start: &[u8], end: &[u8]) -> Option<u8> {
	let start_chunk = parse_chunk_key(start)?;
	let end_kind = parse_delete_range_end(end)?;
	let file_tag = start_chunk.file_tag;

	match end_kind {
		DeleteRangeEnd::Chunk(end_chunk) if end_chunk.file_tag == file_tag => Some(file_tag),
		DeleteRangeEnd::ChunkRangeEnd(end_file_tag) if end_file_tag == file_tag + 1 => {
			Some(file_tag)
		}
		_ => None,
	}
}

fn classify_key(key: &[u8]) -> Option<EntryKind> {
	if key.len() == 8 && sqlite_file_tag_for_key(key).is_some() {
		return Some(EntryKind::Page);
	}

	if key.len() == 4 && sqlite_file_tag_for_key(key).is_some() {
		return Some(EntryKind::Metadata);
	}

	None
}

fn parse_chunk_key(key: &[u8]) -> Option<ChunkKey> {
	if key.len() != 8
		|| key[0] != SQLITE_PREFIX
		|| key[1] != SQLITE_SCHEMA_VERSION
		|| key[2] != SQLITE_CHUNK_PREFIX
	{
		return None;
	}

	Some(ChunkKey {
		file_tag: key[3],
		chunk_index: u32::from_be_bytes([key[4], key[5], key[6], key[7]]),
	})
}

fn parse_delete_range_end(key: &[u8]) -> Option<DeleteRangeEnd> {
	if let Some(chunk_key) = parse_chunk_key(key) {
		return Some(DeleteRangeEnd::Chunk(chunk_key));
	}

	if key.len() == 4
		&& key[0] == SQLITE_PREFIX
		&& key[1] == SQLITE_SCHEMA_VERSION
		&& key[2] == SQLITE_CHUNK_PREFIX
	{
		return Some(DeleteRangeEnd::ChunkRangeEnd(key[3]));
	}

	None
}

fn sqlite_meta_key(file_tag: u8) -> ep::KvKey {
	vec![
		SQLITE_PREFIX,
		SQLITE_SCHEMA_VERSION,
		SQLITE_META_PREFIX,
		file_tag,
	]
}

fn sqlite_chunk_key(file_tag: u8, chunk_index: u32) -> ep::KvKey {
	let mut key = vec![
		SQLITE_PREFIX,
		SQLITE_SCHEMA_VERSION,
		SQLITE_CHUNK_PREFIX,
		file_tag,
	];
	key.extend_from_slice(&chunk_index.to_be_bytes());
	key
}

fn sqlite_chunk_range_end(file_tag: u8) -> ep::KvKey {
	vec![
		SQLITE_PREFIX,
		SQLITE_SCHEMA_VERSION,
		SQLITE_CHUNK_PREFIX,
		file_tag + 1,
	]
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn summarize_sqlite_put_counts_page_and_meta_entries() {
		let keys = vec![
			vec![0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x03],
			vec![0x08, 0x01, 0x00, 0x00],
			vec![0x99],
		];
		let values = vec![vec![1; 4096], vec![2; 8], vec![3; 32]];
		let summary = summarize_put(&keys, &values).expect("should classify sqlite put");

		assert_eq!(summary.page_count, 1);
		assert_eq!(summary.metadata_count, 1);
		assert_eq!(summary.payload_bytes, 4104);
		assert_eq!(summary.request_bytes, 4116);
	}

	#[test]
	fn summarize_sqlite_get_ignores_non_sqlite_keys() {
		let keys = vec![
			vec![0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x03],
			vec![0x01, 0x02, 0x03],
		];
		let summary = summarize_get(&keys).expect("should classify sqlite get");

		assert_eq!(summary.page_count, 1);
		assert_eq!(summary.metadata_count, 0);
		assert_eq!(summary.request_bytes, 8);
	}

	#[test]
	fn summarize_sqlite_delete_range_matches_chunk_range_end() {
		let start = vec![0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x02];
		let end = vec![0x08, 0x01, 0x01, 0x01];
		let summary =
			summarize_delete_range(&start, &end).expect("should classify sqlite truncate");

		assert!(summary.matched);
		assert_eq!(summary.page_count, 0);
		assert_eq!(summary.request_bytes, 12);
	}

	#[test]
	fn summarize_sqlite_delete_range_counts_explicit_end_chunk() {
		let start = vec![0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x02];
		let end = vec![0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x05];
		let summary =
			summarize_delete_range(&start, &end).expect("should classify sqlite truncate");

		assert_eq!(summary.page_count, 3);
	}

	#[test]
	fn summarize_response_counts_only_sqlite_values() {
		let keys = vec![
			vec![0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x03],
			vec![0x99],
		];
		let values = vec![vec![1; 32], vec![2; 64]];

		assert_eq!(summarize_response(&keys, &values), 32);
	}

	#[test]
	fn summarize_sqlite_truncate_counts_meta_and_tail_payload() {
		let summary = summarize_truncate(
			0,
			&vec![9; 10],
			3,
			Some(&ep::SqlitePageUpdate {
				chunk_index: 3,
				data: vec![7; 128],
			}),
		);

		assert!(summary.matched);
		assert_eq!(summary.page_count, 1);
		assert_eq!(summary.metadata_count, 1);
		assert_eq!(summary.payload_bytes, 138);
		assert_eq!(summary.request_bytes, 162);
	}
}
