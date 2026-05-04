use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail, ensure};
use depot::{
	cold_tier::ColdTier,
	conveyer::branch::resolve_database_branch,
	keys,
	ltx::{DecodedLtx, decode_ltx_v3},
	types::{
		BranchState, BucketId, ColdShardRef, CommitRow, DatabaseBranchId, decode_cold_shard_ref,
		decode_commit_row, decode_compaction_root, decode_database_branch_record,
		decode_database_pointer, decode_db_head, decode_db_history_pin,
		decode_pitr_interval_coverage, decode_retired_cold_object, decode_sqlite_cmp_dirty,
	},
};
use futures_util::TryStreamExt;
use rivet_pools::__rivet_util::Id;
use sha2::{Digest, Sha256};
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};

use super::{FaultProfile, FaultScenario, LogicalOp};

pub(crate) struct DepotInvariantScanner {
	db: Arc<universaldb::Database>,
	cold_tier: Option<Arc<dyn ColdTier>>,
	database_id: String,
}

#[derive(Default)]
struct BranchRows {
	commits: BTreeMap<u64, CommitRow>,
	deltas: BTreeMap<u64, DecodedLtx>,
	shards: BTreeMap<(u32, u64), DecodedLtx>,
	cold_refs: Vec<ColdRefCoverage>,
}

struct ColdRefCoverage {
	reference: ColdShardRef,
	pages: Option<BTreeSet<u32>>,
}

impl DepotInvariantScanner {
	pub(crate) fn new(
		db: Arc<universaldb::Database>,
		cold_tier: Option<Arc<dyn ColdTier>>,
		database_id: String,
	) -> Self {
		Self {
			db,
			cold_tier,
			database_id,
		}
	}

	pub(crate) async fn verify(&self) -> Result<()> {
		let database_id = self.database_id.clone();
		let cold_tier = self.cold_tier.clone();
		let violations = self
			.db
			.run(move |tx| {
				let database_id = database_id.clone();
				let cold_tier = cold_tier.clone();
				async move {
					let mut scan = InvariantScan::new(&tx, database_id, cold_tier);
					scan.run().await?;
					Ok(scan.violations)
				}
			})
			.await?;

		if violations.is_empty() {
			return Ok(());
		}

		bail!("depot invariant violations: {}", violations.join("; "))
	}
}

struct InvariantScan<'a> {
	tx: &'a universaldb::Transaction,
	database_id: String,
	cold_tier: Option<Arc<dyn ColdTier>>,
	violations: Vec<String>,
}

impl<'a> InvariantScan<'a> {
	fn new(
		tx: &'a universaldb::Transaction,
		database_id: String,
		cold_tier: Option<Arc<dyn ColdTier>>,
	) -> Self {
		Self {
			tx,
			database_id,
			cold_tier,
			violations: Vec::new(),
		}
	}

	async fn run(&mut self) -> Result<()> {
		let Some(branch_id) = self.check_database_pointer().await? else {
			return Ok(());
		};
		self.check_branch_record(branch_id).await?;
		let Some(head) = self.check_live_head(branch_id).await? else {
			return Ok(());
		};
		let rows = self.check_branch_rows(branch_id, head.head_txid).await?;
		self.check_pidx(branch_id, head.db_size_pages, &rows)
			.await?;
		self.check_compaction_metadata(branch_id, head.head_txid, &rows)
			.await?;
		self.check_restore_points(branch_id).await?;
		self.check_history_pins(branch_id, head.head_txid).await?;
		Ok(())
	}

	async fn check_database_pointer(&mut self) -> Result<Option<DatabaseBranchId>> {
		let resolved = resolve_database_branch(
			self.tx,
			BucketId::from_gas_id(Id::nil()),
			&self.database_id,
			Serializable,
		)
		.await?;
		let mut scanned_current = None;
		for (key, value) in scan_prefix(self.tx, keys::database_pointer_cur_prefix()).await? {
			let decoded_key = keys::decode_database_pointer_cur_key(&key);
			let pointer = decode_database_pointer(&value);
			match (decoded_key, pointer) {
				(Ok((_bucket_branch_id, database_id)), Ok(pointer))
					if database_id == self.database_id =>
				{
					if scanned_current.replace(pointer.current_branch).is_some() {
						self.violate("database pointer appeared more than once");
					}
				}
				(Ok(_), Ok(_)) => {}
				(Err(err), _) => {
					self.violate(format!("database pointer key failed to decode: {err:#}"))
				}
				(_, Err(err)) => {
					self.violate(format!("database pointer value failed to decode: {err:#}"))
				}
			}
		}

		let Some(current) = resolved else {
			self.violate(format!(
				"database pointer for {} is missing",
				self.database_id
			));
			return Ok(None);
		};
		if let Some(scanned_current) = scanned_current
			&& scanned_current != current
		{
			self.violate("database pointer scan disagreed with branch resolution");
		}
		Ok(Some(current))
	}

