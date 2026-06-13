use std::{
	collections::BTreeMap,
	env, fs,
	io::Write,
	path::{Path, PathBuf},
	process::{Command as StdCommand, Stdio},
	time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tokio::{process::Command, time::sleep};
use url::Url;

const DEFAULT_CLOUD_API: &str = "https://cloud-api.rivet.dev";
const DEFAULT_ENGINE_ENDPOINT: &str = "http://127.0.0.1:6420";
const DEFAULT_API_PEER_ENDPOINT: &str = "http://127.0.0.1:6421";
const DEFAULT_NAMESPACE: &str = "production";
const LOCAL_NAMESPACE: &str = "default";
const POOL_NAME: &str = "default";

#[derive(Parser)]
#[command(name = "rivet", version, about = "Rivet Cloud CLI")]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Run a local Rivet engine and wire it to a serverless handler.
	Dev(DevOpts),
	/// Build and deploy the current project to Rivet Cloud.
	Deploy(DeployOpts),
	/// Bootstrap the GitHub Actions deploy workflow.
	Init(InitOpts),
}

#[derive(Parser)]
struct DevOpts {
	/// Provider preset for the local serverless handler URL.
	#[arg(long, value_enum, default_value_t = Provider::Node)]
	provider: Provider,
	/// Supabase function name when --provider=supabase.
	#[arg(long)]
	fn_name: Option<String>,
	/// Explicit local serverless handler URL.
	#[arg(long)]
	url: Option<String>,
	/// Runner config name to register.
	#[arg(long, default_value = POOL_NAME)]
	runner: String,
	/// Local engine endpoint.
	#[arg(long, default_value = DEFAULT_ENGINE_ENDPOINT)]
	endpoint: String,
	/// Local api-peer endpoint. Used for health checks in some engine builds.
	#[arg(long, default_value = DEFAULT_API_PEER_ENDPOINT)]
	api_peer_endpoint: String,
	/// Path to a rivet-engine binary. Defaults to RIVET_ENGINE_BINARY or bundled binary.
	#[arg(long)]
	engine_binary: Option<PathBuf>,
	/// Remove the local engine database before starting.
	#[arg(long)]
	clean: bool,
}

#[derive(Copy, Clone, Eq, PartialEq, ValueEnum)]
enum Provider {
	Cloudflare,
	Supabase,
	Node,
}

#[derive(Parser)]
struct DeployOpts {
	/// Rivet Cloud API token. Also writes ~/.rivet/credentials for later commands.
	#[arg(long)]
	token: Option<String>,
	/// Cloud namespace to deploy to.
	#[arg(long, default_value = DEFAULT_NAMESPACE)]
	namespace: String,
	/// Override project from /tokens/api/inspect.
	#[arg(long)]
	project: Option<String>,
	/// Override organization from /tokens/api/inspect.
	#[arg(long)]
	org: Option<String>,
	/// Dockerfile to build.
	#[arg(long, default_value = "Dockerfile")]
	dockerfile: PathBuf,
	/// Docker build context.
	#[arg(long, default_value = ".")]
	build_context: PathBuf,
	/// Environment override, repeatable as KEY=VAL.
	#[arg(long = "env")]
	env_vars: Vec<String>,
	/// Skip prompts.
	#[arg(long)]
	yes: bool,
	/// Cloud API endpoint.
	#[arg(long, default_value = DEFAULT_CLOUD_API)]
	cloud_api: String,
	/// Image repository name in Rivet's registry. Defaults to the project slug.
	#[arg(long)]
	image: Option<String>,
	/// Image tag. Defaults to the current git short SHA, or a timestamp outside git.
	#[arg(long)]
	tag: Option<String>,
}

#[derive(Parser)]
struct InitOpts {
	/// Rivet Cloud API token to set as a GitHub secret when gh is available.
	#[arg(long)]
	token: Option<String>,
	/// Write files without prompting.
	#[arg(long)]
	yes: bool,
}

#[derive(Serialize, Deserialize)]
struct Credentials {
	rivet_cloud_token: String,
}

#[derive(Deserialize)]
struct TokenInspectResponse {
	project: String,
	organization: String,
}

#[derive(Deserialize)]
struct NamespaceResponse {
	namespace: Namespace,
}

