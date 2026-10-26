//! Shared SQLite execution types for local and remote depot client backends.

mod page_bytes;

pub use page_bytes::PageBytes;

pub const HEAD_FENCE_MISMATCH_GROUP: &str = "depot";
pub const HEAD_FENCE_MISMATCH_CODE: &str = "head_fence_mismatch";

pub fn is_head_fence_mismatch(group: &str, code: &str) -> bool {
	group == HEAD_FENCE_MISMATCH_GROUP && code == HEAD_FENCE_MISMATCH_CODE
}

/// The error a remote SQLite request reports when its transaction changed more pages than one
/// commit may carry.
pub const TRANSACTION_TOO_LARGE_GROUP: &str = "sqlite";
pub const TRANSACTION_TOO_LARGE_CODE: &str = "transaction_too_large";

pub fn is_transaction_too_large(group: &str, code: &str) -> bool {
	group == TRANSACTION_TOO_LARGE_GROUP && code == TRANSACTION_TOO_LARGE_CODE
}

/// Pages per storage shard. Shared rather than each side keeping its own copy: the client cuts
/// staged commit segments on these boundaries and the engine rejects segments that are not cut on
/// them, so two definitions could disagree and make every large commit fail.
pub const SHARD_SIZE: u32 = 64;

/// Shards one staged commit segment may span.
pub const COMMIT_SEGMENT_MAX_SHARDS: u32 = 5;

/// Actor-side ceiling: 128 MiB of 4 KiB pages. The server additionally enforces
/// its configured filesystem transaction byte cap before admitting each segment.
/// Large commits stream as bounded segments and publish at one commit sequence.
pub const MAX_COMMIT_DIRTY_PAGES: usize = 32_768;

/// Where a staged commit segment carrying `pgno` must start.
pub fn segment_start_for_page(pgno: u32) -> u32 {
	pgno / SHARD_SIZE * SHARD_SIZE
}

/// Cuts pages into shard-aligned segments, each spanning at most `COMMIT_SEGMENT_MAX_SHARDS`.
///
/// Shard alignment remains part of the existing wire protocol; the server now
/// writes these pages into one ordinary SQLite file through FS transactions.
///
/// `pages` must be sorted ascending by page number; the caller sorts once rather than this sorting
/// per call.
pub fn cut_page_segments<T>(pages: &[T], pgno_of: impl Fn(&T) -> u32) -> Vec<(u32, &[T])> {
	let mut segments = Vec::new();
	let mut start = 0;
	while start < pages.len() {
		let first_shard = pgno_of(&pages[start]) / SHARD_SIZE;
		// Saturating, so pages at the very top of the page space cut one final segment rather than
		// wrapping into an empty one and looping.
		let shard_limit = first_shard.saturating_add(COMMIT_SEGMENT_MAX_SHARDS);
		let end = pages[start..]
			.iter()
			.position(|page| pgno_of(page) / SHARD_SIZE >= shard_limit)
			.map_or(pages.len(), |offset| start + offset);
		segments.push((first_shard * SHARD_SIZE, &pages[start..end]));
		start = end;
	}

	segments
}

#[derive(Clone, Debug, PartialEq)]
pub enum BindParam {
	Null,
	Integer(i64),
	Float(f64),
	Text(String),
	Blob(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecResult {
	pub changes: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueryResult {
	pub columns: Vec<String>,
	pub rows: Vec<Vec<ColumnValue>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecuteResult {
	pub columns: Vec<String>,
	pub rows: Vec<Vec<ColumnValue>>,
	pub changes: i64,
	pub last_insert_row_id: Option<i64>,
}

impl ExecuteResult {
	pub fn into_query_result(self) -> QueryResult {
		QueryResult {
			columns: self.columns,
			rows: self.rows,
		}
	}

	pub fn into_exec_result(self) -> ExecResult {
		ExecResult {
			changes: self.changes,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColumnValue {
	Null,
	Integer(i64),
	Float(f64),
	Text(String),
	Blob(Vec<u8>),
}


