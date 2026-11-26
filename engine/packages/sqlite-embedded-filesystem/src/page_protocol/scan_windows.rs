//! Retains bounded file ranges across the planner's reads at one SQLite head.
use super::database::{Database, PAGE, PageRange, page_ranges_for};
use anyhow::{Context, Result, ensure};
use bytes::Bytes;
use depot_client::filesystem::{CacheReservation, Error};
use std::collections::VecDeque;

const WINDOW_PAGES: u32 = 1024;
const MAX_WINDOWS: usize = 2;
/// Even a completely cold window may fetch at most eight pages per planned cold page.
const MAX_WINDOW_EXPANSION: usize = 8;
/// Require 1 MiB of distinct follow-up reads before widening the next window.
const MIN_FOLLOWUP_PAGES: u32 = WINDOW_PAGES / 4;
pub(super) const MIN_PLANNED_PAGES: usize = 32;

#[derive(Default)]
pub(super) struct ScanWindows {
	buffers: VecDeque<ScanWindow>,
	last_dense: Option<u32>,
	seen: [u64; WINDOW_PAGES as usize / 64],
	initial_pages: u32,
}

impl ScanWindows {
	/// Only pages newly used after the first batch can earn read-ahead in the next window.
	fn enter(&mut self, first: u32, group: &[(u32, usize)]) -> bool {
		if self.last_dense == Some(first) {
			return false;
		}
		let continues = self
			.last_dense
			.is_some_and(|last| last.abs_diff(first) == WINDOW_PAGES)
			&& self.seen_pages() >= self.initial_pages + MIN_FOLLOWUP_PAGES;
		self.last_dense = Some(first);
		self.seen.fill(0);
		for &(pgno, _) in group {
			self.record(pgno - first);
		}
		self.initial_pages = self.seen_pages();
		continues
	}

	fn seen_pages(&self) -> u32 {
		self.seen.iter().map(|word| word.count_ones()).sum()
	}

	fn record(&mut self, offset: u32) {
		self.seen[(offset / 64) as usize] |= 1 << (offset % 64);
	}

	fn planned_page(&mut self, pgno: u32) -> Option<Bytes> {
		let first = window_start(pgno);
		if self.last_dense == Some(first) {
			self.record(pgno - first);
		}
		self.buffers.iter().find_map(|window| window.page(pgno))
	}
}

struct ScanWindow {
	first: u32,
	bytes: Bytes,
}

impl ScanWindow {
	fn page(&self, pgno: u32) -> Option<Bytes> {
		let offset = pgno.checked_sub(self.first)?;
		if offset >= (self.bytes.len() / PAGE) as u32 {
			return None;
		}
		let start = offset as usize * PAGE;
		Some(self.bytes.slice(start..start + PAGE))
	}
}

// Charge the buffer until the last reply slice is dropped, including after eviction.
struct WindowBuffer {
	bytes: Vec<u8>,
	_charge: CacheReservation,
}

impl AsRef<[u8]> for WindowBuffer {
	fn as_ref(&self) -> &[u8] {
		&self.bytes
	}
}

struct WindowFill {
	range: PageRange,
	charge: CacheReservation,
}