	async fn check_branch_record(&mut self, branch_id: DatabaseBranchId) -> Result<()> {
		let key = keys::branches_list_key(branch_id);
		match get_value(self.tx, &key).await? {
			Some(value) => match decode_database_branch_record(&value) {
				Ok(record) => {
					if record.branch_id != branch_id {
						self.violate("database branch record id did not match its key");
					}
					if record.state != BranchState::Live {
						self.violate("database pointer targeted a non-live branch");
					}
					if let Some(parent) = record.parent {
						if get_value(self.tx, &keys::branches_list_key(parent))
							.await?
							.is_none()
						{
							self.violate("database branch parent record was missing");
						}
					}
				}
				Err(err) => {
					self.violate(format!("database branch record failed to decode: {err:#}"))
				}
			},
			None => self.violate("database branch record was missing"),
		}
		Ok(())
	}

	async fn check_live_head(
		&mut self,
		branch_id: DatabaseBranchId,
	) -> Result<Option<depot::types::DBHead>> {
		let Some(value) = get_value(self.tx, &keys::branch_meta_head_key(branch_id)).await? else {
			self.violate("live branch head row was missing");
			return Ok(None);
		};

		let head = match decode_db_head(&value) {
			Ok(head) => head,
			Err(err) => {
				self.violate(format!("live branch head failed to decode: {err:#}"));
				return Ok(None);
			}
		};

		if head.branch_id != branch_id {
			self.violate("live branch head branch id did not match its key");
		}
		if head.head_txid > 0
			&& get_value(self.tx, &keys::branch_commit_key(branch_id, head.head_txid))
				.await?
				.is_none()
		{
			self.violate(format!("head commit row {} was missing", head.head_txid));
		}
		Ok(Some(head))
	}

	async fn check_branch_rows(
		&mut self,
		branch_id: DatabaseBranchId,
		head_txid: u64,
	) -> Result<BranchRows> {
		let commits = self.check_commits(branch_id, head_txid).await?;
		let deltas = self.check_deltas(branch_id, &commits).await?;
		let shards = self.check_shards(branch_id, head_txid, &commits).await?;
		let cold_refs = self.check_cold_refs(branch_id, &commits).await?;
		Ok(BranchRows {
			commits,
			deltas,
			shards,
			cold_refs,
		})
	}

	async fn check_commits(
		&mut self,
		branch_id: DatabaseBranchId,
		head_txid: u64,
	) -> Result<BTreeMap<u64, CommitRow>> {
		let mut commits = BTreeMap::new();
		for (key, value) in scan_prefix(self.tx, keys::branch_commit_prefix(branch_id)).await? {
			match (
				decode_branch_commit_txid(branch_id, &key),
				decode_commit_row(&value),
			) {
				(Ok(txid), Ok(row)) => {
					commits.insert(txid, row);
				}
				(Err(err), _) => self.violate(format!("commit key failed to decode: {err:#}")),
				(_, Err(err)) => self.violate(format!("commit row failed to decode: {err:#}")),
			}
		}

		let compacted_through = self.compacted_commit_floor(branch_id, head_txid).await?;
		for txid in compacted_through.saturating_add(1)..=head_txid {
			if !commits.contains_key(&txid) {
				self.violate(format!(
					"commit rows were not contiguous. Missing txid {txid}"
				));
			}
		}
		for txid in commits.keys().copied() {
			if txid > head_txid {
				self.violate(format!("commit row {txid} was above head txid {head_txid}"));
			}
		}
		Ok(commits)
	}

	async fn compacted_commit_floor(
		&mut self,
		branch_id: DatabaseBranchId,
		head_txid: u64,
	) -> Result<u64> {
		let Some(value) = get_value(self.tx, &keys::branch_compaction_root_key(branch_id)).await?
		else {
			return Ok(0);
		};
		let root = match decode_compaction_root(&value) {
			Ok(root) => root,
			Err(_) => return Ok(0),
		};
		Ok(root.hot_watermark_txid.min(head_txid))
	}

