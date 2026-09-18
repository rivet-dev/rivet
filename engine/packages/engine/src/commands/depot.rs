use std::{path::PathBuf, process, sync::Arc, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::{Parser, Subcommand as ClapSubcommand};
use depot::conveyer::Db;
use depot::doctor::{DoctorInput, DoctorSelector, SkipOptions, doctor, exit_code_for_verdict};
use depot_client_types::{ColumnValue, QueryResult};
use gas::db::{Database as GasolineDatabase, DatabaseKv};
use gas::prelude::{Id, StandaloneCtx};
use serde_json::{Value, json};
use universaldb::{Database, utils::IsolationLevel::*};
use uuid::Uuid;

use super::{depot_transfer, depot_vacuum};

#[derive(Parser)]
pub enum SubCommand {
	/// Diagnose Depot-backed SQLite storage for one database
	Doctor(DoctorOpts),
	/// Execute SQL against one Depot-backed SQLite database
	Execute(ExecuteOpts),
	/// Export one Depot database without mutating its storage
	Export(ExportOpts),
	/// Import a Depot export into an empty local RocksDB database
	Import(ImportOpts),
	/// Resolve actor IDs to their owning namespace IDs
	LookupActorNamespaces(LookupActorNamespacesOpts),
	/// Scan current Depot databases for a specific corruption pattern using snapshot reads only
	ScanCorruption(ScanCorruptionOpts),
	/// Repair one Depot database after writing and verifying an exact backup
	Repair(RepairOpts),
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		match self {
			Self::Doctor(opts) => opts.execute(config).await,
			Self::Execute(opts) => opts.execute(config).await,
			Self::Export(opts) => opts.execute(config).await,
			Self::Import(opts) => opts.execute(config).await,
			Self::LookupActorNamespaces(opts) => opts.execute(config).await,
			Self::ScanCorruption(opts) => opts.execute(config).await,
			Self::Repair(opts) => opts.execute(config).await,
		}
	}
}

#[derive(Parser)]
pub struct LookupActorNamespacesOpts {
	/// Actor IDs to resolve
	#[arg(required = true)]
	actor_ids: Vec<Id>,
}

impl LookupActorNamespacesOpts {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let pools = rivet_pools::Pools::new(config).await?;
		let udb = pools.udb()?;
		let mut mappings = Vec::with_capacity(self.actor_ids.len());
		for actor_id in self.actor_ids {
			let namespace_id = lookup_actor_namespace_id(&udb, actor_id).await?;
			mappings.push(json!({
				"actor_id": actor_id,
				"namespace_id": namespace_id,
			}));
		}
		println!("{}", serde_json::to_string_pretty(&mappings)?);
		Ok(())
	}
}

#[derive(Parser)]
pub struct ScanCorruptionOpts {
	#[command(subcommand)]
	method: CorruptionScanMethod,
}

#[derive(ClapSubcommand)]
enum CorruptionScanMethod {
	/// Detect selected hot shards missing live pages after no-cold multi-slice compaction
	PartialHotShard(HotShardHistoryScanOpts),
}

impl ScanCorruptionOpts {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		match self.method {
			CorruptionScanMethod::PartialHotShard(opts) => opts.execute(config).await,
		}
	}
}

#[derive(Parser)]
pub struct HotShardHistoryScanOpts {
	/// Stop each snapshot range transaction after this many milliseconds and resume from a cursor
	#[arg(long, default_value_t = 2_000)]
	transaction_max_ms: u64,
	/// Stop each snapshot range transaction after retaining this many MiB and resume from a cursor
	#[arg(long, default_value_t = 4)]
	transaction_max_mb: usize,
	/// Number of current database branches to inspect concurrently
	#[arg(long, default_value_t = 4)]
	concurrency: usize,
	/// Inspect at most this many current database branches; intended for smoke tests
	#[arg(long)]
	database_limit: Option<usize>,
}