#[derive(Deserialize)]
struct NamespacesResponse {
	namespaces: Vec<Namespace>,
	pagination: Option<Pagination>,
}

#[derive(Deserialize)]
struct Pagination {
	cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Namespace {
	name: String,
	display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedPoolResponse {
	managed_pool: Option<ManagedPool>,
}

#[derive(Deserialize)]
struct ManagedPool {
	status: Option<String>,
	error: Option<ManagedPoolError>,
}

#[derive(Deserialize)]
struct ManagedPoolError {
	message: Option<String>,
}

struct CloudClient {
	http: reqwest::Client,
	base: Url,
	token: String,
}

impl CloudClient {
	fn new(base: &str, token: String) -> Result<Self> {
		Ok(Self {
			http: reqwest::Client::new(),
			base: Url::parse(base).context("invalid Cloud API endpoint")?,
			token,
		})
	}

	async fn request<T: DeserializeOwned>(
		&self,
		method: Method,
		path: &str,
		body: Option<Value>,
	) -> Result<Option<T>> {
		let url = self.base.join(path.trim_start_matches('/'))?;
		let mut request = self
			.http
			.request(method, url)
			.bearer_auth(&self.token)
			.header("Content-Type", "application/json");
		if let Some(body) = body {
			request = request.json(&body);
		}
		let response = request.send().await.context("Cloud API request failed")?;
		if response.status() == StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let status = response.status();
		let text = response.text().await.unwrap_or_default();
		if !status.is_success() {
			bail!("Cloud API error {status}: {text}");
		}
		if text.trim().is_empty() {
			return Ok(None);
		}
		Ok(Some(serde_json::from_str(&text).with_context(|| {
			format!("Cloud API returned invalid JSON for {path}")
		})?))
	}

	async fn request_ok<T: DeserializeOwned>(
		&self,
		method: Method,
		path: &str,
		body: Option<Value>,
	) -> Result<Option<T>> {
		let url = self.base.join(path.trim_start_matches('/'))?;
		let mut request = self
			.http
			.request(method, url)
			.bearer_auth(&self.token)
			.header("Content-Type", "application/json");
		if let Some(body) = body {
			request = request.json(&body);
		}
		let response = request.send().await.context("Cloud API request failed")?;
		let status = response.status();
		let text = response.text().await.unwrap_or_default();
		if !status.is_success() {
			bail!("Cloud API error {status}: {text}");
		}
		if text.trim().is_empty() {
			return Ok(None);
		}
		Ok(Some(serde_json::from_str(&text).with_context(|| {
			format!("Cloud API returned invalid JSON for {path}")
		})?))
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();
	match cli.command {
		Commands::Dev(opts) => dev(opts).await,
		Commands::Deploy(opts) => deploy(opts).await,
		Commands::Init(opts) => init(opts).await,
	}
}

async fn dev(opts: DevOpts) -> Result<()> {
	let handler_url = opts.url.unwrap_or_else(|| match opts.provider {
		Provider::Cloudflare => "http://localhost:8787/api/rivet".to_string(),
		Provider::Supabase => format!(
			"http://localhost:54321/functions/v1/{}",
			opts.fn_name.as_deref().unwrap_or("rivetkit")
		),
		Provider::Node => "http://localhost:3000/api/rivet".to_string(),
	});

	let engine = opts
		.engine_binary
		.or_else(|| env::var_os("RIVET_ENGINE_BINARY").map(PathBuf::from))
		.unwrap_or_else(default_engine_binary);
	if !engine.exists() {
		bail!(
			"rivet-engine binary not found at {}. Set --engine-binary or RIVET_ENGINE_BINARY.",
			engine.display()
		);
	}

	if opts.clean {
		let db_path = default_engine_db_path();
		if db_path.exists() {
			eprintln!("removing {}", db_path.display());
			fs::remove_dir_all(&db_path)
				.with_context(|| format!("remove {}", db_path.display()))?;
		}
	}

	let config_path = write_dev_engine_config()?;
	eprintln!("starting local Rivet engine: {}", engine.display());
	let mut child = Command::new(&engine)
		.arg("--config")
		.arg(&config_path)
		.arg("start")
		.env(
			"RUST_LOG",
			env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
		)
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.spawn()
		.with_context(|| format!("spawn {}", engine.display()))?;

	wait_for_engine(
		&opts.endpoint,
		&opts.api_peer_endpoint,
		Duration::from_secs(45),
	)
	.await?;
	register_local_runner_config(&opts.endpoint, &opts.runner, &handler_url).await?;

	eprintln!("rivet dev ready");
	eprintln!("engine: {}", opts.endpoint);
	eprintln!("handler: {}", handler_url);
	eprintln!("press Ctrl-C to stop");

	tokio::select! {
		status = child.wait() => {
			let status = status.context("wait for rivet-engine")?;
			if !status.success() {
				bail!("rivet-engine exited with {status}");
			}
		}
		_ = tokio::signal::ctrl_c() => {
			eprintln!("stopping local Rivet engine");
			let _ = child.kill().await;
		}
	}

	Ok(())
}

async fn deploy(opts: DeployOpts) -> Result<()> {
	let token = resolve_token(opts.token.as_deref())?;
	if let Some(token) = &opts.token {
		write_credentials(token)?;
	}

	if !opts.dockerfile.exists() {
		bail!("Dockerfile not found: {}", opts.dockerfile.display());
	}

	let cloud = CloudClient::new(&opts.cloud_api, token.clone())?;
	eprintln!("inspecting Rivet Cloud token");
	let inspect: TokenInspectResponse = cloud
		.request(Method::GET, "/tokens/api/inspect", None)
		.await?
		.context("token inspect returned no body")?;
	let project = opts.project.unwrap_or(inspect.project);
	let organization = opts.org.unwrap_or(inspect.organization);
	let namespace = ensure_namespace(&cloud, &project, &organization, &opts.namespace).await?;

	let registry = registry_endpoint(&opts.cloud_api)?;
	let dashboard = dashboard_endpoint(&opts.cloud_api)?;
	let image_name = opts.image.unwrap_or_else(|| project.clone());
	let tag = opts.tag.unwrap_or_else(default_image_tag);
	let image_ref = format!("{registry}/{image_name}:{tag}");
	let dashboard_url = format!(
		"{dashboard}/orgs/{}/projects/{}/ns/{}?skipOnboarding=1",
		encode(&organization),
		encode(&project),
		encode(&namespace.name)
	);

	if !opts.yes {
		eprintln!(
			"deploying {} to project={} namespace={} image={}",
			opts.build_context.display(),
			project,
			namespace.name,
			image_ref
		);
	}

	eprintln!("enabling managed pool");
	create_or_update_pool(
		&cloud,
		&project,
		&organization,
		&namespace.name,
		json!({ "displayName": "Default" }),
	)
	.await?;
	wait_for_pool(&cloud, &project, &organization, &namespace.name, false).await?;

	eprintln!("logging in to Rivet registry");
	docker_login(&registry, &token)?;

	eprintln!("building Docker image");
	docker_build(&opts.build_context, &opts.dockerfile, &image_ref)?;

	eprintln!("pushing Docker image");
	run_command("docker", &["push", &image_ref], None)?;

	eprintln!("upserting managed pool");
	let mut pool_body = json!({
		"displayName": "Default",
		"maxConcurrentActors": 1000,
		"image": {
			"repository": image_name,
			"tag": tag,
		},
	});
	let env_map = parse_env_vars(&opts.env_vars)?;
	if !env_map.is_empty() {
		pool_body["environment"] = serde_json::to_value(env_map)?;
	}
	create_or_update_pool(&cloud, &project, &organization, &namespace.name, pool_body).await?;
	wait_for_pool(&cloud, &project, &organization, &namespace.name, true).await?;

	println!("{dashboard_url}");
	Ok(())
}

async fn init(opts: InitOpts) -> Result<()> {
	let workflow_path = PathBuf::from(".github/workflows/deploy.yml");
	if workflow_path.exists() && !opts.yes {
		bail!(
			"{} already exists; pass --yes to overwrite",
			workflow_path.display()
		);
	}
	fs::create_dir_all(workflow_path.parent().unwrap())?;
	fs::write(&workflow_path, deploy_workflow())?;
	eprintln!("wrote {}", workflow_path.display());

	let dockerfile = PathBuf::from("Dockerfile");
	if !dockerfile.exists() {
		fs::write(&dockerfile, default_dockerfile())?;
		eprintln!("wrote Dockerfile");
	}

	if let Some(token) = opts.token {
		if command_exists("gh") {
			let mut child = StdCommand::new("gh")
				.args(["secret", "set", "RIVET_CLOUD_TOKEN", "--body-file", "-"])
				.stdin(Stdio::piped())
				.stdout(Stdio::inherit())
				.stderr(Stdio::inherit())
				.spawn()
				.context("run gh secret set")?;
			child
				.stdin
				.as_mut()
				.context("gh stdin unavailable")?
				.write_all(token.as_bytes())?;
			let status = child.wait()?;
			if !status.success() {
				bail!("gh secret set failed with {status}");
			}
		} else {
			eprintln!("gh not found; run:");
			eprintln!("gh secret set RIVET_CLOUD_TOKEN --body '<token>'");
		}
	} else {
		eprintln!("set your GitHub secret with:");
		eprintln!("gh secret set RIVET_CLOUD_TOKEN");
	}

	Ok(())
}

async fn ensure_namespace(
	cloud: &CloudClient,
	project: &str,
	org: &str,
	namespace: &str,
) -> Result<Namespace> {
	let path = format!(
		"/projects/{}/namespaces/{}?org={}",
		encode(project),
		encode(namespace),
		encode(org)
	);
	if let Some(response) = cloud
		.request::<NamespaceResponse>(Method::GET, &path, None)
		.await?
	{
		return Ok(response.namespace);
	}

	let list_path = format!(
		"/projects/{}/namespaces?org={}&limit=100",
		encode(project),
		encode(org)
	);
	if let Some(response) = cloud
		.request::<NamespacesResponse>(Method::GET, &list_path, None)
		.await?
	{
		let _next_cursor = response.pagination.and_then(|p| p.cursor);
		if let Some(found) = response.namespaces.into_iter().find(|ns| {
			ns.name == namespace
				|| ns
					.display_name
					.as_ref()
					.is_some_and(|display| display.eq_ignore_ascii_case(namespace))
		}) {
			return Ok(found);
		}
	}

	eprintln!("creating namespace {namespace}");
	let create_path = format!(
		"/projects/{}/namespaces?org={}",
		encode(project),
		encode(org)
	);
	let response: NamespaceResponse = cloud
		.request(
			Method::POST,
			&create_path,
			Some(json!({ "displayName": namespace })),
		)
		.await?
		.context("namespace create returned no body")?;
	Ok(response.namespace)
}

async fn create_or_update_pool(
	cloud: &CloudClient,
	project: &str,
	org: &str,
	namespace: &str,
	body: Value,
) -> Result<()> {
	let path = format!(
		"/projects/{}/namespaces/{}/managed-pools/{}?org={}",
		encode(project),
		encode(namespace),
		POOL_NAME,
		encode(org)
	);
	let _: Option<Value> = cloud.request_ok(Method::PUT, &path, Some(body)).await?;
	Ok(())
}

async fn get_pool(
	cloud: &CloudClient,
	project: &str,
	org: &str,
	namespace: &str,
) -> Result<Option<ManagedPool>> {
	let path = format!(
		"/projects/{}/namespaces/{}/managed-pools/{}?org={}",
		encode(project),
		encode(namespace),
		POOL_NAME,
		encode(org)
	);
	Ok(cloud
		.request::<ManagedPoolResponse>(Method::GET, &path, None)
		.await?
		.and_then(|r| r.managed_pool))
}

async fn wait_for_pool(
	cloud: &CloudClient,
	project: &str,
	org: &str,
	namespace: &str,
	throw_on_error: bool,
) -> Result<()> {
	for _ in 0..180 {
		let pool = get_pool(cloud, project, org, namespace)
			.await?
			.context("managed pool disappeared while polling")?;
		let status = pool.status.unwrap_or_else(|| "unknown".to_string());
		eprintln!("pool status: {status}");
		match status.as_str() {
			"ready" => return Ok(()),
			"error" if throw_on_error => {
				bail!(
					"managed pool entered error state: {}",
					pool.error
						.and_then(|e| e.message)
						.unwrap_or_else(|| "unknown error".to_string())
				);
			}
			"error" => return Ok(()),
			_ => sleep(Duration::from_secs(2)).await,
		}
	}
	bail!("timed out waiting for managed pool to become ready")
}

async fn wait_for_engine(endpoint: &str, api_peer_endpoint: &str, timeout: Duration) -> Result<()> {
	let client = reqwest::Client::new();
	let deadline = tokio::time::Instant::now() + timeout;
	let urls = [
		format!("{endpoint}/health"),
		format!("{endpoint}/metadata"),
		format!("{api_peer_endpoint}/health"),
	];
	while tokio::time::Instant::now() < deadline {
		for url in &urls {
			if let Ok(response) = client.get(url).send().await {
				if response.status().is_success() {
					return Ok(());
				}
			}
		}
		sleep(Duration::from_millis(500)).await;
	}
	bail!("timed out waiting for local engine at {endpoint}")
}

async fn register_local_runner_config(
	endpoint: &str,
	runner: &str,
	handler_url: &str,
) -> Result<()> {
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
					"request_lifespan": 900,
					"slots_per_runner": 100,
					"min_runners": 0,
					"max_runners": 3,
					"runners_margin": 1,
					"metadata_poll_interval": 1,
					"drain_grace_period": 5,
					"actor_eviction_period": 30,
					"actor_eviction_rate": 100
				}
			}
		}
	});
	let response = reqwest::Client::new()
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