	async fn check_deltas(
		&mut self,
		branch_id: DatabaseBranchId,
		commits: &BTreeMap<u64, CommitRow>,
	) -> Result<BTreeMap<u64, DecodedLtx>> {
		let mut chunks = BTreeMap::<u64, BTreeMap<u32, Vec<u8>>>::new();
		for (key, value) in scan_prefix(self.tx, keys::branch_delta_prefix(branch_id)).await? {
			let txid = match keys::decode_branch_delta_chunk_txid(branch_id, &key) {
				Ok(txid) => txid,
				Err(err) => {
					self.violate(format!("delta chunk key failed to decode txid: {err:#}"));
					continue;
				}
			};
			match keys::decode_branch_delta_chunk_idx(branch_id, txid, &key) {
				Ok(chunk_idx) => {
					chunks.entry(txid).or_default().insert(chunk_idx, value);
				}
				Err(err) => {
					self.violate(format!("delta chunk key failed to decode index: {err:#}"))
				}
			}
		}

		let mut deltas = BTreeMap::new();
		for (txid, chunk_map) in chunks {
			for expected_idx in 0..u32::try_from(chunk_map.len()).unwrap_or(u32::MAX) {
				if !chunk_map.contains_key(&expected_idx) {
					self.violate(format!(
						"delta txid {txid} was missing chunk {expected_idx}"
					));
				}
			}

			let mut bytes = Vec::new();
			for chunk in chunk_map.values() {
				bytes.extend_from_slice(chunk);
			}
			match decode_ltx_v3(&bytes) {
				Ok(delta) => {
					self.check_ltx_pages("delta", txid, &delta, commits);
					deltas.insert(txid, delta);
				}
				Err(err) => self.violate(format!("delta txid {txid} failed to decode: {err:#}")),
			}
		}
		Ok(deltas)
	}

	async fn check_shards(
		&mut self,
		branch_id: DatabaseBranchId,
		head_txid: u64,
		commits: &BTreeMap<u64, CommitRow>,
	) -> Result<BTreeMap<(u32, u64), DecodedLtx>> {
		let mut shards = BTreeMap::new();
		let compacted_through = self.compacted_commit_floor(branch_id, head_txid).await?;
		for (key, value) in scan_prefix(self.tx, keys::branch_shard_prefix(branch_id)).await? {
			let Some((shard_id, as_of_txid)) = decode_branch_shard_version_key(branch_id, &key)?
			else {
				continue;
			};
			match decode_ltx_v3(&value) {
				Ok(shard) => {
					// Hot shard cache rows can outlive compacted commit rows. The compaction
					// root is the fence that says those commits are now represented by cold
					// coverage, so stale hot rows below the fence only need shape validation.
					if commits.contains_key(&as_of_txid) || as_of_txid > compacted_through {
						self.check_ltx_pages("hot shard", as_of_txid, &shard, commits);
					} else {
						self.check_ltx_page_shape("hot shard", as_of_txid, &shard);
					}
					self.check_ltx_shard_pages("hot shard", shard_id, as_of_txid, &shard);
					shards.insert((shard_id, as_of_txid), shard);
				}
				Err(err) => self.violate(format!(
					"hot shard {shard_id}/{as_of_txid} failed to decode: {err:#}"
				)),
			}
		}
		Ok(shards)
	}

