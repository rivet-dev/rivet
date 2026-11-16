use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

use super::secret::connect;
use crate::{
	cloud::{
		CloudClient,
		v2::{self, EnvValue, PoolUpsert, SecretRef},
	},
	cluster_env::{EnvChange, check_secrets_exist, edit_env, parse_set_args, unpinned_message},
	target::CloudArgs,
};

#[derive(Parser)]
pub struct Opts {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// List a runner pool's environment variables.
	List(ListOpts),
	/// Set variables on a runner pool, which rolls the pool.
	Set(SetOpts),
	/// Remove variables from a runner pool, which rolls the pool.
	Unset(UnsetOpts),
}

#[derive(Parser)]
pub struct PoolArgs {
	#[command(flatten)]
	cloud: CloudArgs,
	/// The runner pool.
	#[arg(long, default_value = "default")]
	pool: String,
	/// Cluster region of the pool. Defaults to the cluster's only region.
	#[arg(long)]
	region: Option<String>,
}

#[derive(Parser)]
pub struct ListOpts {
	#[command(flatten)]
	pool: PoolArgs,
	/// Emit raw JSON instead of formatted output.
	#[arg(long)]
	json: bool,
}

#[derive(Parser)]
pub struct SetOpts {
	#[command(flatten)]
	pool: PoolArgs,
	/// A string variable, as `KEY=VALUE`. Repeatable.
	#[arg(long = "env", value_name = "KEY=VALUE")]
	env: Vec<String>,
	/// A variable taking a cluster secret's latest version, as `KEY` for a
	/// secret named like the variable, or `KEY=SECRET_NAME`. Repeatable. Set
	/// the secret first with `rivet cluster secret set`.
	#[arg(long = "secret", value_name = "KEY[=SECRET_NAME]")]
	secret: Vec<String>,
}

#[derive(Parser)]
pub struct UnsetOpts {
	#[command(flatten)]
	pool: PoolArgs,
	/// Names of the variables to remove.
	#[arg(required = true)]
	keys: Vec<String>,
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		match self.command {
			Commands::List(opts) => opts.execute().await,
			Commands::Set(opts) => opts.execute().await,
			Commands::Unset(opts) => opts.execute().await,
		}
	}
}

/// A connected pool: its cluster, region, build, allowlist and variables.
struct Pool {
	cloud: CloudClient,
	cluster: String,
	region: String,
	name: String,
	build_id: String,
	cidr_allowlist: Vec<String>,
	env: BTreeMap<String, EnvValue>,
}

impl PoolArgs {
	async fn load(self) -> Result<Pool> {
		let (cloud, cluster, _) = connect(&self.cloud).await?;
		let region = v2::resolve_region(&cloud, &cluster, self.region).await?;
		let Some(pool) = v2::find_runner_pool(&cloud, &cluster, &region, &self.pool).await? else {
			bail!(
				"runner pool {} not found in region {region} of cluster {cluster}; deploy to it first with `rivet deploy`",
				self.pool
			);
		};
		let Some(build_id) = pool.build_id else {
			bail!(
				"runner pool {} has no build yet; deploy to it first with `rivet deploy`",
				self.pool
			);
		};
		Ok(Pool {
			cloud,
			cluster,
			region,
			name: self.pool,
			build_id,
			cidr_allowlist: pool.cidr_allowlist,
			env: pool.env,
		})
	}
}

impl Pool {
	/// Replaces the pool's variables with the change's, rolling it on its
	/// current build, and reports any variables the write unpinned.
	async fn save(&self, change: &EnvChange) -> Result<()> {
		match v2::put_runner_pool(
			&self.cloud,
			&self.cluster,
			&self.region,
			&self.name,
			&self.build_id,
			&self.cidr_allowlist,
			Some(&change.env),
		)
		.await?
		{
			PoolUpsert::Applied => {}
			PoolUpsert::Conflict(message) => {
				bail!("runner pool {} refused the change: {message}", self.name)
			}
		}
		if let Some(message) = unpinned_message(&self.name, &change.unpinned) {
			println!("{message}");
		}
		Ok(())
	}
}

impl ListOpts {
	async fn execute(self) -> Result<()> {
		let pool = self.pool.load().await?;
		if self.json {
			println!("{}", serde_json::to_string_pretty(&pool.env)?);
			return Ok(());
		}
		if pool.env.is_empty() {
			println!("runner pool {} has no environment variables", pool.name);
			return Ok(());
		}
		for (key, value) in &pool.env {
			match value {
				EnvValue::String(value) => println!("{key}={value}"),
				EnvValue::Secret(SecretRef {
					name,
					pinned_version: None,
				}) => println!("{key}=<secret {name}>"),
				EnvValue::Secret(SecretRef {
					name,
					pinned_version: Some(version),
				}) => println!("{key}=<secret {name}, pinned at version {version}>"),
			}
		}
		Ok(())
	}
}

impl SetOpts {
	async fn execute(self) -> Result<()> {
		let set = parse_set_args(&self.env, &self.secret)?;
		if set.is_empty() {
			bail!("pass at least one --env KEY=VALUE or --secret KEY=SECRET_NAME");
		}
		let pool = self.pool.load().await?;
		let secrets = v2::list_secrets(&pool.cloud, &pool.cluster).await?;
		check_secrets_exist(&pool.cluster, &set, &secrets)?;
		let Some(change) = edit_env(&pool.env, &set, &BTreeSet::new()) else {
			println!("runner pool {} already has these variables", pool.name);
			return Ok(());
		};
		pool.save(&change).await?;
		println!("set variables on runner pool {}; it is rolling", pool.name);
		Ok(())
	}
}

impl UnsetOpts {
	async fn execute(self) -> Result<()> {
		let pool = self.pool.load().await?;
		let unset: BTreeSet<String> = self.keys.into_iter().collect();
		let Some(change) = edit_env(&pool.env, &BTreeMap::new(), &unset) else {
			println!("runner pool {} has none of these variables", pool.name);
			return Ok(());
		};
		pool.save(&change).await?;
		println!(
			"removed variables from runner pool {}; it is rolling",
			pool.name
		);
		Ok(())
	}
}