fn resolve_token(flag: Option<&str>) -> Result<String> {
	if let Some(token) = flag {
		return Ok(token.to_string());
	}
	if let Ok(token) = env::var("RIVET_CLOUD_TOKEN") {
		if !token.trim().is_empty() {
			return Ok(token);
		}
	}
	let path = credentials_path()?;
	if path.exists() {
		let credentials: Credentials = serde_json::from_str(&fs::read_to_string(&path)?)?;
		if !credentials.rivet_cloud_token.trim().is_empty() {
			return Ok(credentials.rivet_cloud_token);
		}
	}
	bail!("missing Rivet Cloud token; pass --token or set RIVET_CLOUD_TOKEN")
}

fn write_credentials(token: &str) -> Result<()> {
	let path = credentials_path()?;
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}
	let contents = serde_json::to_string_pretty(&Credentials {
		rivet_cloud_token: token.to_string(),
	})?;
	write_secret_file(&path, contents.as_bytes())?;
	Ok(())
}

#[cfg(unix)]
fn write_secret_file(path: &Path, contents: &[u8]) -> Result<()> {
	let mut file = OpenOptions::new()
		.create(true)
		.write(true)
		.truncate(true)
		.mode(0o600)
		.open(path)
		.with_context(|| format!("open {}", path.display()))?;
	file.write_all(contents)?;
	file.sync_all()?;
	let mut perms = file.metadata()?.permissions();
	use std::os::unix::fs::PermissionsExt;
	perms.set_mode(0o600);
	fs::set_permissions(path, perms)?;
	Ok(())
}