	async fn check_cold_refs(
		&mut self,
		branch_id: DatabaseBranchId,
		commits: &BTreeMap<u64, CommitRow>,
	) -> Result<Vec<ColdRefCoverage>> {
		let mut refs = Vec::new();
		let mut seen = BTreeSet::new();
		for (key, value) in scan_prefix(
			self.tx,
			keys::branch_compaction_cold_shard_prefix(branch_id),
		)
		.await?
		{
			let key_parts = match decode_cold_shard_key(branch_id, &key) {
				Ok(parts) => parts,
				Err(err) => {
					self.violate(format!("cold shard key failed to decode: {err:#}"));
					continue;
				}
			};
			let reference = match decode_cold_shard_ref(&value) {
				Ok(reference) => reference,
				Err(err) => {
					self.violate(format!("cold shard ref failed to decode: {err:#}"));
					continue;
				}
			};
			if key_parts != (reference.shard_id, reference.as_of_txid) {
				self.violate("cold shard ref key did not match encoded metadata");
			}
			if !seen.insert((reference.shard_id, reference.as_of_txid)) {
				self.violate("duplicate cold shard ref was present");
			}
			if let Some(commit) = commits.get(&reference.as_of_txid) {
				if reference.as_of_txid > reference.max_txid
					|| reference.min_txid > reference.max_txid
				{
					self.violate("cold shard ref txid range was invalid");
				}
				if reference.shard_id > commit.db_size_pages / keys::SHARD_SIZE {
					self.violate("cold shard ref was beyond database size");
				}
			} else {
				self.violate(format!(
					"cold shard ref pointed at missing commit {}",
					reference.as_of_txid
				));
			}
			let pages = self.check_cold_object(&reference, commits).await?;
			refs.push(ColdRefCoverage { reference, pages });
		}
		Ok(refs)
	}

	async fn check_cold_object(
		&mut self,
		reference: &ColdShardRef,
		commits: &BTreeMap<u64, CommitRow>,
	) -> Result<Option<BTreeSet<u32>>> {
		let Some(cold_tier) = self.cold_tier.clone() else {
			return Ok(None);
		};
		let Some(bytes) = cold_tier.get_object(&reference.object_key).await? else {
			self.violate(format!("cold object {} was missing", reference.object_key));
			return Ok(Some(BTreeSet::new()));
		};
		if reference.size_bytes != bytes.len() as u64 {
			self.violate("cold object size did not match its ref");
		}
		if reference.content_hash != content_hash(&bytes) {
			self.violate("cold object content hash did not match its ref");
		}
		let pages = match decode_ltx_v3(&bytes) {
			Ok(blob) => {
				self.check_ltx_pages("cold shard", reference.as_of_txid, &blob, commits);
				self.check_ltx_shard_pages(
					"cold shard",
					reference.shard_id,
					reference.as_of_txid,
					&blob,
				);
				Some(blob.pages.iter().map(|page| page.pgno).collect())
			}
			Err(err) => {
				self.violate(format!("cold object failed to decode as LTX: {err:#}"));
				Some(BTreeSet::new())
			}
		};
		Ok(pages)
	}

	async fn check_pidx(
		&mut self,
		branch_id: DatabaseBranchId,
		db_size_pages: u32,
		rows: &BranchRows,
	) -> Result<()> {
		for (key, value) in scan_prefix(self.tx, keys::branch_pidx_prefix(branch_id)).await? {
			let pgno = match decode_branch_pidx_pgno(branch_id, &key) {
				Ok(pgno) => pgno,
				Err(err) => {
					self.violate(format!("PIDX key failed to decode: {err:#}"));
					continue;
				}
			};
			let owner_txid = match decode_pidx_txid(&value) {
				Ok(owner_txid) => owner_txid,
				Err(err) => {
					self.violate(format!("PIDX value failed to decode: {err:#}"));
					continue;
				}
			};
			if pgno == 0 || pgno > db_size_pages {
				self.violate(format!(
					"PIDX page {pgno} was outside database size {db_size_pages}"
				));
			}
			if !self.page_has_backing(pgno, owner_txid, rows) {
				self.violate(format!(
					"PIDX page {pgno} pointed at missing backing txid {owner_txid}"
				));
			}
		}
		Ok(())
	}

