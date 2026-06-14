use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use reqwest::Client;
use rivetkit_engine_process::EngineProcessManager;
use serde_json::json;
use tokio::process::{Child, Command};

use crate::{
	DEFAULT_ENGINE_ENDPOINT, LOCAL_NAMESPACE, POOL_NAME, SUPABASE_FN_DEFAULT,
	engine_runner::engine_config, util::encode,
};

const HANDLER_METADATA_TIMEOUT: Duration = Duration::from_secs(30);
const HANDLER_METADATA_RETRY: Duration = Duration::from_millis(200);
const HANDLER_METADATA_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Parser)]
pub struct Opts {
	/// Serverless platform preset. Omit to run a custom dev server you point at
	/// with --port or --url.
	#[arg(long, value_enum)]
	provider: Option<Provider>,
	/// Handler port. Required in the default (no provider) mode unless --url is
	/// set. Overrides the provider's default port.
	#[arg(long)]
	port: Option<u16>,
	/// Supabase function name when --provider=supabase.
	#[arg(long, default_value = SUPABASE_FN_DEFAULT)]
	fn_name: String,
	/// Explicit full handler URL. Overrides port and path construction.
	#[arg(long)]
	url: Option<String>,
	/// Path to a rivet-engine binary. Defaults to RIVET_ENGINE_BINARY_PATH, a
	/// binary next to this CLI, a local build, or an auto-downloaded release.
	#[arg(long)]
	engine_binary: Option<PathBuf>,
	/// Dev server command to spawn. Everything after `--`.
	#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
	command: Vec<String>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum Provider {
	/// Generic serverless handler. The CLI assigns a free port and passes it as
	/// the PORT environment variable.
	Serverless,
	Cloudflare,
	Supabase,
	/// Run only the engine, do not spawn a handler.
	None,
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		let mut config = engine_config(self.engine_binary.clone());
		config.public_url = resolve_engine_public_url(&self)?;
		if self.provider == Some(Provider::Supabase) {
			config.bind_host = Some("0.0.0.0".to_string());
		}

		// Engine-only mode: start (or reuse) the engine and wait.
		if matches!(self.provider, Some(Provider::None)) {
			let _engine = EngineProcessManager::start_or_reuse(config).await?;
			tracing::info!(
				engine = DEFAULT_ENGINE_ENDPOINT,
				"engine ready (no handler); press Ctrl-C to stop"
			);
			tokio::signal::ctrl_c().await.context("listen for ctrl-c")?;
			return Ok(());
		}

		let plan = HandlerPlan::resolve(&self)?;

		// Start (or reuse) the engine. The engine is intentionally orphaned, so
		// it survives this process and a later `rivet dev` reattaches to it.
		let _engine = EngineProcessManager::start_or_reuse(config).await?;

		let mut child = plan.spawn()?;

		tokio::select! {
			result = wait_for_handler_metadata(&plan.handler_url) => {
				if let Err(err) = result {
					let _ = child.kill().await;
					return Err(err);
				}
			}
			status = child.wait() => {
				let status = status.context("wait for dev server")?;
				bail!("dev server exited before the Rivet handler became ready: {status}");
			}
		}

		if let Err(err) =
			register_runner_config(DEFAULT_ENGINE_ENDPOINT, POOL_NAME, &plan.handler_url).await
		{
			let _ = child.kill().await;
			return Err(err);
		}

		tracing::info!(
			engine = DEFAULT_ENGINE_ENDPOINT,
			handler = %plan.handler_url,
			"rivet dev ready; press Ctrl-C to stop"
		);

		tokio::select! {
			status = child.wait() => {
				let status = status.context("wait for dev server")?;
				if !status.success() {
					bail!("dev server exited with {status}");
				}
			}
			_ = tokio::signal::ctrl_c() => {
				tracing::info!(
					"stopping dev server (engine keeps running; use `rivet engine` to manage it)"
				);
				let _ = child.kill().await;
			}
		}

		Ok(())
	}
}

/// Resolved spawn plan for the dev server: where it listens, the command to
/// run, and any environment the CLI injects.
#[derive(Debug)]
struct HandlerPlan {
	handler_url: String,
	program: String,
	args: Vec<String>,
	env: Vec<(String, String)>,
}

