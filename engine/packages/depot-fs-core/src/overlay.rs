//! The overlay a child volume reads as: its own rows on top of each base link's rows at that
//! link's cut, nearest first, down to the root. A pure layer merge that knows nothing of
//! tablets or transport: it takes what each level answered, whiteouts included, and yields the
//! visible rows. Every read path of a volume with a base calls it and nothing else knows about
//! links.
//!
//! The rules are the same for every row kind. A point read takes the nearest level that holds
//! a version of the key: a row is the answer, and a whiteout, a delete the level keeps because
//! the volume has a base, means absent whatever deeper levels hold. A range read is one merged
//! iteration over the levels in key order, where a nearer level's row or whiteout hides the
//! same key from deeper levels, and a nearer level's range whiteout hides the KV keys it spans
//! from deeper levels. A level never hides itself: its own rows in a range it whited out are
//! tombstones of their own, and a row it wrote after the whiteout is visible.
use crate::store::{Row, RowKey};

/// What one level holds of a key: no version, a whiteout, or a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Layered {
	Absent,
	Whiteout,
	Row(Row),
}

impl From<Option<Option<Row>>> for Layered {
	fn from(version: Option<Option<Row>>) -> Self {
		match version {
			None => Self::Absent,
			Some(None) => Self::Whiteout,
			Some(Some(row)) => Self::Row(row),
		}
	}
}

/// A range whiteout one level holds: the KV keys of file `ino` from `start` (inclusive) to
/// `end` (exclusive, unbounded when `None`) are absent below that level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RangeWhiteout {
	pub ino: u64,
	pub start: Vec<u8>,
	pub end: Option<Vec<u8>>,
}

impl RangeWhiteout {
	/// Whether `key` is a KV key this whiteout spans.
	pub fn covers(&self, key: &RowKey) -> bool {
		let RowKey::Kv(ino, key) = key else {
			return false;
		};
		*ino == self.ino && key >= &self.start && self.end.as_ref().is_none_or(|end| key < end)
	}

	/// The whiteout a stored range whiteout row describes, or `None` for any other row.
	pub fn from_row(key: &RowKey, row: &Row) -> Option<Self> {
		match (key, row) {
			(RowKey::KvRangeWhiteout(ino, start), Row::KvRangeWhiteout(end)) => Some(Self {
				ino: *ino,
				start: start.clone(),
				end: end.clone(),
			}),
			_ => None,
		}
	}
}

/// Resolves a point read over the levels nearest first, each with the range whiteouts it holds
/// that may span the key. The first level that holds a version of the key decides; a nearer
/// level's range whiteout spanning the key hides every deeper level.
pub fn resolve<'a>(
	levels: impl IntoIterator<Item = (Layered, &'a [RangeWhiteout])>,
	key: &RowKey,
) -> Option<Row> {
	for (level, whiteouts) in levels {
		match level {
			Layered::Row(row) => return Some(row),
			Layered::Whiteout => return None,
			Layered::Absent => {}
		}
		if whiteouts.iter().any(|whiteout| whiteout.covers(key)) {
			return None;
		}
	}
	None
}

/// What one level answered for a page of a scan: its rows and whiteouts after the cursor in
/// key order, whether the level has nothing after them, the range whiteouts it holds that may
/// span the range, and the last key it examined when that is past its last row, as when the
/// level left out rows that never read through.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Level {
	pub rows: Vec<(RowKey, Option<Row>)>,
	pub complete: bool,
	pub whiteouts: Vec<RangeWhiteout>,
	pub last_key: Option<RowKey>,
}

/// One page of a merged scan. `next_after` is the last key the merge decided, the cursor the
/// next page continues from; `complete` says every level was read to its end, so there is no
/// next page. A page may be short of `limit` and incomplete at once, when it stopped where the
/// shortest level's page ended or hid every key it saw, and the caller then reads on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MergedPage {
	pub rows: Vec<(RowKey, Row)>,
	pub next_after: Option<RowKey>,
	pub complete: bool,
}

