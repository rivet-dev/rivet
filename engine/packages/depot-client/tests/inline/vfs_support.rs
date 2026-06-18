use std::collections::BTreeMap;
use std::sync::{
	Arc,
	atomic::{AtomicU64, Ordering},
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
		ColdShardRef, CompactionRoot, DBHead, DatabaseBranchId, DirtyPage, decode_db_head,
		encode_cold_shard_ref, encode_compaction_root,
	},
	workflows::compaction::DeltasAvailable,
};
use parking_lot::Mutex;
use rivet_envoy_protocol as protocol;
use rivet_pools::{__rivet_util::Id, NodeId};
use sha2::{Digest, Sha256};
use universaldb::utils::IsolationLevel::Serializable;

use super::super::SqliteTransport;

pub(crate) struct DirectStorage {
	db: Arc<universaldb::Database>,
	node_id: NodeId,
	cold_tier: Option<Arc<dyn ColdTier>>,
	actor_dbs: scc::HashMap<String, Arc<Db>>,
	page_mirrors: scc::HashMap<String, Arc<Mutex<DirectActorPages>>>,
	compaction_signals: Arc<Mutex<Vec<DeltasAvailable>>>,
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

	pub(crate) fn stats(&self) -> DirectStorageStats {
		DirectStorageStats {
			depot_get_pages: self.counters.depot_get_pages.load(Ordering::SeqCst),
			mirror_reads: self.counters.mirror_reads.load(Ordering::SeqCst),
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
		let head = self.read_head(actor_id).await?;
		let shard_id = pgno / SHARD_SIZE;
		let snapshot = self.snapshot_pages(actor_id).await;
		let shard_pgnos = (1..=head.db_size_pages)
			.filter(|candidate_pgno| *candidate_pgno / SHARD_SIZE == shard_id)
			.collect::<Vec<_>>();
		let missing_pgnos = shard_pgnos
			.iter()
			.filter(|candidate_pgno| !snapshot.pages.contains_key(candidate_pgno))
			.copied()
			.collect::<Vec<_>>();
		let fetched_pages = if missing_pgnos.is_empty() {
			Vec::new()
		} else {
			self.actor_db(actor_id.to_string())
				.await
				.get_pages(missing_pgnos)
				.await?
		};
		let mut fetched_by_pgno = fetched_pages
			.into_iter()
			.filter_map(|page| page.bytes.map(|bytes| (page.pgno, bytes)))
			.collect::<BTreeMap<_, _>>();
		fetched_by_pgno.entry(pgno).or_insert(bytes);

		let dirty_pages = shard_pgnos
			.into_iter()
			.filter_map(|candidate_pgno| {
				snapshot
					.pages
					.get(&candidate_pgno)
					.cloned()
					.or_else(|| fetched_by_pgno.remove(&candidate_pgno))
					.map(|bytes| DirtyPage {
						pgno: candidate_pgno,
						bytes,
					})
			})
			.collect::<Vec<_>>();
		if dirty_pages.is_empty() {
			bail!("cold-ref seed could not load pages for shard {shard_id}");
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
		let covered_pgnos = dirty_pages
			.iter()
			.map(|page| {
				if page.pgno / SHARD_SIZE != shard_id {
					bail!("cold-ref seed page {} is outside shard {shard_id}", page.pgno);
				}
				Ok(page.pgno)
			})
			.collect::<Result<Vec<_>>>()?;
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
				let covered_pgnos = covered_pgnos.clone();
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
					for covered_pgno in covered_pgnos {
						tx.informal().clear(&branch_pidx_key(branch_id, covered_pgno));
					}
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
		let head = self.read_head(actor_id).await?;
		Ok((head.branch_id, head.head_txid))
	}

	async fn read_head(&self, actor_id: &str) -> Result<DBHead> {
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
					decode_db_head(&head)
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
	) -> anyhow::Result<depot::types::GetPagesResult> {
		self.get_pages_with_options(actor_id, pgnos, depot::types::GetPagesOptions::default())
			.await
	}

	pub(crate) async fn get_pages_with_options(
		&self,
		actor_id: &str,
		pgnos: &[u32],
		options: depot::types::GetPagesOptions,
	) -> anyhow::Result<depot::types::GetPagesResult> {
		if let Some(message) = self.hooks.take_get_pages_error() {
			return Err(anyhow::anyhow!(message));
		}

		let actor_db = self.actor_db(actor_id.to_string()).await;
		self.counters.depot_get_pages.fetch_add(1, Ordering::SeqCst);
		actor_db
			.get_pages_with_options(pgnos.to_vec(), options)
			.await
	}

	pub(crate) async fn read_mirror(
		&self,
		actor_id: &str,
		pgnos: &[u32],
	) -> Vec<depot::types::FetchedPage> {
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

pub(crate) struct DirectDepotTransport {
	storage: Arc<DirectStorage>,
}

impl DirectDepotTransport {
	pub(crate) fn new(storage: Arc<DirectStorage>) -> Self {
		Self { storage }
	}

	pub(crate) fn direct_hooks(&self) -> Arc<DirectTransportHooks> {
		Arc::clone(&self.storage.hooks)
	}
}

#[async_trait]
impl SqliteTransport for DirectDepotTransport {
	async fn get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		self.storage.hooks.record_get_pages_request(request.clone());
		let pgnos = request.pgnos.clone();
		match self
			.storage
			.get_pages_with_options(
				&request.actor_id,
				&pgnos,
				depot::types::GetPagesOptions {
					expected_head_txid: request.expected_head_txid,
				},
			)
			.await
		{
			Ok(result) => Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
				protocol::SqliteGetPagesOk {
					pages: result
						.pages
						.into_iter()
						.map(protocol_fetched_page)
						.collect(),
					head_txid: Some(result.head_txid),
				},
			)),
			Err(err) => Ok(protocol::SqliteGetPagesResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}

	async fn commit(
		&self,
		request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		self.storage
			.hooks
			.apply_commit_hooks(request.clone())
			.await?;

		let actor_id = request.actor_id.clone();
		let dirty_pages = request
			.dirty_pages
			.into_iter()
			.map(storage_dirty_page)
			.collect::<Vec<_>>();
		let actor_db = self.storage.actor_db(actor_id).await;
		match actor_db
			.commit_with_options(
				dirty_pages,
				request.db_size_pages,
				request.now_ms,
				depot::types::CommitOptions {
					expected_head_txid: request.expected_head_txid,
				},
			)
			.await
		{
			Ok(result) => Ok(protocol::SqliteCommitResponse::SqliteCommitOk(
				protocol::SqliteCommitOk {
					head_txid: Some(result.head_txid),
				},
			)),
			Err(err) => Ok(protocol::SqliteCommitResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}

	async fn commit_stage_begin(
		&self,
		request: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse> {
		self.storage
			.hooks
			.record_stage_begin_request(request.clone());

		let actor_id = request.actor_id.clone();
		let actor_db = self.storage.actor_db(actor_id).await;
		match actor_db
			.commit_stage_begin(
				request.dirty_pgnos,
				request.db_size_pages,
				request.now_ms,
				depot::types::CommitOptions {
					expected_head_txid: request.expected_head_txid,
				},
			)
			.await
		{
			Ok(result) => Ok(
				protocol::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(
					protocol::SqliteCommitStageBeginOk {
						stage_id: *result.stage_id.as_bytes(),
						max_pages_per_batch: result.max_pages_per_batch,
						max_batch_bytes: result.max_batch_bytes,
						observed_head_txid: result.observed_head_txid,
						staged_txid: result.staged_txid,
					},
				),
			),
			Err(err) => Ok(protocol::SqliteCommitStageBeginResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}

	async fn commit_stage_pages(
		&self,
		request: protocol::SqliteCommitStagePagesRequest,
	) -> Result<protocol::SqliteCommitStagePagesResponse> {
		self.storage
			.hooks
			.record_stage_pages_request(request.clone());
		if let Some(message) = self.storage.hooks.take_stage_pages_error() {
			return Ok(protocol::SqliteCommitStagePagesResponse::SqliteErrorResponse(
				injected_sqlite_error_response(message),
			));
		}

		let actor_id = request.actor_id.clone();
		let actor_db = self.storage.actor_db(actor_id).await;
		match actor_db
			.commit_stage_pages(
				stage_id_from_protocol(&request.stage_id)?,
				request.batch_idx,
				request
					.dirty_pages
					.into_iter()
					.map(storage_dirty_page)
					.collect(),
			)
			.await
		{
			Ok(()) => Ok(protocol::SqliteCommitStagePagesResponse::SqliteCommitStagePagesOk),
			Err(err) => Ok(protocol::SqliteCommitStagePagesResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}

	async fn commit_stage_complete(
		&self,
		request: protocol::SqliteCommitStageCompleteRequest,
	) -> Result<protocol::SqliteCommitStageCompleteResponse> {
		self.storage
			.hooks
			.record_stage_complete_request(request.clone());
		if let Some(message) = self.storage.hooks.take_stage_complete_error() {
			return Ok(
				protocol::SqliteCommitStageCompleteResponse::SqliteErrorResponse(
					injected_sqlite_error_response(message),
				),
			);
		}

		let actor_id = request.actor_id.clone();
		let actor_db = self.storage.actor_db(actor_id).await;
		match actor_db
			.commit_stage_complete(
				stage_id_from_protocol(&request.stage_id)?,
				request.page_batch_count,
			)
			.await
		{
			Ok(()) => Ok(
				protocol::SqliteCommitStageCompleteResponse::SqliteCommitStageCompleteOk,
			),
			Err(err) => Ok(
				protocol::SqliteCommitStageCompleteResponse::SqliteErrorResponse(
					sqlite_error_response(&err),
				),
			),
		}
	}

	async fn commit_stage_finalize(
		&self,
		request: protocol::SqliteCommitStageFinalizeRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		self.storage
			.hooks
			.record_stage_finalize_request(request.clone());

		let actor_id = request.actor_id.clone();
		let actor_db = self.storage.actor_db(actor_id).await;
		match actor_db
			.commit_stage_finalize(stage_id_from_protocol(&request.stage_id)?)
			.await
		{
			Ok(result) => Ok(protocol::SqliteCommitResponse::SqliteCommitOk(
				protocol::SqliteCommitOk {
					head_txid: Some(result.head_txid),
				},
			)),
			Err(err) => Ok(protocol::SqliteCommitResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}

	async fn commit_stage_abort(
		&self,
		request: protocol::SqliteCommitStageAbortRequest,
	) -> Result<protocol::SqliteCommitStageAbortResponse> {
		self.storage
			.hooks
			.record_stage_abort_request(request.clone());

		let actor_id = request.actor_id.clone();
		let actor_db = self.storage.actor_db(actor_id).await;
		match actor_db
			.commit_stage_abort(stage_id_from_protocol(&request.stage_id)?)
			.await
		{
			Ok(()) => Ok(protocol::SqliteCommitStageAbortResponse::SqliteCommitStageAbortOk),
			Err(err) => Ok(protocol::SqliteCommitStageAbortResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}
}

pub(crate) struct DirectMirrorTransport {
	storage: Arc<DirectStorage>,
}

impl DirectMirrorTransport {
	pub(crate) fn new(storage: Arc<DirectStorage>) -> Self {
		Self { storage }
	}

	pub(crate) fn direct_hooks(&self) -> Arc<DirectTransportHooks> {
		Arc::clone(&self.storage.hooks)
	}
}

#[async_trait]
impl SqliteTransport for DirectMirrorTransport {
	async fn get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		self.storage.hooks.record_get_pages_request(request.clone());
		if let Some(message) = self.storage.hooks.take_get_pages_error() {
			return Err(anyhow::anyhow!(message));
		}

		let pages = self
			.storage
			.read_mirror(&request.actor_id, &request.pgnos)
			.await;
		Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
			protocol::SqliteGetPagesOk {
				pages: pages.into_iter().map(protocol_fetched_page).collect(),
				head_txid: None,
			},
		))
	}

	async fn commit(
		&self,
		request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		self.storage
			.hooks
			.apply_commit_hooks(request.clone())
			.await?;

		let actor_id = request.actor_id.clone();
		let dirty_pages = request
			.dirty_pages
			.into_iter()
			.map(storage_dirty_page)
			.collect::<Vec<_>>();
		match self
			.storage
			.apply_commit(&actor_id, dirty_pages, request.db_size_pages)
			.await
		{
			Ok(()) => Ok(protocol::SqliteCommitResponse::SqliteCommitOk(
				protocol::SqliteCommitOk { head_txid: None },
			)),
			Err(err) => Ok(protocol::SqliteCommitResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DirectStorageStats {
	pub(crate) depot_get_pages: u64,
	pub(crate) mirror_reads: u64,
	pub(crate) mirror_seeds: u64,
	pub(crate) cold_gets: u64,
}

#[derive(Default)]
struct DirectStorageCounters {
	depot_get_pages: AtomicU64,
	mirror_reads: AtomicU64,
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
	fail_next_stage_pages: Mutex<Option<String>>,
	fail_next_stage_complete: Mutex<Option<String>>,
	hang_next_commit: Mutex<bool>,
	pause_next_commit: Mutex<Option<DirectCommitGate>>,
	get_pages_requests: Mutex<Vec<protocol::SqliteGetPagesRequest>>,
	commit_requests: Mutex<Vec<protocol::SqliteCommitRequest>>,
	stage_begin_requests: Mutex<Vec<protocol::SqliteCommitStageBeginRequest>>,
	stage_pages_requests: Mutex<Vec<protocol::SqliteCommitStagePagesRequest>>,
	stage_complete_requests: Mutex<Vec<protocol::SqliteCommitStageCompleteRequest>>,
	stage_finalize_requests: Mutex<Vec<protocol::SqliteCommitStageFinalizeRequest>>,
	stage_abort_requests: Mutex<Vec<protocol::SqliteCommitStageAbortRequest>>,
}

impl DirectTransportHooks {
	pub(crate) fn fail_next_commit(&self, message: impl Into<String>) {
		*self.fail_next_commit.lock() = Some(message.into());
	}

	pub(crate) fn fail_next_get_pages(&self, message: impl Into<String>) {
		*self.fail_next_get_pages.lock() = Some(message.into());
	}

	pub(crate) fn fail_next_stage_pages(&self, message: impl Into<String>) {
		*self.fail_next_stage_pages.lock() = Some(message.into());
	}

	pub(crate) fn fail_next_stage_complete(&self, message: impl Into<String>) {
		*self.fail_next_stage_complete.lock() = Some(message.into());
	}

	pub(crate) fn hang_next_commit(&self) {
		*self.hang_next_commit.lock() = true;
	}

	pub(crate) fn commit_requests(
		&self,
	) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteCommitRequest>> {
		self.commit_requests.lock()
	}

	pub(crate) fn get_pages_requests(
		&self,
	) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteGetPagesRequest>> {
		self.get_pages_requests.lock()
	}

	pub(crate) fn stage_begin_requests(
		&self,
	) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteCommitStageBeginRequest>> {
		self.stage_begin_requests.lock()
	}

	pub(crate) fn stage_pages_requests(
		&self,
	) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteCommitStagePagesRequest>> {
		self.stage_pages_requests.lock()
	}

	pub(crate) fn stage_complete_requests(
		&self,
	) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteCommitStageCompleteRequest>> {
		self.stage_complete_requests.lock()
	}

	pub(crate) fn stage_finalize_requests(
		&self,
	) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteCommitStageFinalizeRequest>> {
		self.stage_finalize_requests.lock()
	}

	pub(crate) fn stage_abort_requests(
		&self,
	) -> parking_lot::MutexGuard<'_, Vec<protocol::SqliteCommitStageAbortRequest>> {
		self.stage_abort_requests.lock()
	}

	pub(crate) fn record_get_pages_request(&self, req: protocol::SqliteGetPagesRequest) {
		self.get_pages_requests.lock().push(req);
	}

	pub(crate) fn record_commit_request(&self, req: protocol::SqliteCommitRequest) {
		self.commit_requests.lock().push(req);
	}

	pub(crate) fn record_stage_begin_request(
		&self,
		req: protocol::SqliteCommitStageBeginRequest,
	) {
		self.stage_begin_requests.lock().push(req);
	}

	pub(crate) fn record_stage_pages_request(
		&self,
		req: protocol::SqliteCommitStagePagesRequest,
	) {
		self.stage_pages_requests.lock().push(req);
	}

	pub(crate) fn record_stage_complete_request(
		&self,
		req: protocol::SqliteCommitStageCompleteRequest,
	) {
		self.stage_complete_requests.lock().push(req);
	}

	pub(crate) fn record_stage_finalize_request(
		&self,
		req: protocol::SqliteCommitStageFinalizeRequest,
	) {
		self.stage_finalize_requests.lock().push(req);
	}

	pub(crate) fn record_stage_abort_request(
		&self,
		req: protocol::SqliteCommitStageAbortRequest,
	) {
		self.stage_abort_requests.lock().push(req);
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

	pub(crate) fn take_stage_pages_error(&self) -> Option<String> {
		self.fail_next_stage_pages.lock().take()
	}

	pub(crate) fn take_stage_complete_error(&self) -> Option<String> {
		self.fail_next_stage_complete.lock().take()
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

	pub(crate) async fn apply_commit_hooks(
		&self,
		req: protocol::SqliteCommitRequest,
	) -> Result<()> {
		self.record_commit_request(req);
		if self.take_commit_hang() {
			std::future::pending().await
		}
		if let Some(message) = self.take_commit_error() {
			return Err(anyhow::anyhow!(message));
		}
		self.pause_commit_if_requested();
		Ok(())
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

fn sqlite_error_reason(err: &anyhow::Error) -> String {
	err.chain()
		.map(ToString::to_string)
		.collect::<Vec<_>>()
		.join(": ")
}

pub(crate) fn sqlite_error_response(err: &anyhow::Error) -> protocol::SqliteErrorResponse {
	let structured = depot_error(err)
		.map(|err| rivet_error::RivetError::extract(&err.clone().build()))
		.unwrap_or_else(|| rivet_error::RivetError::extract(err));
	protocol::SqliteErrorResponse {
		group: structured.group().to_string(),
		code: structured.code().to_string(),
		message: sqlite_error_reason(err),
		metadata: structured.metadata().map(|metadata| metadata.to_string()),
	}
}

fn injected_sqlite_error_response(message: String) -> protocol::SqliteErrorResponse {
	protocol::SqliteErrorResponse {
		group: "depot".to_string(),
		code: "injected_stage_error".to_string(),
		message,
		metadata: None,
	}
}

fn stage_id_from_protocol(stage_id: &protocol::SqliteStageId) -> Result<uuid::Uuid> {
	uuid::Uuid::from_slice(stage_id).map_err(Into::into)
}

fn depot_error(err: &anyhow::Error) -> Option<&SqliteStorageError> {
	err.chain()
		.find_map(|source| source.downcast_ref::<SqliteStorageError>())
}