	async fn check_compaction_metadata(
		&mut self,
		branch_id: DatabaseBranchId,
		head_txid: u64,
		rows: &BranchRows,
	) -> Result<()> {
		if let Some(value) =
			get_value(self.tx, &keys::branch_compaction_root_key(branch_id)).await?
		{
			match decode_compaction_root(&value) {
				Ok(root) => {
					if root.schema_version == 0 {
						self.violate("compaction root schema version was zero");
					}
					if root.hot_watermark_txid > head_txid || root.cold_watermark_txid > head_txid {
						self.violate("compaction root watermark was above branch head");
					}
				}
				Err(err) => self.violate(format!("compaction root failed to decode: {err:#}")),
			}
		}

		if let Some(value) = get_value(self.tx, &keys::sqlite_cmp_dirty_key(branch_id)).await? {
			match decode_sqlite_cmp_dirty(&value) {
				Ok(dirty) => {
					if dirty.observed_head_txid > head_txid {
						self.violate("dirty marker observed a head above the branch head");
					}
				}
				Err(err) => self.violate(format!("dirty marker failed to decode: {err:#}")),
			}
		}

		for (key, value) in scan_prefix(
			self.tx,
			keys::branch_compaction_retired_cold_object_prefix(branch_id),
		)
		.await?
		{
			match decode_retired_cold_object(&value) {
				Ok(retired) => {
					let expected_key = keys::branch_compaction_retired_cold_object_key(
						branch_id,
						content_hash(retired.object_key.as_bytes()),
					);
					if key != expected_key {
						self.violate("retired cold object key did not match object key hash");
					}
					if retired.delete_after_ms < retired.retired_at_ms {
						self.violate("retired cold object delete fence preceded retirement time");
					}
				}
				Err(err) => self.violate(format!("retired cold object failed to decode: {err:#}")),
			}
		}

		for (key, value) in
			scan_prefix(self.tx, keys::branch_pitr_interval_prefix(branch_id)).await?
		{
			if let Err(err) = keys::decode_branch_pitr_interval_bucket(branch_id, &key) {
				self.violate(format!("PITR interval key failed to decode: {err:#}"));
				continue;
			}
			match decode_pitr_interval_coverage(&value) {
				Ok(coverage) => {
					if coverage.txid > head_txid || !rows.commits.contains_key(&coverage.txid) {
						self.violate("PITR interval covered a missing commit");
					}
					if coverage.expires_at_ms < coverage.wall_clock_ms {
						self.violate("PITR interval expired before its commit wall clock");
					}
				}
				Err(err) => self.violate(format!("PITR interval failed to decode: {err:#}")),
			}
		}
		Ok(())
	}

	async fn check_restore_points(&mut self, branch_id: DatabaseBranchId) -> Result<()> {
		for (_key, value) in
			scan_prefix(self.tx, keys::restore_point_prefix(&self.database_id)).await?
		{
			match depot::types::decode_restore_point_record(&value) {
				Ok(record) => {
					if record.database_branch_id != branch_id {
						self.violate("restore point referenced a different database branch");
					}
					let vtx_value = get_value(
						self.tx,
						&keys::branch_vtx_key(branch_id, record.versionstamp),
					)
					.await?;
					if vtx_value.is_none() {
						self.violate("restore point versionstamp had no VTX row");
					}
				}
				Err(err) => self.violate(format!("restore point failed to decode: {err:#}")),
			}
		}
		Ok(())
	}

	async fn check_history_pins(
		&mut self,
		branch_id: DatabaseBranchId,
		head_txid: u64,
	) -> Result<()> {
		for (_key, value) in scan_prefix(self.tx, keys::db_pin_prefix(branch_id)).await? {
			match decode_db_history_pin(&value) {
				Ok(pin) => {
					if pin.at_txid > head_txid {
						self.violate("history pin referenced a txid above branch head");
					}
					if get_value(
						self.tx,
						&keys::branch_vtx_key(branch_id, pin.at_versionstamp),
					)
					.await?
					.is_none()
					{
						self.violate("history pin versionstamp had no VTX row");
					}
				}
				Err(err) => self.violate(format!("history pin failed to decode: {err:#}")),
			}
		}
		Ok(())
	}

	fn check_ltx_pages(
		&mut self,
		kind: &str,
		txid: u64,
		ltx: &DecodedLtx,
		commits: &BTreeMap<u64, CommitRow>,
	) {
		self.check_ltx_page_shape(kind, txid, ltx);
		let db_size_pages = commits.get(&txid).map(|commit| commit.db_size_pages);
		for page in &ltx.pages {
			if let Some(db_size_pages) = db_size_pages {
				if page.pgno > db_size_pages {
					self.violate(format!(
						"{kind} txid {txid} page {} exceeded database size",
						page.pgno
					));
				}
			} else {
				self.violate(format!("{kind} txid {txid} had no matching commit row"));
			}
		}
	}