impl HotShardHistoryScanOpts {
	async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let transaction_max_bytes = self
			.transaction_max_mb
			.checked_mul(1024 * 1024)
			.context("--transaction-max-mb is too large")?;
		let pools = rivet_pools::Pools::new(config).await?;
		let udb = pools.udb()?;
		let report = depot::recovery::scan_hot_shard_history_corruption(
			&udb,
			depot::recovery::HotShardHistoryScanOptions {
				transaction_max_duration: Duration::from_millis(self.transaction_max_ms),
				transaction_max_bytes,
				concurrency: self.concurrency,
				database_limit: self.database_limit,
			},
		)
		.await
		.context("scan Depot hot shard history corruption")?;
		println!("{}", serde_json::to_string_pretty(&report)?);
		Ok(())
	}
}

#[derive(Parser)]
pub struct ExportOpts {
	#[arg(long)]
	bucket_id: Option<Uuid>,
	#[arg(long)]
	database_id: Option<String>,
	#[arg(long)]
	actor_id: Option<Id>,
	#[arg(long)]
	output: PathBuf,
}

impl ExportOpts {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		let udb = pools.udb()?;
		let bucket_database = self.bucket_id.is_some() || self.database_id.is_some();
		let actor = self.actor_id.is_some();
		ensure!(
			usize::from(bucket_database) + usize::from(actor) == 1,
			"provide exactly one selector: --bucket-id/--database-id or --actor-id"
		);
		let target = if let Some(actor_id) = self.actor_id {
			let namespace_id = lookup_actor_namespace_id(&udb, actor_id).await?;
			depot_transfer::ExportTarget {
				bucket_id: namespace_id,
				database_id: actor_id.to_string(),
			}
		} else {
			depot_transfer::ExportTarget {
				bucket_id: Id::v1(
					self.bucket_id
						.context("--bucket-id is required with --database-id")?,
					0,
				),
				database_id: self
					.database_id
					.context("--database-id is required with --bucket-id")?,
			}
		};
		let summary = depot_transfer::export_database(&config, &udb, target, &self.output)
			.await
			.context("export Depot database")?;

		println!("{}", serde_json::to_string_pretty(&summary)?);
		Ok(())
	}
}

#[derive(Parser)]
pub struct ImportOpts {
	#[arg(long)]
	input: PathBuf,
}

impl ImportOpts {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		depot_transfer::verify_local_import_config(&config)?;
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		let udb = pools.udb()?;
		let summary = depot_transfer::import_database(&config, &udb, &self.input)
			.await
			.context("import Depot database")?;

		println!("{}", serde_json::to_string_pretty(&summary)?);
		Ok(())
	}
}

#[derive(Parser)]
pub struct RepairOpts {
	#[command(subcommand)]
	strategy: RepairStrategy,
}

#[derive(ClapSubcommand)]
enum RepairStrategy {
	/// Rebuild a database whose committed b-tree is inconsistent even though its pages are intact
	Vacuum(VacuumRepairOpts),
}

impl RepairOpts {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		match self.strategy {
			RepairStrategy::Vacuum(opts) => opts.execute(config).await,
		}
	}
}

#[derive(Parser)]
pub struct DoctorOpts {
	#[arg(long)]
	bucket_id: Option<Uuid>,
	#[arg(long)]
	database_id: Option<String>,
	#[arg(long)]
	actor_id: Option<Id>,
	#[arg(long)]
	database_branch_id: Option<Uuid>,
	#[arg(long)]
	artifact_dir: Option<PathBuf>,
	#[arg(long)]
	skip_full_integrity_check: bool,
	#[arg(long)]
	skip_first_bad_txid: bool,
	#[arg(long)]
	skip_page_provenance: bool,
	#[arg(long)]
	skip_resolver_compare: bool,
	#[arg(long)]
	min_txid: Option<u64>,
	#[arg(long)]
	max_txid: Option<u64>,
}

