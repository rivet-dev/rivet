//! Shared SQLite execution types for local and remote depot client backends.

pub const HEAD_FENCE_MISMATCH_GROUP: &str = "depot";
pub const HEAD_FENCE_MISMATCH_CODE: &str = "head_fence_mismatch";

pub fn is_head_fence_mismatch(group: &str, code: &str) -> bool {
	group == HEAD_FENCE_MISMATCH_GROUP && code == HEAD_FENCE_MISMATCH_CODE
}

/// Pages per storage shard. Shared rather than each side keeping its own copy: the client cuts
/// staged commit segments on these boundaries and the engine rejects segments that are not cut on
/// them, so two definitions could disagree and make every large commit fail.
pub const SHARD_SIZE: u32 = 64;

/// Shards one staged commit segment may span.
pub const COMMIT_SEGMENT_MAX_SHARDS: u32 = 5;

/// Dirty pages one commit may carry, staged or not.
///
/// A PIDX row is only about 64 bytes of key and value, but that is not what FoundationDB charges
/// against its 10 MB transaction limit. Each `set` also carries a write conflict range, and the
/// conflict ranges dominate: measured against a real cluster, the finalize transaction costs roughly
/// 170 bytes per page, so the hard ceiling is near 61,000 pages. A commit of 60,146 pages publishes
/// and one of 62,152 fails with a non-retryable `transaction_too_large`.
///
/// Half of that measured ceiling is the cap. The remaining headroom is not slack: truncate cleanup
/// deletes every above-EOF PIDX row in the same transaction and is not bounded by this constant at
/// all, so a commit that shrinks a large database pays for pages this number never counted.
///
/// Shared rather than duplicated so the client can refuse an oversized commit before staging any of
/// it. The engine still enforces it, since an old client is not bound by a new value here, but a
/// client that knows the number does not have to send 128 MiB to be told no.
pub const MAX_COMMIT_DIRTY_PAGES: usize = 32_768;

/// Where a staged commit segment carrying `pgno` must start.
pub fn segment_start_for_page(pgno: u32) -> u32 {
	pgno / SHARD_SIZE * SHARD_SIZE
}

/// Cuts pages into shard-aligned segments, each spanning at most `COMMIT_SEGMENT_MAX_SHARDS`.
///
/// The load-bearing property is that no shard's pages are split across two segments. Compaction
/// folds a segment into a shard image, so a split shard could be folded from one segment and written
/// as an image missing the other's newer pages.
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
	pub readonly: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecuteResult {
	pub columns: Vec<String>,
	pub rows: Vec<Vec<ColumnValue>>,
	pub changes: i64,
	pub last_insert_row_id: Option<i64>,
	pub readonly: Option<bool>,
	pub commit_seq: Option<u64>,
}

impl ExecuteResult {
	pub fn into_query_result(self) -> QueryResult {
		QueryResult {
			columns: self.columns,
			rows: self.rows,
			readonly: self.readonly,
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

#[cfg(test)]
mod tests {
	use super::{ColumnValue, ExecuteResult};

	#[test]
	fn execute_result_preserves_result_and_route_metadata() {
		let result = ExecuteResult {
			columns: vec!["id".to_owned(), "name".to_owned()],
			rows: vec![vec![
				ColumnValue::Integer(7),
				ColumnValue::Text("alpha".to_owned()),
			]],
			changes: 3,
			last_insert_row_id: Some(42),
			readonly: Some(false),
			commit_seq: Some(7),
		};

		assert_eq!(result.columns, vec!["id", "name"]);
		assert_eq!(
			result.rows,
			vec![vec![
				ColumnValue::Integer(7),
				ColumnValue::Text("alpha".to_owned())
			]]
		);
		assert_eq!(result.changes, 3);
		assert_eq!(result.last_insert_row_id, Some(42));
	}

	#[test]
	fn execute_result_projects_query_and_exec_results() {
		let result = ExecuteResult {
			columns: vec!["count".to_owned()],
			rows: vec![vec![ColumnValue::Integer(9)]],
			changes: 2,
			last_insert_row_id: Some(10),
			readonly: Some(false),
			commit_seq: Some(8),
		};

		let query_result = result.clone().into_query_result();
		assert_eq!(query_result.columns, vec!["count"]);
		assert_eq!(query_result.rows, vec![vec![ColumnValue::Integer(9)]]);

		let exec_result = result.into_exec_result();
		assert_eq!(exec_result.changes, 2);
	}
}
