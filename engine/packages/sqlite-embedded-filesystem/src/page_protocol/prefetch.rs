//! Drives the SQLite planner against the actor's ordinary Depot files and KV sidecars.
use super::database::{Database, OVERFLOW_PREFIX, PAGE, RECENT_PREFIX, storage_key};
use anyhow::{Result, ensure};
use async_trait::async_trait;
use bytes::Bytes;
use depot_fs_core::{FsKvDelete, FsKvDeleteRange, FsKvPut, FsOp};
use sqlite_prefetch::{CommitBuilder, CommitPlan, KvOp, Sidecar, driver};

/// One owned speculative read, tied to the database head that admitted it. The scope cancels
/// on drop; ordinary reads, commits and lane shutdown also join the task explicitly.
pub(super) struct BackgroundRows {
	head: u64,
	scope: rivet_env::TaskScope,
	task: rivet_env::tokio::task::JoinHandle<Option<Vec<Result<sqlite_prefetch::LookupResult>>>>,
}

struct Store<'a> {
	database: &'a Database,
}

#[async_trait]
impl driver::Store for Store<'_> {
	fn refused(&self, error: &anyhow::Error) -> bool {
		super::refused_without_running(error)
	}

	async fn read_pages(&self, pages: &[u32]) -> Result<Vec<Option<Bytes>>> {
		self.database.read_pages(pages).await
	}
	async fn kv_get(&self, sidecar: Sidecar, key: &[u8]) -> Result<Option<Vec<u8>>> {
		Ok(self
			.database
			.fs
			.kv_get(
				self.database.prefetch_ino,
				&storage_key(prefix(sidecar), key),
			)
			.await?)
	}
	async fn kv_scan(
		&self,
		sidecar: Sidecar,
		start: &[u8],
		end: &[u8],
		limit: usize,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		let prefix = prefix(sidecar);
		let start = storage_key(prefix, start);
		let end = storage_key(prefix, end);
		let rows = self
			.database
			.fs
			.kv_scan(
				self.database.prefetch_ino,
				(
					std::ops::Bound::Included(start.as_slice()),
					std::ops::Bound::Included(end.as_slice()),
				),
				limit,
			)
			.await?;
		// Both bounds carry the prefix, so every row does; the planner sees its own keys.
		Ok(rows
			.into_iter()
			.map(|(key, value)| (key[1..].to_vec(), value))
			.collect())
	}
}

impl Database {
	/// Starts leaf metadata lookups while the caller sends its page reply. This reads no
	/// pages or durable state and cannot change the planner until the owning lane joins it.
	pub fn start_prefetch(&mut self, env: &std::sync::Arc<dyn rivet_env::Env>) {
		if self.background.is_some() || self.pending_lookups.is_empty() {
			return;
		}
		let heads: Vec<_> = std::mem::take(&mut self.pending_lookups)
			.into_iter()
			.filter_map(|lookup| match lookup {
				sqlite_prefetch::Lookup::Head(head) => Some(head),
				_ => None,
			})
			.collect();
		if heads.is_empty() {
			return;
		}
		let fs = self.fs.clone();
		let ino = self.prefetch_ino;
		let scope = env.task_scope();
		let task = scope.spawn_result(async move {
			use futures_util::StreamExt;
			futures_util::stream::iter(heads)
				.map(|head| {
					let fs = fs.clone();
					async move {
						Ok(sqlite_prefetch::LookupResult::Head {
							head,
							value: fs
								.kv_get(ino, &storage_key(OVERFLOW_PREFIX, &head.to_be_bytes()))
								.await?,
						})
					}
				})
				.buffer_unordered(16)
				.collect()
				.await
		});
		self.background = Some(BackgroundRows {
			head: self.root.txid,
			scope,
			task,
		});
	}

	async fn finish_prefetch(&mut self) {
		let Some(background) = self.background.take() else {
			return;
		};
		if !background.task.is_finished() {
			// Optional metadata must not put an unrelated demand read or write behind its
			// storage latency. Cancellation drops only reads; discard knowledge of the
			// pending heads so a later request can resolve them again at the current head.
			background.scope.shutdown().await;
			let _ = background.task.await;
			self.prefetcher.lookups_cancelled();
			self.pending_lookups.clear();
			return;
		}
		let results = background.task.await;
		background.scope.shutdown().await;
		if background.head != self.root.txid {
			return;
		}
		if let Ok(Some(results)) = results {
			for result in results {
				match result {
					Ok(result) => self.pending_lookups.extend(self.prefetcher.apply(result)),
					Err(error) => tracing::debug!(?error, "SQLite leaf metadata read failed"),
				}
			}
		}
	}

	/// Stops and joins speculative reads before releasing the filesystem opening.
	pub async fn shutdown_prefetch(&mut self) {
		if let Some(background) = self.background.take() {
			background.scope.shutdown().await;
			let _ = background.task.await;
		}
		self.pending_lookups.clear();
	}

	fn retain_prefetch(&mut self) {
		self.prefetch_charge.take();
		self.prefetch_charge = self.fs.reserve_cache(self.prefetcher.memory_bytes());
		if self.prefetch_charge.is_none() {
			self.prefetcher = sqlite_prefetch::Prefetcher::open(
				Default::default(),
				self.prefetcher.size_pages(),
				0,
				Default::default(),
			);
			self.pending_lookups.clear();
		}
	}

