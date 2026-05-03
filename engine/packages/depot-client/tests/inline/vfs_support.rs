use std::collections::BTreeMap;
use std::sync::{
	Arc,
	atomic::{AtomicBool, AtomicU64, Ordering},
	mpsc,
};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use depot::{
	cold_tier::{ColdTier, ColdTierObjectMetadata},
	conveyer::{Db, db::CompactionSignaler},
	error::SqliteStorageError,
	fault::DepotFaultController,
	keys::{
		SHARD_SIZE, branch_compaction_cold_shard_key, branch_compaction_root_key,
		branch_delta_chunk_key, branch_meta_head_key, branch_pidx_key, branch_shard_key,
	},
	ltx::{LtxHeader, encode_ltx_v3},
	types::{
		ColdShardRef, CompactionRoot, DatabaseBranchId, DirtyPage, decode_db_head,
		encode_cold_shard_ref, encode_compaction_root,
	},
	workflows::compaction::DeltasAvailable,
};
use parking_lot::Mutex;
use rivet_envoy_protocol as protocol;
use rivet_pools::{__rivet_util::Id, NodeId};
use sha2::{Digest, Sha256};
use universaldb::utils::IsolationLevel::Serializable;

pub(crate) struct DirectStorage {
	db: Arc<universaldb::Database>,
	node_id: NodeId,
	cold_tier: Option<Arc<dyn ColdTier>>,
	actor_dbs: scc::HashMap<String, Arc<Db>>,
	page_mirrors: scc::HashMap<String, Arc<Mutex<DirectActorPages>>>,
	compaction_signals: Arc<Mutex<Vec<DeltasAvailable>>>,
	strict: AtomicBool,
	counters: Arc<DirectStorageCounters>,
	fault_controller: Option<DepotFaultController>,
	pub(crate) hooks: Arc<DirectTransportHooks>,
}

#[derive(Clone, Default)]
pub(crate) struct DirectActorPages {
	pub(crate) db_size_pages: u32,
	pub(crate) pages: BTreeMap<u32, Vec<u8>>,
}

impl DirectStorage {
	pub(crate) fn new(db: universaldb::Database) -> Self {
		Self::new_inner(db, None, None)
	}

	pub(crate) fn new_with_cold_tier(
		db: universaldb::Database,
		cold_tier: Arc<dyn ColdTier>,
	) -> Self {
		Self::new_inner(db, Some(cold_tier), None)
	}

	pub(crate) fn new_with_cold_tier_and_fault_controller(
		db: universaldb::Database,
		cold_tier: Arc<dyn ColdTier>,
		fault_controller: DepotFaultController,
	) -> Self {
		Self::new_inner(db, Some(cold_tier), Some(fault_controller))
	}

	fn new_inner(
		db: universaldb::Database,
		cold_tier: Option<Arc<dyn ColdTier>>,
		fault_controller: Option<DepotFaultController>,
	) -> Self {
		let counters = Arc::new(DirectStorageCounters::default());
		let cold_tier = cold_tier.map(|inner| {
			Arc::new(CountingColdTier {
				inner,
				counters: Arc::clone(&counters),
			}) as Arc<dyn ColdTier>
		});

		Self {
			db: Arc::new(db),
			node_id: NodeId::new(),
			cold_tier,
			actor_dbs: scc::HashMap::new(),
			page_mirrors: scc::HashMap::new(),
			compaction_signals: Arc::new(Mutex::new(Vec::new())),
			strict: AtomicBool::new(false),
			counters,
			fault_controller,
			hooks: Arc::new(DirectTransportHooks::default()),
		}
	}