impl Database {
	/// Serves an existing prefetch plan without changing the set of pages it returns.
	pub(super) async fn read_planned_pages(&self, pgnos: &[u32]) -> Result<Vec<Option<Bytes>>> {
		ensure!(pgnos.iter().all(|&pgno| pgno > 0), "invalid SQLite page");
		ensure!(!self.fs.is_fenced(), Error::Fenced);
		let mut windows = match self.scan_windows.get() {
			Some(windows) => Some(windows.lock().await),
			None => None,
		};

		let mut pages = vec![None; pgnos.len()];
		let mut misses = Vec::new();
		for (index, &pgno) in pgnos.iter().enumerate() {
			if pgno <= self.root.size_pages {
				pages[index] = windows
					.as_mut()
					.and_then(|windows| windows.planned_page(pgno));
				if pages[index].is_none() {
					misses.push((pgno, index));
				}
			}
		}
		drop(windows);
		misses.sort_unstable();

		let mut exact = Vec::new();
		let mut fill = None;
		for group in misses.chunk_by(|a, b| window_start(a.0) == window_start(b.0)) {
			let first = window_start(group[0].0);
			let len = (self.root.size_pages - first + 1).min(WINDOW_PAGES) as usize;
			let minimum = MIN_PLANNED_PAGES.max(len.div_ceil(MAX_WINDOW_EXPANSION));
			if fill.is_none()
				&& is_dense(group, minimum)
				&& self
					.scan_windows
					.get_or_init(Default::default)
					.lock()
					.await
					.enter(first, group)
				&& self.fs.has_cold_file_chunks(
					self.ino,
					group
						.iter()
						.map(|&(pgno, _)| u64::from(pgno - 1) * PAGE as u64),
					minimum,
				) && let Some(charge) = self.reserve_scan_window(len * PAGE).await
			{
				fill = Some(WindowFill {
					range: PageRange {
						first_pgno: first,
						len,
						pages: group
							.iter()
							.map(|&(pgno, index)| (index, (pgno - first) as usize))
							.collect(),
					},
					charge,
				});
				continue;
			}
			exact.push(group);
		}
		if let Some(fill) = fill {
			let clock = self.fs.clock();
			let deadline = self.fs.deadline()?;
			let ranges = page_ranges_for(exact.into_iter().flatten().copied());
			let widened = self.read_scan_ranges(fill, ranges, &mut pages);
			// Depot gives each queued batch its own timeout. Bound the whole widened read.
			if let Err(error) = rivet_env::timeout_at(clock.as_ref(), deadline, widened)
				.await
				.map_err(|_| Error::DeadlineExceeded)?
			{
				// The smaller plan may fit the limits or avoid the cold read entirely.
				if !matches!(
					error.downcast_ref::<Error>(),
					Some(
						Error::TooLarge
							| Error::Overloaded | Error::ColdReadRefused
							| Error::Lagging | Error::BaseUnavailable
					)
				) {
					return Err(error);
				}
				let exact = self.read_exact_ranges(page_ranges_for(misses), &mut pages);
				rivet_env::timeout_at(clock.as_ref(), deadline, exact)
					.await
					.map_err(|_| Error::DeadlineExceeded)??;
			}
		} else if !misses.is_empty() {
			self.read_exact_ranges(page_ranges_for(misses), &mut pages)
				.await?;
		}
		ensure!(!self.fs.is_fenced(), Error::Fenced);
		Ok(pages)
	}

	async fn reserve_scan_window(&self, bytes: usize) -> Option<CacheReservation> {
		if let Some(charge) = self.fs.reserve_cache(bytes) {
			return Some(charge);
		}
		let windows = self.scan_windows.get()?;
		let oldest = windows.lock().await.buffers.pop_front()?;
		// Reply slices can still hold the reservation, so the retry may fall back to exact reads.
		drop(oldest);
		self.fs.reserve_cache(bytes)
	}

	async fn read_scan_ranges(
		&self,
		fill: WindowFill,
		exact: Vec<PageRange>,
		pages: &mut [Option<Bytes>],
	) -> Result<()> {
		// Depot schedules the window and exact ranges together with its existing batch limits.
		let buffers = self
			.fetch_page_ranges(std::iter::once(&fill.range).chain(&exact))
			.await?;
		let mut buffers = buffers.into_iter();
		let bytes = Bytes::from_owner(WindowBuffer {
			bytes: buffers.next().context("missing SQLite scan window")?,
			_charge: fill.charge,
		});
		fill.range.fill_pages(&bytes, pages);
		let first = fill.range.first_pgno;
		let mut windows = self.scan_windows.get_or_init(Default::default).lock().await;
		windows.buffers.retain(|cached| cached.first != first);
		if windows.buffers.len() == MAX_WINDOWS {
			windows.buffers.pop_front();
		}
		windows.buffers.push_back(ScanWindow { first, bytes });
		drop(windows);
		for (range, bytes) in exact.into_iter().zip(buffers) {
			range.fill_pages(&Bytes::from(bytes), pages);
		}
		Ok(())
	}
}

fn window_start(pgno: u32) -> u32 {
	pgno - (pgno - 1) % WINDOW_PAGES
}

fn is_dense(group: &[(u32, usize)], minimum: usize) -> bool {
	if group.len() < minimum {
		return false;
	}
	let distinct = group.chunk_by(|a, b| a.0 == b.0).count();
	let span = (group[group.len() - 1].0 - group[0].0) as usize + 1;
	distinct >= minimum && span <= distinct * 4
}