#[cfg(not(unix))]
fn write_secret_file(path: &Path, contents: &[u8]) -> Result<()> {
	fs::write(path, contents)?;
	Ok(())
}

fn credentials_path() -> Result<PathBuf> {
	Ok(dirs::home_dir()
		.context("could not resolve home directory")?
		.join(".rivet")
		.join("credentials"))
}

fn default_engine_binary() -> PathBuf {
	let exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("rivet"));
	let name = if cfg!(windows) {
		"rivet-engine.exe"
	} else {
		"rivet-engine"
	};
	exe.parent().unwrap_or_else(|| Path::new(".")).join(name)
}

fn default_engine_db_path() -> PathBuf {
	dirs::data_local_dir()
		.map(|dir| dir.join("rivet-engine").join("db"))
		.unwrap_or_else(|| PathBuf::from("./data/db"))
}

fn write_dev_engine_config() -> Result<PathBuf> {
	let dir = dirs::data_local_dir()
		.unwrap_or_else(|| PathBuf::from("."))
		.join("rivet")
		.join("cli");
	fs::create_dir_all(&dir)?;
	let path = dir.join("dev-engine.json");
	let db_path = default_engine_db_path();
	let config = json!({
		"file_system": { "path": db_path },
		"guard": { "port": 6420 },
		"api_peer": { "port": 6421 },
		"metrics": { "port": 6430 },
		"telemetry": { "enabled": false }
	});
	fs::write(&path, serde_json::to_string_pretty(&config)?)?;
	Ok(path)
}

