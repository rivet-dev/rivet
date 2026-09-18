//! Client-side batching for range scans.
//!
//! FoundationDB never hands a client the whole of a range in one fetch. It returns a batch sized by
//! the streaming mode, flags whether more remains, and the client loops until the range is drained.
//! The RocksDB and Postgres drivers have no equivalent of that batching built in, so this module
//! supplies it: [`chunk_budget`] turns a streaming mode into the per-fetch row and byte budget the
//! driver enforces, and [`stream_range`] runs the continuation loop that turns those fetches into a
//! stream.
//!
//! Without this a caller that streams a large range and stops early still pays for the entire range
//! up front, because the driver materializes all of it before the first item is yielded.

use std::future::Future;

use anyhow::Result;
use futures_util::{StreamExt, stream};

use crate::{
	options::StreamingMode,
	range_option::RangeOption,
	value::{Stream, Value, Values},
};

/// Row and byte ceilings for a single fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkBudget {
	pub rows: usize,
	/// `0` means no byte ceiling, matching `RangeOption::target_bytes`.
	pub bytes: usize,
}

const KIB: usize = 1024;

/// Per-fetch budget for `iteration` of a scan in `mode`, where the first fetch is iteration 1.
///
/// The budgets mirror the shape of FoundationDB's client batching rather than its exact tables.
/// `Exact` delivers the caller's whole row limit in one fetch. `Iterator` starts small and grows, so
/// a caller that reads a handful of rows and stops does not pay for a large fetch, while a caller
/// that drains a large range reaches an efficient batch size within a few round trips. The remaining
/// modes are fixed, sized by how much the caller said it was willing to over-read.
pub fn chunk_budget(mode: StreamingMode, iteration: usize, limit: Option<usize>) -> ChunkBudget {
	// Iteration is 1-based, so index 0 is the first fetch.
	let step = iteration.saturating_sub(1);

	match mode {
		// A row limit is required for this mode. Treat a missing one as `Large` rather than
		// returning the whole range, since the point of chunking is that no mode does that.
		StreamingMode::Exact => match limit {
			Some(limit) => ChunkBudget {
				rows: limit,
				bytes: 0,
			},
			None => ChunkBudget {
				rows: usize::MAX,
				bytes: 1024 * KIB,
			},
		},
		StreamingMode::Small => ChunkBudget {
			rows: 256,
			bytes: 32 * KIB,
		},
		StreamingMode::Medium => ChunkBudget {
			rows: 4_096,
			bytes: 256 * KIB,
		},
		StreamingMode::Large | StreamingMode::WantAll => ChunkBudget {
			rows: 16_384,
			bytes: 1024 * KIB,
		},
		StreamingMode::Serial => ChunkBudget {
			rows: 65_536,
			bytes: 4096 * KIB,
		},
		StreamingMode::Iterator => {
			const ROWS: [usize; 5] = [256, 1_024, 4_096, 16_384, 65_536];
			const BYTES: [usize; 5] = [32 * KIB, 128 * KIB, 512 * KIB, 1024 * KIB, 4096 * KIB];

			let step = step.min(ROWS.len() - 1);
			ChunkBudget {
				rows: ROWS[step],
				bytes: BYTES[step],
			}
		}
	}
}

/// Drive `fetch` over successive chunks of `opt` until the range is drained, yielding each row as it
/// arrives.
///
/// `fetch` receives the range for one chunk, already narrowed to that chunk's budget, plus the
/// 1-based iteration. The caller's own `limit` is tracked across chunks and is what bounds the
/// total number of rows yielded, so the budget only decides how the work is split up.
pub fn stream_range<'a, F, Fut>(opt: RangeOption<'a>, fetch: F) -> Stream<'a, Value>
where
	F: Fn(RangeOption<'a>, usize) -> Fut + Send + 'a,
	Fut: Future<Output = Result<Values>> + Send + 'a,
{
	// `None` marks the scan as finished, which is how a fetch error terminates the stream after
	// yielding the error rather than retrying the same chunk forever. The fetch itself rides in the
	// state so the loop owns it outright, rather than borrowing it out of the closure.
	let init = (fetch, Some((opt, 1usize)));

	let chunks = stream::unfold(init, |(fetch, state)| async move {
		let Some((opt, iteration)) = state else {
			return None;
		};

		let budget = chunk_budget(opt.mode, iteration, opt.limit);
		let mut chunk_opt = opt.clone();
		chunk_opt.limit = Some(match opt.limit {
			Some(limit) => limit.min(budget.rows),
			None => budget.rows,
		});
		chunk_opt.target_bytes = budget.bytes;

		let values = match fetch(chunk_opt, iteration).await {
			Ok(values) => values,
			Err(err) => return Some((Err(err), (fetch, None))),
		};

		let next = opt.next_range(&values).map(|opt| (opt, iteration + 1));

		Some((Ok(values), (fetch, next)))
	});

	Box::pin(
		chunks
			.map(|chunk| match chunk {
				Ok(values) => values
					.into_iter()
					.map(|kv| Ok(Value::from_keyvalue(kv)))
					.collect::<Vec<_>>(),
				Err(err) => vec![Err(err)],
			})
			.flat_map(stream::iter),
	)
}

#[cfg(test)]
#[path = "../tests/unit/chunk.rs"]
mod tests;
