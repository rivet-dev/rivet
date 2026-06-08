use std::{
	collections::{BTreeMap, BTreeSet},
	fs::{self, File},
	future::Future,
	io::{Read, Seek, SeekFrom, Write},
	path::{Path, PathBuf},
	pin::Pin,
	sync::Arc,
	time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, ensure};
use futures_util::TryStreamExt;
use gas::prelude::Id;
use rivet_pools::NodeId;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use universaldb::{
	KeySelector, RangeOption, options::StreamingMode, utils::IsolationLevel::Snapshot,
};
use uuid::Uuid;

use crate::conveyer::{
	branch,
	db::Db,
	keys::{self, PAGE_SIZE, SHARD_SIZE},
	ltx::{DecodedLtx, decode_ltx_v3},
	types::{
		BucketId, CommitRow, DatabaseBranchId, DatabaseBranchRecord, DepotReadMode,
		GetPagesOptions, PageSourceKind, PageSourceProvenance as DepotPageSourceProvenance,
		decode_bucket_pointer, decode_commit_row, decode_database_branch_record,
		decode_database_pointer, decode_db_head,
	},
};

const SQLITE_CHECK_ROW_LIMIT: usize = 50;
const REPORT_ROW_LIMIT: usize = 100_000;
const EARLY_SCAN_TIMEOUT: Duration = Duration::from_millis(2_500);
const SCAN_CHUNK_TIMEOUT: Duration = Duration::from_secs(5);
const SCAN_CHUNK_ROW_LIMIT: usize = 1_024;

#[derive(Clone)]
pub struct DoctorInput {
	pub selector: DoctorSelector,
	pub artifact_dir: Option<PathBuf>,
	pub skip: SkipOptions,
	pub min_txid: Option<u64>,
	pub max_txid: Option<u64>,
	#[doc(hidden)]
	pub progress_hook: Option<DoctorProgressHook>,
}

impl std::fmt::Debug for DoctorInput {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("DoctorInput")
			.field("selector", &self.selector)
			.field("artifact_dir", &self.artifact_dir)
			.field("skip", &self.skip)
			.field("min_txid", &self.min_txid)
			.field("max_txid", &self.max_txid)
			.field(
				"progress_hook",
				&self.progress_hook.as_ref().map(|_| "<hook>"),
			)
			.finish()
	}
}

