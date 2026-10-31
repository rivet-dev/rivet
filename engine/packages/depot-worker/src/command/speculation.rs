//! Speculative first commands of a volume the host creates.
//!
//! A command marked `SPECULATIVE_CREATE` that opens a missing volume first runs against an
//! empty volume held in memory, up to its first commit. Its writes become the creating open's
//! initial writes, so the replicas, their claim and the command's commit share one disk write.
//! The commit waits for the open's answer. When the open created the volume with those writes,
//! the commit succeeds and the command continues to its effects and result: the volume was
//! empty, so the handler's reads of nothing were true, and its writes are now durable.
//! Only the command's first write transaction runs on the capture. Every later write
//! transaction of its handle, the command's own or one of an effect it started, waits for the
//! open's answer and runs on the opened volume. Snapshots claim nothing: they read the capture
//! until the open answers and the opened volume after. The command resumes after its creating
//! commit only once that volume's filesystem is open, so a speculative handler must commit or
//! drop its first write transaction before it begins another.
//! Otherwise the commit fails before any effect, the run is dropped, and the command runs as
//! usual on the opened volume, where it reads whatever history the volume holds.

use bytes::Bytes;
use depot_client::filesystem::{Filesystem, capture::CaptureJournal};
use rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody;
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};
use tokio::sync::watch;

/// The first command of a volume this open may create, run speculatively before the open.
#[derive(Clone)]
pub(crate) struct Speculation {
	pub tag: depot_protocol::command::CommandTag,
	pub operation_id: [u8; 16],
	pub body: Bytes,
	/// The deadline tracker of the command whose admission started the open.
	pub commits: Arc<super::CommandCommits>,
}

/// A first command run against an empty volume up to its first commit, which waits for the
/// creating open's answer through `journal`.
pub(crate) struct SpeculativeRun {
	pub operation_id: [u8; 16],
	/// The opening state the command ran with. When its writes create the volume, the command
	/// was the first of that opening, so the opening keeps this state.
	pub shared: std::sync::Arc<super::OpeningState>,
	/// The range locks the command holds. When its writes create the volume, the opening
	/// keeps them, so its later commands lock against the same ranges.
	pub locked_ranges: Arc<parking_lot::Mutex<std::collections::BTreeMap<Bytes, super::LockEntry>>>,
	/// Where the command's handle runs its later transactions.
	pub writer: SpeculationGuard,
	/// The deadline tracker of the command whose admission started the open.
	pub commits: Arc<super::CommandCommits>,
	pub writes: Vec<VolumeLogEntryBody>,
	pub journal: Arc<CaptureJournal>,
	pub handler: futures_util::future::BoxFuture<'static, anyhow::Result<Vec<u8>>>,
}

/// A speculative command's result, kept until that command collects it.
pub(crate) struct Speculated {
	pub operation_id: [u8; 16],
	pub result: anyhow::Result<Vec<u8>>,
	/// Whether the caller that started the open timed out before the command finished. Its
	/// later work was then refused, so a retry runs the command again on the opened volume
	/// instead of collecting this result.
	pub caller_timed_out: bool,
}

/// Where a speculative command's handle runs its transactions after the first one: the opened
/// volume's filesystem and `kv` inode once the creating commit made the volume, or nothing when
/// the speculation failed.
/// The opened volume's filesystem and `kv` inode.
type OpenedWriter = (Arc<Filesystem>, u64);

pub(crate) struct SpeculativeWriter {
	first_claimed: AtomicBool,
	/// `None` until the open answers; then the opened writer, or `None` inside when it failed.
	outcome: watch::Sender<Option<Option<OpenedWriter>>>,
}

impl SpeculativeWriter {
	/// Whether this call is the handle's first transaction, which runs on the capture.
	pub(crate) fn claim_first_transaction(&self) -> bool {
		!self.first_claimed.swap(true, Ordering::AcqRel)
	}

	/// The opened writer, once the creating commit moved the handle there.
	pub(crate) fn moved(&self) -> Option<OpenedWriter> {
		self.outcome.borrow().clone().flatten()
	}

	/// Waits for the open's answer. The host resolves it before the open returns or is dropped.
	pub(crate) async fn outcome(&self) -> Option<OpenedWriter> {
		let mut outcome = self.outcome.subscribe();
		match outcome.wait_for(|outcome| outcome.is_some()).await {
			Ok(outcome) => outcome.clone().flatten(),
			Err(_) => None,
		}
	}
}

/// The host's side of a [`SpeculativeWriter`]. Dropping it unresolved fails the speculation, so
/// a waiting transaction never outlives an open that was cancelled or failed.
pub(crate) struct SpeculationGuard(Arc<SpeculativeWriter>);

impl SpeculationGuard {
	pub(crate) fn new() -> Self {
		Self(Arc::new(SpeculativeWriter {
			first_claimed: AtomicBool::new(false),
			outcome: watch::Sender::new(None),
		}))
	}

	pub(crate) fn writer(&self) -> Arc<SpeculativeWriter> {
		self.0.clone()
	}

	/// Answers every waiting and later transaction of the handle; only the first answer counts.
	pub(crate) fn resolve(&self, moved: Option<OpenedWriter>) {
		self.0.outcome.send_if_modified(|outcome| match outcome {
			Some(_) => false,
			None => {
				*outcome = Some(moved);
				true
			}
		});
	}
}

impl Drop for SpeculationGuard {
	fn drop(&mut self) {
		self.resolve(None);
	}
}