	fn check_ltx_page_shape(&mut self, kind: &str, txid: u64, ltx: &DecodedLtx) {
		if ltx.header.page_size != keys::PAGE_SIZE {
			self.violate(format!("{kind} txid {txid} had unexpected page size"));
		}
		if ltx.header.min_txid > txid || ltx.header.max_txid < txid {
			self.violate(format!(
				"{kind} txid {txid} header did not cover its key txid"
			));
		}
		for page in &ltx.pages {
			if page.pgno == 0 {
				self.violate(format!("{kind} txid {txid} contained page 0"));
			}
			if page.bytes.len() != keys::PAGE_SIZE as usize {
				self.violate(format!(
					"{kind} txid {txid} page {} had invalid size",
					page.pgno
				));
			}
		}
	}

	fn check_ltx_shard_pages(
		&mut self,
		kind: &str,
		shard_id: u32,
		as_of_txid: u64,
		ltx: &DecodedLtx,
	) {
		for page in &ltx.pages {
			if page.pgno / keys::SHARD_SIZE != shard_id {
				self.violate(format!(
					"{kind} {shard_id}/{as_of_txid} contained page {} from another shard",
					page.pgno
				));
			}
		}
	}

	fn page_has_backing(&self, pgno: u32, owner_txid: u64, rows: &BranchRows) -> bool {
		if rows
			.deltas
			.get(&owner_txid)
			.is_some_and(|delta| delta.get_page(pgno).is_some())
		{
			return true;
		}
		let shard_id = pgno / keys::SHARD_SIZE;
		if rows
			.shards
			.iter()
			.any(|(&(candidate_shard, as_of_txid), shard)| {
				candidate_shard == shard_id
					&& as_of_txid >= owner_txid
					&& shard.get_page(pgno).is_some()
			}) {
			return true;
		}
		rows.cold_refs.iter().any(|reference| {
			reference.reference.shard_id == shard_id
				&& reference.reference.min_txid <= owner_txid
				&& reference.reference.max_txid >= owner_txid
				&& reference
					.pages
					.as_ref()
					.map_or(true, |pages| pages.contains(&pgno))
		})
	}

	fn violate(&mut self, message: impl Into<String>) {
		self.violations.push(message.into());
	}
}

async fn get_value(tx: &universaldb::Transaction, key: &[u8]) -> Result<Option<Vec<u8>>> {
	Ok(tx
		.informal()
		.get(key, Serializable)
		.await?
		.map(Vec::<u8>::from))
}

async fn scan_prefix(
	tx: &universaldb::Transaction,
	prefix: Vec<u8>,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Snapshot,
	);
	let mut rows = Vec::new();
	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}
	Ok(rows)
}

fn decode_branch_commit_txid(branch_id: DatabaseBranchId, key: &[u8]) -> Result<u64> {
	let prefix = keys::branch_commit_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch commit key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u64>()] = suffix
		.try_into()
		.map_err(|_| anyhow!("branch commit key suffix had invalid length"))?;
	Ok(u64::from_be_bytes(bytes))
}

fn decode_branch_pidx_pgno(branch_id: DatabaseBranchId, key: &[u8]) -> Result<u32> {
	let prefix = keys::branch_pidx_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("PIDX key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u32>()] = suffix
		.try_into()
		.map_err(|_| anyhow!("PIDX key suffix had invalid length"))?;
	Ok(u32::from_be_bytes(bytes))
}

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	let bytes: [u8; std::mem::size_of::<u64>()] = value
		.try_into()
		.map_err(|_| anyhow!("PIDX value had invalid length"))?;
	Ok(u64::from_be_bytes(bytes))
}

fn decode_branch_shard_version_key(
	branch_id: DatabaseBranchId,
	key: &[u8],
) -> Result<Option<(u32, u64)>> {
	let prefix = keys::branch_shard_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch shard key did not start with expected prefix")?;
	if suffix.len() == std::mem::size_of::<u32>() {
		return Ok(None);
	}
	ensure!(
		suffix.len() == std::mem::size_of::<u32>() + 1 + std::mem::size_of::<u64>()
			&& suffix[std::mem::size_of::<u32>()] == b'/',
		"branch shard key suffix had invalid length"
	);
	let shard_id = u32::from_be_bytes(
		suffix[..std::mem::size_of::<u32>()]
			.try_into()
			.context("decode branch shard id")?,
	);
	let as_of_txid = u64::from_be_bytes(
		suffix[std::mem::size_of::<u32>() + 1..]
			.try_into()
			.context("decode branch shard txid")?,
	);
	Ok(Some((shard_id, as_of_txid)))
}