pub type DoctorProgressHook = Arc<
	dyn Fn(DoctorProgressPhase) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorProgressPhase {
	AfterStartSnapshot,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SkipOptions {
	pub full_integrity_check: bool,
	pub first_bad_txid: bool,
	pub page_provenance: bool,
	pub resolver_compare: bool,
}

#[derive(Debug, Clone)]
pub enum DoctorSelector {
	BucketDatabase {
		bucket_id: Uuid,
		database_id: String,
	},
	Actor {
		namespace_id: Id,
		actor_id: Id,
	},
	DatabaseBranch {
		database_branch_id: Uuid,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorVerdictKind {
	Healthy,
	Corrupt,
	Inconclusive,
	Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorruptionClass {
	None,
	DeltaHistory,
	DepotReconstruction,
	HotCompactionState,
	SqliteStructure,
	Unknown,
	Unsupported,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorVerdict {
	pub verdict: DoctorVerdictKind,
	pub corruption_class: CorruptionClass,
	pub unsupported_reason: Option<&'static str>,
	pub reason: Option<&'static str>,
	pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
	pub verdict: DoctorVerdict,
	pub generated_at_ms: i64,
	pub duration_ms: u128,
	pub selected_head_txid: Option<u64>,
	pub selected_db_size_pages: Option<u32>,
	pub first_bad_txid: Option<u64>,
	pub previous_good_txid: Option<u64>,
	pub identity: Value,
	pub analysis_scope: Value,
	pub facts: Value,
	pub analyses: Value,
	pub artifacts: Value,
}

#[derive(Debug, Clone, Serialize)]
struct LimitedRows<T> {
	total_count: usize,
	emitted_count: usize,
	omitted_count: usize,
	truncated_reason: Option<&'static str>,
	rows: Vec<T>,
}

impl<T> LimitedRows<T> {
	fn from_rows(mut rows: Vec<T>) -> Self {
		let total_count = rows.len();
		let truncated = rows.len() > REPORT_ROW_LIMIT;
		if truncated {
			rows.truncate(REPORT_ROW_LIMIT);
		}
		let emitted_count = rows.len();
		Self {
			total_count,
			emitted_count,
			omitted_count: total_count.saturating_sub(emitted_count),
			truncated_reason: truncated.then_some("report_limit"),
			rows,
		}
	}
}

#[derive(Debug, Clone)]
struct Privacy {
	key: [u8; 16],
}

impl Privacy {
	fn new() -> Self {
		Self {
			key: *Uuid::new_v4().as_bytes(),
		}
	}

	fn hash_bytes(&self, bytes: &[u8]) -> String {
		let mut hasher = Sha256::new();
		hasher.update(self.key);
		hasher.update(bytes);
		hex_lower(&hasher.finalize())
	}

	fn hash_uuid(&self, value: Uuid) -> String {
		self.hash_bytes(value.as_bytes())
	}

	fn hash_id(&self, value: Id) -> String {
		self.hash_bytes(&value.as_bytes())
	}

	fn hash_str(&self, value: &str) -> String {
		self.hash_bytes(value.as_bytes())
	}
}

#[derive(Debug, Clone)]
struct ResolvedIdentity {
	bucket_id: Option<BucketId>,
	database_id: Option<String>,
	branch_id: DatabaseBranchId,
	bucket_branch_id: Option<crate::types::BucketBranchId>,
	selector_kind: &'static str,
	namespace_id: Option<Id>,
	actor_id: Option<Id>,
}

#[derive(Debug, Clone)]
struct MissingActorDatabase {
	bucket_id: BucketId,
	database_id: String,
	namespace_id: Id,
	actor_id: Id,
}

#[derive(Debug, Clone)]
enum ResolvedSelector {
	Resolved(ResolvedIdentity),
	MissingActorDatabase(MissingActorDatabase),
}

#[derive(Debug, Clone, Serialize)]
struct SnapshotFacts {
	head_txid: Option<u64>,
	head_versionstamp: Option<String>,
	current_branch_hash: String,
	commit_row_count: usize,
	delta_chunk_row_count: usize,
	pidx_row_count: usize,
	installed_hot_shard_count: usize,
	staged_hot_shard_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct CommitFact {
	txid: u64,
	wall_clock_ms: i64,
	versionstamp: String,
	db_size_pages: u32,
	post_apply_checksum: u64,
	at_or_below_selected_txid: bool,
	dirty_page_count: Option<usize>,
	min_dirty_page: Option<u32>,
	max_dirty_page: Option<u32>,
	decode_status: String,
	decode_error: Option<String>,
}

#[derive(Debug, Clone)]
struct CommitData {
	txid: u64,
	row: CommitRow,
	delta: Option<DecodedDelta>,
	delta_decode_error: Option<String>,
	delta_chunk_count: usize,
	delta_chunk_indexes: Vec<u32>,
	delta_encoded_hash: String,
	delta_encoded_bytes: usize,
}

#[derive(Debug, Clone)]
struct DecodedDelta {
	decoded: DecodedLtx,
	encoded_hash: String,
	encoded_bytes: usize,
	chunk_count: usize,
	chunk_indexes: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
struct DeltaChunkFact {
	txid: u64,
	chunk_index: u32,
	encoded_byte_len: usize,
	chunk_hash: String,
	at_or_below_selected_txid: bool,
}

#[derive(Debug, Clone, Serialize)]
struct DeltaFact {
	txid: u64,
	chunk_count: usize,
	chunk_indexes: Vec<u32>,
	total_encoded_bytes: usize,
	encoded_delta_hash: String,
	decode_status: String,
	decode_error: Option<String>,
	decoded_min_txid: Option<u64>,
	decoded_max_txid: Option<u64>,
	decoded_page_size: Option<u32>,
	decoded_database_size: Option<u32>,
	dirty_page_count: Option<usize>,
	dirty_page_hashes: Vec<PageHashFact>,
}

#[derive(Debug, Clone, Serialize)]
struct PageHashFact {
	pgno: u32,
	hash: String,
}

#[derive(Debug, Clone, Serialize)]
struct VtxFact {
	txid: u64,
	versionstamp: String,
	row_key_hash: String,
	commit_row_exists: bool,
	commit_row_versionstamp_matches: bool,
	at_or_below_selected_txid: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PidxFact {
	pgno: u32,
	owner_txid: u64,
	encoded_owner_value_hash: String,
	inside_selected_eof: bool,
	owner_txid_exists: bool,
	owner_delta_contains_page: bool,
	computed_replay_owner_txid: Option<u64>,
	stored_owner_matches_computed_owner: bool,
}

#[derive(Debug, Clone, Serialize)]
struct HotShardFact {
	row_key_hash: String,
	shard_id: Option<u32>,
	as_of_txid: Option<u64>,
	encoded_size: usize,
	encoded_hash: String,
	decode_status: String,
	decode_error: Option<String>,
	decoded_page_count: Option<usize>,
	min_page: Option<u32>,
	max_page: Option<u32>,
	eligible_at_selected_txid: bool,
}

#[derive(Debug, Clone)]
struct HotShardData {
	shard_id: u32,
	as_of_txid: u64,
	decoded: DecodedLtx,
	key_hash: String,
}

#[derive(Debug, Clone, Serialize)]
struct PageProvenanceFact {
	pgno: u32,
	replay_winner_txid: Option<u64>,
	replay_winner_hash: String,
	depot_winner_kind: &'static str,
	depot_winner_txid: Option<u64>,
	depot_winner_hash: String,
	stored_pidx_owner_txid: Option<u64>,
	computed_replay_owner_txid: Option<u64>,
	candidate_count: usize,
	candidate_summaries: Vec<Value>,
	inside_selected_eof: bool,
	differs_between_images: bool,
}

#[derive(Debug, Clone)]
struct ReplayResult {
	path: PathBuf,
	page_size: u32,
	db_size_pages: u32,
	image_hash: String,
	page_hashes: BTreeMap<u32, String>,
	computed_owners: BTreeMap<u32, u64>,
	zero_filled_pages: Vec<u32>,
	dirty_pages_above_eof: Vec<Value>,
	commit_facts: Vec<CommitFact>,
	delta_facts: Vec<DeltaFact>,
}

#[derive(Debug, Clone)]
struct ResolverResult {
	path: Option<PathBuf>,
	image_hash: Option<String>,
	page_hashes: BTreeMap<u32, String>,
	provenance: Vec<PageProvenanceFact>,
	hot_mismatch_pages: Vec<u32>,
	error: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
struct SqliteChecks {
	path_kind: &'static str,
	quick_check: SqliteCheckResult,
	integrity_check: Option<SqliteCheckResult>,
	pragmas: Value,
	header: Value,
	schema: Value,
}

#[derive(Debug, Clone, Serialize)]
struct SqliteCheckResult {
	ran: bool,
	ok: bool,
	rows: Vec<String>,
	error: Option<String>,
	truncated: bool,
}

pub fn exit_code_for_verdict(verdict: DoctorVerdictKind) -> i32 {
	match verdict {
		DoctorVerdictKind::Healthy => 0,
		DoctorVerdictKind::Corrupt => 1,
		DoctorVerdictKind::Inconclusive => 2,
		DoctorVerdictKind::Unsupported => 4,
	}
}

pub async fn doctor(db: &universaldb::Database, input: DoctorInput) -> Result<DoctorReport> {
	let started = Instant::now();
	let generated_at_ms = now_ms()?;
	let privacy = Privacy::new();
	let temp_dir = tempfile::Builder::new()
		.prefix("rivet-depot-doctor-")
		.tempdir()
		.context("create depot doctor temp dir")?;

	tracing::info!(phase = "resolve", "resolving depot doctor selector");
	let resolved = match resolve_selector(db, &input.selector).await? {
		ResolvedSelector::Resolved(resolved) => resolved,
		ResolvedSelector::MissingActorDatabase(missing) => {
			return actor_database_missing_report(
				db,
				started,
				generated_at_ms,
				&privacy,
				&input,
				missing,
			)
			.await;
		}
	};
	let selected_branch = resolved.branch_id;

	tracing::info!(phase = "snapshot", "capturing depot doctor start snapshot");
	let start_snapshot = capture_snapshot(db, selected_branch, &privacy).await?;
	if let Some(hook) = &input.progress_hook {
		hook(DoctorProgressPhase::AfterStartSnapshot).await?;
	}
	let branch_record = load_branch_record(db, selected_branch).await?;
	let Some(branch_record) = branch_record else {
		return Ok(unsupported_report(
			started,
			generated_at_ms,
			&privacy,
			&input,
			&resolved,
			"unsupported_storage_kind",
			"selected database does not have a Depot branch record",
		));
	};

	if let Some(report) = detect_unsupported(
		db,
		&input,
		&resolved,
		&branch_record,
		started,
		generated_at_ms,
		&privacy,
	)
	.await?
	{
		return Ok(report);
	}

	let Some(head_txid) = start_snapshot.head_txid else {
		return Ok(inconclusive_report(
			started,
			generated_at_ms,
			&privacy,
			&input,
			&resolved,
			start_snapshot,
			"head_missing",
			"selected branch has no live head",
		));
	};
	if input.max_txid == Some(0) {
		return Ok(inconclusive_report(
			started,
			generated_at_ms,
			&privacy,
			&input,
			&resolved,
			start_snapshot,
			"invalid_max_txid",
			"--max-txid must be greater than 0",
		));
	}
	let selected_txid =
		match select_existing_txid(db, selected_branch, head_txid, input.max_txid).await? {
			Some(txid) => txid,
			None => {
				return Ok(inconclusive_report(
					started,
					generated_at_ms,
					&privacy,
					&input,
					&resolved,
					start_snapshot,
					"max_txid_before_first_commit",
					"no commit exists at or below requested --max-txid",
				));
			}
		};

	tracing::info!(
		phase = "facts",
		selected_txid,
		"loading depot doctor storage facts"
	);
	let storage = load_storage_facts(db, selected_branch, selected_txid, &privacy).await?;
	let first_bad_min_txid = input.min_txid.unwrap_or(1);
	ensure!(first_bad_min_txid > 0, "--min-txid must be greater than 0");
	ensure!(
		first_bad_min_txid <= selected_txid,
		"--min-txid {} is after selected txid {}",
		first_bad_min_txid,
		selected_txid
	);

	tracing::info!(
		phase = "replay",
		selected_txid,
		"building sequential SQLite image"
	);
	let sequential_path = temp_dir.path().join("current-sequential.sqlite");
	let replay = build_sequential_replay(
		&sequential_path,
		&storage.commits,
		selected_txid,
		storage.selected_db_size_pages,
		&privacy,
	)?;

	tracing::info!(
		phase = "sqlite_check",
		image = "sequential",
		"checking SQLite image"
	);
	let sequential_checks =
		sqlite_checks("sequential", &replay.path, !input.skip.full_integrity_check)?;
	let replay_has_delta_errors = replay
		.delta_facts
		.iter()
		.any(|delta| delta.decode_status != "ok");
	let preliminary_pidx_integrity = analyze_pidx(&storage, &replay, selected_txid, head_txid);

	let resolver = if input.skip.resolver_compare
		|| replay_has_delta_errors
		|| !analysis_ok(&preliminary_pidx_integrity)
	{
		None
	} else {
		tracing::info!(
			phase = "resolver_compare",
			selected_txid,
			"building Depot-resolved SQLite image"
		);
		let resolver_path = temp_dir.path().join("current-depot-resolved.sqlite");
		Some(
			build_resolver_image(
				&resolver_path,
				db,
				&resolved,
				&storage,
				&replay,
				selected_txid,
				input.skip.page_provenance,
				&privacy,
			)
			.await?,
		)
	};

	let resolver_checks = if let Some(resolver) = &resolver {
		if let Some(path) = &resolver.path {
			tracing::info!(
				phase = "sqlite_check",
				image = "depot_resolved",
				"checking SQLite image"
			);
			Some(sqlite_checks(
				"depot_resolved",
				path,
				!input.skip.full_integrity_check,
			)?)
		} else {
			None
		}
	} else {
		None
	};

	let comparison = resolver
		.as_ref()
		.filter(|resolver| resolver.error.is_none())
		.map(|resolver| compare_images(&replay, resolver, storage.selected_db_size_pages));
	let head_valid = sqlite_checks_ok(&sequential_checks)
		&& resolver_checks
			.as_ref()
			.map(sqlite_checks_ok)
			.unwrap_or(true)
		&& comparison
			.as_ref()
			.and_then(|value| value.get("byte_identical"))
			.and_then(Value::as_bool)
			.unwrap_or(true);

	let first_bad = if !head_valid && !input.skip.first_bad_txid {
		tracing::info!(phase = "first_bad_txid", "searching first bad txid");
		if !sqlite_checks_ok(&sequential_checks) {
			Some(find_first_bad_txid(
				&temp_dir,
				&storage.commits,
				first_bad_min_txid,
				selected_txid,
				!input.skip.full_integrity_check,
			)?)
		} else if comparison
			.as_ref()
			.and_then(|value| value.get("byte_identical"))
			.and_then(Value::as_bool)
			== Some(false)
		{
			Some(
				find_first_resolver_divergence(
					&temp_dir,
					db,
					&resolved,
					&storage,
					first_bad_min_txid,
					selected_txid,
					input.skip.page_provenance,
					&privacy,
				)
				.await?,
			)
		} else {
			Some(find_first_bad_txid(
				&temp_dir,
				&storage.commits,
				first_bad_min_txid,
				selected_txid,
				!input.skip.full_integrity_check,
			)?)
		}
	} else {
		None
	};

	tracing::info!(phase = "snapshot", "capturing depot doctor end snapshot");
	let end_snapshot = capture_snapshot(db, selected_branch, &privacy).await?;
	let changed = snapshot_changed(&start_snapshot, &end_snapshot);
	let commit_chain_analysis = analyze_commit_chain(&storage.commits, selected_txid);
	let versionstamp_index_analysis = analyze_vtx(&storage);
	let delta_integrity_analysis = analyze_delta_integrity(&replay);
	let pidx_integrity_analysis = preliminary_pidx_integrity;

	let mut verdict = classify_report(
		&sequential_checks,
		resolver_checks.as_ref(),
		comparison.as_ref(),
		resolver.as_ref(),
		changed,
		&commit_chain_analysis,
		&versionstamp_index_analysis,
		&delta_integrity_analysis,
		&pidx_integrity_analysis,
		resolver
			.as_ref()
			.and_then(|resolver| resolver.error.as_ref()),
	);
	if changed {
		verdict.reason = Some("database_changed_during_diagnosis");
	}

	let first_bad_txid = first_bad
		.as_ref()
		.and_then(|value| value.get("first_bad_txid").and_then(Value::as_u64));
	let previous_good_txid = first_bad
		.as_ref()
		.and_then(|value| value.get("previous_good_txid").and_then(Value::as_u64));

	let facts = json!({
		"storage_inventory": storage.inventory,
		"commits": LimitedRows::from_rows(replay.commit_facts.clone()),
		"vtx": LimitedRows::from_rows(storage.vtx_facts.clone()),
		"delta_chunks": LimitedRows::from_rows(storage.delta_chunk_facts.clone()),
		"deltas": LimitedRows::from_rows(replay.delta_facts.clone()),
		"pidx": LimitedRows::from_rows(storage.pidx_facts(&replay)),
		"hot_shards": LimitedRows::from_rows(storage.hot_shard_facts.clone()),
		"compaction": storage.compaction,
		"page_provenance": if input.skip.page_provenance || resolver.is_none() {
			json!({ "skipped": true, "skip_reason": if input.skip.resolver_compare { "resolver_compare_skipped" } else if replay_has_delta_errors { "delta_integrity_failed" } else if !analysis_ok(&pidx_integrity_analysis) { "pidx_integrity_failed" } else { "disabled_by_flag" } })
		} else {
			json!(LimitedRows::from_rows(resolver.as_ref().map(|x| x.provenance.clone()).unwrap_or_default()))
		},
		"sqlite_images": {
			"sequential": {
				"path_preserved_in_artifacts": input.artifact_dir.is_some(),
				"image_hash": replay.image_hash,
				"page_size": replay.page_size,
				"page_count": replay.db_size_pages,
				"zero_filled_pages": LimitedRows::from_rows(replay.zero_filled_pages.clone()),
				"dirty_pages_above_eof": LimitedRows::from_rows(replay.dirty_pages_above_eof.clone()),
			},
			"depot_resolved": resolver.as_ref().map(|resolver| json!({
				"path_preserved_in_artifacts": input.artifact_dir.is_some() && resolver.path.is_some(),
				"image_hash": resolver.image_hash,
				"error": resolver.error,
			})),
		},
		"sqlite_checks": {
			"sequential": sequential_checks,
			"depot_resolved": resolver_checks,
		},
		"suspect_pages": suspect_pages(&replay, resolver.as_ref(), comparison.as_ref()),
	});

	let analyses = json!({
		"commit_chain": commit_chain_analysis,
		"versionstamp_index": versionstamp_index_analysis,
		"delta_integrity": delta_integrity_analysis,
		"pidx_integrity": pidx_integrity_analysis,
		"page_provenance": if input.skip.page_provenance || resolver.is_none() {
			json!({ "skipped": true, "skip_reason": if input.skip.resolver_compare { "resolver_compare_skipped" } else if replay_has_delta_errors { "delta_integrity_failed" } else if !analysis_ok(&pidx_integrity_analysis) { "pidx_integrity_failed" } else { "disabled_by_flag" } })
		} else {
			comparison.clone().unwrap_or_else(|| json!({ "skipped": true }))
		},
		"resolver_errors": resolver
			.as_ref()
			.and_then(|resolver| resolver.error.clone())
			.unwrap_or_else(|| json!({ "ok": true })),
		"hot_compaction_mismatches": resolver.as_ref().map(|resolver| {
			json!({
				"mismatch_count": resolver.hot_mismatch_pages.len(),
				"pages": LimitedRows::from_rows(resolver.hot_mismatch_pages.clone()),
			})
		}).unwrap_or_else(|| json!({ "skipped": true })),
		"sqlite_reconstruction": comparison.unwrap_or_else(|| json!({ "skipped": true, "skip_reason": if input.skip.resolver_compare { "resolver_compare_skipped" } else if replay_has_delta_errors { "delta_integrity_failed" } else if !analysis_ok(&pidx_integrity_analysis) { "pidx_integrity_failed" } else { "not_available" } })),
		"first_bad_txid": first_bad.unwrap_or_else(|| {
			if head_valid {
				json!({ "enabled": false, "skip_reason": "selected_head_is_valid" })
			} else {
				json!({ "enabled": false, "skip_reason": "disabled_by_flag" })
			}
		}),
		"truncate_regrow": analyze_truncate_regrow(&storage.commits, &replay),
		"storage_consistency": analyze_storage_consistency(&storage, &replay, selected_txid),
	});

	let mut report = DoctorReport {
		verdict,
		generated_at_ms,
		duration_ms: started.elapsed().as_millis(),
		selected_head_txid: Some(selected_txid),
		selected_db_size_pages: Some(storage.selected_db_size_pages),
		first_bad_txid,
		previous_good_txid,
		identity: identity_json(&privacy, &resolved, &branch_record),
		analysis_scope: analysis_scope_json(
			&input,
			&resolved,
			head_txid,
			selected_txid,
			&start_snapshot,
			&end_snapshot,
		),
		facts,
		analyses,
		artifacts: json!({ "preserved": false, "files": [] }),
	};

	if let Some(artifact_dir) = &input.artifact_dir {
		tracing::warn!(
			path = %artifact_dir.display(),
			"depot doctor artifacts may contain raw customer SQLite data"
		);
		preserve_artifacts(artifact_dir, &temp_dir, &mut report)?;
	}

	Ok(report)
}

#[derive(Debug, Clone)]
struct StorageFacts {
	inventory: Value,
	compaction: Value,
	commits: Vec<CommitData>,
	vtx_facts: Vec<VtxFact>,
	delta_chunk_facts: Vec<DeltaChunkFact>,
	pidx_rows: BTreeMap<u32, u64>,
	hot_shard_facts: Vec<HotShardFact>,
	hot_shards: Vec<HotShardData>,
	selected_db_size_pages: u32,
}

impl StorageFacts {
	fn pidx_facts(&self, replay: &ReplayResult) -> Vec<PidxFact> {
		let commit_txids = self
			.commits
			.iter()
			.map(|commit| commit.txid)
			.collect::<BTreeSet<_>>();
		let deltas = self
			.commits
			.iter()
			.filter_map(|commit| commit.delta.as_ref().map(|delta| (commit.txid, delta)))
			.collect::<BTreeMap<_, _>>();
		self.pidx_rows
			.iter()
			.map(|(pgno, owner_txid)| {
				let computed = replay.computed_owners.get(pgno).copied();
				PidxFact {
					pgno: *pgno,
					owner_txid: *owner_txid,
					encoded_owner_value_hash: hash_u64(*owner_txid),
					inside_selected_eof: *pgno <= self.selected_db_size_pages,
					owner_txid_exists: commit_txids.contains(owner_txid),
					owner_delta_contains_page: deltas
						.get(owner_txid)
						.is_some_and(|delta| delta.decoded.get_page(*pgno).is_some()),
					computed_replay_owner_txid: computed,
					stored_owner_matches_computed_owner: computed == Some(*owner_txid),
				}
			})
			.collect()
	}
}

async fn resolve_selector(
	db: &universaldb::Database,
	selector: &DoctorSelector,
) -> Result<ResolvedSelector> {
	match selector {
		DoctorSelector::BucketDatabase {
			bucket_id,
			database_id,
		} => {
			let bucket = BucketId::from_uuid(*bucket_id);
			let database = database_id.clone();
			let branch_id = db
				.txn("depot_doctor_resolve_bucket_database", move |tx| {
					let database = database.clone();
					async move {
						branch::resolve_database_branch(&tx, bucket, &database, Snapshot)
							.await?
							.context("Depot database not found")
					}
				})
				.await?;
			Ok(ResolvedSelector::Resolved(ResolvedIdentity {
				bucket_id: Some(bucket),
				database_id: Some(database_id.clone()),
				branch_id,
				bucket_branch_id: None,
				selector_kind: "bucket_database",
				namespace_id: None,
				actor_id: None,
			}))
		}
		DoctorSelector::Actor {
			namespace_id,
			actor_id,
		} => {
			let bucket = BucketId::from_gas_id(*namespace_id);
			let database = actor_id.to_string();
			let branch_id = db
				.txn("depot_doctor_resolve_actor", move |tx| {
					let database = database.clone();
					async move {
						branch::resolve_database_branch(&tx, bucket, &database, Snapshot).await
					}
				})
				.await?;
			let Some(branch_id) = branch_id else {
				return Ok(ResolvedSelector::MissingActorDatabase(
					MissingActorDatabase {
						bucket_id: bucket,
						database_id: actor_id.to_string(),
						namespace_id: *namespace_id,
						actor_id: *actor_id,
					},
				));
			};
			Ok(ResolvedSelector::Resolved(ResolvedIdentity {
				bucket_id: Some(bucket),
				database_id: Some(actor_id.to_string()),
				branch_id,
				bucket_branch_id: None,
				selector_kind: "actor",
				namespace_id: Some(*namespace_id),
				actor_id: Some(*actor_id),
			}))
		}
		DoctorSelector::DatabaseBranch { database_branch_id } => {
			let branch_id = DatabaseBranchId::from_uuid(*database_branch_id);
			let pointer = find_current_pointer_for_branch(db, branch_id).await?;
			Ok(ResolvedSelector::Resolved(ResolvedIdentity {
				bucket_id: pointer.as_ref().map(|(bucket_id, _, _)| *bucket_id),
				database_id: pointer
					.as_ref()
					.map(|(_, _, database_id)| database_id.clone()),
				branch_id,
				bucket_branch_id: pointer.map(|(_, bucket_branch_id, _)| bucket_branch_id),
				selector_kind: "database_branch",
				namespace_id: None,
				actor_id: None,
			}))
		}
	}
}

async fn find_current_pointer_for_branch(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
) -> Result<Option<(BucketId, crate::types::BucketBranchId, String)>> {
	let bucket_rows =
		scan_prefix_rows(db, keys::bucket_pointer_cur_prefix(), "bucket_pointer_cur").await?;
	let mut buckets_by_branch = BTreeMap::new();
	for row in bucket_rows {
		let bucket_id = keys::decode_bucket_pointer_cur_bucket_id(&row.key)?;
		let pointer = decode_bucket_pointer(&row.value)?;
		buckets_by_branch.insert(pointer.current_branch, bucket_id);
	}

	let rows = scan_prefix_rows(
		db,
		keys::database_pointer_cur_prefix(),
		"database_pointer_cur",
	)
	.await?;
	let mut found = None;
	for row in rows {
		let pointer = decode_database_pointer(&row.value)?;
		if pointer.current_branch == branch_id {
			let (bucket_branch_id, database_id) = keys::decode_database_pointer_cur_key(&row.key)?;
			let bucket_id = buckets_by_branch
				.get(&bucket_branch_id)
				.copied()
				.context("database pointer bucket branch did not have a current bucket pointer")?;
			found = Some((bucket_id, bucket_branch_id, database_id));
			break;
		}
	}
	Ok(found)
}

async fn load_branch_record(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
) -> Result<Option<DatabaseBranchRecord>> {
	db.txn("depot_doctor_load_branch_record", move |tx| async move {
		let Some(bytes) = tx
			.informal()
			.get(&keys::branches_list_key(branch_id), Snapshot)
			.await?
		else {
			return Ok(None);
		};
		Ok(Some(decode_database_branch_record(&bytes)?))
	})
	.await
}

async fn select_existing_txid(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
	head_txid: u64,
	requested_max_txid: Option<u64>,
) -> Result<Option<u64>> {
	let cap = requested_max_txid.map_or(head_txid, |max| max.min(head_txid));
	let prefix = keys::branch_commit_prefix(branch_id);
	let rows = scan_prefix_rows(db, prefix.clone(), "branch_commit_select_txid").await?;
	Ok(rows
		.into_iter()
		.filter_map(|row| decode_u64_suffix(&prefix, &row.key).ok())
		.filter(|txid| *txid <= cap)
		.max())
}

async fn detect_unsupported(
	db: &universaldb::Database,
	input: &DoctorInput,
	resolved: &ResolvedIdentity,
	branch_record: &DatabaseBranchRecord,
	started: Instant,
	generated_at_ms: i64,
	privacy: &Privacy,
) -> Result<Option<DoctorReport>> {
	let mut reason = if branch_record.parent.is_some()
		|| branch_record.parent_versionstamp.is_some()
		|| branch_record.fork_depth != 0
		|| branch_record.created_from_restore_point.is_some()
	{
		Some("selected branch is not a current root branch")
	} else {
		None
	};

	if reason.is_none() {
		reason = db
			.txn("depotdoctor", {
				let branch_id = resolved.branch_id;
				let bucket_id = resolved.bucket_id;
				let database_id = resolved.database_id.clone();
				move |tx| {
					let database_id = database_id.clone();
					async move {
						if tx
							.informal()
							.get(&keys::branch_meta_head_at_fork_key(branch_id), Snapshot)
							.await?
							.is_some()
						{
							return Ok(Some("branch has head_at_fork metadata"));
						}
						for (label, key) in [
							(
								"cold compact metadata is present",
								keys::branch_meta_cold_compact_key(branch_id),
							),
							(
								"cold compactor lease metadata is present",
								keys::branch_meta_cold_lease_key(branch_id),
							),
							(
								"cold drained manifest metadata is present",
								keys::branch_manifest_cold_drained_txid_key(branch_id),
							),
						] {
							if tx.informal().get(&key, Snapshot).await?.is_some() {
								return Ok(Some(label));
							}
						}
						if let (Some(bucket_id), Some(database_id)) =
							(bucket_id, database_id.as_ref())
						{
							if tx
								.informal()
								.get(&keys::bucket_policy_pitr_key(bucket_id), Snapshot)
								.await?
								.is_some()
							{
								return Ok(Some("bucket PITR policy is present"));
							}
							if tx
								.informal()
								.get(
									&keys::database_pitr_policy_key(bucket_id, database_id),
									Snapshot,
								)
								.await?
								.is_some()
							{
								return Ok(Some("database PITR policy is present"));
							}
						}
						Ok(None)
					}
				}
			})
			.await?;
	}

	if reason.is_none() {
		let branch_id = resolved.branch_id;
		for (label, prefix) in [
			(
				"cold shard rows are present",
				keys::branch_compaction_cold_shard_prefix(branch_id),
			),
			(
				"retired cold object rows are present",
				keys::branch_compaction_retired_cold_object_prefix(branch_id),
			),
			(
				"PITR interval rows are present",
				keys::branch_pitr_interval_prefix(branch_id),
			),
			(
				"database pin rows are present",
				keys::db_pin_prefix(branch_id),
			),
		] {
			if prefix_has_rows(db, prefix, label).await? {
				reason = Some(label);
				break;
			}
		}
	}

	if reason.is_none() {
		if let Some(bucket_branch_id) = resolved
			.bucket_branch_id
			.or(Some(branch_record.bucket_branch))
		{
			for (label, prefix) in [
				(
					"bucket fork pin rows are present",
					keys::bucket_fork_pin_prefix(bucket_branch_id),
				),
				(
					"bucket child rows are present",
					keys::bucket_child_prefix(bucket_branch_id),
				),
			] {
				if prefix_has_rows(db, prefix, label).await? {
					reason = Some(label);
					break;
				}
			}
		}
	}

	if reason.is_none() {
		if let Some(database_id) = resolved.database_id.as_ref() {
			if prefix_has_rows(
				db,
				keys::restore_point_prefix(database_id),
				"restore_point_rows",
			)
			.await?
			{
				reason = Some("restore point rows are present");
			}
		}
	}

	if let Some(message) = reason {
		return Ok(Some(unsupported_report(
			started,
			generated_at_ms,
			privacy,
			input,
			resolved,
			"unsupported_unexpected_storage_shape",
			message,
		)));
	}

	if resolved.selector_kind == "database_branch" && resolved.database_id.is_none() {
		return Ok(Some(unsupported_report(
			started,
			generated_at_ms,
			privacy,
			input,
			resolved,
			"unsupported_unexpected_storage_shape",
			"--database-branch-id did not point at a current database pointer",
		)));
	}

	Ok(None)
}

async fn capture_snapshot(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
	privacy: &Privacy,
) -> Result<SnapshotFacts> {
	let (head_txid, head_versionstamp) = db
		.txn("depot_doctor_capture_snapshot", move |tx| {
			let privacy = privacy.clone();
			async move {
				let head = tx
					.informal()
					.get(&keys::branch_meta_head_key(branch_id), Snapshot)
					.await?
					.map(|bytes| decode_db_head(&bytes))
					.transpose()?;
				let head_versionstamp = if let Some(head) = &head {
					tx.informal()
						.get(
							&keys::branch_commit_key(branch_id, head.head_txid),
							Snapshot,
						)
						.await?
						.map(|bytes| decode_commit_row(&bytes))
						.transpose()?
						.map(|commit| privacy.hash_bytes(&commit.versionstamp))
				} else {
					None
				};
				Ok((head.as_ref().map(|head| head.head_txid), head_versionstamp))
			}
		})
		.await?;

	Ok(SnapshotFacts {
		head_txid,
		head_versionstamp,
		current_branch_hash: privacy.hash_uuid(branch_id.as_uuid()),
		commit_row_count: count_prefix_rows(
			db,
			keys::branch_commit_prefix(branch_id),
			"branch_commit_count",
		)
		.await?,
		delta_chunk_row_count: count_prefix_rows(
			db,
			keys::branch_delta_prefix(branch_id),
			"branch_delta_count",
		)
		.await?,
		pidx_row_count: count_prefix_rows(
			db,
			keys::branch_pidx_prefix(branch_id),
			"branch_pidx_count",
		)
		.await?,
		installed_hot_shard_count: count_prefix_rows(
			db,
			keys::branch_shard_prefix(branch_id),
			"branch_hot_shard_count",
		)
		.await?,
		staged_hot_shard_count: count_prefix_rows(
			db,
			keys::branch_compaction_stage_prefix(branch_id),
			"branch_compaction_stage_count",
		)
		.await?,
	})
}

async fn load_storage_facts(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
	selected_txid: u64,
	privacy: &Privacy,
) -> Result<StorageFacts> {
	let commit_scan =
		scan_prefix_rows_with_stats(db, keys::branch_commit_prefix(branch_id), "branch_commit")
			.await?;
	let delta_scan =
		scan_prefix_rows_with_stats(db, keys::branch_delta_prefix(branch_id), "branch_delta")
			.await?;
	let pidx_scan =
		scan_prefix_rows_with_stats(db, keys::branch_pidx_prefix(branch_id), "branch_pidx").await?;
	let vtx_scan =
		scan_prefix_rows_with_stats(db, keys::branch_vtx_prefix(branch_id), "branch_vtx").await?;
	let hot_scan =
		scan_prefix_rows_with_stats(db, keys::branch_shard_prefix(branch_id), "branch_hot_shard")
			.await?;
	let staged_scan = scan_prefix_rows_with_stats(
		db,
		keys::branch_compaction_stage_prefix(branch_id),
		"branch_compaction_stage",
	)
	.await?;
	let cold_rows = count_prefix_rows(
		db,
		keys::branch_compaction_cold_shard_prefix(branch_id),
		"branch_cold_shard",
	)
	.await?;
	let retired_cold_rows = count_prefix_rows(
		db,
		keys::branch_compaction_retired_cold_object_prefix(branch_id),
		"branch_retired_cold_object",
	)
	.await?;
	let pitr_rows = count_prefix_rows(
		db,
		keys::branch_pitr_interval_prefix(branch_id),
		"branch_pitr_interval",
	)
	.await?;
	let pin_rows = count_prefix_rows(db, keys::db_pin_prefix(branch_id), "db_pin").await?;
	let commit_chunk_count = commit_scan.chunk_count;
	let delta_chunk_count = delta_scan.chunk_count;
	let pidx_chunk_count = pidx_scan.chunk_count;
	let vtx_chunk_count = vtx_scan.chunk_count;
	let hot_chunk_count = hot_scan.chunk_count;
	let staged_chunk_count = staged_scan.chunk_count;
	let commit_timed_out_chunk_count = commit_scan.timed_out_chunk_count;
	let delta_timed_out_chunk_count = delta_scan.timed_out_chunk_count;
	let pidx_timed_out_chunk_count = pidx_scan.timed_out_chunk_count;
	let vtx_timed_out_chunk_count = vtx_scan.timed_out_chunk_count;
	let hot_timed_out_chunk_count = hot_scan.timed_out_chunk_count;
	let staged_timed_out_chunk_count = staged_scan.timed_out_chunk_count;
	let commit_rows = commit_scan.rows;
	let delta_rows = delta_scan.rows;
	let pidx_raw_rows = pidx_scan.rows;
	let vtx_rows = vtx_scan.rows;
	let hot_rows = hot_scan.rows;
	let staged_rows = staged_scan.rows;

	let mut commit_map = BTreeMap::new();
	for row in commit_rows {
		let txid = decode_u64_suffix(&keys::branch_commit_prefix(branch_id), &row.key)
			.context("decode commit txid")?;
		let commit = decode_commit_row(&row.value)
			.with_context(|| format!("decode commit row for txid {txid}"))?;
		commit_map.insert(txid, commit);
	}

	let mut chunks_by_txid = BTreeMap::<u64, Vec<(u32, Vec<u8>)>>::new();
	let mut delta_chunk_facts = Vec::new();
	for row in delta_rows {
		let txid = keys::decode_branch_delta_chunk_txid(branch_id, &row.key)?;
		let chunk_idx = keys::decode_branch_delta_chunk_idx(branch_id, txid, &row.key)?;
		delta_chunk_facts.push(DeltaChunkFact {
			txid,
			chunk_index: chunk_idx,
			encoded_byte_len: row.value.len(),
			chunk_hash: privacy.hash_bytes(&row.value),
			at_or_below_selected_txid: txid <= selected_txid,
		});
		if txid <= selected_txid {
			chunks_by_txid
				.entry(txid)
				.or_default()
				.push((chunk_idx, row.value));
		}
	}

	let mut commits = Vec::new();
	for (txid, row) in commit_map
		.iter()
		.filter(|(txid, _)| **txid <= selected_txid)
	{
		let (
			delta,
			delta_decode_error,
			delta_chunk_count,
			delta_chunk_indexes,
			delta_encoded_hash,
			delta_encoded_bytes,
		) = if let Some(chunks) = chunks_by_txid.get(txid) {
			let (chunk_count, chunk_indexes, encoded_hash, encoded_bytes) =
				delta_chunk_metadata(chunks, &privacy);
			match decode_delta_chunks(chunks, &privacy) {
				Ok(delta) => (
					Some(delta),
					None,
					chunk_count,
					chunk_indexes,
					encoded_hash,
					encoded_bytes,
				),
				Err(err) => (
					None,
					Some(err.to_string()),
					chunk_count,
					chunk_indexes,
					encoded_hash,
					encoded_bytes,
				),
			}
		} else {
			(
				None,
				Some("delta chunks missing".to_string()),
				0,
				Vec::new(),
				privacy.hash_bytes(&[]),
				0,
			)
		};
		commits.push(CommitData {
			txid: *txid,
			row: row.clone(),
			delta,
			delta_decode_error,
			delta_chunk_count,
			delta_chunk_indexes,
			delta_encoded_hash,
			delta_encoded_bytes,
		});
	}

	let selected_db_size_pages = commit_map
		.get(&selected_txid)
		.map(|commit| commit.db_size_pages)
		.unwrap_or(0);

	let mut pidx_rows = BTreeMap::new();
	for row in pidx_raw_rows {
		let pgno = decode_u32_suffix(&keys::branch_pidx_prefix(branch_id), &row.key)
			.context("decode pidx pgno")?;
		if row.value.len() == std::mem::size_of::<u64>() {
			pidx_rows.insert(pgno, u64::from_be_bytes(row.value.as_slice().try_into()?));
		}
	}

	let mut vtx_facts = Vec::new();
	for row in vtx_rows {
		let Some(versionstamp) = row
			.key
			.strip_prefix(keys::branch_vtx_prefix(branch_id).as_slice())
		else {
			continue;
		};
		if versionstamp.len() != 16 || row.value.len() != 8 {
			continue;
		}
		let txid = u64::from_be_bytes(row.value.as_slice().try_into()?);
		let commit = commit_map.get(&txid);
		vtx_facts.push(VtxFact {
			txid,
			versionstamp: privacy.hash_bytes(versionstamp),
			row_key_hash: privacy.hash_bytes(&row.key),
			commit_row_exists: commit.is_some(),
			commit_row_versionstamp_matches: commit
				.is_some_and(|commit| commit.versionstamp.as_slice() == versionstamp),
			at_or_below_selected_txid: txid <= selected_txid,
		});
	}

	let mut hot_shard_facts = Vec::new();
	let mut hot_shards = Vec::new();
	for row in hot_rows {
		let decoded_key = decode_hot_shard_key(branch_id, &row.key);
		let decoded_blob = decode_ltx_v3(&row.value);
		let (decode_status, decode_error, decoded_page_count, min_page, max_page) =
			match &decoded_blob {
				Ok(decoded) => {
					let min = decoded.pages.iter().map(|page| page.pgno).min();
					let max = decoded.pages.iter().map(|page| page.pgno).max();
					("ok".to_string(), None, Some(decoded.pages.len()), min, max)
				}
				Err(err) => ("error".to_string(), Some(err.to_string()), None, None, None),
			};
		let (shard_id, as_of_txid) = decoded_key.unwrap_or((u32::MAX, u64::MAX));
		if let Ok(decoded) = decoded_blob {
			if as_of_txid <= selected_txid {
				hot_shards.push(HotShardData {
					shard_id,
					as_of_txid,
					decoded,
					key_hash: privacy.hash_bytes(&row.key),
				});
			}
		}
		hot_shard_facts.push(HotShardFact {
			row_key_hash: privacy.hash_bytes(&row.key),
			shard_id: (shard_id != u32::MAX).then_some(shard_id),
			as_of_txid: (as_of_txid != u64::MAX).then_some(as_of_txid),
			encoded_size: row.value.len(),
			encoded_hash: privacy.hash_bytes(&row.value),
			decode_status,
			decode_error,
			decoded_page_count,
			min_page,
			max_page,
			eligible_at_selected_txid: as_of_txid <= selected_txid,
		});
	}

	let inventory = json!({
		"commit_rows": commit_map.len(),
		"vtx_rows": vtx_facts.len(),
		"delta_chunk_rows": delta_chunk_facts.len(),
		"pidx_rows": pidx_rows.len(),
		"branch_rows": 1,
		"head_rows": usize::from(commit_map.contains_key(&selected_txid)),
		"hot_shard_rows": hot_shard_facts.len(),
		"staged_hot_shard_rows": staged_rows.len(),
		"cold_compaction_rows": cold_rows,
		"retired_cold_object_rows": retired_cold_rows,
		"pitr_rows": pitr_rows,
		"pin_rows": pin_rows,
		"approximate_raw_bytes_scanned": delta_chunk_facts.iter().map(|row| row.encoded_byte_len).sum::<usize>(),
		"scan_chunks": {
			"commits": commit_chunk_count,
			"deltas": delta_chunk_count,
			"pidx": pidx_chunk_count,
			"vtx": vtx_chunk_count,
			"hot_shards": hot_chunk_count,
			"staged_hot_shards": staged_chunk_count,
		},
		"scan_timed_out_chunks": {
			"commits": commit_timed_out_chunk_count,
			"deltas": delta_timed_out_chunk_count,
			"pidx": pidx_timed_out_chunk_count,
			"vtx": vtx_timed_out_chunk_count,
			"hot_shards": hot_timed_out_chunk_count,
			"staged_hot_shards": staged_timed_out_chunk_count,
		},
	});
	let compaction = json!({
		"hot_present": !hot_shard_facts.is_empty() || !staged_rows.is_empty(),
		"installed_hot_shard_count": hot_shard_facts.len(),
		"staged_hot_shard_count": staged_rows.len(),
		"cold_present": cold_rows > 0 || retired_cold_rows > 0,
		"conclusion": if !hot_shard_facts.is_empty() || !staged_rows.is_empty() {
			"hot compaction state exists; hot compaction must be included in diagnosis"
		} else {
			"hot compaction state not detected"
		},
	});

	Ok(StorageFacts {
		inventory,
		compaction,
		commits,
		vtx_facts,
		delta_chunk_facts,
		pidx_rows,
		hot_shard_facts,
		hot_shards,
		selected_db_size_pages,
	})
}

fn delta_chunk_metadata(
	chunks: &[(u32, Vec<u8>)],
	privacy: &Privacy,
) -> (usize, Vec<u32>, String, usize) {
	let mut chunks = chunks.to_vec();
	chunks.sort_by_key(|(idx, _)| *idx);
	let chunk_count = chunks.len();
	let mut encoded = Vec::new();
	let mut indexes = Vec::new();
	for (idx, chunk) in chunks {
		indexes.push(idx);
		encoded.extend_from_slice(&chunk);
	}
	(
		chunk_count,
		indexes,
		privacy.hash_bytes(&encoded),
		encoded.len(),
	)
}

fn decode_delta_chunks(chunks: &[(u32, Vec<u8>)], privacy: &Privacy) -> Result<DecodedDelta> {
	let mut chunks = chunks.to_vec();
	chunks.sort_by_key(|(idx, _)| *idx);
	let mut encoded = Vec::new();
	let mut indexes = Vec::new();
	for (expected, (idx, chunk)) in chunks.iter().enumerate() {
		ensure!(
			*idx == u32::try_from(expected).unwrap_or(u32::MAX),
			"delta chunks are not contiguous from chunk 0"
		);
		indexes.push(*idx);
		encoded.extend_from_slice(chunk);
	}
	let decoded = decode_ltx_v3(&encoded)?;
	Ok(DecodedDelta {
		encoded_hash: privacy.hash_bytes(&encoded),
		encoded_bytes: encoded.len(),
		chunk_count: chunks.len(),
		chunk_indexes: indexes,
		decoded,
	})
}

fn build_sequential_replay(
	path: &Path,
	commits: &[CommitData],
	selected_txid: u64,
	selected_db_size_pages: u32,
	privacy: &Privacy,
) -> Result<ReplayResult> {
	let mut file = File::create(path).context("create sequential replay SQLite image")?;
	let mut page_size = PAGE_SIZE;
	let mut computed_owners = BTreeMap::<u32, u64>::new();
	let mut zero_filled_pages = BTreeSet::<u32>::new();
	let mut dirty_pages_above_eof = Vec::new();
	let mut commit_facts = Vec::new();
	let mut delta_facts = Vec::new();
	let mut current_size = 0u32;

	for commit in commits.iter().filter(|commit| commit.txid <= selected_txid) {
		let mut dirty_pages = Vec::new();
		let mut decode_status = "missing_delta".to_string();
		let mut decode_error = None;
		if let Some(delta) = &commit.delta {
			page_size = delta.decoded.header.page_size;
			decode_status = "ok".to_string();
			let mut page_length_error = None;
			for page in &delta.decoded.pages {
				if page.bytes.len() != page_size as usize {
					page_length_error = Some(format!(
						"decoded page {} had {} bytes, expected {}",
						page.pgno,
						page.bytes.len(),
						page_size
					));
					break;
				}
				let offset = u64::from(page.pgno - 1) * u64::from(page_size);
				file.seek(SeekFrom::Start(offset))?;
				file.write_all(&page.bytes)?;
				computed_owners.insert(page.pgno, commit.txid);
				dirty_pages.push(page.pgno);
				if page.pgno > commit.row.db_size_pages {
					dirty_pages_above_eof.push(json!({
						"txid": commit.txid,
						"pgno": page.pgno,
						"post_commit_db_size_pages": commit.row.db_size_pages,
					}));
				}
			}
			if let Some(error) = page_length_error {
				decode_status = "wrong_page_length".to_string();
				decode_error = Some(error);
			}
			delta_facts.push(DeltaFact {
				txid: commit.txid,
				chunk_count: delta.chunk_count,
				chunk_indexes: delta.chunk_indexes.clone(),
				total_encoded_bytes: delta.encoded_bytes,
				encoded_delta_hash: delta.encoded_hash.clone(),
				decode_status: decode_status.clone(),
				decode_error: decode_error.clone(),
				decoded_min_txid: Some(delta.decoded.header.min_txid),
				decoded_max_txid: Some(delta.decoded.header.max_txid),
				decoded_page_size: Some(delta.decoded.header.page_size),
				decoded_database_size: Some(delta.decoded.header.commit),
				dirty_page_count: Some(delta.decoded.pages.len()),
				dirty_page_hashes: delta
					.decoded
					.pages
					.iter()
					.map(|page| PageHashFact {
						pgno: page.pgno,
						hash: privacy.hash_bytes(&page.bytes),
					})
					.collect(),
			});
		} else {
			decode_error = commit.delta_decode_error.clone();
			delta_facts.push(DeltaFact {
				txid: commit.txid,
				chunk_count: commit.delta_chunk_count,
				chunk_indexes: commit.delta_chunk_indexes.clone(),
				total_encoded_bytes: commit.delta_encoded_bytes,
				encoded_delta_hash: commit.delta_encoded_hash.clone(),
				decode_status: if commit.delta_chunk_count == 0 {
					"missing".to_string()
				} else {
					"decode_error".to_string()
				},
				decode_error: decode_error.clone(),
				decoded_min_txid: None,
				decoded_max_txid: None,
				decoded_page_size: None,
				decoded_database_size: None,
				dirty_page_count: None,
				dirty_page_hashes: Vec::new(),
			});
		}

		let old_size = current_size;
		current_size = commit.row.db_size_pages;
		file.set_len(u64::from(current_size) * u64::from(page_size))?;
		computed_owners.retain(|pgno, _| *pgno <= current_size);
		for pgno in 1..=current_size {
			if !computed_owners.contains_key(&pgno) {
				zero_filled_pages.insert(pgno);
			}
		}

		commit_facts.push(CommitFact {
			txid: commit.txid,
			wall_clock_ms: commit.row.wall_clock_ms,
			versionstamp: privacy.hash_bytes(&commit.row.versionstamp),
			db_size_pages: commit.row.db_size_pages,
			post_apply_checksum: commit.row.post_apply_checksum,
			at_or_below_selected_txid: true,
			dirty_page_count: Some(dirty_pages.len()),
			min_dirty_page: dirty_pages.iter().copied().min(),
			max_dirty_page: dirty_pages.iter().copied().max(),
			decode_status,
			decode_error,
		});

		if current_size < old_size {
			for pgno in current_size + 1..=old_size {
				computed_owners.remove(&pgno);
			}
		}
	}

	file.sync_all()?;
	let page_hashes = read_page_hashes(path, page_size, selected_db_size_pages, privacy)?;
	Ok(ReplayResult {
		path: path.to_path_buf(),
		page_size,
		db_size_pages: selected_db_size_pages,
		image_hash: privacy.hash_bytes(&fs::read(path)?),
		page_hashes,
		computed_owners,
		zero_filled_pages: zero_filled_pages.into_iter().collect(),
		dirty_pages_above_eof,
		commit_facts,
		delta_facts,
	})
}

async fn build_resolver_image(
	path: &Path,
	udb: &universaldb::Database,
	resolved: &ResolvedIdentity,
	storage: &StorageFacts,
	replay: &ReplayResult,
	selected_txid: u64,
	skip_page_provenance: bool,
	privacy: &Privacy,
) -> Result<ResolverResult> {
	let Some(bucket_id) = resolved.bucket_id else {
		return Ok(build_mirrored_resolver_image(
			path,
			storage,
			replay,
			selected_txid,
			skip_page_provenance,
			privacy,
		)?);
	};
	let Some(database_id) = &resolved.database_id else {
		return Ok(build_mirrored_resolver_image(
			path,
			storage,
			replay,
			selected_txid,
			skip_page_provenance,
			privacy,
		)?);
	};

	let depot_db = Db::new(
		Arc::new(udb.clone()),
		Id::v1(bucket_id.as_uuid(), 0),
		database_id.clone(),
		NodeId::new(),
	);
	let pgnos = (1..=storage.selected_db_size_pages).collect::<Vec<_>>();
	let pages = depot_db
		.get_pages_with_options(
			pgnos,
			GetPagesOptions {
				expected_head_txid: None,
				mode: DepotReadMode::DiagnosticNoSideEffects,
				collect_provenance: !skip_page_provenance,
				diagnostic_max_txid: Some(selected_txid),
			},
		)
		.await;
	let pages = match pages {
		Ok(pages) => pages,
		Err(err) => {
			return Ok(ResolverResult {
				path: None,
				image_hash: None,
				page_hashes: BTreeMap::new(),
				provenance: Vec::new(),
				hot_mismatch_pages: Vec::new(),
				error: Some(resolver_error_json(&err, privacy)),
			});
		}
	};

	let mut file = File::create(path).context("create Depot-resolved SQLite image")?;
	file.set_len(u64::from(storage.selected_db_size_pages) * u64::from(replay.page_size))?;
	for page in pages.pages {
		if page.pgno > storage.selected_db_size_pages {
			continue;
		}
		let bytes = page
			.bytes
			.unwrap_or_else(|| vec![0; replay.page_size as usize]);
		ensure!(
			bytes.len() == replay.page_size as usize,
			"real Depot resolver returned page {} with {} bytes, expected {}",
			page.pgno,
			bytes.len(),
			replay.page_size
		);
		let offset = u64::from(page.pgno - 1) * u64::from(replay.page_size);
		file.seek(SeekFrom::Start(offset))?;
		file.write_all(&bytes)?;
	}
	file.sync_all()?;

	let page_hashes = read_page_hashes(
		path,
		replay.page_size,
		storage.selected_db_size_pages,
		privacy,
	)?;
	let provenance = if skip_page_provenance {
		Vec::new()
	} else {
		pages
			.provenance
			.iter()
			.map(|row| {
				page_provenance_from_real_resolver(row, &page_hashes, storage, replay, privacy)
			})
			.collect::<Vec<_>>()
	};
	let hot_mismatch_pages = provenance
		.iter()
		.filter(|row| {
			row.depot_winner_kind == "hot_shard"
				&& replay.page_hashes.get(&row.pgno) != page_hashes.get(&row.pgno)
		})
		.map(|row| row.pgno)
		.collect();

	Ok(ResolverResult {
		path: Some(path.to_path_buf()),
		image_hash: Some(privacy.hash_bytes(&fs::read(path)?)),
		page_hashes,
		provenance,
		hot_mismatch_pages,
		error: None,
	})
}

fn build_mirrored_resolver_image(
	path: &Path,
	storage: &StorageFacts,
	replay: &ReplayResult,
	selected_txid: u64,
	skip_page_provenance: bool,
	privacy: &Privacy,
) -> Result<ResolverResult> {
	let mut hot_by_shard = BTreeMap::<u32, &HotShardData>::new();
	for shard in &storage.hot_shards {
		if shard.as_of_txid <= selected_txid {
			match hot_by_shard.get(&shard.shard_id) {
				Some(existing) if existing.as_of_txid >= shard.as_of_txid => {}
				_ => {
					hot_by_shard.insert(shard.shard_id, shard);
				}
			}
		}
	}

	let delta_by_txid = storage
		.commits
		.iter()
		.filter_map(|commit| commit.delta.as_ref().map(|delta| (commit.txid, delta)))
		.collect::<BTreeMap<_, _>>();

	let mut file = File::create(path).context("create Depot-resolved SQLite image")?;
	file.set_len(u64::from(storage.selected_db_size_pages) * u64::from(replay.page_size))?;

	let mut provenance = Vec::new();
	let mut hot_mismatch_pages = Vec::new();
	for pgno in 1..=storage.selected_db_size_pages {
		let pidx_owner = storage.pidx_rows.get(&pgno).copied();
		let replay_owner = replay.computed_owners.get(&pgno).copied();
		let replay_hash = replay
			.page_hashes
			.get(&pgno)
			.cloned()
			.unwrap_or_else(|| privacy.hash_bytes(&vec![0; replay.page_size as usize]));
		let mut candidate_summaries = Vec::new();
		let mut winner_kind = "zero_fill";
		let mut winner_txid = None;
		let mut winner_bytes = vec![0; replay.page_size as usize];

		if let Some(owner_txid) = pidx_owner {
			if let Some(delta) = delta_by_txid.get(&owner_txid) {
				if let Some(bytes) = delta.decoded.get_page(pgno) {
					winner_kind = "pidx_delta";
					winner_txid = Some(owner_txid);
					winner_bytes = bytes.to_vec();
					candidate_summaries.push(json!({
						"kind": "pidx_delta",
						"txid": owner_txid,
						"result": "won",
					}));
				} else {
					candidate_summaries.push(json!({
						"kind": "stale_delta",
						"txid": owner_txid,
						"result": "lost",
						"reason": "delta_does_not_contain_page",
					}));
				}
			} else {
				candidate_summaries.push(json!({
					"kind": "missing_delta",
					"txid": owner_txid,
					"result": "lost",
					"reason": "delta_missing_or_malformed",
				}));
			}
		}

		if winner_kind == "zero_fill" {
			let shard_id = pgno / SHARD_SIZE;
			if let Some(shard) = hot_by_shard.get(&shard_id) {
				if let Some(bytes) = shard.decoded.get_page(pgno) {
					winner_kind = "hot_shard";
					winner_txid = Some(shard.as_of_txid);
					winner_bytes = bytes.to_vec();
					candidate_summaries.push(json!({
						"kind": "hot_shard",
						"shard_id": shard.shard_id,
						"as_of_txid": shard.as_of_txid,
						"row_key_hash": shard.key_hash,
						"result": "won",
					}));
				} else {
					candidate_summaries.push(json!({
						"kind": "hot_shard",
						"shard_id": shard.shard_id,
						"as_of_txid": shard.as_of_txid,
						"row_key_hash": shard.key_hash,
						"result": "lost",
						"reason": "shard_does_not_contain_page",
					}));
				}
			}
		}

		let offset = u64::from(pgno - 1) * u64::from(replay.page_size);
		file.seek(SeekFrom::Start(offset))?;
		file.write_all(&winner_bytes)?;
		let winner_hash = privacy.hash_bytes(&winner_bytes);
		let differs = winner_hash != replay_hash;
		if differs && winner_kind == "hot_shard" {
			hot_mismatch_pages.push(pgno);
		}
		if !skip_page_provenance {
			if candidate_summaries.is_empty() {
				candidate_summaries.push(json!({
					"kind": "zero_fill",
					"result": "won",
				}));
			}
			provenance.push(PageProvenanceFact {
				pgno,
				replay_winner_txid: replay_owner,
				replay_winner_hash: replay_hash,
				depot_winner_kind: winner_kind,
				depot_winner_txid: winner_txid,
				depot_winner_hash: winner_hash,
				stored_pidx_owner_txid: pidx_owner,
				computed_replay_owner_txid: replay_owner,
				candidate_count: candidate_summaries.len(),
				candidate_summaries,
				inside_selected_eof: true,
				differs_between_images: differs,
			});
		}
	}

	file.sync_all()?;
	Ok(ResolverResult {
		path: Some(path.to_path_buf()),
		image_hash: Some(privacy.hash_bytes(&fs::read(path)?)),
		page_hashes: read_page_hashes(
			path,
			replay.page_size,
			storage.selected_db_size_pages,
			privacy,
		)?,
		provenance,
		hot_mismatch_pages,
		error: None,
	})
}

fn page_provenance_from_real_resolver(
	row: &DepotPageSourceProvenance,
	page_hashes: &BTreeMap<u32, String>,
	storage: &StorageFacts,
	replay: &ReplayResult,
	privacy: &Privacy,
) -> PageProvenanceFact {
	let replay_hash = replay
		.page_hashes
		.get(&row.pgno)
		.cloned()
		.unwrap_or_else(|| privacy.hash_bytes(&vec![0; replay.page_size as usize]));
	let depot_hash = page_hashes
		.get(&row.pgno)
		.cloned()
		.unwrap_or_else(|| privacy.hash_bytes(&vec![0; replay.page_size as usize]));
	let candidate_summaries = row
		.candidates
		.iter()
		.map(|candidate| {
			json!({
				"kind": page_source_kind_label(candidate.kind),
				"txid": candidate.txid,
				"shard_id": candidate.shard_id,
				"result": candidate_result_label(candidate.result),
				"reason": candidate.reason,
			})
		})
		.collect::<Vec<_>>();
	PageProvenanceFact {
		pgno: row.pgno,
		replay_winner_txid: replay.computed_owners.get(&row.pgno).copied(),
		replay_winner_hash: replay_hash.clone(),
		depot_winner_kind: page_source_kind_label(row.winner_kind),
		depot_winner_txid: row.winner_txid,
		depot_winner_hash: depot_hash.clone(),
		stored_pidx_owner_txid: storage.pidx_rows.get(&row.pgno).copied(),
		computed_replay_owner_txid: replay.computed_owners.get(&row.pgno).copied(),
		candidate_count: candidate_summaries.len(),
		candidate_summaries,
		inside_selected_eof: true,
		differs_between_images: replay_hash != depot_hash,
	}
}

fn page_source_kind_label(kind: PageSourceKind) -> &'static str {
	match kind {
		PageSourceKind::PidxDelta => "pidx_delta",
		PageSourceKind::HistoricalDelta => "historical_delta",
		PageSourceKind::MissingDelta => "missing_delta",
		PageSourceKind::StaleDelta => "stale_delta",
		PageSourceKind::HotShard => "hot_shard",
		PageSourceKind::Cold => "cold",
		PageSourceKind::ZeroFill => "zero_fill",
		PageSourceKind::OutOfRange => "out_of_range",
	}
}

fn candidate_result_label(
	result: crate::conveyer::types::PageSourceCandidateResult,
) -> &'static str {
	match result {
		crate::conveyer::types::PageSourceCandidateResult::Won => "won",
		crate::conveyer::types::PageSourceCandidateResult::Lost => "lost",
		crate::conveyer::types::PageSourceCandidateResult::Selected => "selected",
	}
}

fn resolver_error_json(err: &anyhow::Error, privacy: &Privacy) -> Value {
	let debug = format!("{err:#}");
	let kind = if debug.contains("decode source blob") {
		"malformed_source_blob"
	} else if debug.contains("ShardCoverageMissing") || debug.contains("shard coverage") {
		"shard_coverage_missing"
	} else if debug.contains("diagnostic max txid commit row is missing") {
		"missing_commit"
	} else {
		"resolver_failed"
	};
	json!({
		"ok": false,
		"kind": kind,
		"message": "Depot diagnostic resolver failed",
		"debug_error_hash": privacy.hash_str(&debug),
	})
}

fn sqlite_checks(
	path_kind: &'static str,
	path: &Path,
	full_integrity: bool,
) -> Result<SqliteChecks> {
	let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
		.with_context(|| format!("open {path_kind} SQLite image read-only"))?;
	conn.execute_batch("PRAGMA query_only=ON;")?;
	let quick_check = run_sqlite_check(&conn, "PRAGMA quick_check");
	let integrity_check = full_integrity.then(|| run_sqlite_check(&conn, "PRAGMA integrity_check"));
	let pragmas = json!({
		"page_count": pragma_i64(&conn, "PRAGMA page_count"),
		"freelist_count": pragma_i64(&conn, "PRAGMA freelist_count"),
		"page_size": pragma_i64(&conn, "PRAGMA page_size"),
		"schema_version": pragma_i64(&conn, "PRAGMA schema_version"),
		"user_version": pragma_i64(&conn, "PRAGMA user_version"),
	});
	let header = sqlite_header_facts(path)?;
	let schema = schema_summary(&conn);
	Ok(SqliteChecks {
		path_kind,
		quick_check,
		integrity_check,
		pragmas,
		header,
		schema,
	})
}

fn run_sqlite_check(conn: &Connection, pragma: &str) -> SqliteCheckResult {
	let mut rows = Vec::new();
	let mut truncated = false;
	let result = (|| -> Result<()> {
		let mut stmt = conn.prepare(pragma)?;
		let mut query = stmt.query([])?;
		while let Some(row) = query.next()? {
			if rows.len() >= SQLITE_CHECK_ROW_LIMIT {
				truncated = true;
				continue;
			}
			rows.push(row.get::<_, String>(0)?);
		}
		Ok(())
	})();
	let ok = result.is_ok() && rows.iter().all(|row| row.eq_ignore_ascii_case("ok"));
	SqliteCheckResult {
		ran: true,
		ok,
		rows,
		error: result.err().map(|err| err.to_string()),
		truncated,
	}
}

fn pragma_i64(conn: &Connection, pragma: &str) -> Value {
	match conn.query_row(pragma, [], |row| row.get::<_, i64>(0)) {
		Ok(value) => json!(value),
		Err(err) => json!({ "error": err.to_string() }),
	}
}

fn schema_summary(conn: &Connection) -> Value {
	let result = (|| -> Result<Value> {
		let mut stmt = conn.prepare(
			"SELECT type, name, tbl_name, rootpage, sql FROM sqlite_master ORDER BY rootpage",
		)?;
		let rows = stmt
			.query_map([], |row| {
				Ok(json!({
					"object_type": row.get::<_, String>(0)?,
					"name": row.get::<_, String>(1)?,
					"table_name": row.get::<_, String>(2)?,
					"root_page": row.get::<_, i64>(3)?,
					"sql": row.get::<_, Option<String>>(4)?,
				}))
			})?
			.collect::<rusqlite::Result<Vec<_>>>()?;
		Ok(json!({ "ok": true, "rows": LimitedRows::from_rows(rows) }))
	})();
	result.unwrap_or_else(|err| json!({ "ok": false, "error": err.to_string() }))
}

fn sqlite_header_facts(path: &Path) -> Result<Value> {
	let mut file = File::open(path)?;
	let mut header = [0u8; 100];
	let read = file.read(&mut header)?;
	if read < 100 {
		return Ok(json!({ "readable": false, "file_size": file.metadata()?.len() }));
	}
	Ok(json!({
		"readable": true,
		"page_size": u16::from_be_bytes([header[16], header[17]]),
		"change_counter": u32::from_be_bytes(header[24..28].try_into()?),
		"page_count": u32::from_be_bytes(header[28..32].try_into()?),
		"freelist_trunk_page": u32::from_be_bytes(header[32..36].try_into()?),
		"freelist_count": u32::from_be_bytes(header[36..40].try_into()?),
		"schema_cookie": u32::from_be_bytes(header[40..44].try_into()?),
	}))
}

fn sqlite_checks_ok(checks: &SqliteChecks) -> bool {
	checks.quick_check.ok
		&& checks
			.integrity_check
			.as_ref()
			.map(|check| check.ok)
			.unwrap_or(true)
}

fn compare_images(replay: &ReplayResult, resolver: &ResolverResult, page_count: u32) -> Value {
	let Some(resolver_image_hash) = resolver.image_hash.as_ref() else {
		return json!({
			"skipped": true,
			"skip_reason": "resolver_failed",
			"error": resolver.error,
		});
	};
	let differing_pages = (1..=page_count)
		.filter(|pgno| replay.page_hashes.get(pgno) != resolver.page_hashes.get(pgno))
		.collect::<Vec<_>>();
	let hot_disagree = resolver
		.provenance
		.iter()
		.filter(|row| row.differs_between_images && row.depot_winner_kind == "hot_shard")
		.map(|row| row.pgno)
		.collect::<Vec<_>>();
	let pidx_owner_disagree = resolver
		.provenance
		.iter()
		.filter(|row| row.stored_pidx_owner_txid != row.computed_replay_owner_txid)
		.map(|row| row.pgno)
		.collect::<Vec<_>>();
	let zero_fill_nonzero = resolver
		.provenance
		.iter()
		.filter(|row| row.differs_between_images && row.depot_winner_kind == "zero_fill")
		.map(|row| row.pgno)
		.collect::<Vec<_>>();
	json!({
		"byte_identical": replay.image_hash == *resolver_image_hash,
		"sequential_image_hash": replay.image_hash,
		"depot_resolved_image_hash": resolver_image_hash,
		"first_differing_page": differing_pages.first().copied(),
		"differing_page_count": differing_pages.len(),
		"differing_pages": LimitedRows::from_rows(differing_pages),
		"mismatch_buckets": {
			"hot_shard_won_but_replay_delta_disagrees": LimitedRows::from_rows(hot_disagree),
			"stored_pidx_owner_disagrees_with_computed_replay_owner": LimitedRows::from_rows(pidx_owner_disagree),
			"depot_zero_filled_but_replay_nonzero": LimitedRows::from_rows(zero_fill_nonzero),
		}
	})
}

fn find_first_bad_txid(
	temp_dir: &TempDir,
	commits: &[CommitData],
	min_txid: u64,
	selected_txid: u64,
	full_integrity: bool,
) -> Result<Value> {
	let mut previous_good_txid = None;
	for commit in commits
		.iter()
		.filter(|commit| commit.txid >= min_txid && commit.txid <= selected_txid)
	{
		let path = temp_dir
			.path()
			.join(format!("first-bad-candidate-{}.sqlite", commit.txid));
		let privacy = Privacy::new();
		let replay = build_sequential_replay(
			&path,
			commits,
			commit.txid,
			commit.row.db_size_pages,
			&privacy,
		)?;
		let checks = sqlite_checks("first_bad_candidate", &replay.path, full_integrity)?;
		if sqlite_checks_ok(&checks) {
			previous_good_txid = Some(commit.txid);
			continue;
		}
		let changed_pages = commit
			.delta
			.as_ref()
			.map(|delta| {
				delta
					.decoded
					.pages
					.iter()
					.map(|page| page.pgno)
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		return Ok(json!({
			"enabled": true,
			"search_strategy": "linear_exact",
			"search_status": "found",
			"confidence": "exact_for_sequential_replay",
			"earliest_bad_txid_found": commit.txid,
			"last_validated_good_txid": previous_good_txid,
			"previous_good_txid": previous_good_txid,
			"first_bad_txid": commit.txid,
			"sqlite_failure_at_first_bad_txid": checks,
			"pages_changed_by_first_bad_txid": LimitedRows::from_rows(changed_pages),
			"database_size_after_first_bad_txid": commit.row.db_size_pages,
			"depot_resolved_evidence": "not_searched",
		}));
	}
	Ok(json!({
		"enabled": true,
		"search_strategy": "linear_exact",
		"search_status": "not_found",
		"previous_good_txid": previous_good_txid,
		"first_bad_txid": null,
	}))
}

async fn find_first_resolver_divergence(
	temp_dir: &TempDir,
	db: &universaldb::Database,
	resolved: &ResolvedIdentity,
	storage: &StorageFacts,
	min_txid: u64,
	selected_txid: u64,
	skip_page_provenance: bool,
	privacy: &Privacy,
) -> Result<Value> {
	let mut previous_good_txid = None;
	for commit in storage
		.commits
		.iter()
		.filter(|commit| commit.txid >= min_txid && commit.txid <= selected_txid)
	{
		let replay_path = temp_dir.path().join(format!(
			"resolver-divergence-sequential-{}.sqlite",
			commit.txid
		));
		let resolver_path = temp_dir
			.path()
			.join(format!("resolver-divergence-depot-{}.sqlite", commit.txid));
		let replay = build_sequential_replay(
			&replay_path,
			&storage.commits,
			commit.txid,
			commit.row.db_size_pages,
			privacy,
		)?;
		let mut storage_at_txid = storage.clone();
		storage_at_txid.selected_db_size_pages = commit.row.db_size_pages;
		let resolver = build_resolver_image(
			&resolver_path,
			db,
			resolved,
			&storage_at_txid,
			&replay,
			commit.txid,
			skip_page_provenance,
			privacy,
		)
		.await?;
		let comparison = compare_images(&replay, &resolver, commit.row.db_size_pages);
		if comparison.get("byte_identical").and_then(Value::as_bool) == Some(true) {
			previous_good_txid = Some(commit.txid);
			continue;
		}

		return Ok(json!({
			"enabled": true,
			"search_strategy": "linear_exact",
			"search_status": "found",
			"confidence": "exact_for_resolver_comparison",
			"earliest_bad_txid_found": commit.txid,
			"last_validated_good_txid": previous_good_txid,
			"previous_good_txid": previous_good_txid,
			"first_bad_txid": commit.txid,
			"sqlite_failure_at_first_bad_txid": null,
			"depot_resolved_evidence": comparison,
		}));
	}

	Ok(json!({
		"enabled": true,
		"search_strategy": "linear_exact",
		"search_status": "not_found",
		"confidence": "exact_for_resolver_comparison",
		"previous_good_txid": previous_good_txid,
		"first_bad_txid": null,
		"depot_resolved_evidence": "not_found",
	}))
}

fn classify_report(
	sequential: &SqliteChecks,
	resolver_checks: Option<&SqliteChecks>,
	comparison: Option<&Value>,
	resolver: Option<&ResolverResult>,
	changed: bool,
	commit_chain: &Value,
	versionstamp_index: &Value,
	delta_integrity: &Value,
	pidx_integrity: &Value,
	resolver_error: Option<&Value>,
) -> DoctorVerdict {
	if changed {
		return DoctorVerdict {
			verdict: DoctorVerdictKind::Inconclusive,
			corruption_class: CorruptionClass::Unknown,
			unsupported_reason: None,
			reason: Some("database_changed_during_diagnosis"),
			message: "database changed while doctor was running".to_string(),
		};
	}

	if !sqlite_checks_ok(sequential) {
		return DoctorVerdict {
			verdict: DoctorVerdictKind::Corrupt,
			corruption_class: CorruptionClass::DeltaHistory,
			unsupported_reason: None,
			reason: Some("sequential_replay_invalid"),
			message: "raw delta history replays into an invalid SQLite image".to_string(),
		};
	}

	if !analysis_ok(delta_integrity) {
		return DoctorVerdict {
			verdict: DoctorVerdictKind::Corrupt,
			corruption_class: CorruptionClass::DeltaHistory,
			unsupported_reason: None,
			reason: Some("delta_integrity_failed"),
			message: "raw delta history contains missing or malformed delta data".to_string(),
		};
	}

	if !analysis_ok(commit_chain) {
		return DoctorVerdict {
			verdict: DoctorVerdictKind::Corrupt,
			corruption_class: CorruptionClass::DeltaHistory,
			unsupported_reason: None,
			reason: Some("commit_chain_failed"),
			message: "commit history has missing txids before the selected head".to_string(),
		};
	}

	if !analysis_ok(versionstamp_index) {
		return DoctorVerdict {
			verdict: DoctorVerdictKind::Corrupt,
			corruption_class: CorruptionClass::Unknown,
			unsupported_reason: None,
			reason: Some("versionstamp_index_failed"),
			message: "versionstamp index does not match commit history".to_string(),
		};
	}

	if !analysis_ok(pidx_integrity) {
		return DoctorVerdict {
			verdict: DoctorVerdictKind::Corrupt,
			corruption_class: CorruptionClass::DepotReconstruction,
			unsupported_reason: None,
			reason: Some("pidx_integrity_failed"),
			message: "PIDX page ownership does not match replayed delta history".to_string(),
		};
	}

	if resolver_error.is_some() {
		return DoctorVerdict {
			verdict: DoctorVerdictKind::Corrupt,
			corruption_class: CorruptionClass::DepotReconstruction,
			unsupported_reason: None,
			reason: Some("depot_resolver_failed"),
			message: "Depot diagnostic resolver failed while sequential replay was valid"
				.to_string(),
		};
	}

	if comparison
		.and_then(|value| value.get("byte_identical"))
		.and_then(Value::as_bool)
		== Some(false)
	{
		let hot = resolver.is_some_and(|resolver| !resolver.hot_mismatch_pages.is_empty());
		return DoctorVerdict {
			verdict: DoctorVerdictKind::Corrupt,
			corruption_class: if hot {
				CorruptionClass::HotCompactionState
			} else {
				CorruptionClass::DepotReconstruction
			},
			unsupported_reason: None,
			reason: Some("depot_resolved_image_differs"),
			message: "Depot-resolved image differs from sequential replay".to_string(),
		};
	}

	if resolver_checks.is_some_and(|checks| !sqlite_checks_ok(checks)) {
		return DoctorVerdict {
			verdict: DoctorVerdictKind::Corrupt,
			corruption_class: CorruptionClass::DepotReconstruction,
			unsupported_reason: None,
			reason: Some("depot_resolved_image_invalid"),
			message: "Depot-resolved image is invalid while sequential replay is valid".to_string(),
		};
	}

	DoctorVerdict {
		verdict: DoctorVerdictKind::Healthy,
		corruption_class: CorruptionClass::None,
		unsupported_reason: None,
		reason: None,
		message: "selected database is healthy within the supported hot/delta scope".to_string(),
	}
}

fn analysis_ok(value: &Value) -> bool {
	value.get("ok").and_then(Value::as_bool).unwrap_or(true)
}

fn analyze_commit_chain(commits: &[CommitData], selected_txid: u64) -> Value {
	let txids = commits.iter().map(|commit| commit.txid).collect::<Vec<_>>();
	let missing = if txids.is_empty() {
		Vec::new()
	} else {
		(1..=selected_txid)
			.filter(|txid| !txids.binary_search(txid).is_ok())
			.collect::<Vec<_>>()
	};
	json!({
		"ok": missing.is_empty(),
		"selected_txid": selected_txid,
		"commit_count": commits.len(),
		"missing_txids": LimitedRows::from_rows(missing),
	})
}

fn analyze_vtx(storage: &StorageFacts) -> Value {
	let vtx_txids = storage
		.vtx_facts
		.iter()
		.filter(|row| row.at_or_below_selected_txid)
		.map(|row| row.txid)
		.collect::<BTreeSet<_>>();
	let commits_missing_vtx = storage
		.commits
		.iter()
		.filter(|commit| !vtx_txids.contains(&commit.txid))
		.map(|commit| commit.txid)
		.collect::<Vec<_>>();
	let missing_commit = storage
		.vtx_facts
		.iter()
		.filter(|row| row.at_or_below_selected_txid && !row.commit_row_exists)
		.map(|row| row.txid)
		.collect::<Vec<_>>();
	let mismatched = storage
		.vtx_facts
		.iter()
		.filter(|row| row.at_or_below_selected_txid && !row.commit_row_versionstamp_matches)
		.map(|row| row.txid)
		.collect::<Vec<_>>();
	json!({
		"ok": commits_missing_vtx.is_empty() && missing_commit.is_empty() && mismatched.is_empty(),
		"commits_missing_vtx": LimitedRows::from_rows(commits_missing_vtx),
		"vtx_rows_missing_commit": LimitedRows::from_rows(missing_commit),
		"vtx_rows_mismatched_commit_versionstamp": LimitedRows::from_rows(mismatched),
	})
}

fn analyze_delta_integrity(replay: &ReplayResult) -> Value {
	let bad = replay
		.delta_facts
		.iter()
		.filter(|delta| delta.decode_status != "ok")
		.map(|delta| delta.txid)
		.collect::<Vec<_>>();
	json!({
		"ok": bad.is_empty(),
		"bad_delta_txids": LimitedRows::from_rows(bad),
	})
}

fn analyze_pidx(
	storage: &StorageFacts,
	replay: &ReplayResult,
	selected_txid: u64,
	actual_head_txid: u64,
) -> Value {
	let pidx = storage.pidx_facts(replay);
	let mismatches = pidx
		.iter()
		.filter(|row| {
			row.inside_selected_eof
				&& if row.owner_txid > selected_txid {
					selected_txid == actual_head_txid
				} else {
					!row.stored_owner_matches_computed_owner
				}
		})
		.map(|row| row.pgno)
		.collect::<Vec<_>>();
	json!({
		"ok": mismatches.is_empty(),
		"selected_txid": selected_txid,
		"actual_head_txid": actual_head_txid,
		"mismatched_pages": LimitedRows::from_rows(mismatches),
	})
}

fn analyze_truncate_regrow(commits: &[CommitData], replay: &ReplayResult) -> Value {
	let mut shrinks = Vec::new();
	let mut regrows = Vec::new();
	let mut previous = 0u32;
	for commit in commits {
		if commit.row.db_size_pages < previous {
			shrinks.push(json!({
				"txid": commit.txid,
				"from_pages": previous,
				"to_pages": commit.row.db_size_pages,
				"removed_page_start": commit.row.db_size_pages.saturating_add(1),
				"removed_page_end": previous,
			}));
		} else if commit.row.db_size_pages > previous {
			regrows.push(json!({
				"txid": commit.txid,
				"from_pages": previous,
				"to_pages": commit.row.db_size_pages,
			}));
		}
		previous = commit.row.db_size_pages;
	}
	json!({
		"shrink_count": shrinks.len(),
		"regrow_count": regrows.len(),
		"shrinks": LimitedRows::from_rows(shrinks),
		"regrows": LimitedRows::from_rows(regrows),
		"dirty_pages_above_eof": LimitedRows::from_rows(replay.dirty_pages_above_eof.clone()),
	})
}

fn analyze_storage_consistency(
	storage: &StorageFacts,
	replay: &ReplayResult,
	selected_txid: u64,
) -> Value {
	let unreachable_owners = storage
		.pidx_rows
		.iter()
		.filter(|(_, txid)| **txid > selected_txid)
		.map(|(pgno, txid)| json!({ "pgno": pgno, "owner_txid": txid }))
		.collect::<Vec<_>>();
	let pages_with_no_owner = (1..=storage.selected_db_size_pages)
		.filter(|pgno| !replay.computed_owners.contains_key(pgno))
		.collect::<Vec<_>>();
	json!({
		"selected_page_unreachable_owners": LimitedRows::from_rows(unreachable_owners),
		"pages_with_no_current_owner": LimitedRows::from_rows(pages_with_no_owner),
	})
}

fn suspect_pages(
	replay: &ReplayResult,
	resolver: Option<&ResolverResult>,
	comparison: Option<&Value>,
) -> Value {
	let differing = comparison
		.and_then(|value| value.get("differing_pages"))
		.cloned()
		.unwrap_or_else(|| json!({ "rows": [] }));
	json!({
		"differing_pages": differing,
		"zero_filled_pages": LimitedRows::from_rows(replay.zero_filled_pages.clone()),
		"hot_mismatch_pages": resolver
			.map(|resolver| json!(LimitedRows::from_rows(resolver.hot_mismatch_pages.clone())))
			.unwrap_or_else(|| json!({ "skipped": true })),
	})
}

async fn actor_database_missing_report(
	db: &universaldb::Database,
	started: Instant,
	generated_at_ms: i64,
	privacy: &Privacy,
	input: &DoctorInput,
	missing: MissingActorDatabase,
) -> Result<DoctorReport> {
	let legacy_head_present = legacy_database_head_present(db, &missing.database_id).await?;
	let legacy_commit_row_count = count_prefix_rows(
		db,
		keys::commit_prefix(&missing.database_id),
		"legacy_commit",
	)
	.await?;
	let legacy_delta_chunk_row_count =
		count_prefix_rows(db, keys::delta_prefix(&missing.database_id), "legacy_delta").await?;
	let legacy_pidx_row_count = count_prefix_rows(
		db,
		keys::pidx_delta_prefix(&missing.database_id),
		"legacy_pidx",
	)
	.await?;
	let legacy_hot_shard_count = count_prefix_rows(
		db,
		keys::shard_prefix(&missing.database_id),
		"legacy_hot_shard",
	)
	.await?;
	let legacy_database_scoped_present = legacy_head_present
		|| legacy_commit_row_count > 0
		|| legacy_delta_chunk_row_count > 0
		|| legacy_pidx_row_count > 0
		|| legacy_hot_shard_count > 0;
	let detected_storage_kind = if legacy_database_scoped_present {
		"legacy_depot_database_scoped"
	} else {
		"actor_without_depot_sqlite_database"
	};
	let (verdict, corruption_class, reason, message) = if legacy_database_scoped_present {
		(
			DoctorVerdictKind::Unsupported,
			CorruptionClass::Unsupported,
			"unsupported_legacy_depot_database_scoped_storage",
			"actor has legacy database-scoped Depot SQLite storage; depot doctor currently supports branch-scoped Depot actor storage only",
		)
	} else {
		(
			DoctorVerdictKind::Inconclusive,
			CorruptionClass::Unknown,
			"depot_actor_database_not_found",
			"actor exists in Pegboard but has no Depot SQLite database branch; the actor may not have opened or written SQLite yet",
		)
	};

	Ok(DoctorReport {
		verdict: DoctorVerdict {
			verdict,
			corruption_class,
			unsupported_reason: (verdict == DoctorVerdictKind::Unsupported).then_some(reason),
			reason: Some(reason),
			message: message.to_string(),
		},
		generated_at_ms,
		duration_ms: started.elapsed().as_millis(),
		selected_head_txid: None,
		selected_db_size_pages: None,
		first_bad_txid: None,
		previous_good_txid: None,
		identity: missing_actor_identity_json(privacy, &missing, detected_storage_kind),
		analysis_scope: json!({
			"requested_selector_type": "actor",
			"max_txid": input.max_txid,
			"full_integrity_check": !input.skip.full_integrity_check,
			"first_bad_txid_search": !input.skip.first_bad_txid,
			"page_provenance_collection": !input.skip.page_provenance,
			"depot_resolver_comparison": !input.skip.resolver_compare,
		}),
		facts: json!({
			"actor_lookup": {
				"pegboard_actor_record_found": true,
			},
			"depot_branch": {
				"found": false,
			},
			"legacy_depot_database_scoped": {
				"present": legacy_database_scoped_present,
				"head_present": legacy_head_present,
				"commit_row_count": legacy_commit_row_count,
				"delta_chunk_row_count": legacy_delta_chunk_row_count,
				"pidx_row_count": legacy_pidx_row_count,
				"hot_shard_count": legacy_hot_shard_count,
			},
		}),
		analyses: json!({
			"missing_actor_database": {
				"reason": reason,
				"detected_storage_kind": detected_storage_kind,
			}
		}),
		artifacts: json!({ "preserved": false, "files": [] }),
	})
}

async fn legacy_database_head_present(
	db: &universaldb::Database,
	database_id: &str,
) -> Result<bool> {
	let key = keys::meta_head_key(database_id);
	db.txn("depot_doctor_legacy_head_present", move |tx| {
		let key = key.clone();
		async move { Ok(tx.informal().get(&key, Snapshot).await?.is_some()) }
	})
	.await
}

fn unsupported_report(
	started: Instant,
	generated_at_ms: i64,
	privacy: &Privacy,
	input: &DoctorInput,
	resolved: &ResolvedIdentity,
	reason: &'static str,
	message: &str,
) -> DoctorReport {
	DoctorReport {
		verdict: DoctorVerdict {
			verdict: DoctorVerdictKind::Unsupported,
			corruption_class: CorruptionClass::Unsupported,
			unsupported_reason: Some(reason),
			reason: Some(reason),
			message: message.to_string(),
		},
		generated_at_ms,
		duration_ms: started.elapsed().as_millis(),
		selected_head_txid: None,
		selected_db_size_pages: None,
		first_bad_txid: None,
		previous_good_txid: None,
		identity: identity_json_without_record(privacy, resolved),
		analysis_scope: json!({
			"requested_selector_type": resolved.selector_kind,
			"max_txid": input.max_txid,
			"full_integrity_check": !input.skip.full_integrity_check,
			"first_bad_txid_search": !input.skip.first_bad_txid,
			"page_provenance_collection": !input.skip.page_provenance,
			"depot_resolver_comparison": !input.skip.resolver_compare,
		}),
		facts: json!({ "unsupported": true }),
		analyses: json!({ "unsupported_reason": reason }),
		artifacts: json!({ "preserved": false, "files": [] }),
	}
}

fn inconclusive_report(
	started: Instant,
	generated_at_ms: i64,
	privacy: &Privacy,
	input: &DoctorInput,
	resolved: &ResolvedIdentity,
	snapshot: SnapshotFacts,
	reason: &'static str,
	message: &str,
) -> DoctorReport {
	DoctorReport {
		verdict: DoctorVerdict {
			verdict: DoctorVerdictKind::Inconclusive,
			corruption_class: CorruptionClass::Unknown,
			unsupported_reason: None,
			reason: Some(reason),
			message: message.to_string(),
		},
		generated_at_ms,
		duration_ms: started.elapsed().as_millis(),
		selected_head_txid: None,
		selected_db_size_pages: None,
		first_bad_txid: None,
		previous_good_txid: None,
		identity: identity_json_without_record(privacy, resolved),
		analysis_scope: json!({
			"requested_selector_type": resolved.selector_kind,
			"max_txid": input.max_txid,
			"start_snapshot": snapshot,
		}),
		facts: json!({}),
		analyses: json!({}),
		artifacts: json!({ "preserved": false, "files": [] }),
	}
}

fn identity_json(
	privacy: &Privacy,
	resolved: &ResolvedIdentity,
	branch_record: &DatabaseBranchRecord,
) -> Value {
	let mut value = identity_json_without_record(privacy, resolved);
	if let Some(object) = value.as_object_mut() {
		object.insert(
			"selected_branch_is_root_branch".to_string(),
			json!(
				branch_record.parent.is_none()
					&& branch_record.parent_versionstamp.is_none()
					&& branch_record.fork_depth == 0
			),
		);
		object.insert(
			"detected_storage_kind".to_string(),
			json!("actor_v2_depot_branch"),
		);
		object.insert(
			"lifecycle_generation".to_string(),
			json!(branch_record.lifecycle_generation),
		);
	}
	value
}

fn identity_json_without_record(privacy: &Privacy, resolved: &ResolvedIdentity) -> Value {
	json!({
		"bucket_id_hash": resolved.bucket_id.map(|id| privacy.hash_uuid(id.as_uuid())),
		"database_id_hash": resolved.database_id.as_ref().map(|id| privacy.hash_str(id)),
		"database_branch_id_hash": privacy.hash_uuid(resolved.branch_id.as_uuid()),
		"bucket_branch_id_hash": resolved.bucket_branch_id.map(|id| privacy.hash_uuid(id.as_uuid())),
		"namespace_id_hash": resolved.namespace_id.map(|id| privacy.hash_id(id)),
		"actor_id_hash": resolved.actor_id.map(|id| privacy.hash_id(id)),
		"current_database_pointer_hash": resolved.database_id.as_ref().map(|id| privacy.hash_str(id)),
		"current_branch_pointer_hash": privacy.hash_uuid(resolved.branch_id.as_uuid()),
		"selected_branch_is_current_database_pointer": resolved.database_id.is_some(),
	})
}

fn missing_actor_identity_json(
	privacy: &Privacy,
	missing: &MissingActorDatabase,
	detected_storage_kind: &'static str,
) -> Value {
	json!({
		"bucket_id_hash": privacy.hash_uuid(missing.bucket_id.as_uuid()),
		"database_id_hash": privacy.hash_str(&missing.database_id),
		"database_branch_id_hash": Value::Null,
		"bucket_branch_id_hash": Value::Null,
		"namespace_id_hash": privacy.hash_id(missing.namespace_id),
		"actor_id_hash": privacy.hash_id(missing.actor_id),
		"current_database_pointer_hash": privacy.hash_str(&missing.database_id),
		"current_branch_pointer_hash": Value::Null,
		"selected_branch_is_current_database_pointer": false,
		"selected_branch_is_root_branch": false,
		"detected_storage_kind": detected_storage_kind,
	})
}

fn analysis_scope_json(
	input: &DoctorInput,
	resolved: &ResolvedIdentity,
	actual_head_txid: u64,
	selected_txid: u64,
	start: &SnapshotFacts,
	end: &SnapshotFacts,
) -> Value {
	json!({
		"requested_selector_type": resolved.selector_kind,
		"min_txid": input.min_txid,
		"max_txid": input.max_txid,
		"actual_head_txid": actual_head_txid,
		"selected_txid": selected_txid,
		"selected_txid_is_live_head": selected_txid == actual_head_txid,
		"full_integrity_check": !input.skip.full_integrity_check,
		"first_bad_txid_search": !input.skip.first_bad_txid,
		"page_provenance_collection": !input.skip.page_provenance && !input.skip.resolver_compare,
		"depot_resolver_comparison": !input.skip.resolver_compare,
		"artifacts_preserved": input.artifact_dir.is_some(),
		"start_snapshot": start,
		"end_snapshot": end,
		"changed_during_diagnosis": snapshot_changed(start, end),
	})
}

fn snapshot_changed(start: &SnapshotFacts, end: &SnapshotFacts) -> bool {
	start.head_txid != end.head_txid
		|| start.head_versionstamp != end.head_versionstamp
		|| start.current_branch_hash != end.current_branch_hash
		|| start.commit_row_count != end.commit_row_count
		|| start.delta_chunk_row_count != end.delta_chunk_row_count
		|| start.pidx_row_count != end.pidx_row_count
		|| start.installed_hot_shard_count != end.installed_hot_shard_count
		|| start.staged_hot_shard_count != end.staged_hot_shard_count
}

fn preserve_artifacts(
	artifact_dir: &Path,
	temp_dir: &TempDir,
	report: &mut DoctorReport,
) -> Result<()> {
	prepare_artifact_dir(artifact_dir)?;
	let mut files = Vec::new();
	for file_name in ["current-sequential.sqlite", "current-depot-resolved.sqlite"] {
		let src = temp_dir.path().join(file_name);
		if src.exists() {
			let dst = artifact_dir.join(file_name);
			fs::copy(&src, &dst)?;
			files.push(file_name.to_string());
		}
	}

	let raw_summary = artifact_dir.join("raw-summary.json");
	fs::write(&raw_summary, serde_json::to_vec_pretty(&report.facts)?)?;
	files.push("raw-summary.json".to_string());

	let mut artifact_report = report.clone();
	artifact_report.artifacts = json!({ "preserved": true, "files": files });
	fs::write(
		artifact_dir.join("doctor-report.json"),
		serde_json::to_vec_pretty(&artifact_report)?,
	)?;
	files.push("doctor-report.json".to_string());

	let manifest = json!({
		"warning": "Depot doctor artifacts may contain raw customer SQLite data.",
		"files": files,
	});
	fs::write(
		artifact_dir.join("artifact-manifest.json"),
		serde_json::to_vec_pretty(&manifest)?,
	)?;
	files.push("artifact-manifest.json".to_string());
	report.artifacts = json!({ "preserved": true, "directory": artifact_dir.display().to_string(), "files": files });
	Ok(())
}

fn prepare_artifact_dir(path: &Path) -> Result<()> {
	if path.exists() {
		ensure!(path.is_dir(), "artifact-dir exists but is not a directory");
		ensure!(
			fs::read_dir(path)?.next().is_none(),
			"artifact-dir exists and is not empty"
		);
	} else {
		#[cfg(unix)]
		{
			use std::os::unix::fs::DirBuilderExt;
			fs::DirBuilder::new().mode(0o700).create(path)?;
		}
		#[cfg(not(unix))]
		{
			fs::create_dir(path)?;
		}
	}
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
	}
	Ok(())
}

#[derive(Debug)]
struct ScannedRow {
	key: Vec<u8>,
	value: Vec<u8>,
}

#[derive(Debug)]
struct ScannedRows {
	rows: Vec<ScannedRow>,
	chunk_count: usize,
	timed_out_chunk_count: usize,
}

#[derive(Debug)]
struct ScanRowsChunk {
	rows: Vec<ScannedRow>,
	last_key: Option<Vec<u8>>,
	exhausted: bool,
	timed_out: bool,
}

#[derive(Debug)]
struct CountChunk {
	count: usize,
	last_key: Option<Vec<u8>>,
	exhausted: bool,
	timed_out: bool,
}

async fn scan_prefix_rows(
	db: &universaldb::Database,
	prefix: Vec<u8>,
	label: &'static str,
) -> Result<Vec<ScannedRow>> {
	Ok(scan_prefix_rows_with_stats(db, prefix, label).await?.rows)
}

async fn scan_prefix_rows_with_stats(
	db: &universaldb::Database,
	prefix: Vec<u8>,
	label: &'static str,
) -> Result<ScannedRows> {
	let mut rows = Vec::new();
	let mut last_key = None;
	let mut chunk_index = 0usize;
	let mut timed_out_chunk_count = 0usize;
	loop {
		chunk_index += 1;
		let chunk_last_key = last_key.clone();
		let chunk = tokio::time::timeout(
			SCAN_CHUNK_TIMEOUT,
			db.txn("depotdoctor", {
				let prefix = prefix.clone();
				move |tx| {
					let prefix = prefix.clone();
					let chunk_last_key = chunk_last_key.clone();
					async move { scan_prefix_chunk(&tx, prefix, chunk_last_key).await }
				}
			}),
		)
		.await
		.with_context(|| format!("scan {label} chunk timed out"))??;
		let row_count = chunk.rows.len();
		if chunk.timed_out {
			timed_out_chunk_count += 1;
		}
		rows.extend(chunk.rows);
		tracing::info!(
			label,
			chunk_index,
			row_count,
			total_rows = rows.len(),
			timed_out = chunk.timed_out,
			exhausted = chunk.exhausted,
			"scanned depot doctor prefix chunk"
		);
		if chunk.exhausted {
			break;
		}
		let Some(new_last_key) = chunk.last_key else {
			ensure!(
				!chunk.timed_out,
				"scan {label} made no progress before early transaction budget"
			);
			break;
		};
		last_key = Some(new_last_key);
	}
	Ok(ScannedRows {
		rows,
		chunk_count: chunk_index,
		timed_out_chunk_count,
	})
}

async fn count_prefix_rows(
	db: &universaldb::Database,
	prefix: Vec<u8>,
	label: &'static str,
) -> Result<usize> {
	let mut count = 0usize;
	let mut last_key = None;
	let mut chunk_index = 0usize;
	loop {
		chunk_index += 1;
		let chunk_last_key = last_key.clone();
		let chunk = tokio::time::timeout(
			SCAN_CHUNK_TIMEOUT,
			db.txn("depotdoctor", {
				let prefix = prefix.clone();
				move |tx| {
					let prefix = prefix.clone();
					let chunk_last_key = chunk_last_key.clone();
					async move { count_prefix_chunk(&tx, prefix, chunk_last_key).await }
				}
			}),
		)
		.await
		.with_context(|| format!("count {label} chunk timed out"))??;
		count += chunk.count;
		tracing::info!(
			label,
			chunk_index,
			chunk_count = chunk.count,
			total_count = count,
			timed_out = chunk.timed_out,
			exhausted = chunk.exhausted,
			"counted depot doctor prefix chunk"
		);
		if chunk.exhausted {
			break;
		}
		let Some(new_last_key) = chunk.last_key else {
			ensure!(
				!chunk.timed_out,
				"count {label} made no progress before early transaction budget"
			);
			break;
		};
		last_key = Some(new_last_key);
	}
	Ok(count)
}

async fn prefix_has_rows(
	db: &universaldb::Database,
	prefix: Vec<u8>,
	label: &'static str,
) -> Result<bool> {
	tokio::time::timeout(
		SCAN_CHUNK_TIMEOUT,
		db.txn("depot_doctor_prefix_has_rows", move |tx| {
			let prefix = prefix.clone();
			async move {
				let (range_start, range_end) =
					universaldb::tuple::Subspace::from_bytes(prefix).range();
				let informal = tx.informal();
				let mut stream = informal.get_ranges_keyvalues(
					RangeOption {
						begin: KeySelector::first_greater_or_equal(range_start),
						end: KeySelector::first_greater_or_equal(range_end),
						mode: StreamingMode::WantAll,
						..RangeOption::default()
					},
					Snapshot,
				);
				Ok(stream.try_next().await?.is_some())
			}
		}),
	)
	.await
	.with_context(|| format!("exists {label} chunk timed out"))?
}

async fn scan_prefix_chunk(
	tx: &universaldb::Transaction,
	prefix: Vec<u8>,
	last_key: Option<Vec<u8>>,
) -> Result<ScanRowsChunk> {
	let scan_started = Instant::now();
	let (range_start, range_end) = universaldb::tuple::Subspace::from_bytes(prefix).range();
	let begin = last_key.map_or_else(
		|| KeySelector::first_greater_or_equal(range_start),
		KeySelector::first_greater_than,
	);
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			begin,
			end: KeySelector::first_greater_or_equal(range_end),
			mode: StreamingMode::WantAll,
			..RangeOption::default()
		},
		Snapshot,
	);
	let mut rows = Vec::new();
	let mut last_key = None;
	let mut timed_out = false;
	let mut exhausted = false;
	loop {
		if rows.len() >= SCAN_CHUNK_ROW_LIMIT {
			break;
		}
		if scan_started.elapsed() > EARLY_SCAN_TIMEOUT {
			timed_out = true;
			break;
		}
		let Some(entry) = stream.try_next().await? else {
			exhausted = true;
			break;
		};
		last_key = Some(entry.key().to_vec());
		rows.push(ScannedRow {
			key: entry.key().to_vec(),
			value: entry.value().to_vec(),
		});
	}
	Ok(ScanRowsChunk {
		rows,
		last_key,
		exhausted,
		timed_out,
	})
}

async fn count_prefix_chunk(
	tx: &universaldb::Transaction,
	prefix: Vec<u8>,
	last_key: Option<Vec<u8>>,
) -> Result<CountChunk> {
	let scan_started = Instant::now();
	let (range_start, range_end) = universaldb::tuple::Subspace::from_bytes(prefix).range();
	let begin = last_key.map_or_else(
		|| KeySelector::first_greater_or_equal(range_start),
		KeySelector::first_greater_than,
	);
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			begin,
			end: KeySelector::first_greater_or_equal(range_end),
			mode: StreamingMode::WantAll,
			..RangeOption::default()
		},
		Snapshot,
	);
	let mut count = 0usize;
	let mut last_key = None;
	let mut timed_out = false;
	let mut exhausted = false;
	loop {
		if count >= SCAN_CHUNK_ROW_LIMIT {
			break;
		}
		if scan_started.elapsed() > EARLY_SCAN_TIMEOUT {
			timed_out = true;
			break;
		}
		let Some(entry) = stream.try_next().await? else {
			exhausted = true;
			break;
		};
		count += 1;
		last_key = Some(entry.key().to_vec());
	}
	Ok(CountChunk {
		count,
		last_key,
		exhausted,
		timed_out,
	})
}

fn decode_u64_suffix(prefix: &[u8], key: &[u8]) -> Result<u64> {
	let suffix = key
		.strip_prefix(prefix)
		.context("key did not start with expected prefix")?;
	ensure!(suffix.len() == 8, "expected 8 byte suffix");
	Ok(u64::from_be_bytes(suffix.try_into()?))
}

fn decode_u32_suffix(prefix: &[u8], key: &[u8]) -> Result<u32> {
	let suffix = key
		.strip_prefix(prefix)
		.context("key did not start with expected prefix")?;
	ensure!(suffix.len() == 4, "expected 4 byte suffix");
	Ok(u32::from_be_bytes(suffix.try_into()?))
}

fn decode_hot_shard_key(branch_id: DatabaseBranchId, key: &[u8]) -> Option<(u32, u64)> {
	let prefix = keys::branch_shard_prefix(branch_id);
	let suffix = key.strip_prefix(prefix.as_slice())?;
	if suffix.len() != 13 || suffix[4] != b'/' {
		return None;
	}
	let shard_id = u32::from_be_bytes(suffix[0..4].try_into().ok()?);
	let as_of_txid = u64::from_be_bytes(suffix[5..13].try_into().ok()?);
	Some((shard_id, as_of_txid))
}

fn read_page_hashes(
	path: &Path,
	page_size: u32,
	page_count: u32,
	privacy: &Privacy,
) -> Result<BTreeMap<u32, String>> {
	let mut file = File::open(path)?;
	let mut hashes = BTreeMap::new();
	let mut buf = vec![0; page_size as usize];
	for pgno in 1..=page_count {
		file.seek(SeekFrom::Start(u64::from(pgno - 1) * u64::from(page_size)))?;
		buf.fill(0);
		let _ = file.read(&mut buf)?;
		hashes.insert(pgno, privacy.hash_bytes(&buf));
	}
	Ok(hashes)
}

fn now_ms() -> Result<i64> {
	let duration = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system clock is before unix epoch")?;
	i64::try_from(duration.as_millis()).context("timestamp exceeds i64")
}

fn hex_lower(bytes: &[u8]) -> String {
	const HEX: &[u8; 16] = b"0123456789abcdef";
	let mut out = String::with_capacity(bytes.len() * 2);
	for byte in bytes {
		out.push(HEX[(byte >> 4) as usize] as char);
		out.push(HEX[(byte & 0x0f) as usize] as char);
	}
	out
}

fn hash_u64(value: u64) -> String {
	let mut hasher = Sha256::new();
	hasher.update(value.to_be_bytes());
	hex_lower(&hasher.finalize())
}
