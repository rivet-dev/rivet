//! Read-only Depot inspection helpers for internal API routes.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use futures_util::TryStreamExt;
use rivet_pools::NodeId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use universaldb::{RangeOption, options::StreamingMode, utils::IsolationLevel::Snapshot};
use uuid::Uuid;

use crate::{
	conveyer::{
		keys,
		types::{
			BucketId, DatabaseBranchId, decode_bucket_branch_record, decode_bucket_pointer,
			decode_cold_shard_ref, decode_commit_row, decode_compaction_root,
			decode_database_branch_record, decode_database_pointer, decode_db_head,
			decode_db_history_pin, decode_pitr_interval_coverage, decode_retired_cold_object,
			decode_sqlite_cmp_dirty,
		},
	},
	gc,
};

pub const DEFAULT_LIMIT: usize = 100;
pub const MAX_LIMIT: usize = 1000;
pub const DEFAULT_SAMPLE_LIMIT: usize = 20;

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogQuery {
	pub bucket_id: Option<String>,
	pub database_id: Option<String>,
	pub limit: Option<usize>,
	pub cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SampleQuery {
	pub sample_limit: Option<usize>,
	pub include_history: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RowsQuery {
	pub limit: Option<usize>,
	pub cursor: Option<String>,
	pub include_bytes: Option<bool>,
	pub before_txid: Option<u64>,
	pub after_txid: Option<u64>,
	pub from_pgno: Option<u32>,
	pub shard_id: Option<u32>,
	pub state: Option<String>,
	pub kind: Option<String>,
	pub job_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawScanQuery {
	pub prefix: Option<String>,
	pub start_after: Option<String>,
	pub limit: Option<usize>,
	pub decode: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectResponse {
	pub node_id: String,
	pub generated_at_ms: i64,
	pub scope: Value,
	#[serde(flatten)]
	pub data: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaginatedRowsResponse {
	pub node_id: String,
	pub generated_at_ms: i64,
	pub scope: Value,
	pub rows: Vec<Value>,
	pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogResponse {
	pub node_id: String,
	pub generated_at_ms: i64,
	pub scope: Value,
	pub buckets: Vec<Value>,
	pub databases: Vec<Value>,
	pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum RowFamily {
	Commits,
	Pidx,
	Deltas,
	Shards,
	ColdShards,
	RetiredColdObjects,
	PitrIntervals,
	Pins,
	StagedHotShards,
}

impl RowFamily {
	pub fn parse(value: &str) -> Result<Self> {
		match value {
			"commits" => Ok(Self::Commits),
			"pidx" => Ok(Self::Pidx),
			"deltas" => Ok(Self::Deltas),
			"shards" => Ok(Self::Shards),
			"cold-shards" => Ok(Self::ColdShards),
			"retired-cold-objects" => Ok(Self::RetiredColdObjects),
			"pitr-intervals" => Ok(Self::PitrIntervals),
			"pins" => Ok(Self::Pins),
			"staged-hot-shards" => Ok(Self::StagedHotShards),
			_ => bail!("unsupported Depot inspect row family: {value}"),
		}
	}

	fn as_str(self) -> &'static str {
		match self {
			Self::Commits => "commits",
			Self::Pidx => "pidx",
			Self::Deltas => "deltas",
			Self::Shards => "shards",
			Self::ColdShards => "cold-shards",
			Self::RetiredColdObjects => "retired-cold-objects",
			Self::PitrIntervals => "pitr-intervals",
			Self::Pins => "pins",
			Self::StagedHotShards => "staged-hot-shards",
		}
	}

	fn scan_prefix(self, branch_id: DatabaseBranchId) -> Vec<u8> {
		match self {
			Self::Commits => keys::branch_commit_prefix(branch_id),
			Self::Pidx => keys::branch_pidx_prefix(branch_id),
			Self::Deltas => keys::branch_delta_prefix(branch_id),
			Self::Shards => keys::branch_shard_prefix(branch_id),
			Self::ColdShards => keys::branch_compaction_cold_shard_prefix(branch_id),
			Self::RetiredColdObjects => {
				keys::branch_compaction_retired_cold_object_prefix(branch_id)
			}
			Self::PitrIntervals => keys::branch_pitr_interval_prefix(branch_id),
			Self::Pins => keys::db_pin_prefix(branch_id),
			Self::StagedHotShards => keys::branch_compaction_stage_prefix(branch_id),
		}
	}
}

pub async fn summary(db: &universaldb::Database, node_id: NodeId) -> Result<InspectResponse> {
	let counters = db
		.run(|tx| async move {
			Ok(json!({
				"bucket_pointers": count_prefix(&tx, keys::bucket_pointer_cur_prefix()).await?,
				"database_pointers": count_prefix(&tx, keys::database_pointer_cur_prefix()).await?,
				"database_branches": count_prefix(&tx, vec![keys::SQLITE_SUBSPACE_PREFIX, keys::BRANCHES_PARTITION]).await?,
				"bucket_branches": count_prefix(&tx, vec![keys::SQLITE_SUBSPACE_PREFIX, keys::BUCKET_BRANCH_PARTITION]).await?,
				"dirty_branches": count_prefix(&tx, vec![keys::SQLITE_SUBSPACE_PREFIX, keys::SQLITE_CMP_DIRTY_PARTITION]).await?,
				"queued_compaction_rows": count_prefix(&tx, vec![keys::SQLITE_SUBSPACE_PREFIX, keys::CMPC_PARTITION]).await?,
			}))
		})
		.await?;

	response(
		node_id,
		json!({ "kind": "summary" }),
		json!({
			"cold_tier": { "configured": false, "kind": "unknown" },
			"counters": counters,
		}),
	)
}

pub async fn catalog(
	db: &universaldb::Database,
	node_id: NodeId,
	query: CatalogQuery,
) -> Result<CatalogResponse> {
	let limit = page_limit(query.limit)?;
	let cursor = decode_optional_key(query.cursor.as_deref())?;
	let bucket_filter = query
		.bucket_id
		.as_deref()
		.map(parse_bucket_id)
		.transpose()?;
	let database_filter = query.database_id.clone();
	let rows = db
		.run(move |tx| {
			let cursor = cursor.clone();
			let database_filter = database_filter.clone();
			async move {
				let bucket_pointer = if let Some(bucket_id) = bucket_filter {
					tx_get_decoded(
						&tx,
						keys::bucket_pointer_cur_key(bucket_id),
						decode_bucket_pointer,
					)
					.await?
					.map(|pointer| (bucket_id, pointer))
				} else {
					None
				};
				let bucket_branch_filter = bucket_pointer
					.as_ref()
					.map(|(_bucket_id, pointer)| pointer.current_branch);
				let scanned = scan_prefix_page(
					&tx,
					keys::database_pointer_cur_prefix(),
					cursor.as_deref(),
					limit,
				)
				.await?;
				let mut rows = Vec::new();
				let mut next_cursor = None;
				for row in scanned.rows {
					let (bucket_branch_id, database_id) =
						keys::decode_database_pointer_cur_key(&row.key)?;
					if bucket_branch_filter.is_some_and(|filter| filter != bucket_branch_id) {
						continue;
					}
					if database_filter
						.as_deref()
						.is_some_and(|filter| filter != database_id)
					{
						continue;
					}
					let pointer = decode_database_pointer(&row.value)?;
					rows.push(json!({
						"key": encode_key(&row.key),
						"bucket_branch_id": bucket_branch_id,
						"database_id": database_id,
						"current_database_branch_id": pointer.current_branch,
						"last_swapped_at_ms": pointer.last_swapped_at_ms,
					}));
				}
				if scanned.has_more {
					next_cursor = scanned.next_cursor;
				}

				let buckets = if let Some((bucket_id, pointer)) = bucket_pointer {
					vec![json!({
						"key": encode_key(&keys::bucket_pointer_cur_key(bucket_id)),
						"bucket_id": bucket_id,
						"current_bucket_branch_id": pointer.current_branch,
						"last_swapped_at_ms": pointer.last_swapped_at_ms,
					})]
				} else {
					let bucket_rows =
						scan_prefix_page(&tx, keys::bucket_pointer_cur_prefix(), None, limit)
							.await?;
					let mut buckets = Vec::new();
					for row in bucket_rows.rows {
						let bucket_id = keys::decode_bucket_pointer_cur_bucket_id(&row.key)?;
						let pointer = decode_bucket_pointer(&row.value)?;
						buckets.push(json!({
							"key": encode_key(&row.key),
							"bucket_id": bucket_id,
							"current_bucket_branch_id": pointer.current_branch,
							"last_swapped_at_ms": pointer.last_swapped_at_ms,
						}));
					}
					buckets
				};

				Ok((buckets, rows, next_cursor))
			}
		})
		.await?;

	Ok(CatalogResponse {
		node_id: node_id.to_string(),
		generated_at_ms: now_ms()?,
		scope: json!({ "kind": "catalog" }),
		buckets: rows.0,
		databases: rows.1,
		next_cursor: rows.2.map(|key| encode_key(&key)),
	})
}

pub async fn bucket(
	db: &universaldb::Database,
	node_id: NodeId,
	bucket_id: BucketId,
	query: SampleQuery,
) -> Result<InspectResponse> {
	let sample_limit = sample_limit(query.sample_limit)?;
	let include_history = query.include_history.unwrap_or(false);
	let data = db
		.run(move |tx| async move {
			let pointer = tx_get_decoded(
				&tx,
				keys::bucket_pointer_cur_key(bucket_id),
				decode_bucket_pointer,
			)
			.await?;
			let current_branch = match &pointer {
				Some(pointer) => Some(pointer.current_branch),
				None => None,
			};
			let branch_record = if let Some(branch_id) = current_branch {
				tx_get_decoded(
					&tx,
					keys::bucket_branches_list_key(branch_id),
					decode_bucket_branch_record,
				)
				.await?
			} else {
				None
			};
			let catalog = if let Some(branch_id) = current_branch {
				summary_for_prefix(&tx, keys::bucket_catalog_prefix(branch_id), sample_limit)
					.await?
			} else {
				empty_summary()
			};
			let tombstones = if let Some(branch_id) = current_branch {
				summary_for_prefix(
					&tx,
					keys::bucket_branches_database_tombstone_prefix(branch_id),
					sample_limit,
				)
				.await?
			} else {
				empty_summary()
			};
			let history = if include_history {
				summary_for_prefix(
					&tx,
					keys::bucket_pointer_history_prefix(bucket_id),
					sample_limit,
				)
				.await?
			} else {
				empty_summary()
			};

			Ok(json!({
				"bucket_id": bucket_id,
				"pointer": pointer,
				"current_branch": branch_record,
				"summaries": {
					"catalog": catalog,
					"tombstones": tombstones,
					"pointer_history": history,
				},
				"links": {
					"catalog": "/depot/inspect/catalog",
				}
			}))
		})
		.await?;

	response(
		node_id,
		json!({ "kind": "bucket", "bucket_id": bucket_id }),
		data,
	)
}

pub async fn database(
	db: &universaldb::Database,
	node_id: NodeId,
	bucket_id: BucketId,
	database_id: String,
	query: SampleQuery,
) -> Result<InspectResponse> {
	let sample_limit = sample_limit(query.sample_limit)?;
	let scope_database_id = database_id.clone();
	let data = db
		.run(move |tx| {
			let database_id = database_id.clone();
			async move {
				let bucket_pointer = tx_get_decoded(
					&tx,
					keys::bucket_pointer_cur_key(bucket_id),
					decode_bucket_pointer,
				)
				.await?;
				let Some(bucket_pointer) = bucket_pointer else {
					return Ok(
						json!({ "bucket_id": bucket_id, "database_id": database_id, "pointer": null }),
					);
				};
				let pointer = tx_get_decoded(
					&tx,
					keys::database_pointer_cur_key(bucket_pointer.current_branch, &database_id),
					decode_database_pointer,
				)
				.await?;
				let branch = if let Some(pointer) = &pointer {
					branch_blob_in_tx(&tx, pointer.current_branch, sample_limit).await?
				} else {
					json!(null)
				};

				Ok(json!({
					"bucket_id": bucket_id,
					"database_id": database_id,
					"bucket_branch_id": bucket_pointer.current_branch,
					"pointer": pointer,
					"branch": branch,
				}))
			}
		})
		.await?;

	response(
		node_id,
		json!({ "kind": "database", "bucket_id": bucket_id, "database_id": scope_database_id }),
		data,
	)
}

pub async fn branch(
	db: &universaldb::Database,
	node_id: NodeId,
	branch_id: DatabaseBranchId,
	query: SampleQuery,
) -> Result<InspectResponse> {
	let sample_limit = sample_limit(query.sample_limit)?;
	let data = db
		.run(move |tx| async move { branch_blob_in_tx(&tx, branch_id, sample_limit).await })
		.await?;

	response(
		node_id,
		json!({ "kind": "branch", "branch_id": branch_id }),
		data,
	)
}

pub async fn branch_rows(
	db: &universaldb::Database,
	node_id: NodeId,
	branch_id: DatabaseBranchId,
	family: RowFamily,
	query: RowsQuery,
) -> Result<PaginatedRowsResponse> {
	let limit = page_limit(query.limit)?;
	let cursor = decode_optional_key(query.cursor.as_deref())?;
	let prefix = family.scan_prefix(branch_id);
	let include_bytes = query.include_bytes.unwrap_or(false);
	let scan = db
		.run(move |tx| {
			let prefix = prefix.clone();
			let cursor = cursor.clone();
			async move { scan_prefix_page(&tx, prefix, cursor.as_deref(), limit).await }
		})
		.await?;
	let mut rows = Vec::new();
	for row in scan.rows {
		rows.push(decode_row_value(
			branch_id,
			family,
			&row.key,
			&row.value,
			include_bytes,
		));
	}

	Ok(PaginatedRowsResponse {
		node_id: node_id.to_string(),
		generated_at_ms: now_ms()?,
		scope: json!({
			"kind": "branch_rows",
			"branch_id": branch_id,
			"family": family.as_str(),
		}),
		rows,
		next_cursor: scan.next_cursor.map(|key| encode_key(&key)),
	})
}

pub async fn raw_key(
	db: &universaldb::Database,
	node_id: NodeId,
	key: Vec<u8>,
) -> Result<InspectResponse> {
	let value = db
		.run({
			let key = key.clone();
			move |tx| {
				let key = key.clone();
				async move {
					Ok(tx
						.informal()
						.get(&key, Snapshot)
						.await?
						.map(Vec::<u8>::from))
				}
			}
		})
		.await?;
	let decoded = best_effort_decode(&key, value.as_deref());

	response(
		node_id,
		json!({ "kind": "raw_key" }),
		json!({
			"key": encode_key(&key),
			"value": value.as_ref().map(|value| encode_key(value)),
			"value_size": value.as_ref().map(Vec::len),
			"decoded": decoded,
		}),
	)
}

pub async fn raw_scan(
	db: &universaldb::Database,
	node_id: NodeId,
	query: RawScanQuery,
) -> Result<PaginatedRowsResponse> {
	let limit = page_limit(query.limit)?;
	let prefix = decode_optional_key(query.prefix.as_deref())?.unwrap_or_default();
	let cursor = decode_optional_key(query.start_after.as_deref())?;
	let decode = query.decode.unwrap_or(true);
	let scan = db
		.run(move |tx| {
			let prefix = prefix.clone();
			let cursor = cursor.clone();
			async move { scan_prefix_page(&tx, prefix, cursor.as_deref(), limit).await }
		})
		.await?;
	let rows = scan
		.rows
		.into_iter()
		.map(|row| {
			json!({
				"key": encode_key(&row.key),
				"value_size": row.value.len(),
				"value": encode_key(&row.value),
				"decoded": decode.then(|| best_effort_decode(&row.key, Some(&row.value))),
			})
		})
		.collect();

	Ok(PaginatedRowsResponse {
		node_id: node_id.to_string(),
		generated_at_ms: now_ms()?,
		scope: json!({ "kind": "raw_scan" }),
		rows,
		next_cursor: scan.next_cursor.map(|key| encode_key(&key)),
	})
}

pub fn decode_key_response(node_id: NodeId, key: Vec<u8>) -> Result<InspectResponse> {
	response(
		node_id,
		json!({ "kind": "raw_decode_key" }),
		json!({
			"key": encode_key(&key),
			"decoded": best_effort_decode(&key, None),
		}),
	)
}

pub async fn page_trace(
	db: &universaldb::Database,
	node_id: NodeId,
	branch_id: DatabaseBranchId,
	pgno: u32,
) -> Result<InspectResponse> {
	let head = db
		.run(move |tx| async move {
			tx_get_decoded(&tx, keys::branch_meta_head_key(branch_id), decode_db_head).await
		})
		.await?;
	let outcome = if head.as_ref().is_some_and(|head| pgno <= head.db_size_pages) {
		"found"
	} else {
		"above_eof"
	};

	response(
		node_id,
		json!({ "kind": "page", "branch_id": branch_id, "pgno": pgno }),
		json!({
			"read_cap": head,
			"outcome": outcome,
			"source": { "kind": "unknown", "branch_id": branch_id },
			"steps": [],
			"bytes": null,
		}),
	)
}

pub fn decode_path_key(value: &str) -> Result<Vec<u8>> {
	BASE64_URL_SAFE_NO_PAD
		.decode(value)
		.context("decode unpadded base64url Depot inspect key")
}

fn response(node_id: NodeId, scope: Value, data: Value) -> Result<InspectResponse> {
	Ok(InspectResponse {
		node_id: node_id.to_string(),
		generated_at_ms: now_ms()?,
		scope,
		data,
	})
}

fn page_limit(limit: Option<usize>) -> Result<usize> {
	let limit = limit.unwrap_or(DEFAULT_LIMIT);
	ensure!(
		limit <= MAX_LIMIT,
		"Depot inspect limit exceeds hard cap of {MAX_LIMIT}"
	);
	Ok(limit)
}

fn sample_limit(limit: Option<usize>) -> Result<usize> {
	let limit = limit.unwrap_or(DEFAULT_SAMPLE_LIMIT);
	ensure!(
		limit <= MAX_LIMIT,
		"Depot inspect sample_limit exceeds hard cap of {MAX_LIMIT}"
	);
	Ok(limit)
}

fn now_ms() -> Result<i64> {
	let duration = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system clock is before unix epoch")?;
	i64::try_from(duration.as_millis()).context("timestamp exceeds i64")
}

fn encode_key(key: &[u8]) -> String {
	BASE64_URL_SAFE_NO_PAD.encode(key)
}

fn decode_optional_key(value: Option<&str>) -> Result<Option<Vec<u8>>> {
	value.map(decode_path_key).transpose()
}

fn parse_bucket_id(value: &str) -> Result<BucketId> {
	Ok(BucketId::from_uuid(
		Uuid::parse_str(value).context("parse Depot inspect bucket id")?,
	))
}

async fn tx_get_decoded<T, F>(
	tx: &universaldb::Transaction,
	key: Vec<u8>,
	decode: F,
) -> Result<Option<T>>
where
	F: Fn(&[u8]) -> Result<T>,
{
	let Some(value) = tx.informal().get(&key, Snapshot).await? else {
		return Ok(None);
	};

	Ok(Some(decode(&value)?))
}

struct ScannedRow {
	key: Vec<u8>,
	value: Vec<u8>,
}

struct ScanPage {
	rows: Vec<ScannedRow>,
	next_cursor: Option<Vec<u8>>,
	has_more: bool,
}

async fn scan_prefix_page(
	tx: &universaldb::Transaction,
	prefix: Vec<u8>,
	cursor: Option<&[u8]>,
	limit: usize,
) -> Result<ScanPage> {
	if let Some(cursor) = cursor {
		ensure!(
			cursor.starts_with(&prefix),
			"Depot inspect cursor is outside the requested prefix"
		);
	}

	let (range_start, range_end) = universaldb::tuple::Subspace::from_bytes(prefix).range();
	let begin = cursor
		.map(universaldb::KeySelector::first_greater_than)
		.unwrap_or_else(|| universaldb::KeySelector::first_greater_or_equal(range_start));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			begin,
			end: universaldb::KeySelector::first_greater_or_equal(range_end),
			limit: Some(limit.saturating_add(1)),
			mode: StreamingMode::WantAll,
			..RangeOption::default()
		},
		Snapshot,
	);
	let mut rows = Vec::new();
	while let Some(entry) = stream.try_next().await? {
		rows.push(ScannedRow {
			key: entry.key().to_vec(),
			value: entry.value().to_vec(),
		});
	}

	let has_more = rows.len() > limit;
	let overflow = if has_more { rows.pop() } else { None };
	let next_cursor = if has_more {
		rows.last()
			.map(|row| row.key.clone())
			.or_else(|| overflow.map(|row| row.key))
	} else {
		None
	};

	Ok(ScanPage {
		rows,
		next_cursor,
		has_more,
	})
}

async fn count_prefix(tx: &universaldb::Transaction, prefix: Vec<u8>) -> Result<usize> {
	let (range_start, range_end) = universaldb::tuple::Subspace::from_bytes(prefix).range();
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			begin: universaldb::KeySelector::first_greater_or_equal(range_start),
			end: universaldb::KeySelector::first_greater_or_equal(range_end),
			mode: StreamingMode::WantAll,
			..RangeOption::default()
		},
		Snapshot,
	);
	let mut count = 0;
	while stream.try_next().await?.is_some() {
		count += 1;
	}

	Ok(count)
}

async fn summary_for_prefix(
	tx: &universaldb::Transaction,
	prefix: Vec<u8>,
	sample_limit: usize,
) -> Result<Value> {
	let scan = scan_prefix_page(tx, prefix, None, sample_limit).await?;
	Ok(json!({
		"count": scan.rows.len(),
		"truncated": scan.has_more,
		"sample": scan.rows.into_iter().map(|row| {
			json!({
				"key": encode_key(&row.key),
				"value_size": row.value.len(),
				"decoded": best_effort_decode(&row.key, Some(&row.value)),
			})
		}).collect::<Vec<_>>(),
		"next_cursor": scan.next_cursor.map(|key| encode_key(&key)),
	}))
}

fn empty_summary() -> Value {
	json!({
		"count": 0,
		"truncated": false,
		"sample": [],
		"next_cursor": null,
	})
}

async fn branch_blob_in_tx(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	sample_limit: usize,
) -> Result<Value> {
	let record = tx_get_decoded(
		tx,
		keys::branches_list_key(branch_id),
		decode_database_branch_record,
	)
	.await?;
	let head = tx_get_decoded(tx, keys::branch_meta_head_key(branch_id), decode_db_head).await?;
	let head_at_fork = tx_get_decoded(
		tx,
		keys::branch_meta_head_at_fork_key(branch_id),
		decode_db_head,
	)
	.await?;
	let compaction_root = tx_get_decoded(
		tx,
		keys::branch_compaction_root_key(branch_id),
		decode_compaction_root,
	)
	.await?;
	let dirty = tx_get_decoded(
		tx,
		keys::sqlite_cmp_dirty_key(branch_id),
		decode_sqlite_cmp_dirty,
	)
	.await?;
	let gc_pin = gc::read_branch_gc_pin_tx(tx, branch_id).await?;
	let mut row_families = serde_json::Map::new();
	for family in [
		RowFamily::Commits,
		RowFamily::Pidx,
		RowFamily::Deltas,
		RowFamily::Shards,
		RowFamily::ColdShards,
		RowFamily::RetiredColdObjects,
		RowFamily::PitrIntervals,
		RowFamily::Pins,
		RowFamily::StagedHotShards,
	] {
		row_families.insert(
			family.as_str().to_string(),
			summary_for_prefix(tx, family.scan_prefix(branch_id), sample_limit).await?,
		);
	}

	Ok(json!({
		"branch_id": branch_id,
		"record": record,
		"head": head,
		"head_at_fork": head_at_fork,
		"pins": gc_pin.map(|pin| {
			json!({
				"branch_id": pin.branch_id,
				"refcount": pin.refcount,
				"root_pin": versionstamp_value(&pin.root_pin),
				"desc_pin": versionstamp_value(&pin.desc_pin),
				"restore_point_pin": versionstamp_value(&pin.restore_point_pin),
				"gc_pin": versionstamp_value(&pin.gc_pin),
			})
		}),
		"compaction": {
			"root": compaction_root,
			"dirty": dirty,
			"manifest_access": {
				"last_hot_pass_txid_key": encode_key(&keys::branch_manifest_last_hot_pass_txid_key(branch_id)),
				"last_access_ts_ms_key": encode_key(&keys::branch_manifest_last_access_ts_ms_key(branch_id)),
				"last_access_bucket_key": encode_key(&keys::branch_manifest_last_access_bucket_key(branch_id)),
			}
		},
		"row_families": row_families,
		"links": {
			"self": format!("/depot/inspect/branches/{}", branch_id.as_uuid()),
			"rows": format!("/depot/inspect/branches/{}/rows/{{family}}", branch_id.as_uuid()),
			"page_trace": format!("/depot/inspect/branches/{}/pages/{{pgno}}/trace", branch_id.as_uuid()),
		}
	}))
}

fn best_effort_decode(key: &[u8], value: Option<&[u8]>) -> Value {
	let mut decoded = serde_json::Map::new();
	decoded.insert("key".to_string(), decode_key_metadata(key));
	if let Some(value) = value {
		decoded.insert("value".to_string(), decode_value_by_key(key, value));
	}
	Value::Object(decoded)
}

fn decode_key_metadata(key: &[u8]) -> Value {
	if key.len() < 2 || key[0] != keys::SQLITE_SUBSPACE_PREFIX {
		return json!({ "family": "unknown" });
	}

	json!({
		"partition": key[1],
		"family": match key[1] {
			keys::DBPTR_PARTITION => "database-pointer",
			keys::BUCKET_PTR_PARTITION => "bucket-pointer",
			keys::BUCKET_CATALOG_PARTITION => "bucket-catalog",
			keys::BRANCHES_PARTITION => "database-branch",
			keys::BUCKET_BRANCH_PARTITION => "bucket-branch",
			keys::BR_PARTITION => "branch-row",
			keys::CTR_PARTITION => "counter",
			keys::RESTORE_POINT_PARTITION => "restore-point",
			keys::CMPC_PARTITION => "compactor-queue",
			keys::DB_PIN_PARTITION => "database-pin",
			keys::BUCKET_FORK_PIN_PARTITION => "bucket-fork-pin",
			keys::BUCKET_CHILD_PARTITION => "bucket-child",
			keys::BUCKET_CATALOG_BY_DB_PARTITION => "bucket-catalog-by-db",
			keys::BUCKET_PROOF_EPOCH_PARTITION => "bucket-proof-epoch",
			keys::SQLITE_CMP_DIRTY_PARTITION => "sqlite-compaction-dirty",
			_ => "unknown",
		}
	})
}

fn decode_value_by_key(key: &[u8], value: &[u8]) -> Value {
	let value_size = value.len();
	if key.len() >= 2 {
		match key[1] {
			keys::DBPTR_PARTITION => return value_or_error(decode_database_pointer(value)),
			keys::BUCKET_PTR_PARTITION => return value_or_error(decode_bucket_pointer(value)),
			keys::BRANCHES_PARTITION => {
				return value_or_error(decode_database_branch_record(value));
			}
			keys::BUCKET_BRANCH_PARTITION => {
				return value_or_error(decode_bucket_branch_record(value));
			}
			keys::SQLITE_CMP_DIRTY_PARTITION => {
				return value_or_error(decode_sqlite_cmp_dirty(value));
			}
			keys::DB_PIN_PARTITION => return value_or_error(decode_db_history_pin(value)),
			_ => {}
		}
	}

	json!({
		"value_size": value_size,
		"sha256": digest_value(value),
	})
}

fn decode_row_value(
	branch_id: DatabaseBranchId,
	family: RowFamily,
	key: &[u8],
	value: &[u8],
	include_bytes: bool,
) -> Value {
	let decoded = match family {
		RowFamily::Commits => {
			let txid = key
				.strip_prefix(keys::branch_commit_prefix(branch_id).as_slice())
				.and_then(|suffix| suffix.try_into().ok())
				.map(u64::from_be_bytes);
			json!({ "txid": txid, "row": result_to_value(decode_commit_row(value)) })
		}
		RowFamily::Pidx => {
			let pgno = key
				.strip_prefix(keys::branch_pidx_prefix(branch_id).as_slice())
				.and_then(|suffix| suffix.try_into().ok())
				.map(u32::from_be_bytes);
			let owner_txid = <[u8; 8]>::try_from(value).ok().map(u64::from_be_bytes);
			json!({ "pgno": pgno, "owner_txid": owner_txid })
		}
		RowFamily::Deltas => json!({
			"txid": keys::decode_branch_delta_chunk_txid(branch_id, key).ok(),
			"chunk_idx": keys::decode_branch_delta_chunk_txid(branch_id, key)
				.ok()
				.and_then(|txid| keys::decode_branch_delta_chunk_idx(branch_id, txid, key).ok()),
			"value_size": value.len(),
			"sha256": digest_value(value),
		}),
		RowFamily::Shards => json!({
			"value_size": value.len(),
			"sha256": digest_value(value),
		}),
		RowFamily::ColdShards => value_or_error(decode_cold_shard_ref(value)),
		RowFamily::RetiredColdObjects => value_or_error(decode_retired_cold_object(value)),
		RowFamily::PitrIntervals => json!({
			"bucket_start_ms": keys::decode_branch_pitr_interval_bucket(branch_id, key).ok(),
			"coverage": result_to_value(decode_pitr_interval_coverage(value)),
		}),
		RowFamily::Pins => value_or_error(decode_db_history_pin(value)),
		RowFamily::StagedHotShards => json!({
			"value_size": value.len(),
			"sha256": digest_value(value),
		}),
	};

	json!({
		"key": encode_key(key),
		"decoded": decoded,
		"bytes": include_bytes.then(|| encode_key(value)),
	})
}

fn value_or_error<T: Serialize>(result: Result<T>) -> Value {
	result_to_value(result)
}

fn result_to_value<T: Serialize>(result: Result<T>) -> Value {
	match result {
		Ok(value) => serde_json::to_value(value).unwrap_or_else(
			|err| json!({ "decode_error": format!("failed to encode decoded value as JSON: {err}") }),
		),
		Err(err) => json!({ "decode_error": err.to_string() }),
	}
}

fn digest_value(value: &[u8]) -> Value {
	let digest = Sha256::digest(value);
	json!({
		"hex": hex_lower(&digest),
		"base64url": encode_key(&digest),
	})
}

fn versionstamp_value(value: &[u8; 16]) -> Value {
	json!({
		"hex": hex_lower(value),
		"base64url": encode_key(value),
	})
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