impl DoctorOpts {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		let udb = pools.udb()?;
		let selector = self.selector(&udb).await?;
		let input = DoctorInput {
			selector,
			artifact_dir: self.artifact_dir,
			skip: SkipOptions {
				full_integrity_check: self.skip_full_integrity_check,
				first_bad_txid: self.skip_first_bad_txid,
				page_provenance: self.skip_page_provenance,
				resolver_compare: self.skip_resolver_compare,
			},
			min_txid: self.min_txid,
			max_txid: self.max_txid,
			progress_hook: None,
		};

		let report = doctor(&udb, input).await.context("run depot doctor")?;
		let code = exit_code_for_verdict(report.verdict.verdict);
		println!("{}", serde_json::to_string_pretty(&report)?);

		if code == 0 {
			Ok(())
		} else {
			process::exit(code);
		}
	}

	async fn selector(&self, udb: &Database) -> Result<DoctorSelector> {
		let bucket_database = self.bucket_id.is_some() || self.database_id.is_some();
		let actor = self.actor_id.is_some();
		let branch = self.database_branch_id.is_some();
		let selector_count =
			usize::from(bucket_database) + usize::from(actor) + usize::from(branch);
		if selector_count != 1 {
			bail!(
				"provide exactly one selector: --bucket-id/--database-id, --actor-id, or --database-branch-id"
			);
		}

		if bucket_database {
			return Ok(DoctorSelector::BucketDatabase {
				bucket_id: self
					.bucket_id
					.context("--bucket-id is required with --database-id")?,
				database_id: self
					.database_id
					.clone()
					.context("--database-id is required with --bucket-id")?,
			});
		}

		if actor {
			let actor_id = self.actor_id.context("--actor-id is required")?;
			let namespace_id = lookup_actor_namespace_id(udb, actor_id).await?;
			return Ok(DoctorSelector::Actor {
				namespace_id,
				actor_id,
			});
		}

		Ok(DoctorSelector::DatabaseBranch {
			database_branch_id: self
				.database_branch_id
				.context("--database-branch-id is required")?,
		})
	}
}

#[derive(Parser)]
pub struct ExecuteOpts {
	#[arg(long)]
	bucket_id: Option<Uuid>,
	#[arg(long)]
	database_id: Option<String>,
	#[arg(long)]
	actor_id: Option<Id>,
	#[arg(short = 'q', long)]
	query: String,
}

impl ExecuteOpts {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		let udb = pools.udb()?;
		let target = self.target(&udb).await?;
		let db = Arc::new(
			Db::new(
				Arc::new((*udb).clone()),
				target.bucket_id,
				target.database_id.clone(),
				pools.node_id(),
			)
			.with_config(config.clone()),
		);

		let sqlite = depot_client_embedded::open_database_from_embedded_depot(
			db,
			target.database_id,
			0,
			tokio::runtime::Handle::current(),
			None,
		)
		.await
		.context("open Depot-backed SQLite database")?;
		let result = sqlite.exec(self.query).await.context("execute SQL");
		let close_result = sqlite.close().await.context("close SQLite database");
		let result = match (result, close_result) {
			(Ok(result), Ok(())) => result,
			(Err(error), _) => return Err(error),
			(Ok(_), Err(error)) => return Err(error),
		};

		println!(
			"{}",
			serde_json::to_string_pretty(&query_result_json(result))?
		);

		Ok(())
	}

	async fn target(&self, udb: &Database) -> Result<ExecuteTarget> {
		let bucket_database = self.bucket_id.is_some() || self.database_id.is_some();
		let actor = self.actor_id.is_some();
		let selector_count = usize::from(bucket_database) + usize::from(actor);
		if selector_count != 1 {
			bail!("provide exactly one selector: --bucket-id/--database-id or --actor-id");
		}

		if bucket_database {
			let bucket_id = self
				.bucket_id
				.context("--bucket-id is required with --database-id")?;
			let database_id = self
				.database_id
				.clone()
				.context("--database-id is required with --bucket-id")?;
			return Ok(ExecuteTarget {
				bucket_id: Id::v1(bucket_id, 0),
				database_id,
			});
		}

		let actor_id = self.actor_id.context("--actor-id is required")?;
		let namespace_id = lookup_actor_namespace_id(udb, actor_id).await?;
		Ok(ExecuteTarget {
			bucket_id: namespace_id,
			database_id: actor_id.to_string(),
		})
	}
}