fn parse_env_vars(vars: &[String]) -> Result<BTreeMap<String, String>> {
	let mut map = BTreeMap::new();
	for var in vars {
		let Some((key, value)) = var.split_once('=') else {
			bail!("--env must be KEY=VAL, got {var}");
		};
		if key.is_empty() {
			bail!("--env key cannot be empty");
		}
		map.insert(key.to_string(), value.to_string());
	}
	Ok(map)
}

fn registry_endpoint(cloud_api: &str) -> Result<String> {
	derive_endpoint(cloud_api, "registry")
}

fn dashboard_endpoint(cloud_api: &str) -> Result<String> {
	derive_endpoint(cloud_api, "dashboard")
}

fn derive_endpoint(input: &str, subdomain: &str) -> Result<String> {
	let mut url = Url::parse(input)?;
	let host = url.host_str().context("endpoint missing host")?;
	let next_host = if let Some(rest) = host.strip_prefix("cloud-api.") {
		format!("{subdomain}.{rest}")
	} else if let Some(rest) = host.strip_prefix("api.") {
		format!("{subdomain}.{rest}")
	} else {
		format!("{subdomain}.{host}")
	};
	url.set_host(Some(&next_host))?;
	url.set_path("");
	url.set_query(None);
	url.set_fragment(None);
	Ok(url.as_str().trim_end_matches('/').to_string())
}

