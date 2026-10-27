//! An in-process page server that follows the Rivet 3.x engine's SQLite page protocol.
//!
//! It keeps the parts of `sqlite-embedded-filesystem`'s page protocol a VFS can observe: a
//! commit's head is the sequence of the filesystem entry it wrote, which grows by more than one
//! per commit; every read and write checks `expected_head_txid`; and a write without one is only
//! accepted while the database is empty. Tests can also lose a commit reply after the commit
//! applied, hold replies, fail commits before they apply, or move the head as another writer.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Once};

use anyhow::{Result, bail};
use async_trait::async_trait;
use rivet_envoy_protocol as protocol;
use sqlite_embedded::vfs::{SqliteCommitIndeterminateError, SqliteTransport};
use sqlite_types::{HEAD_FENCE_MISMATCH_CODE, HEAD_FENCE_MISMATCH_GROUP};
use tokio::sync::{Mutex, watch};

/// The page server's head before any commit. A new 3.x file already has a nonzero sequence.
pub const INITIAL_HEAD: u64 = 5;
/// How far each commit moves the head, as filesystem entries other than the commit's own
/// page writes take sequences too.
pub const HEAD_STEP: u64 = 3;

/// Flush retry settings for the test process. The flags are read once per process.
pub fn init_flags() {
	static INIT: Once = Once::new();
	INIT.call_once(|| unsafe {
		// SAFETY: runs once, before any test opens a database and reads the flags.
		std::env::set_var("RIVETKIT_SQLITE_OPT_FLUSH_RETRY_DEADLINE_MS", "1500");
		std::env::set_var("RIVETKIT_SQLITE_OPT_FLUSH_RETRY_BACKOFF_MIN_MS", "10");
		std::env::set_var("RIVETKIT_SQLITE_OPT_FLUSH_RETRY_BACKOFF_MAX_MS", "50");
	});
}

#[derive(Default)]
struct Storage {
	pages: BTreeMap<u32, Vec<u8>>,
	size_pages: u32,
	head: u64,
	lose_next_reply: bool,
	fail_commits: bool,
}

pub struct PageServer {
	storage: Mutex<Storage>,
	hold_replies: watch::Sender<bool>,
	applied_commits: watch::Sender<u64>,
}

impl PageServer {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			storage: Mutex::new(Storage {
				head: INITIAL_HEAD,
				..Storage::default()
			}),
			hold_replies: watch::Sender::new(false),
			applied_commits: watch::Sender::new(0),
		})
	}

	pub async fn head(&self) -> u64 {
		self.storage.lock().await.head
	}

	/// Applies the next commit but answers it with an unknown outcome, as when the connection
	/// drops after the request was sent.
	pub async fn lose_next_reply(&self) {
		self.storage.lock().await.lose_next_reply = true;
	}

	/// Refuses every commit before applying it, as an unreachable engine would.
	pub async fn fail_commits(&self) {
		self.storage.lock().await.fail_commits = true;
	}

	/// Moves the head as a commit from some other writer would.
	pub async fn write_as_another_writer(&self) {
		self.storage.lock().await.head += HEAD_STEP;
	}

	/// Holds commit replies after the commits apply, until released.
	pub fn hold_replies(&self, hold: bool) {
		self.hold_replies.send_replace(hold);
	}

	pub fn applied_commits(&self) -> u64 {
		*self.applied_commits.borrow()
	}

	pub async fn wait_for_applied_commits(&self, count: u64) {
		let mut applied = self.applied_commits.subscribe();
		let _ = applied.wait_for(|applied| *applied >= count).await;
	}
}

fn fence_mismatch() -> protocol::SqliteErrorResponse {
	protocol::SqliteErrorResponse {
		group: HEAD_FENCE_MISMATCH_GROUP.to_string().into(),
		code: HEAD_FENCE_MISMATCH_CODE.to_string().into(),
		message: "SQLite head fence mismatch".to_string().into(),
	}
}

#[async_trait]
impl SqliteTransport for PageServer {
	async fn get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		let storage = self.storage.lock().await;
		if request
			.expected_head_txid
			.is_some_and(|expected| expected != storage.head)
		{
			return Ok(protocol::SqliteGetPagesResponse::SqliteErrorResponse(
				fence_mismatch(),
			));
		}
		let pages = request
			.pgnos
			.iter()
			.map(|pgno| protocol::SqliteFetchedPage {
				pgno: *pgno,
				bytes: (*pgno <= storage.size_pages)
					.then(|| storage.pages.get(pgno).cloned())
					.flatten()
					.map(Into::into),
			})
			.collect();
		Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
			protocol::SqliteGetPagesOk {
				pages,
				head_txid: Some(storage.head),
			},
		))
	}

	async fn commit(
		&self,
		request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		let (head, lost) = {
			let mut storage = self.storage.lock().await;
			if storage.fail_commits {
				bail!("page server is unreachable");
			}
			let fenced = match request.expected_head_txid {
				None => storage.size_pages != 0,
				Some(expected) => expected != storage.head,
			};
			if fenced {
				return Ok(protocol::SqliteCommitResponse::SqliteErrorResponse(
					fence_mismatch(),
				));
			}
			for page in request.dirty_pages {
				if page.pgno <= request.db_size_pages {
					storage.pages.insert(page.pgno, page.bytes.to_vec());
				}
			}
			storage.size_pages = request.db_size_pages;
			let size_pages = storage.size_pages;
			storage.pages.retain(|pgno, _| *pgno <= size_pages);
			storage.head += HEAD_STEP;
			let lost = std::mem::take(&mut storage.lose_next_reply);
			(storage.head, lost)
		};
		self.applied_commits.send_modify(|applied| *applied += 1);
		let mut hold = self.hold_replies.subscribe();
		let _ = hold.wait_for(|hold| !*hold).await;
		if lost {
			return Err(SqliteCommitIndeterminateError("commit".to_string()).into());
		}
		Ok(protocol::SqliteCommitResponse::SqliteCommitOk(
			protocol::SqliteCommitOk {
				head_txid: Some(head),
			},
		))
	}

	async fn commit_stage_begin(
		&self,
		_request: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse> {
		bail!("these tests keep every commit under the staging threshold")
	}

	async fn commit_stage_segment(
		&self,
		_request: protocol::SqliteCommitStageSegmentRequest,
	) -> Result<protocol::SqliteCommitStageSegmentResponse> {
		bail!("these tests keep every commit under the staging threshold")
	}

	async fn commit_finalize(
		&self,
		_request: protocol::SqliteCommitFinalizeRequest,
	) -> Result<protocol::SqliteCommitFinalizeResponse> {
		bail!("these tests keep every commit under the staging threshold")
	}
}