struct ExecuteTarget {
	bucket_id: Id,
	database_id: String,
}

fn query_result_json(result: QueryResult) -> Value {
	json!({
		"columns": result.columns,
		"rows": result.rows.into_iter().map(row_json).collect::<Vec<_>>(),
	})
}

fn row_json(row: Vec<ColumnValue>) -> Value {
	Value::Array(row.into_iter().map(column_value_json).collect())
}

fn column_value_json(value: ColumnValue) -> Value {
	match value {
		ColumnValue::Null => Value::Null,
		ColumnValue::Integer(value) => json!(value),
		ColumnValue::Float(value) => json!(value),
		ColumnValue::Text(value) => json!(value),
		ColumnValue::Blob(value) => json!({
			"type": "blob",
			"base64": STANDARD.encode(value),
		}),
	}
}

async fn lookup_actor_namespace_id(udb: &Database, actor_id: Id) -> Result<Id> {
	udb.txn("engine_depot_lookup_actor_namespace", |tx| async move {
		let tx = tx.with_subspace(pegboard::keys::subspace());
		let namespace_id_key = pegboard::keys::actor::NamespaceIdKey::new(actor_id);

		tx.read_opt(&namespace_id_key, Serializable)
			.await?
			.with_context(|| format!("actor namespace id not found for actor_id {actor_id}"))
	})
	.await
	.context("look up actor namespace id")
}

#[derive(Parser)]
pub struct VacuumRepairOpts {
	/// Treat positional database values as database IDs in this bucket. Without this, values are actor IDs.
	#[arg(long)]
	bucket_id: Option<Uuid>,
	/// Prove the rebuild against a throwaway copy without writing a backup or changing storage
	#[arg(long)]
	dry_run: bool,
	/// New absolute directory that will contain one verified pre-repair export per database
	#[arg(long, value_name = "DIR")]
	backup_dir: Option<PathBuf>,
	/// Absolute directory for throwaway vacuum candidates. Defaults to a temp directory.
	#[arg(long, value_name = "DIR")]
	candidate_dir: Option<PathBuf>,
	/// Proceed even when a table cannot be scanned, accepting the loss of rows it cannot reach
	#[arg(long)]
	allow_unreadable_tables: bool,
	/// Database IDs when --bucket-id is present; otherwise actor IDs
	#[arg(value_name = "DATABASE", required = true)]
	databases: Vec<String>,
}

impl VacuumRepairOpts {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let pools = rivet_pools::Pools::new(config.clone()).await?;
		let udb = pools.udb()?;
		// The repair commits like an actor does, so it needs a workflow context to signal the
		// compaction manager for the deltas it produces.
		let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
		let ctx = StandaloneCtx::new(
			DatabaseKv::new(config.clone(), pools.clone()).await?,
			config.clone(),
			pools.clone(),
			cache,
			"depot_repair_vacuum",
			Id::new_v1(config.dc_label()),
			Id::new_v1(config.dc_label()),
		)?;

		let mut targets = Vec::with_capacity(self.databases.len());
		for database in &self.databases {
			let target = if let Some(bucket_id) = self.bucket_id {
				depot_transfer::ExportTarget {
					bucket_id: Id::v1(bucket_id, 0),
					database_id: database.clone(),
				}
			} else {
				let actor_id = database
					.parse::<Id>()
					.with_context(|| format!("parse actor id {database}"))?;
				let namespace_id = lookup_actor_namespace_id(&udb, actor_id).await?;
				depot_transfer::ExportTarget {
					bucket_id: namespace_id,
					database_id: actor_id.to_string(),
				}
			};
			targets.push(target);
		}

		let candidate_dir = depot_vacuum::candidate_dir(self.candidate_dir.as_ref())?;

