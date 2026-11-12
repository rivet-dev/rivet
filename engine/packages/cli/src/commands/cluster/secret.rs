use std::io::{IsTerminal, Read};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

use crate::{
	cloud::{CloudClient, v2},
	cluster_env::{
		MAX_SECRET_BYTES, SecretUsage, delete_plan, secret_usage, set_outcome, set_plan,
		validate_secret_name,
	},
	target::{CloudArgs, ClusterTarget, Target},
	terminal,
};

#[derive(Parser)]
pub struct Opts {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// List the cluster's secrets. Values are never shown.
	List(ListOpts),
	/// Set a secret's value, creating the secret when it does not exist.
	/// Every runner pool whose env names the secret rolls onto the new value,
	/// except a pool a rollback pinned to an older version.
	#[command(alias = "rollout")]
	Set(SetOpts),
	/// Delete a secret. Refused while a runner pool's env names it.
	Delete(DeleteOpts),
}

#[derive(Parser)]
pub struct ListOpts {
	#[command(flatten)]
	cloud: CloudArgs,
	/// Emit raw JSON instead of formatted output.
	#[arg(long)]
	json: bool,
}

/// Sets a secret's value. The value never goes on the command line, where it
/// would land in shell history and the process list: in a terminal it is
/// prompted for without echo, otherwise it is read from stdin, or from the
/// environment variable `--from-env` names.
#[derive(Parser)]
pub struct SetOpts {
	/// Name of the secret, such as `DATABASE_URL` or `db-password`.
	name: String,
	#[command(flatten)]
	cloud: CloudArgs,
	/// Read the value from this environment variable, such as a CI secret,
	/// instead of a prompt or stdin.
	#[arg(long, value_name = "VAR")]
	from_env: Option<String>,
	/// Set an empty value. Without it an empty value is refused, so a
	/// script that forgot to pipe the value fails instead of blanking the
	/// secret.
	#[arg(long)]
	allow_empty: bool,
	/// Set the value without asking to confirm. Required when there is no terminal
	/// to confirm on, such as in CI.
	#[arg(long)]
	yes: bool,
}

#[derive(Parser)]
pub struct DeleteOpts {
	/// Name of the secret to delete.
	name: String,
	#[command(flatten)]
	cloud: CloudArgs,
	/// Delete without asking to confirm. Required when there is no terminal to
	/// confirm on, such as in CI.
	#[arg(long)]
	yes: bool,
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		match self.command {
			Commands::List(opts) => opts.execute().await,
			Commands::Set(opts) => opts.execute().await,
			Commands::Delete(opts) => opts.execute().await,
		}
	}
}

/// The cluster the command targets, and its organization when one was named.
pub(super) async fn connect(cloud: &CloudArgs) -> Result<(CloudClient, String, Option<String>)> {
	let (client, _, target) = cloud.connect().await?;
	match target {
		Target::Cluster(ClusterTarget { cluster, org }) => Ok((client, cluster, org)),
		Target::Project(_) => {
			bail!("no cluster selected; pass --cluster or run `rivet link --cluster <name>`")
		}
	}
}

/// The secret's latest version, or `None` when the cluster does not hold it,
/// and how the cluster's runner pools use it.
async fn inspect_secret(
	cloud: &CloudClient,
	cluster: &str,
	name: &str,
) -> Result<(Option<u64>, SecretUsage)> {
	let latest_version = v2::list_secrets(cloud, cluster)
		.await?
		.into_iter()
		.find(|secret| secret.name == name)
		.map(|secret| secret.latest_version);
	let pools = v2::list_runner_pools(cloud, cluster).await?;
	Ok((latest_version, secret_usage(name, &pools)))
}

impl ListOpts {
	async fn execute(self) -> Result<()> {
		let (cloud, cluster, _) = connect(&self.cloud).await?;
		let secrets = v2::list_secrets(&cloud, &cluster).await?;
		if self.json {
			println!("{}", serde_json::to_string_pretty(&secrets)?);
			return Ok(());
		}
		if secrets.is_empty() {
			println!("cluster {cluster} has no secrets");
			return Ok(());
		}
		for secret in secrets {
			println!(
				"{}\tversion {}\tupdated {}",
				secret.name, secret.latest_version, secret.updated_at
			);
		}
		Ok(())
	}
}

impl SetOpts {
	async fn execute(self) -> Result<()> {
		validate_secret_name(&self.name)?;
		let (cloud, cluster, org) = connect(&self.cloud).await?;
		let (latest_version, usage) = inspect_secret(&cloud, &cluster, &self.name).await?;
		eprintln!(
			"{}",
			set_plan(&cluster, org.as_deref(), &self.name, latest_version, &usage)
		);
		if !self.yes && !terminal::confirm("set this secret?", "set the secret")? {
			eprintln!("aborted");
			return Ok(());
		}

		let (value, source) = read_value(&self.name, self.from_env.as_deref()).await?;
		let value = checked_value(value, source, self.allow_empty)?;
		let result = v2::rollout_secret(&cloud, &cluster, &self.name, &value).await?;
		println!("{}", set_outcome(&result, &usage));
		Ok(())
	}
}