/// Merges one page of each level, nearest first, into at most `limit` visible rows.
///
/// A level's page is a prefix of what the level holds after the cursor, so the merge can decide
/// a key only while every level that may still hold rows has shown everything up to it: it
/// stops at the last key of the shortest incomplete page. Every level's rows are assumed sorted
/// and after the same cursor.
pub fn merge(levels: &[Level], limit: usize) -> MergedPage {
	// The last key the merge may decide: the smallest last key among the incomplete pages, each
	// the key the level examined last, whether or not it answered a row for it.
	let bound = levels
		.iter()
		.filter(|level| !level.complete)
		.filter_map(|level| {
			let last_row = level.rows.last().map(|(key, _)| key);
			match (&level.last_key, last_row) {
				(Some(examined), Some(row)) => Some(examined.max(row)),
				(Some(examined), None) => Some(examined),
				(None, row) => row,
			}
		})
		.min()
		.cloned();
	let mut stopped_by_limit = false;
	let mut heads = vec![0usize; levels.len()];
	let mut page = MergedPage::default();
	loop {
		if page.rows.len() >= limit {
			stopped_by_limit = true;
			break;
		}
		// The next key in order across the levels.
		let Some(key) = levels
			.iter()
			.zip(&heads)
			.filter_map(|(level, head)| level.rows.get(*head).map(|(key, _)| key))
			.min()
			.cloned()
		else {
			break;
		};
		if bound.as_ref().is_some_and(|bound| &key > bound) {
			break;
		}
		// Decide the key nearest first: a level with a version answers; a level without one
		// but with a range whiteout spanning the key hides the deeper levels.
		let mut visible = None;
		for (index, level) in levels.iter().enumerate() {
			let head = heads[index];
			let held = level.rows.get(head).filter(|(found, _)| *found == key);
			if let Some((_, value)) = held {
				heads[index] += 1;
				if visible.is_none() {
					visible = Some(value.clone());
				}
				continue;
			}
			if visible.is_none() && level.whiteouts.iter().any(|whiteout| whiteout.covers(&key)) {
				visible = Some(None);
			}
		}
		if let Some(Some(row)) = visible {
			page.rows.push((key.clone(), row));
		}
		page.next_after = Some(key);
	}
	page.complete = levels
		.iter()
		.zip(&heads)
		.all(|(level, head)| level.complete && *head == level.rows.len());
	// Every key through the bound was decided when the merge ran out of rows, including keys
	// a level examined and left out, so the next page continues past the bound.
	if !page.complete
		&& !stopped_by_limit
		&& let Some(bound) = bound
		&& page.next_after.as_ref().is_none_or(|next| next < &bound)
	{
		page.next_after = Some(bound);
	}
	page
}

/// Adds `added` to a KV file's range whiteouts, merging it with every whiteout it overlaps or
/// touches, so the file keeps one row per disjoint deleted range however often it is range
/// deleted. The result is sorted by start. Whiteouts of other files are kept as they are.
pub fn coalesce(mut whiteouts: Vec<RangeWhiteout>, added: RangeWhiteout) -> Vec<RangeWhiteout> {
	let mut merged = added;
	let mut rest = Vec::with_capacity(whiteouts.len() + 1);
	for whiteout in whiteouts.drain(..) {
		if whiteout.ino != merged.ino {
			rest.push(whiteout);
			continue;
		}
		let starts_inside = |range: &RangeWhiteout, start: &Vec<u8>| {
			start >= &range.start && range.end.as_ref().is_none_or(|end| start <= end)
		};
		if starts_inside(&merged, &whiteout.start) || starts_inside(&whiteout, &merged.start) {
			if whiteout.start < merged.start {
				merged.start = whiteout.start;
			}
			merged.end = match (merged.end.take(), whiteout.end) {
				(Some(a), Some(b)) => Some(a.max(b)),
				(None, _) | (_, None) => None,
			};
		} else {
			rest.push(whiteout);
		}
	}
	rest.push(merged);
	rest.sort_by(|a, b| a.ino.cmp(&b.ino).then_with(|| a.start.cmp(&b.start)));
	rest
}