		// Prove every database before the first write so an inapplicable target cannot leave the
		// batch half repaired.
		let mut baselines = Vec::with_capacity(targets.len());
		let mut preflights = Vec::with_capacity(targets.len());
		for (index, target) in targets.iter().enumerate() {
			let sqlite = depot_vacuum::open_database(&ctx, &pools, target).await?;
			let baseline = depot_vacuum::capture_baseline(&sqlite).await;
			let candidate = candidate_dir.join(format!("{index:04}.sqlite"));
			let preflight = match &baseline {
				Ok(baseline) => {
					depot_vacuum::preflight(
						&sqlite,
						baseline,
						&candidate,
						self.allow_unreadable_tables,
					)
					.await
				}
				Err(_) => Ok(depot_vacuum::VacuumPreflight {
					applicable: false,
					rejection_reason: Some("could not read the database".to_string()),
					candidate_integrity: Vec::new(),
					candidate_row_counts: Default::default(),
					foreign_key_violations: 0,
					recovered_tables: Vec::new(),
				}),
			};
			sqlite.close().await.context("close SQLite database")?;
			let _ = std::fs::remove_file(&candidate);

			let baseline = baseline?;
			let preflight = preflight?;
			if !self.dry_run {
				ensure!(
					preflight.applicable,
					"database {} is not repairable with vacuum: {}",
					target.database_id,
					preflight
						.rejection_reason
						.as_deref()
						.unwrap_or("unknown reason")
				);
			}
			baselines.push(baseline);
			preflights.push(preflight);
		}

		let mut backups = Vec::with_capacity(targets.len());
		if self.dry_run {
			backups.resize_with(targets.len(), || None);
		} else {
			let backup_dir = self
				.backup_dir
				.clone()
				.context("--backup-dir is required unless --dry-run is used")?;
			ensure!(
				backup_dir.is_absolute(),
				"--backup-dir must be an absolute path"
			);
			depot_transfer::create_private_export_root(&config, &backup_dir)?;

			// Complete and verify every backup before the first repair write.
			for (index, target) in targets.iter().enumerate() {
				let output = backup_dir.join(format!("{index:04}"));
				backups.push(Some(
					depot_transfer::export_database(&config, &udb, target.clone(), &output)
						.await
						.with_context(|| format!("back up database {}", target.database_id))?,
				));
			}
			for (target, backup) in targets.iter().zip(&backups) {
				depot_transfer::verify_database_export(
					&config,
					&udb,
					target.clone(),
					&backup
						.as_ref()
						.expect("non-dry repair has backups")
						.artifact_dir,
				)
				.await
				.with_context(|| format!("reverify backup for database {}", target.database_id))?;
			}

			for (target, baseline) in targets.iter().zip(&baselines) {
				let sqlite = depot_vacuum::open_database(&ctx, &pools, target).await?;
				let applied = depot_vacuum::apply(&sqlite).await;
				let verified = match &applied {
					Ok(()) => depot_vacuum::verify_applied(&sqlite, baseline).await,
					Err(_) => Ok(()),
				};
				sqlite.close().await.context("close SQLite database")?;
				applied.with_context(|| format!("repair database {}", target.database_id))?;
				verified
					.with_context(|| format!("verify repaired database {}", target.database_id))?;
			}
		}

		let output = self
			.databases
			.iter()
			.zip(&baselines)
			.zip(&preflights)
			.zip(&backups)
			.map(|(((database_id, baseline), preflight), backup)| {
				let mut entry = depot_vacuum::preflight_json(baseline, preflight);
				entry["database"] = json!(database_id);
				entry["strategy"] = json!("vacuum");
				entry["dry_run"] = json!(self.dry_run);
				entry["backup"] = match backup {
					Some(backup) => json!({ "artifact_dir": backup.artifact_dir }),
					None => Value::Null,
				};
				entry
			})
			.collect::<Vec<_>>();
		println!("{}", serde_json::to_string_pretty(&json!(output))?);

		Ok(())
	}
}