/// Where a rolled-out value came from, for error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ValueSource {
	Prompt,
	Stdin,
	Env,
}

/// The value and where it came from: the environment variable `from_env`
/// names, a hidden prompt in a terminal, or stdin.
async fn read_value(name: &str, from_env: Option<&str>) -> Result<(String, ValueSource)> {
	if let Some(var) = from_env {
		let value = std::env::var(var)
			.with_context(|| format!("environment variable {var} is not set or not valid UTF-8"))?;
		return Ok((value, ValueSource::Env));
	}
	if std::io::stdin().is_terminal() {
		let value = terminal::prompt_hidden(
			format!("Value for secret {name}: "),
			"a secret holds one line; encode a multi-line value, such as a PEM key, as base64 first",
		)
		.await?;
		return Ok((value, ValueSource::Prompt));
	}
	eprintln!("reading the value of secret {name} from stdin until EOF");
	let value = tokio::task::spawn_blocking(|| read_capped(std::io::stdin().lock()))
		.await
		.context("stdin read task failed")??;
	Ok((value, ValueSource::Stdin))
}

/// Reads a value from `reader` without its trailing line ending, refusing one
/// over the size limit without reading the rest of it.
pub(super) fn read_capped(reader: impl Read) -> Result<String> {
	// Room for the value, a `\r\n` a pipe may add, and one byte that proves
	// the input is too large.
	let limit = MAX_SECRET_BYTES + 2;
	let mut bytes = Vec::new();
	reader
		.take(limit as u64 + 1)
		.read_to_end(&mut bytes)
		.context("read the secret's value from stdin")?;
	if bytes.len() > limit {
		bail!("the value on stdin is over the {MAX_SECRET_BYTES} byte limit for a secret");
	}
	let text = String::from_utf8(bytes).context("the value on stdin is not valid UTF-8")?;
	let value = trim_line_end(&text);
	if value.len() > MAX_SECRET_BYTES {
		bail!("the value on stdin is over the {MAX_SECRET_BYTES} byte limit for a secret");
	}
	Ok(value.to_owned())
}

/// `value` without the one line ending a pipe such as `echo` adds.
pub(super) fn trim_line_end(value: &str) -> &str {
	let value = value.strip_suffix('\n').unwrap_or(value);
	value.strip_suffix('\r').unwrap_or(value)
}

/// Refuses a value over the size limit, and an empty value unless
/// `allow_empty`, naming where it came from.
pub(super) fn checked_value(
	value: String,
	source: ValueSource,
	allow_empty: bool,
) -> Result<String> {
	if value.len() > MAX_SECRET_BYTES {
		bail!(
			"the value is {} bytes, over the {MAX_SECRET_BYTES} byte limit for a secret",
			value.len()
		);
	}
	// The API refuses control characters, so a multi-line value fails here
	// instead of after the request.
	if value.chars().any(char::is_control) {
		bail!(
			"the value holds a control character such as a newline or a tab; a secret holds one line, so encode a multi-line value, such as a PEM key, as base64 first"
		);
	}
	if !value.is_empty() || allow_empty {
		return Ok(value);
	}
	match source {
		ValueSource::Prompt => {
			bail!("no value was entered; pass --allow-empty to set an empty value")
		}
		ValueSource::Stdin => bail!(
			"stdin was empty; pipe the value, such as `printf %s \"$VALUE\" | rivet cluster secret set <name>`, use --from-env, or pass --allow-empty"
		),
		ValueSource::Env => bail!(
			"the environment variable is empty; set it or pass --allow-empty to set an empty value"
		),
	}
}

impl DeleteOpts {
	async fn execute(self) -> Result<()> {
		let (cloud, cluster, org) = connect(&self.cloud).await?;
		let (latest_version, usage) = inspect_secret(&cloud, &cluster, &self.name).await?;
		let Some(latest_version) = latest_version else {
			bail!("cluster {cluster} has no secret named {}", self.name);
		};
		eprintln!(
			"{}",
			delete_plan(&cluster, org.as_deref(), &self.name, latest_version, &usage)?
		);
		if !self.yes && !terminal::confirm("delete this secret?", "delete the secret")? {
			eprintln!("aborted");
			return Ok(());
		}
		v2::delete_secret(&cloud, &cluster, &self.name).await?;
		println!("deleted secret {}", self.name);
		Ok(())
	}
}