	pub(crate) async fn actor_db(&self, actor_id: String) -> Arc<Db> {
		let signals = Arc::clone(&self.compaction_signals);
		let cold_tier = self.cold_tier.clone();
		self.actor_dbs
			.entry_async(actor_id.clone())
			.await
			.or_insert_with(|| {
				let compaction_signaler: CompactionSignaler = Arc::new(move |signal| {
					let signals = Arc::clone(&signals);
					Box::pin(async move {
						signals.lock().push(signal);
						Ok(())
					})
				});
				Arc::new(
					if let Some(fault_controller) = self.fault_controller.clone() {
						Db::new_with_compaction_signaler_and_fault_controller_for_test(
							Arc::clone(&self.db),
							Id::nil(),
							actor_id,
							self.node_id,
							cold_tier,
							compaction_signaler,
							fault_controller,
						)
					} else {
						Db::new_with_compaction_signaler(
							Arc::clone(&self.db),
							Id::nil(),
							actor_id,
							self.node_id,
							cold_tier,
							compaction_signaler,
						)
					},
				)
			})
			.get()
			.clone()
	}

	pub(crate) async fn evict_actor_db(&self, actor_id: &str) {
		let _ = self.actor_dbs.remove_async(&actor_id.to_string()).await;
	}

	pub(crate) fn enable_strict_mode(&self) {
		self.strict.store(true, Ordering::SeqCst);
	}

	pub(crate) fn is_strict_mode(&self) -> bool {
		self.strict.load(Ordering::SeqCst)
	}

	pub(crate) fn stats(&self) -> DirectStorageStats {
		DirectStorageStats {
			depot_get_pages: self.counters.depot_get_pages.load(Ordering::SeqCst),
			mirror_reads: self.counters.mirror_reads.load(Ordering::SeqCst),
			mirror_fills: self.counters.mirror_fills.load(Ordering::SeqCst),
			mirror_seeds: self.counters.mirror_seeds.load(Ordering::SeqCst),
			cold_gets: self.counters.cold_gets.load(Ordering::SeqCst),
		}
	}

	pub(crate) fn depot_database(&self) -> Arc<universaldb::Database> {
		Arc::clone(&self.db)
	}

	pub(crate) fn cold_tier(&self) -> Option<Arc<dyn ColdTier>> {
		self.cold_tier.clone()
	}

	pub(crate) async fn poison_mirror_page(
		&self,
		actor_id: &str,
		pgno: u32,
		bytes: Vec<u8>,
		db_size_pages: u32,
	) {
		let mirror = self.page_mirror(actor_id.to_string()).await;
		let mut mirror = mirror.lock();
		mirror.db_size_pages = db_size_pages;
		mirror.pages.insert(pgno, bytes);
	}

	pub(crate) async fn seed_page_as_cold_ref(
		&self,
		actor_id: &str,
		pgno: u32,
		bytes: Vec<u8>,
	) -> Result<()> {
		let snapshot = self.snapshot_pages(actor_id).await;
		let mut dirty_pages = snapshot
			.pages
			.iter()
			.filter(|(candidate_pgno, _)| **candidate_pgno / SHARD_SIZE == pgno / SHARD_SIZE)
			.map(|(pgno, bytes)| DirtyPage {
				pgno: *pgno,
				bytes: bytes.clone(),
			})
			.collect::<Vec<_>>();
		if dirty_pages.is_empty() {
			dirty_pages.push(DirtyPage { pgno, bytes });
		}
		self.seed_pages_as_cold_ref(actor_id, pgno, dirty_pages)
			.await
	}