impl HandlerPlan {
	fn resolve(opts: &Opts) -> Result<Self> {
		let provider = opts.provider;
		let port = resolve_port(provider, opts.port, opts.url.is_some())?;
		let handler_url = match &opts.url {
			Some(url) => url.clone(),
			None => build_handler_url(provider, &opts.fn_name, port),
		};

		let (program, args, env) = match provider {
			Some(Provider::Cloudflare) => {
				let mut args = vec![
					"wrangler".to_string(),
					"dev".to_string(),
					"--port".to_string(),
					port.to_string(),
					// Inject the local engine endpoint as a Worker variable so the
					// handler connects back without any wrangler.toml config. With
					// `nodejs_compat` this also lands in `process.env`. wrangler
					// splits `--var` on the first colon, so the URL stays intact.
					"--var".to_string(),
					format!("RIVET_ENDPOINT:{DEFAULT_ENGINE_ENDPOINT}"),
				];
				args.extend(opts.command.iter().cloned());
				("npx".to_string(), args, Vec::new())
			}
			Some(Provider::Supabase) => {
				let mut args = vec![
					"supabase".to_string(),
					"functions".to_string(),
					"serve".to_string(),
					opts.fn_name.clone(),
					"--no-verify-jwt".to_string(),
				];
				args.extend(opts.command.iter().cloned());
				("npx".to_string(), args, Vec::new())
			}
			Some(Provider::Serverless) => {
				let (program, args) = split_command(&opts.command)?;
				// Serverless handlers learn their port from the PORT env var.
				(program, args, vec![("PORT".to_string(), port.to_string())])
			}
			// Default (no provider): spawn the user's command verbatim.
			None => {
				let (program, args) = split_command(&opts.command)?;
				(program, args, Vec::new())
			}
			Some(Provider::None) => unreachable!("engine-only mode handled before resolve"),
		};

		Ok(Self {
			handler_url,
			program,
			args,
			env,
		})
	}

	fn spawn(&self) -> Result<Child> {
		let mut command = Command::new(&self.program);
		command.args(&self.args);
		for (key, value) in &self.env {
			command.env(key, value);
		}
		command
			.spawn()
			.with_context(|| format!("spawn dev server `{}`", self.program))
	}
}

/// Resolves the handler port for the given provider. Returns an error in the
/// default mode when neither a port nor an explicit URL is provided.
fn resolve_port(provider: Option<Provider>, port: Option<u16>, has_url: bool) -> Result<u16> {
	match provider {
		Some(Provider::Cloudflare) => Ok(port.unwrap_or(8787)),
		Some(Provider::Supabase) => Ok(port.unwrap_or(54321)),
		Some(Provider::Serverless) => match port {
			Some(port) => Ok(port),
			None => pick_free_port(),
		},
		// Default mode: the port is not managed by the CLI, so it must be
		// provided so the runner can be registered. `0` is a sentinel that
		// callers only reach when --url is set (and the port is unused).
		None if has_url => Ok(port.unwrap_or(0)),
		None => port.context("provide --port (or --url) for the default dev server mode"),
		Some(Provider::None) => unreachable!("engine-only mode handled before resolve"),
	}
}

fn build_handler_url(provider: Option<Provider>, fn_name: &str, port: u16) -> String {
	match provider {
		Some(Provider::Supabase) => {
			format!("http://127.0.0.1:{port}/functions/v1/{fn_name}/api/rivet")
		}
		_ => format!("http://127.0.0.1:{port}/api/rivet"),
	}
}

fn split_command(command: &[String]) -> Result<(String, Vec<String>)> {
	let Some((program, args)) = command.split_first() else {
		bail!(
			"provide a dev server command after `--` (for example `rivet dev -- npm run dev`), \
			 or use `--provider none` to run only the engine"
		);
	};
	Ok((program.clone(), args.to_vec()))
}

fn resolve_engine_public_url(opts: &Opts) -> Result<Option<String>> {
	if opts.provider != Some(Provider::Supabase) {
		return Ok(None);
	}

	if let Some(endpoint) = read_env_value("RIVET_ENDPOINT") {
		return Ok(Some(endpoint));
	}

	if let Some(env_file) = supabase_env_file(&opts.command) {
		if let Some(endpoint) = read_dotenv_value(&env_file, "RIVET_ENDPOINT") {
			return Ok(Some(endpoint));
		}
	}

	if let Some(endpoint) = read_dotenv_value(Path::new(".env.local"), "RIVET_ENDPOINT") {
		return Ok(Some(endpoint));
	}

	Ok(None)
}

