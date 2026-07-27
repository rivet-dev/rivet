use std::io::IsTerminal;

use anstyle::{AnsiColor, Style};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Method;

use crate::{
	DEFAULT_CLOUD_API, DEFAULT_NAMESPACE,
	cloud::{CloudClient, PoolConfig, PoolSummary, TokenInspectResponse, get_namespace, list_pools},
	credentials::resolve_token,
};

#[derive(Parser)]
pub struct Opts {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// List the compute pools in a namespace.
	List(ListOpts),
}

#[derive(Parser)]
pub struct ListOpts {
	/// Rivet Cloud API token.
	#[arg(long)]
	token: Option<String>,
	/// Cloud namespace to list pools from.
	#[arg(long, default_value = DEFAULT_NAMESPACE)]
	namespace: String,
	/// Override project from /tokens/api/inspect.
	#[arg(long)]
	project: Option<String>,
	/// Override organization from /tokens/api/inspect.
	#[arg(long)]
	org: Option<String>,
	/// Emit raw JSON instead of formatted output.
	#[arg(long)]
	json: bool,
	/// Cloud API endpoint.
	#[arg(long, default_value = DEFAULT_CLOUD_API)]
	cloud_api: String,
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		match self.command {
			Commands::List(opts) => opts.execute().await,
		}
	}
}

impl ListOpts {
	pub async fn execute(self) -> Result<()> {
		let token = resolve_token(self.token.as_deref())?;
		let cloud = CloudClient::new(&self.cloud_api, token)?;

		let inspect: TokenInspectResponse = cloud
			.request(Method::GET, "/tokens/api/inspect", None)
			.await?
			.context("token inspect returned no body")?;
		let project = self.project.clone().unwrap_or(inspect.project);
		let org = self.org.clone().unwrap_or(inspect.organization);
		let namespace = get_namespace(&cloud, &project, &org, &self.namespace).await?;

		let pools = list_pools(&cloud, &project, &org, &namespace.name).await?;

		if self.json {
			println!("{}", serde_json::to_string(&pools)?);
			return Ok(());
		}

		if pools.is_empty() {
			eprintln!("no compute pools in namespace {}", namespace.name);
			return Ok(());
		}

		let color = std::io::stdout().is_terminal();
		for pool in &pools {
			print_pool(pool, color);
		}
		Ok(())
	}
}

/// Prints a pool as a header line with its colored status followed by indented
/// config detail and, if present, its error in red.
fn print_pool(pool: &PoolSummary, color: bool) {
	let status = pool.status.as_deref().unwrap_or("unknown");
	println!("{}  {}", pool.name, paint(status, status_style(status), color));

	if let Some(config) = &pool.config {
		print_config(config);
	}

	if let Some(error) = &pool.error {
		let style = Style::new().fg_color(Some(AnsiColor::Red.into()));
		println!("  {:<10} {}", "error", paint(&error.message, style, color));
	}
}

fn print_config(config: &PoolConfig) {
	match &config.image {
		Some(image) => println!("  {:<10} {}:{}", "image", image.repository, image.tag),
		None => println!("  {:<10} none", "image"),
	}

	match &config.resources {
		Some(res) => {
			let mut parts = Vec::new();
			if let Some(cpu) = res.cpu {
				parts.push(format!("{cpu} vCPU"));
			}
			if let Some(memory) = &res.memory {
				parts.push(memory.clone());
			}
			if let (Some(min), Some(max)) = (res.min_scale, res.max_scale) {
				parts.push(format!("scale {min}-{max}"));
			}
			if let Some(concurrency) = res.instance_request_concurrency {
				parts.push(format!("concurrency {concurrency}"));
			}
			println!("  {:<10} {}", "resources", parts.join(" · "));
		}
		None => println!("  {:<10} defaults", "resources"),
	}

	let env_count = config.environment.len();
	if env_count > 0 {
		println!("  {:<10} {env_count}", "env vars");
	}
}

/// Maps a compute pool status to its display color. Unknown statuses render
/// without color.
fn status_style(status: &str) -> Style {
	let color = match status {
		"initializing" => Some(AnsiColor::BrightBlack),
		"allocating" | "destroying" => Some(AnsiColor::White),
		"deploying" | "binding" => Some(AnsiColor::Cyan),
		"ready" => Some(AnsiColor::Green),
		"error" => Some(AnsiColor::Red),
		_ => None,
	};
	Style::new().fg_color(color.map(Into::into))
}

fn paint(text: &str, style: Style, enabled: bool) -> String {
	if enabled {
		format!("{}{text}{}", style.render(), style.render_reset())
	} else {
		text.to_string()
	}
}