fn default_image_tag() -> String {
	if let Ok(output) = StdCommand::new("git")
		.args(["rev-parse", "--short=7", "HEAD"])
		.stderr(Stdio::null())
		.output()
	{
		if output.status.success() {
			let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
			if !tag.is_empty() {
				return tag;
			}
		}
	}
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_secs()
		.to_string()
}

fn docker_login(registry: &str, token: &str) -> Result<()> {
	let mut child = StdCommand::new("docker")
		.args(["login", registry, "--username", "rivet", "--password-stdin"])
		.stdin(Stdio::piped())
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.spawn()
		.context("docker login")?;
	child
		.stdin
		.as_mut()
		.context("docker login stdin unavailable")?
		.write_all(token.as_bytes())?;
	let status = child.wait()?;
	if !status.success() {
		bail!("docker login failed with {status}");
	}
	Ok(())
}

fn docker_build(context: &Path, dockerfile: &Path, image_ref: &str) -> Result<()> {
	let context_str = context.to_string_lossy();
	let dockerfile_str = dockerfile.to_string_lossy();
	run_command(
		"docker",
		&[
			"buildx",
			"build",
			"--platform",
			"linux/amd64",
			"--load",
			&context_str,
			"-f",
			&dockerfile_str,
			"-t",
			image_ref,
		],
		None,
	)
}

fn run_command(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<()> {
	eprintln!("$ {} {}", program, args.join(" "));
	let mut command = StdCommand::new(program);
	command
		.args(args)
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit());
	if let Some(cwd) = cwd {
		command.current_dir(cwd);
	}
	let status = command.status().with_context(|| format!("run {program}"))?;
	if !status.success() {
		bail!("{program} failed with {status}");
	}
	Ok(())
}

fn command_exists(program: &str) -> bool {
	StdCommand::new(program)
		.arg("--version")
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status()
		.is_ok_and(|status| status.success())
}

fn encode(value: &str) -> String {
	url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn deploy_workflow() -> &'static str {
	r#"name: Rivet Deploy

on:
  push:
    branches: [main]
  workflow_dispatch:

jobs:
  rivet-deploy:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write
    steps:
      - uses: actions/checkout@v4
      - uses: rivet-dev/deploy-action@v1.1.2
        with:
          rivet-token: ${{ secrets.RIVET_CLOUD_TOKEN }}
"#
}

fn default_dockerfile() -> &'static str {
	r#"FROM node:22-slim
WORKDIR /app
COPY package.json pnpm-lock.yaml* ./
RUN corepack enable && pnpm install --frozen-lockfile
COPY . .
RUN pnpm build
CMD ["pnpm", "start"]
"#
}