fn decode_cold_shard_key(branch_id: DatabaseBranchId, key: &[u8]) -> Result<(u32, u64)> {
	let prefix = keys::branch_compaction_cold_shard_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("cold shard key did not start with expected prefix")?;
	ensure!(
		suffix.len() == std::mem::size_of::<u32>() + 1 + std::mem::size_of::<u64>()
			&& suffix[std::mem::size_of::<u32>()] == b'/',
		"cold shard key suffix had invalid length"
	);
	let shard_id = u32::from_be_bytes(
		suffix[..std::mem::size_of::<u32>()]
			.try_into()
			.context("decode cold shard id")?,
	);
	let as_of_txid = u64::from_be_bytes(
		suffix[std::mem::size_of::<u32>() + 1..]
			.try_into()
			.context("decode cold shard txid")?,
	);
	Ok((shard_id, as_of_txid))
}

fn content_hash(bytes: &[u8]) -> [u8; 32] {
	let digest = Sha256::digest(bytes);
	digest.into()
}

#[test]
fn depot_invariant_scanner_detects_missing_head_commit() -> Result<()> {
	FaultScenario::new("invariant_missing_head_commit")
		.seed(8001)
		.profile(FaultProfile::Simple)
		.setup(|ctx| async move {
			ctx.sql("CREATE TABLE kv (k TEXT PRIMARY KEY, v BLOB NOT NULL);")
				.await
		})
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::Put {
				key: "alpha".to_string(),
				value: vec![1, 2, 3],
			})
			.await
		})
		.verify(|ctx| async move {
			let branch_id = ctx.database_branch_id().await?;
			let db = ctx.depot_database();
			db.run(move |tx| async move {
				let head_bytes = tx
					.informal()
					.get(&keys::branch_meta_head_key(branch_id), Serializable)
					.await?
					.context("head should exist")?;
				let head = decode_db_head(&head_bytes)?;
				tx.informal()
					.clear(&keys::branch_commit_key(branch_id, head.head_txid));
				Ok(())
			})
			.await?;

			let err = ctx
				.verify_depot_invariants()
				.await
				.expect_err("scanner should reject a missing head commit");
			assert!(
				err.to_string().contains("head commit row"),
				"unexpected error: {err:#}"
			);
			Ok(())
		})
		.run()
}

#[test]
fn depot_invariant_scanner_detects_broken_pidx_backing() -> Result<()> {
	FaultScenario::new("invariant_broken_pidx")
		.seed(8002)
		.profile(FaultProfile::Simple)
		.setup(|ctx| async move {
			ctx.sql("CREATE TABLE kv (k TEXT PRIMARY KEY, v BLOB NOT NULL);")
				.await
		})
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::Put {
				key: "alpha".to_string(),
				value: vec![1, 2, 3],
			})
			.await
		})
		.verify(|ctx| async move {
			let branch_id = ctx.database_branch_id().await?;
			let db = ctx.depot_database();
			db.run(move |tx| async move {
				tx.informal()
					.set(&keys::branch_pidx_key(branch_id, 1), &999_u64.to_be_bytes());
				Ok(())
			})
			.await?;

			let err = ctx
				.verify_depot_invariants()
				.await
				.expect_err("scanner should reject missing PIDX backing");
			assert!(
				err.to_string().contains("PIDX page"),
				"unexpected error: {err:#}"
			);
			Ok(())
		})
		.run()
}

#[test]
fn depot_invariant_scanner_detects_cold_ref_missing_referenced_page() -> Result<()> {
	FaultScenario::new("invariant_cold_ref_missing_referenced_page")
		.seed(8003)
		.profile(FaultProfile::Simple)
		.setup(|ctx| async move {
			ctx.sql("CREATE TABLE kv (k TEXT PRIMARY KEY, v BLOB NOT NULL);")
				.await
		})
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::Put {
				key: "alpha".to_string(),
				value: vec![1, 2, 3],
			})
			.await
		})
		.verify(|ctx| async move {
			ctx.seed_page_as_cold_ref_for_harness_test(1).await?;
			ctx.remove_page_from_seeded_cold_ref_for_harness_test(1)
				.await?;

			let err = ctx
				.verify_depot_invariants()
				.await
				.expect_err("scanner should reject cold refs missing the PIDX page");
			assert!(
				err.to_string()
					.contains("PIDX page 1 pointed at missing backing"),
				"unexpected error: {err:#}"
			);
			Ok(())
		})
		.run()
}
