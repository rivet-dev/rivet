//! The contiguous read-ahead window, as a kernel's: it opens on two adjacent misses, doubles
//! each time the next miss lands just past what it prefetched, and closes after two misses that
//! continue no walk. It follows walks in either direction.
//!
//! The server sees misses only, so the window cannot see SQLite consume it. It does not need
//! to: a walk that keeps going misses exactly at the window's far edge, as the kernel's async
//! read-ahead marker does.
use crate::page::Pgno;

pub const INITIAL_PAGES: u32 = 16;
/// The largest window: 1 MiB of 4 KiB pages.
pub const MAX_PAGES: u32 = 256;
/// Misses that continue no walk before the window closes.
const STRAY_MISSES: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
	Forward,
	Backward,
}

#[derive(Debug, Clone, Copy)]
struct Open {
	direction: Direction,
	/// The far end of what the window last prefetched.
	edge: Pgno,
	size: u32,
}

#[derive(Debug, Default)]
pub struct Window {
	open: Option<Open>,
	last_miss: Option<Pgno>,
	strays: u32,
}

/// Pages a window asks for, in walk order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
	pub first: Pgno,
	pub count: u32,
	pub forward: bool,
}

impl Span {
	pub fn pages(&self) -> impl Iterator<Item = Pgno> + '_ {
		(0..self.count).map(move |index| match self.forward {
			true => self.first.saturating_add(index),
			false => self.first.saturating_sub(index),
		})
	}
}

impl Window {
	/// Feeds a miss and returns the pages to prefetch past it, if the window is open. Pages
	/// outside `1..=size_pages` are not returned.
	pub fn miss(&mut self, pgno: Pgno, size_pages: u32) -> Option<Span> {
		let last = self.last_miss.replace(pgno);
		if let Some(open) = self.open {
			let past_edge = match open.direction {
				Direction::Forward => open.edge.checked_add(1) == Some(pgno),
				Direction::Backward => open.edge.checked_sub(1) == Some(pgno),
			};
			if past_edge {
				self.strays = 0;
				let size = (open.size * 2).min(MAX_PAGES);
				return self.extend(pgno, open.direction, size, size_pages);
			}
		}
		let direction = match last {
			Some(last) if last.checked_add(1) == Some(pgno) => Some(Direction::Forward),
			Some(last) if last.checked_sub(1) == Some(pgno) => Some(Direction::Backward),
			_ => None,
		};
		match direction {
			Some(direction) => {
				self.strays = 0;
				let size = match self.open {
					Some(open) if open.direction == direction => open.size,
					_ => INITIAL_PAGES,
				};
				self.extend(pgno, direction, size, size_pages)
			}
			None => {
				self.strays += 1;
				if self.strays >= STRAY_MISSES {
					self.open = None;
				}
				None
			}
		}
	}

	fn extend(
		&mut self,
		pgno: Pgno,
		direction: Direction,
		size: u32,
		size_pages: u32,
	) -> Option<Span> {
		let span = match direction {
			Direction::Forward => {
				let first = pgno.checked_add(1)?;
				let count = size.min(size_pages.saturating_sub(pgno));
				Span {
					first,
					count,
					forward: true,
				}
			}
			Direction::Backward => {
				let first = pgno.checked_sub(1).filter(|first| *first >= 1)?;
				let count = size.min(first);
				Span {
					first,
					count,
					forward: false,
				}
			}
		};
		if span.count == 0 {
			self.open = None;
			return None;
		}
		let edge = span.pages().last()?;
		self.open = Some(Open {
			direction,
			edge,
			size,
		});
		Some(span)
	}

	/// Moves the window's edge back to the last page the reply actually carried, when the
	/// reply cap cut the window short, so the next miss past what was sent continues the walk.
	pub fn sent_through(&mut self, last_sent: Pgno) {
		if let Some(open) = self.open.as_mut() {
			open.edge = last_sent;
		}
	}

	pub fn is_open(&self) -> bool {
		self.open.is_some()
	}
}