	pub(crate) async fn seed_pages_as_cold_ref(
		&self,
		actor_id: &str,
		pgno: u32,
		dirty_pages: Vec<DirtyPage>,
	) -> Result<()> {
		let Some(cold_tier) = &self.cold_tier else {
			bail!("direct storage has no cold tier");
		};
		let (branch_id, head_txid) = self.read_branch_head(actor_id).await?;
		let shard_id = pgno / SHARD_SIZE;
		let object_key = format!(
			"db/{}/strict/shard-{shard_id}.ltx",
			branch_id.as_uuid().simple()
		);
		if dirty_pages.is_empty() {
			bail!("cold-ref seed requires at least one page");
		}
		let object_bytes = encode_ltx_v3(LtxHeader::delta(head_txid, 1, 1_000), &dirty_pages)?;
		let digest = Sha256::digest(&object_bytes);
		let mut content_hash = [0_u8; 32];
		content_hash.copy_from_slice(&digest);
		let cold_ref = ColdShardRef {
			object_key: object_key.clone(),
			object_generation_id: Id::nil(),
			shard_id,
			as_of_txid: head_txid,
			min_txid: 1,
			max_txid: head_txid,
			min_versionstamp: [1; 16],
			max_versionstamp: [2; 16],
			size_bytes: object_bytes.len() as u64,
			content_hash,
			publish_generation: 1,
		};

		cold_tier.put_object(&object_key, &object_bytes).await?;

		self.db
			.run(move |tx| {
				let cold_ref = cold_ref.clone();
				async move {
					tx.informal().set(
						&branch_compaction_cold_shard_key(branch_id, shard_id, head_txid),
						&encode_cold_shard_ref(cold_ref)?,
					);
					tx.informal().set(
						&branch_compaction_root_key(branch_id),
						&encode_compaction_root(CompactionRoot {
							schema_version: 1,
							manifest_generation: 1,
							hot_watermark_txid: head_txid,
							cold_watermark_txid: head_txid,
							cold_watermark_versionstamp: [2; 16],
						})?,
					);
					tx.informal().clear(&branch_pidx_key(branch_id, pgno));
					for txid in 1..=head_txid {
						tx.informal()
							.clear(&branch_delta_chunk_key(branch_id, txid, 0));
						tx.informal()
							.clear(&branch_shard_key(branch_id, shard_id, txid));
					}
					Ok(())
				}
			})
			.await
	}

	pub(crate) async fn read_branch_head(&self, actor_id: &str) -> Result<(DatabaseBranchId, u64)> {
		let actor_id = actor_id.to_string();
		self.db
			.run(move |tx| {
				let actor_id = actor_id.clone();
				async move {
					let branch_id = depot::conveyer::branch::resolve_database_branch(
						&tx,
						depot::types::BucketId::from_gas_id(Id::nil()),
						&actor_id,
						Serializable,
					)
					.await?
					.context("database branch should exist")?;
					let head = tx
						.informal()
						.get(&branch_meta_head_key(branch_id), Serializable)
						.await?
						.context("database head should exist")?;
					Ok((branch_id, decode_db_head(&head)?.head_txid))
				}
			})
			.await
	}

	async fn page_mirror(&self, actor_id: String) -> Arc<Mutex<DirectActorPages>> {
		self.page_mirrors
			.entry_async(actor_id)
			.await
			.or_insert_with(|| Arc::new(Mutex::new(DirectActorPages::default())))
			.get()
			.clone()
	}

	pub(crate) async fn get_pages(
		&self,
		actor_id: &str,
		pgnos: &[u32],
	) -> anyhow::Result<Vec<depot::types::FetchedPage>> {
		if let Some(message) = self.hooks.take_get_pages_error() {
			return Err(anyhow::anyhow!(message));
		}

		let actor_db = self.actor_db(actor_id.to_string()).await;
		self.counters.depot_get_pages.fetch_add(1, Ordering::SeqCst);
		match actor_db.get_pages(pgnos.to_vec()).await {
			Ok(pages) if self.strict.load(Ordering::SeqCst) => Ok(pages),
			Ok(pages) => self.fill_from_mirror(actor_id, pgnos, pages).await,
			Err(err) => {
				if matches!(
					depot_error(&err),
					Some(SqliteStorageError::MetaMissing { operation })
						if *operation == "get_pages"
				) {
					if self.strict.load(Ordering::SeqCst) {
						return Err(anyhow::anyhow!(
							"strict DirectStorage forbids mirror fallback for missing depot metadata"
						));
					}
					Ok(self.read_mirror(actor_id, pgnos).await)
				} else {
					Err(err)
				}
			}
		}
	}

