use std::{path::PathBuf, process, sync::Arc};

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::Parser;
use depot::conveyer::Db;
use depot::doctor::{DoctorInput, DoctorSelector, SkipOptions, doctor, exit_code_for_verdict};
use depot_client_types::{ColumnValue, QueryResult};
use gas::prelude::Id;
use serde_json::{Value, json};
use universaldb::{Database, utils::IsolationLevel::*};
use uuid::Uuid;

#[derive(Parser)]
pub enum SubCommand {
	/// Diagnose Depot-backed SQLite storage for one database
	Doctor(DoctorOpts),
	/// Execute SQL against one Depot-backed SQLite database
	Execute(ExecuteOpts),
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		match self {
			Self::Doctor(opts) => opts.execute(config).await,
			Self::Execute(opts) => opts.execute(config).await,
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
		let pools = rivet_pools::Pools::new(config).await?;
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
		let pools = rivet_pools::Pools::new(config).await?;
		let udb = pools.udb()?;
		let target = self.target(&udb).await?;
		let db = Arc::new(Db::new(
			Arc::new((*udb).clone()),
			target.bucket_id,
			target.database_id.clone(),
			pools.node_id(),
		));

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