	fn take_prefetcher(&mut self) -> sqlite_prefetch::Prefetcher {
		// A failed/cancelled request leaves a conservative empty cache; durable rows remain
		// authoritative. This also lets the adapter borrow the rest of the database while
		// the pure planner is driven.
		std::mem::replace(
			&mut self.prefetcher,
			sqlite_prefetch::Prefetcher::open(
				Default::default(),
				self.root.size_pages,
				0,
				Default::default(),
			),
		)
	}

	/// Sets whether later page replies carry predicted pages. The engine reads this from its
	/// runtime config on every request.
	pub fn set_server_prefetch(&mut self, on: bool) {
		self.server_prefetch = on;
	}

	pub(super) async fn planned_pages(
		&mut self,
		request_id: u64,
		requested: &[u32],
		opening: bool,
	) -> Result<Vec<rivet_envoy_protocol::SqliteFetchedPage>> {
		self.finish_prefetch().await;
		if !self.server_prefetch {
			let bytes = self.read_pages(requested).await?;
			ensure!(
				bytes.len() == requested.len(),
				"page read returned the wrong count"
			);
			return Ok(requested
				.iter()
				.zip(bytes)
				.map(|(pgno, bytes)| rivet_envoy_protocol::SqliteFetchedPage { pgno: *pgno, bytes })
				.collect());
		}
		let now = self.fs.clock().observe()?.boot_ms;
		let mut planner = self.take_prefetcher();
		let pending = std::mem::take(&mut self.pending_lookups);
		let store = Store { database: self };
		if let Err(error) = driver::run_lookups(&mut planner, &store, pending, "leaf").await {
			tracing::debug!(?error, "SQLite overflow metadata read failed");
		}
		let result = driver::read(&mut planner, &store, request_id, requested, now, opening).await;
		self.prefetcher = planner;
		let reply = result?;
		self.pending_lookups = reply.after_reply;
		self.retain_prefetch();
		Ok(reply
			.pages
			.into_iter()
			.map(|(pgno, bytes)| rivet_envoy_protocol::SqliteFetchedPage { pgno, bytes })
			.collect())
	}

	pub(super) async fn prefetch_commit(
		&mut self,
		builder: &CommitBuilder,
		size: u32,
		entry_pages: usize,
	) -> Result<CommitPlan> {
		self.finish_prefetch().await;
		let limit = self
			.fs
			.max_append_bytes()?
			.min(self.fs.max_transaction_bytes());
		let used = entry_pages.saturating_mul(PAGE + 256).saturating_add(512);
		let remaining_payload = self
			.fs
			.max_transaction_bytes()
			.saturating_sub(builder.len().saturating_mul(PAGE));
		// A range tombstone carries no application bytes. At the full transaction cap it
		// must still invalidate old rows, while optional row values must not overflow it.
		let budget = limit.saturating_sub(used).min(
			remaining_payload.saturating_add(sqlite_prefetch::overflow::commit::KV_OP_OVERHEAD),
		);
		// Even dropping an unusable map requires one atomic range tombstone.
		ensure!(
			budget >= sqlite_prefetch::overflow::commit::KV_OP_OVERHEAD,
			"SQLite commit leaves no room for derived metadata"
		);
		let mut planner = self.take_prefetcher();
		let plan = driver::plan_commit(
			&mut planner,
			&Store { database: self },
			builder,
			size,
			self.root.txid.saturating_add(1),
			budget,
		)
		.await;
		self.prefetcher = planner;
		plan
	}

	pub(super) fn final_edits(&self, size: u32) -> Vec<depot_client::filesystem::FileEdit> {
		use depot_client::filesystem::FileEdit;
		let mut edits = vec![FileEdit::Truncate(u64::from(size) * PAGE as u64)];
		if self.remove_old_hint {
			edits.push(FileEdit::RemoveXattrIfPresent(
				b"rivet.sqlite.recent_pages".to_vec(),
			));
		}
		edits
	}

	pub(super) fn prefetch_ops(&self, plan: &CommitPlan) -> Vec<FsOp> {
		let ops: Vec<_> = plan
			.ops
			.iter()
			.map(|op| match op {
				KvOp::Put {
					sidecar,
					key,
					value,
				} => FsOp::FsKvPut(FsKvPut {
					ino: self.prefetch_ino,
					key: storage_key(prefix(*sidecar), key).into(),
					value: value.clone().into(),
				}),
				KvOp::Delete { sidecar, key } => FsOp::FsKvDelete(FsKvDelete {
					ino: self.prefetch_ino,
					key: storage_key(prefix(*sidecar), key).into(),
				}),
				// A range ends inside its own prefix: with no end, at the next prefix.
				KvOp::DeleteRange {
					sidecar,
					start,
					end,
				} => FsOp::FsKvDeleteRange(FsKvDeleteRange {
					ino: self.prefetch_ino,
					start: storage_key(prefix(*sidecar), start).into(),
					end: Some(match end {
						Some(end) => storage_key(prefix(*sidecar), end).into(),
						None => vec![prefix(*sidecar) + 1].into(),
					}),
				}),
			})
			.collect();
		ops
	}

	pub(super) fn prefetch_committed(&mut self, plan: CommitPlan) {
		self.prefetcher.committed(plan);
		self.pending_lookups.clear();
		self.remove_old_hint = false;
		self.retain_prefetch();
	}
}

/// The key prefix of a planner sidecar in the database's one prefetch inode.
fn prefix(sidecar: Sidecar) -> u8 {
	match sidecar {
		Sidecar::Recent => RECENT_PREFIX,
		Sidecar::Overflow => OVERFLOW_PREFIX,
	}
}

impl Drop for Database {
	fn drop(&mut self) {
		self.prefetcher.close();
	}
}