	async fn fill_from_mirror(
		&self,
		actor_id: &str,
		pgnos: &[u32],
		pages: Vec<depot::types::FetchedPage>,
	) -> anyhow::Result<Vec<depot::types::FetchedPage>> {
		self.counters.mirror_fills.fetch_add(1, Ordering::SeqCst);
		if self.strict.load(Ordering::SeqCst) {
			return Err(anyhow::anyhow!(
				"strict DirectStorage forbids mirror-backed cache seeding"
			));
		}

		let mut by_pgno = pages
			.into_iter()
			.map(|page| (page.pgno, page))
			.collect::<BTreeMap<_, _>>();
		let mirror_pages = self.read_mirror(actor_id, pgnos).await;
		for page in mirror_pages {
			if page.bytes.is_some()
				|| by_pgno
					.get(&page.pgno)
					.is_none_or(|existing| existing.bytes.is_none())
			{
				by_pgno.insert(page.pgno, page);
			}
		}
		Ok(pgnos
			.iter()
			.map(|pgno| {
				by_pgno.remove(pgno).unwrap_or(depot::types::FetchedPage {
					pgno: *pgno,
					bytes: None,
				})
			})
			.collect())
	}

	async fn read_mirror(&self, actor_id: &str, pgnos: &[u32]) -> Vec<depot::types::FetchedPage> {
		self.counters.mirror_reads.fetch_add(1, Ordering::SeqCst);
		let mirror = self.page_mirror(actor_id.to_string()).await;
		let mirror = mirror.lock();
		pgnos
			.iter()
			.map(|pgno| depot::types::FetchedPage {
				pgno: *pgno,
				bytes: if *pgno <= mirror.db_size_pages {
					mirror.pages.get(pgno).cloned()
				} else {
					None
				},
			})
			.collect()
	}

	pub(crate) async fn apply_commit(
		&self,
		actor_id: &str,
		dirty_pages: Vec<depot::types::DirtyPage>,
		db_size_pages: u32,
	) -> anyhow::Result<()> {
		self.counters.mirror_seeds.fetch_add(1, Ordering::SeqCst);
		if self.strict.load(Ordering::SeqCst) {
			return Err(anyhow::anyhow!(
				"strict DirectStorage forbids mirror-backed cache seeding"
			));
		}

		let mirror = self.page_mirror(actor_id.to_string()).await;
		let mut mirror = mirror.lock();
		mirror.db_size_pages = db_size_pages;
		mirror.pages.retain(|pgno, _| *pgno <= db_size_pages);
		for page in dirty_pages {
			mirror.pages.insert(page.pgno, page.bytes);
		}
		Ok(())
	}

	pub(crate) async fn snapshot_pages(&self, actor_id: &str) -> DirectActorPages {
		self.page_mirror(actor_id.to_string()).await.lock().clone()
	}

	pub(crate) fn compaction_signals(&self) -> Vec<DeltasAvailable> {
		self.compaction_signals.lock().clone()
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DirectStorageStats {
	pub(crate) depot_get_pages: u64,
	pub(crate) mirror_reads: u64,
	pub(crate) mirror_fills: u64,
	pub(crate) mirror_seeds: u64,
	pub(crate) cold_gets: u64,
}

#[derive(Default)]
struct DirectStorageCounters {
	depot_get_pages: AtomicU64,
	mirror_reads: AtomicU64,
	mirror_fills: AtomicU64,
	mirror_seeds: AtomicU64,
	cold_gets: AtomicU64,
}

struct CountingColdTier {
	inner: Arc<dyn ColdTier>,
	counters: Arc<DirectStorageCounters>,
}

#[async_trait]
impl ColdTier for CountingColdTier {
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		self.inner.put_object(key, bytes).await
	}

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
		self.counters.cold_gets.fetch_add(1, Ordering::SeqCst);
		self.inner.get_object(key).await
	}