fn supabase_env_file(command: &[String]) -> Option<PathBuf> {
	command
		.windows(2)
		.find_map(|window| (window[0] == "--env-file").then(|| PathBuf::from(&window[1])))
}

fn read_env_value(key: &str) -> Option<String> {
	std::env::var(key)
		.ok()
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty())
}

fn read_dotenv_value(path: &Path, key: &str) -> Option<String> {
	let contents = std::fs::read_to_string(path).ok()?;
	contents.lines().find_map(|line| {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			return None;
		}
		let (name, value) = line.split_once('=')?;
		if name.trim() != key {
			return None;
		}
		Some(strip_env_quotes(value.trim()).to_string())
	})
}

fn strip_env_quotes(value: &str) -> &str {
	if value.len() >= 2
		&& ((value.starts_with('"') && value.ends_with('"'))
			|| (value.starts_with('\'') && value.ends_with('\'')))
	{
		&value[1..value.len() - 1]
	} else {
		value
	}
}

/// Allocates a free TCP port for the serverless handler. There is a small
/// window between picking the port and the handler binding it, which is
/// acceptable for local development.
fn pick_free_port() -> Result<u16> {
	let listener = std::net::TcpListener::bind("127.0.0.1:0")
		.context("allocate a free port for the serverless handler")?;
	Ok(listener.local_addr().context("read allocated port")?.port())
}

async fn register_runner_config(endpoint: &str, runner: &str, handler_url: &str) -> Result<()> {
	let url = format!(
		"{}/runner-configs/{}?namespace={}",
		endpoint.trim_end_matches('/'),
		encode(runner),
		LOCAL_NAMESPACE
	);
	let body = json!({
		"datacenters": {
			"default": {
				"serverless": {
					"url": handler_url,
					"headers": {},
					"request_lifespan": 3600,
					"slots_per_runner": 1,
					"min_runners": 0,
					"max_runners": 100000,
					"runners_margin": 0,
					"metadata_poll_interval": 1000
				}
			}
		}
	});
	let response = Client::new()
		.put(url)
		.header("Content-Type", "application/json")
		.bearer_auth("dev")
		.json(&body)
		.send()
		.await
		.context("register local runner config")?;
	if !response.status().is_success() {
		let status = response.status();
		let text = response.text().await.unwrap_or_default();
		bail!("runner config update failed: {status}: {text}");
	}
	Ok(())
}

