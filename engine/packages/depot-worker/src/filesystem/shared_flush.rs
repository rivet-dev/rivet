//! One SlateDB memtable flush at a time per tablet store, shared by every caller that
//! needs its rows durable before it goes on.
//!
//! A caller takes a ticket after its write has reached the memtable, and one flush at a
//! time runs on behalf of every ticket taken before it started. A caller whose ticket a
//! finished flush already covers returns at once; the rest wait for the running flush and
//! then run, or join, the next one. A flush covers a ticket when it starts after the ticket
//! was taken: SlateDB freezes the memtable when the flush starts, and the ticket's write
//! was in it by then. A failed flush covers nothing, so every waiter behind it flushes
//! again itself.
//!
//! Every memtable flush of the store runs here. In production only the tablet's flush
//! pass and a split child's import flush: lifecycle changes are maintainer log ops, durable
//! at quorum, and never flush. A tablet whose maintainer log never started, which only
//! state-machine tests run, flushes its checkpoint and fork changes instead.

use anyhow::Result;
use slatedb::{
	Db,
	config::{FlushOptions, FlushType},
};
use std::sync::atomic::{AtomicU64, Ordering};

/// Why the tablet store flushed its memtable. Each actual flush counts once, under the cause
/// of the caller that ran it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FlushCause {
	/// The tablet's flush pass, which writes volume records and cold positions.
	Pass,
	/// A split child's import of its parent's alarm and pending work rows, so a restart
	/// resumes from the imported cursor.
	Split,
	/// A checkpoint change of a tablet whose maintainer log never started.
	Checkpoint,
	/// A fork publication of a tablet whose maintainer log never started.
	Fork,
	/// A fork reference change of a tablet whose maintainer log never started.
	ForkReference,
}

impl FlushCause {
	pub(crate) fn as_str(self) -> &'static str {
		match self {
			Self::Pass => "pass",
			Self::Split => "split",
			Self::Checkpoint => "checkpoint",
			Self::Fork => "fork",
			Self::ForkReference => "fork_reference",
		}
	}

	/// Counts one memtable flush the store ran for this cause.
	pub(crate) fn record(self) {
		crate::metrics::TABLET_STORE_FLUSHES
			.with_label_values(&[self.as_str()])
			.inc();
	}
}

/// Whether a caller ran the flush that made its write durable or found it covered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SharedFlushRole {
	Led,
	Joined,
}

pub(crate) struct SharedFlush {
	/// Tickets taken so far. Ticket `n` is the `n`th write that asked for durability.
	requested: AtomicU64,
	/// Every ticket at or below this one is durable.
	completed: tokio::sync::watch::Sender<u64>,
	/// Held by the caller running a flush, so flushes never overlap and each later one
	/// covers every ticket taken while the earlier one ran.
	leader: tokio::sync::Mutex<()>,
}

impl Default for SharedFlush {
	fn default() -> Self {
		Self {
			requested: AtomicU64::new(0),
			completed: tokio::sync::watch::Sender::new(0),
			leader: tokio::sync::Mutex::new(()),
		}
	}
}

impl SharedFlush {
	/// Makes every write that completed before this call durable. The caller must have
	/// finished its write first. Waiting here is waiting on SlateDB I/O: a leader never
	/// takes a store lock while it holds `leader`, so a caller holding a volume stripe
	/// may wait.
	pub(crate) async fn flush(&self, db: &Db, cause: FlushCause) -> Result<SharedFlushRole> {
		let ticket = self.ticket();
		self.run(db, cause, ticket).await
	}

	

	fn ticket(&self) -> u64 {
		self.requested.fetch_add(1, Ordering::AcqRel) + 1
	}

	/// Runs a flush covering `ticket`, unless one that already finished covers it.
	async fn run(&self, db: &Db, cause: FlushCause, ticket: u64) -> Result<SharedFlushRole> {
		let _leader = self.leader.lock().await;
		if *self.completed.borrow() >= ticket {
			crate::metrics::FILESYSTEM_SHARED_FLUSHES
				.with_label_values(&["joined"])
				.inc();
			return Ok(SharedFlushRole::Joined);
		}

		// Every ticket taken before this load had its write in the memtable before the
		// flush below freezes it.
		let covers = self.requested.load(Ordering::Acquire);
		db.flush_with_options(FlushOptions {
			flush_type: FlushType::MemTable,
		})
		.await?;
		self.completed
			.send_modify(|completed| *completed = (*completed).max(covers));
		cause.record();
		crate::metrics::FILESYSTEM_SHARED_FLUSHES
			.with_label_values(&["led"])
			.inc();
		Ok(SharedFlushRole::Led)
	}
}