	async fn delete_objects(&self, keys: &[String]) -> Result<()> {
		self.inner.delete_objects(keys).await
	}

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		self.inner.list_prefix(prefix).await
	}
}

#[derive(Default)]
pub(crate) struct DirectTransportHooks {
	fail_next_commit: Mutex<Option<String>>,
	fail_next_get_pages: Mutex<Option<String>>,
	hang_next_commit: Mutex<bool>,
	pause_next_commit: Mutex<Option<DirectCommitGate>>,
	commit_requests: Mutex<Vec<protocol::SqliteCommitRequest>>,
}

impl DirectTransportHooks {
	pub(crate) fn fail_next_commit(&self, message: impl Into<String>) {
		*self.fail_next_commit.lock() = Some(message.into());
	}

	pub(crate) fn fail_next_get_pages(&self, message: impl Into<String>) {
		*self.fail_next_get_pages.lock() = Some(message.into());
	}

	pub(crate) fn hang_next_commit(&self) {
		*self.hang_next_commit.lock() = true;
	}

	pub(crate) fn commit_requests(
		&self,
	) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteCommitRequest>> {
		self.commit_requests.lock()
	}

	pub(crate) fn record_commit_request(&self, req: protocol::SqliteCommitRequest) {
		self.commit_requests.lock().push(req);
	}

	pub(crate) fn pause_next_commit(&self) -> DirectCommitPause {
		let (reached_tx, reached_rx) = mpsc::channel();
		let (resume_tx, resume_rx) = mpsc::channel();
		*self.pause_next_commit.lock() = Some(DirectCommitGate {
			reached: reached_tx,
			resume: resume_rx,
		});
		DirectCommitPause {
			reached: reached_rx,
			resume: resume_tx,
		}
	}

	pub(crate) fn take_commit_error(&self) -> Option<String> {
		self.fail_next_commit.lock().take()
	}

	pub(crate) fn take_get_pages_error(&self) -> Option<String> {
		self.fail_next_get_pages.lock().take()
	}

	pub(crate) fn take_commit_hang(&self) -> bool {
		let mut hang = self.hang_next_commit.lock();
		let should_hang = *hang;
		*hang = false;
		should_hang
	}

	pub(crate) fn pause_commit_if_requested(&self) {
		let Some(gate) = self.pause_next_commit.lock().take() else {
			return;
		};
		let _ = gate.reached.send(());
		let _ = gate.resume.recv();
	}
}

pub(crate) struct DirectCommitPause {
	reached: mpsc::Receiver<()>,
	resume: mpsc::Sender<()>,
}

impl DirectCommitPause {
	pub(crate) fn wait_until_reached(&self) {
		self.reached.recv().expect("commit pause should be reached");
	}

	pub(crate) fn resume(self) {
		self.resume.send(()).expect("commit pause should resume");
	}
}

struct DirectCommitGate {
	reached: mpsc::Sender<()>,
	resume: mpsc::Receiver<()>,
}

pub(crate) fn protocol_fetched_page(
	page: depot::types::FetchedPage,
) -> protocol::SqliteFetchedPage {
	protocol::SqliteFetchedPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}

pub(crate) fn storage_dirty_page(page: protocol::SqliteDirtyPage) -> depot::types::DirtyPage {
	depot::types::DirtyPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}

fn depot_error(err: &anyhow::Error) -> Option<&SqliteStorageError> {
	err.downcast_ref::<SqliteStorageError>()
}

fn sqlite_error_reason(err: &anyhow::Error) -> String {
	err.chain()
		.map(ToString::to_string)
		.collect::<Vec<_>>()
		.join(": ")
}

pub(crate) fn sqlite_error_response(err: &anyhow::Error) -> protocol::SqliteErrorResponse {
	protocol::SqliteErrorResponse {
		message: sqlite_error_reason(err),
	}
}
