//! Vacuum repair for Depot-backed SQLite databases.
//!
//! This strategy targets damage that lives in the committed logical content rather than in the
//! storage materialization: the page images Depot returns are exactly what was committed, but the
//! b-tree those pages describe is internally inconsistent (for example `rowid out of order`). The
//! shard fold is correct, so `repair hot-shard-history` has nothing to restore and rightly refuses.
//!
//! SQLite can rebuild such a database from the rows it can still reach, which is what `VACUUM`
//! does. The risk is that a rebuild silently drops rows stranded behind a damaged cell, so the
//! repair is proven against a throwaway copy before any storage is written: `VACUUM INTO` produces
//! a candidate file, the candidate is checked for integrity and compared row-for-row against the
//! live database, and only a candidate that preserves every readable row is allowed to proceed.

use std::{
	collections::BTreeMap,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::{Context, Result, ensure};
use depot::conveyer::Db;
use depot::conveyer::db::CompactionSignaler;
use depot::workflows::compaction::{
	DATABASE_BRANCH_ID_TAG, DbManagerInput, DeltasAvailable, database_branch_tag_value,
};
use depot_client_types::{ColumnValue, QueryResult};
use futures_util::FutureExt;
use gas::prelude::*;
use serde_json::{Value, json};

use super::depot_transfer::ExportTarget;

/// Rows returned by `PRAGMA integrity_check` on a healthy database.
const INTEGRITY_OK: &str = "ok";

/// Name the vacuum candidate is written to inside the Depot VFS before it is copied to the host
/// filesystem. It must not collide with the database's own VFS name, which is the actor id.
const CANDIDATE_VFS_NAME: &str = "rivet-vacuum-candidate";

/// Facts captured from a database before it is touched, used to prove a candidate rebuild lost
/// nothing. Row counts come from full scans rather than an index so a damaged index cannot hide a
/// row that the rebuild would drop.
#[derive(Debug, Clone)]
pub struct DatabaseBaseline {
	pub integrity: Vec<String>,
	pub schema: Vec<(String, String, String)>,
	pub row_counts: BTreeMap<String, i64>,
	/// Tables whose scan failed outright. These cannot be compared after the rebuild, so their
	/// presence blocks the repair unless the operator accepts the loss explicitly.
	pub unreadable_tables: Vec<String>,
}

impl DatabaseBaseline {
	pub fn is_corrupt(&self) -> bool {
		!(self.integrity.len() == 1 && self.integrity[0] == INTEGRITY_OK)
	}
}

/// Outcome of proving a rebuild against a throwaway copy. `applicable` is the gate: a false value
/// means the repair must not be applied to storage.
#[derive(Debug, Clone)]
pub struct VacuumPreflight {
	pub applicable: bool,
	pub rejection_reason: Option<String>,
	pub candidate_integrity: Vec<String>,
	pub candidate_row_counts: BTreeMap<String, i64>,
	pub foreign_key_violations: usize,
	pub recovered_tables: Vec<String>,
}

fn column_text(value: &ColumnValue) -> String {
	match value {
		ColumnValue::Null => String::new(),
		ColumnValue::Integer(v) => v.to_string(),
		ColumnValue::Float(v) => v.to_string(),
		ColumnValue::Text(v) => v.clone(),
		ColumnValue::Blob(v) => format!("<blob {} bytes>", v.len()),
	}
}

fn first_column_rows(result: &QueryResult) -> Vec<String> {
	result
		.rows
		.iter()
		.filter_map(|row| row.first())
		.map(column_text)
		.collect()
}

fn column_integer(value: &ColumnValue) -> Option<i64> {
	match value {
		ColumnValue::Integer(v) => Some(*v),
		_ => None,
	}
}

/// Quotes a SQLite identifier so a table name from `sqlite_master` cannot terminate the statement.
fn quote_ident(name: &str) -> String {
	format!("\"{}\"", name.replace('"', "\"\""))
}

/// Opens the live Depot-backed database for one target.
///
/// The handle is built the same way `pegboard-envoy` builds an actor's handle, because a repair
/// commits through the same path an actor does. A vacuum rewrites every page, so its commits
/// produce the largest delta burst a database will ever see, and without a compaction signaler
/// nothing would wake the branch's compaction manager for them.
pub async fn open_database(
	ctx: &StandaloneCtx,
	pools: &rivet_pools::Pools,
	target: &ExportTarget,
) -> Result<depot_client::database::NativeDatabaseHandle> {
	let udb = pools.udb()?;
	let db = Arc::new(
		if pools.config().sqlite().unstable_disable_compaction() {
			Db::new(
				Arc::new((*udb).clone()),
				target.bucket_id,
				target.database_id.clone(),
				pools.node_id(),
			)
		} else {
			Db::new_with_compaction_signaler(
				Arc::new((*udb).clone()),
				target.bucket_id,
				target.database_id.clone(),
				pools.node_id(),
				compaction_signaler(ctx, target.database_id.clone()),
			)
		}
		.with_config(pools.config().clone()),
	);
	depot_client_embedded::open_database_from_embedded_depot(
		db,
		target.database_id.clone(),
		0,
		tokio::runtime::Handle::current(),
		None,
	)
	.await
	.with_context(|| format!("open Depot database {}", target.database_id))
}

/// Builds the signaler that wakes a database's compaction manager when a repair commit produces
/// deltas. This mirrors the signaler `pegboard-envoy` installs for a live actor.
fn compaction_signaler(ctx: &StandaloneCtx, actor_id: String) -> CompactionSignaler {
	let ctx = ctx.clone();
	Arc::new(move |signal: DeltasAvailable| {
		let ctx = ctx.clone();
		let actor_id = actor_id.clone();
		async move {
			let tag_value = database_branch_tag_value(signal.database_branch_id);
			let workflow_id = ctx
				.workflow(DbManagerInput {
					database_branch_id: signal.database_branch_id,
					actor_id: Some(actor_id),
				})
				.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
				.unique()
				.dispatch()
				.await?;
			ctx.signal(signal)
				.to_workflow_id(workflow_id)
				.send()
				.await?;
			Ok(())
		}
		.boxed()
	})
}

/// Captures the pre-repair facts used to judge a candidate rebuild.
pub async fn capture_baseline(
	sqlite: &depot_client::database::NativeDatabaseHandle,
) -> Result<DatabaseBaseline> {
	let integrity = first_column_rows(
		&sqlite
			.query("PRAGMA integrity_check".to_string(), None)
			.await
			.context("run integrity_check on the live database")?,
	);

	let schema_result = sqlite
		.query(
			"SELECT type, name, COALESCE(sql, '') FROM sqlite_master ORDER BY type, name"
				.to_string(),
			None,
		)
		.await
		.context("read schema from the live database")?;
	let schema = schema_result
		.rows
		.iter()
		.map(|row| {
			(
				row.first().map(column_text).unwrap_or_default(),
				row.get(1).map(column_text).unwrap_or_default(),
				row.get(2).map(column_text).unwrap_or_default(),
			)
		})
		.collect::<Vec<_>>();

	let mut row_counts = BTreeMap::new();
	let mut unreadable_tables = Vec::new();
	for (kind, name, _) in &schema {
		if kind != "table" || name.starts_with("sqlite_") {
			continue;
		}
		// A full scan, not an indexed count, so a damaged index cannot mask a missing row.
		let sql = format!("SELECT count(*) FROM {}", quote_ident(name));
		match sqlite.query(sql, None).await {
			Ok(result) => {
				let count = result
					.rows
					.first()
					.and_then(|row| row.first())
					.and_then(column_integer)
					.unwrap_or_default();
				row_counts.insert(name.clone(), count);
			}
			Err(_) => unreadable_tables.push(name.clone()),
		}
	}

	Ok(DatabaseBaseline {
		integrity,
		schema,
		row_counts,
		unreadable_tables,
	})
}

/// Builds a candidate rebuild in `candidate_path` and proves it preserves the live database.
///
/// Nothing here writes to Depot storage. `VACUUM INTO` only reads the live database; the candidate
/// is an ordinary local file that is inspected and then discarded by the caller.
pub async fn preflight(
	sqlite: &depot_client::database::NativeDatabaseHandle,
	baseline: &DatabaseBaseline,
	candidate_path: &Path,
	allow_unreadable_tables: bool,
) -> Result<VacuumPreflight> {
	let reject = |reason: String| VacuumPreflight {
		applicable: false,
		rejection_reason: Some(reason),
		candidate_integrity: Vec::new(),
		candidate_row_counts: BTreeMap::new(),
		foreign_key_violations: 0,
		recovered_tables: Vec::new(),
	};

	if !baseline.is_corrupt() {
		return Ok(reject(
			"database already passes integrity_check; vacuum repair only applies to a corrupt database".to_string(),
		));
	}
	if !baseline.unreadable_tables.is_empty() && !allow_unreadable_tables {
		return Ok(reject(format!(
			"tables cannot be scanned and their contents cannot be proven preserved: {}; re-run with --allow-unreadable-tables to accept losing unreachable rows",
			baseline.unreadable_tables.join(", ")
		)));
	}

	if candidate_path.exists() {
		std::fs::remove_file(candidate_path).context("clear stale vacuum candidate")?;
	}

	// `VACUUM INTO` resolves its output file through the VFS of the connection running it, which
	// here is the Depot VFS. A host path would therefore not produce a host file, so the candidate
	// is written to a Depot VFS name and copied onto the filesystem afterwards.
	sqlite
		.query(format!("VACUUM INTO '{CANDIDATE_VFS_NAME}'"), None)
		.await
		.context("build vacuum candidate")?;
	let candidate_bytes = sqlite
		.read_vfs_file(CANDIDATE_VFS_NAME)
		.context("vacuum candidate was not produced")?;
	sqlite.delete_vfs_file(CANDIDATE_VFS_NAME);
	std::fs::write(candidate_path, &candidate_bytes)
		.with_context(|| format!("write vacuum candidate to {}", candidate_path.display()))?;

	// The candidate is now a plain file, so it is inspected directly instead of through Depot.
	let candidate = rusqlite::Connection::open(candidate_path).context("open vacuum candidate")?;

	let candidate_integrity = candidate
		.prepare("PRAGMA integrity_check")
		.and_then(|mut stmt| {
			stmt.query_map([], |row| row.get::<_, String>(0))?
				.collect::<rusqlite::Result<Vec<_>>>()
		})
		.context("run integrity_check on the vacuum candidate")?;
	if !(candidate_integrity.len() == 1 && candidate_integrity[0] == INTEGRITY_OK) {
		return Ok(VacuumPreflight {
			applicable: false,
			rejection_reason: Some(format!(
				"vacuum candidate is still corrupt: {}",
				candidate_integrity.join("; ")
			)),
			candidate_integrity,
			..reject(String::new())
		});
	}

	let foreign_key_violations = candidate
		.prepare("PRAGMA foreign_key_check")
		.and_then(|mut stmt| {
			stmt.query_map([], |_| Ok(()))?
				.collect::<rusqlite::Result<Vec<_>>>()
		})
		.context("run foreign_key_check on the vacuum candidate")?
		.len();
	if foreign_key_violations > 0 {
		return Ok(VacuumPreflight {
			applicable: false,
			rejection_reason: Some(format!(
				"vacuum candidate has {foreign_key_violations} foreign key violations"
			)),
			candidate_integrity,
			foreign_key_violations,
			..reject(String::new())
		});
	}

	// Every schema object must survive. A rebuild that silently drops a table or index is a
	// regression even when the remaining rows check out.
	let mut candidate_schema = candidate
		.prepare("SELECT type, name, COALESCE(sql, '') FROM sqlite_master ORDER BY type, name")
		.and_then(|mut stmt| {
			stmt.query_map([], |row| {
				Ok((
					row.get::<_, String>(0)?,
					row.get::<_, String>(1)?,
					row.get::<_, String>(2)?,
				))
			})?
			.collect::<rusqlite::Result<Vec<_>>>()
		})
		.context("read schema from the vacuum candidate")?;
	candidate_schema.sort();
	let mut expected_schema = baseline.schema.clone();
	expected_schema.sort();
	if candidate_schema != expected_schema {
		let missing = expected_schema
			.iter()
			.filter(|entry| !candidate_schema.contains(entry))
			.map(|(kind, name, _)| format!("{kind} {name}"))
			.collect::<Vec<_>>();
		let added = candidate_schema
			.iter()
			.filter(|entry| !expected_schema.contains(entry))
			.map(|(kind, name, _)| format!("{kind} {name}"))
			.collect::<Vec<_>>();
		return Ok(VacuumPreflight {
			applicable: false,
			rejection_reason: Some(format!(
				"vacuum candidate schema differs; missing [{}] added [{}]",
				missing.join(", "),
				added.join(", ")
			)),
			candidate_integrity,
			foreign_key_violations,
			..reject(String::new())
		});
	}

	// No table may lose rows. A rebuild is allowed to recover more rows than a damaged scan could
	// reach, but never fewer.
	let mut candidate_row_counts = BTreeMap::new();
	let mut recovered_tables = Vec::new();
	for name in baseline.row_counts.keys() {
		let count: i64 = candidate
			.query_row(
				&format!("SELECT count(*) FROM {}", quote_ident(name)),
				[],
				|row| row.get(0),
			)
			.with_context(|| format!("count rows in vacuum candidate table {name}"))?;
		candidate_row_counts.insert(name.clone(), count);
	}
	let mut lost = Vec::new();
	for (name, before) in &baseline.row_counts {
		let after = candidate_row_counts.get(name).copied().unwrap_or_default();
		if after < *before {
			lost.push(format!("{name}: {before} -> {after}"));
		} else if after > *before {
			recovered_tables.push(format!("{name}: {before} -> {after}"));
		}
	}
	if !lost.is_empty() {
		return Ok(VacuumPreflight {
			applicable: false,
			rejection_reason: Some(format!("vacuum candidate loses rows: {}", lost.join(", "))),
			candidate_integrity,
			candidate_row_counts,
			foreign_key_violations,
			recovered_tables,
		});
	}

	Ok(VacuumPreflight {
		applicable: true,
		rejection_reason: None,
		candidate_integrity,
		candidate_row_counts,
		foreign_key_violations,
		recovered_tables,
	})
}

/// Rebuilds the live database in place. Only called after `preflight` proved an equivalent rebuild
/// is safe. SQLite runs `VACUUM` as a single transaction, so a failure leaves the database as it
/// was rather than partially rewritten.
pub async fn apply(sqlite: &depot_client::database::NativeDatabaseHandle) -> Result<()> {
	sqlite
		.query("VACUUM".to_string(), None)
		.await
		.context("vacuum the live database")?;
	Ok(())
}

/// Confirms the live database is healthy and complete after the rebuild.
pub async fn verify_applied(
	sqlite: &depot_client::database::NativeDatabaseHandle,
	baseline: &DatabaseBaseline,
) -> Result<()> {
	let after = capture_baseline(sqlite)
		.await
		.context("re-read the database after vacuum")?;
	ensure!(
		!after.is_corrupt(),
		"database is still corrupt after vacuum: {}",
		after.integrity.join("; ")
	);
	for (name, before) in &baseline.row_counts {
		let now = after.row_counts.get(name).copied().unwrap_or_default();
		ensure!(
			now >= *before,
			"table {name} lost rows during vacuum: {before} -> {now}"
		);
	}
	Ok(())
}

pub fn preflight_json(baseline: &DatabaseBaseline, preflight: &VacuumPreflight) -> Value {
	json!({
		"before_integrity": baseline.integrity,
		"before_row_counts": baseline.row_counts,
		"unreadable_tables": baseline.unreadable_tables,
		"applicable": preflight.applicable,
		"rejection_reason": preflight.rejection_reason,
		"candidate_integrity": preflight.candidate_integrity,
		"candidate_row_counts": preflight.candidate_row_counts,
		"foreign_key_violations": preflight.foreign_key_violations,
		"recovered_rows": preflight.recovered_tables,
	})
}

/// Directory that holds throwaway vacuum candidates for one run.
pub fn candidate_dir(base: Option<&PathBuf>) -> Result<PathBuf> {
	let dir = match base {
		Some(dir) => {
			ensure!(
				dir.is_absolute(),
				"--candidate-dir must be an absolute path"
			);
			dir.clone()
		}
		None => std::env::temp_dir().join("rivet-depot-vacuum"),
	};
	std::fs::create_dir_all(&dir)
		.with_context(|| format!("create vacuum candidate directory {}", dir.display()))?;
	Ok(dir)
}