async fn wait_for_handler_metadata(handler_url: &str) -> Result<()> {
	let metadata_url = format!("{}/metadata", handler_url.trim_end_matches('/'));
	let client = Client::new();
	let deadline = Instant::now() + HANDLER_METADATA_TIMEOUT;
	let mut last_error: Option<String> = None;

	loop {
		if Instant::now() >= deadline {
			bail!(
				"Rivet handler metadata did not become ready at {metadata_url} within {}s (last error: {})",
				HANDLER_METADATA_TIMEOUT.as_secs(),
				last_error.as_deref().unwrap_or("no request attempted")
			);
		}

		match client
			.get(&metadata_url)
			.timeout(HANDLER_METADATA_REQUEST_TIMEOUT)
			.send()
			.await
		{
			Ok(response) if response.status().is_success() => return Ok(()),
			Ok(response) => {
				let status = response.status();
				let body = response.text().await.unwrap_or_default();
				last_error = Some(format!("HTTP {status}: {body}"));
			}
			Err(err) => {
				last_error = Some(err.to_string());
			}
		}

		tokio::time::sleep(HANDLER_METADATA_RETRY).await;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn opts(provider: Option<Provider>) -> Opts {
		Opts {
			provider,
			port: None,
			fn_name: SUPABASE_FN_DEFAULT.to_string(),
			url: None,
			engine_binary: None,
			command: Vec::new(),
		}
	}

	#[test]
	fn cloudflare_provider_uses_default_port_and_wrangler_command() {
		let plan = HandlerPlan::resolve(&opts(Some(Provider::Cloudflare))).unwrap();

		assert_eq!(plan.handler_url, "http://127.0.0.1:8787/api/rivet");
		assert_eq!(plan.program, "npx");
		assert_eq!(
			plan.args,
			[
				"wrangler",
				"dev",
				"--port",
				"8787",
				"--var",
				"RIVET_ENDPOINT:http://127.0.0.1:6420"
			]
		);
		assert!(plan.env.is_empty());
	}

	#[test]
	fn cloudflare_provider_allows_custom_port_and_appended_args() {
		let mut opts = opts(Some(Provider::Cloudflare));
		opts.port = Some(8788);
		opts.command = vec!["--local-protocol".into(), "http".into()];

		let plan = HandlerPlan::resolve(&opts).unwrap();

		assert_eq!(plan.handler_url, "http://127.0.0.1:8788/api/rivet");
		assert_eq!(
			plan.args,
			[
				"wrangler",
				"dev",
				"--port",
				"8788",
				"--var",
				"RIVET_ENDPOINT:http://127.0.0.1:6420",
				"--local-protocol",
				"http"
			]
		);
	}

	#[test]
	fn supabase_provider_uses_default_port_function_and_no_verify_jwt() {
		let plan = HandlerPlan::resolve(&opts(Some(Provider::Supabase))).unwrap();

		assert_eq!(
			plan.handler_url,
			"http://127.0.0.1:54321/functions/v1/rivet/api/rivet"
		);
		assert_eq!(plan.program, "npx");
		assert_eq!(
			plan.args,
			["supabase", "functions", "serve", "rivet", "--no-verify-jwt"]
		);
		assert!(plan.env.is_empty());
	}

	#[test]
	fn supabase_provider_allows_custom_function_port_and_appended_args() {
		let mut opts = opts(Some(Provider::Supabase));
		opts.port = Some(4000);
		opts.fn_name = "actors".into();
		opts.command = vec!["--env-file".into(), ".env.local".into()];

		let plan = HandlerPlan::resolve(&opts).unwrap();

		assert_eq!(
			plan.handler_url,
			"http://127.0.0.1:4000/functions/v1/actors/api/rivet"
		);
		assert_eq!(
			plan.args,
			[
				"supabase",
				"functions",
				"serve",
				"actors",
				"--no-verify-jwt",
				"--env-file",
				".env.local"
			]
		);
	}

	#[test]
	fn serverless_provider_injects_port_env_for_command() {
		let mut opts = opts(Some(Provider::Serverless));
		opts.port = Some(3001);
		opts.command = vec!["node".into(), "handler.js".into()];

		let plan = HandlerPlan::resolve(&opts).unwrap();

		assert_eq!(plan.handler_url, "http://127.0.0.1:3001/api/rivet");
		assert_eq!(plan.program, "node");
		assert_eq!(plan.args, ["handler.js"]);
		assert_eq!(plan.env, [("PORT".to_string(), "3001".to_string())]);
	}

	#[test]
	fn default_mode_requires_port_or_url() {
		let mut opts = opts(None);
		opts.command = vec!["npm".into(), "run".into(), "dev".into()];

		let error = HandlerPlan::resolve(&opts).unwrap_err().to_string();

		assert!(error.contains("provide --port"));
	}

	#[test]
	fn explicit_url_overrides_handler_url() {
		let mut opts = opts(None);
		opts.url = Some("http://127.0.0.1:9000/custom".into());
		opts.command = vec!["npm".into(), "run".into(), "dev".into()];

		let plan = HandlerPlan::resolve(&opts).unwrap();

		assert_eq!(plan.handler_url, "http://127.0.0.1:9000/custom");
		assert_eq!(plan.program, "npm");
		assert_eq!(plan.args, ["run", "dev"]);
	}

	#[test]
	fn supabase_public_engine_url_reads_passed_env_file() {
		let temp = tempfile::tempdir().unwrap();
		let env_path = temp.path().join(".env.local");
		std::fs::write(
			&env_path,
			"RIVET_ENDPOINT=\"http://host.docker.internal:6420\"\n",
		)
		.unwrap();
		let mut opts = opts(Some(Provider::Supabase));
		opts.command = vec!["--env-file".into(), env_path.to_string_lossy().into_owned()];

		let public_url = resolve_engine_public_url(&opts).unwrap();

		assert_eq!(
			public_url,
			Some("http://host.docker.internal:6420".to_string())
		);
	}
}
