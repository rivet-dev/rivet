#![recursion_limit = "256"]

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use rivet_env::Env;

use anyhow::{Context, Result};
use clap::Parser;
// use once_cell::sync::Lazy;

#[cfg(target_os = "linux")]
mod allocator;
mod depot_command;
mod depot_peer;
mod epoxy_peer;
mod epoxy_services;
mod host_metrics;
mod object_store_canary;
mod peer;
mod run_config;
mod services;

// static LONG_VERSION: Lazy<String> = Lazy::new(|| rivet_build_meta::pretty_print());

#[derive(Parser)]
// #[command(name = "Rivet", version, long_version = LONG_VERSION.as_str(), about)]
#[command(name = "Rivet", version, about)]
struct Cli {
	/// Print machine-readable runtime identity without loading configuration or starting services.
	#[arg(long)]
	runtime_info: bool,
	#[command(subcommand)]
	command: Option<SubCommand>,
	/// Path to the config file or directory of config files
	#[clap(long, global = true)]
	config: Vec<PathBuf>,
}

#[derive(clap::Subcommand)]
enum SubCommand {
	#[command(subcommand)]
	#[command(name = "storage")]
	Depot(depot_command::Command),
	#[command(subcommand)]
	Auth(AuthCommand),
	/// Probe the configured object storage with heartbeat-sized requests and export their
	/// latency and stall counts on the metrics port, without starting any engine service.
	ObjectStoreCanary(object_store_canary::Args),
}

#[derive(clap::Subcommand)]
enum AuthCommand {
	/// Inspect JWT signing keys or request an emergency rotation.
	#[command(subcommand)]
	Jwt(depot_command::jwt::Command),
}

fn main() -> Result<()> {
	let cli = Cli::parse();
	if cli.runtime_info {
		println!(
			"{}",
			serde_json::json!({
				"runtime": rivet_guard::ENGINE_RUNTIME,
				"architecture": rivet_guard::ENGINE_ARCHITECTURE,
				"version": env!("CARGO_PKG_VERSION"),
			})
		);
		return Ok(());
	}
	// The storage directory's lock proves to this worker's next process that this one has
	// exited, so it is released only after the runtime has dropped every task, never while
	// detached cleanup could still touch storage. See `StorageDirectory`.
	let retained_directory = Arc::new(OnceLock::new());
	let result = rivet_runtime::run(main_inner(cli, retained_directory.clone()));
	drop(retained_directory);
	result
}

async fn main_inner(
	cli: Cli,
	retained_directory: Arc<OnceLock<Arc<depot_worker::bootstrap::StorageDirectory>>>,
) -> Result<()> {
	// tracing::info!(
	// 	version=%rivet_build_meta::VERSION,
	// 	git_sha=%rivet_build_meta::GIT_SHA,
	// 	built_at=%rivet_build_meta::BUILD_TIMESTAMP,
	// 	"starting rivet",
	// );
	tracing::info!(version = env!("CARGO_PKG_VERSION"), "starting rivet");

	// Load config
	let config = rivet_config::Config::load(
		&cli.config,
		rivet_build_meta::build_meta(),
		rivet_build_meta::compiled_runtime_protocols(),
	)
	.await?;
	tracing::info!(config=?*config, "loaded config");

	let env = Arc::new(rivet_env::ProductionEnv::new()?);
	#[cfg(target_os = "linux")]
	allocator::register_metrics()?;
	#[cfg(all(target_os = "linux", feature = "heap-profile"))]
	let heap_dumps = {
		let scope = (env.clone() as Arc<dyn Env>).task_scope();
		allocator::listen_for_heap_dumps(&scope)?;
		scope
	};
	let (shutdown_trigger, shutdown) = rivet_env::capabilities::shutdown::Shutdown::channel();
	let signals =
		rivet_env::backends::production::signals::listen(env.clone(), shutdown_trigger.clone())?;
	let result = rivet_service_manager::with_shutdown(
		env.clone(),
		shutdown.clone(),
		config.runtime.force_shutdown_duration(),
		async {
			if let Some(SubCommand::ObjectStoreCanary(args)) = &cli.command {
				let _guard = rivet_telemetry::init(&config);
				return services::object_store_canary::start(
					env.clone(),
					config.clone(),
					args.clone(),
					shutdown.clone(),
				)
				.await;
			}

			let object_stores = rivet_engine_context::create_object_stores(&config)?;
			rivet_cluster_config::load(env.clone(), object_stores.primary.clone(), &config).await?;

			// Initialize telemetry (does nothing if telemetry is disabled)
			let _guard = rivet_telemetry::init(&config);
			// Choose identity from the selected role before constructing worker-addressed clients.
			let mut services = run_config::config(config.clone())?.services;
			let operator_command = select_command_services(cli.command.as_ref(), &mut services);
			let storage_role = run_config::depot_role(&services)?
				== Some(rivet_topology::worker::WorkerRole::Storage);

			// A storage worker sizes its log and disk cache from free disk before opening either.
			let disk_budget = if storage_role {
				depot_worker::disk_budget::resolve(&env.fs(), config.depot()).await?
			} else {
				None
			};

			// Reuse the stores created before EngineContext for cluster-config and dynamic-config validation.
			let adapters = rivet_engine_context::production::storage_adapters(
				&config,
				env.clone(),
				object_stores,
				disk_budget
					.as_ref()
					.and_then(|budget| budget.cache.as_ref()),
			)
			.await?;
			let command = match cli.command {
				Some(SubCommand::Depot(command)) => {
					Some(depot_command::OperatorCommand::Storage(command))
				}
				Some(SubCommand::Auth(AuthCommand::Jwt(command))) => {
					Some(depot_command::OperatorCommand::Jwt(command))
				}
				Some(SubCommand::ObjectStoreCanary(_)) | None => None,
			};
			let command_result = if let Some(command) = command {
				// Operator processes are stateless; they never open a storage directory.
				let (service, result) = depot_command::service(command, shutdown_trigger);
				services.push(service);
				Some(result)
			} else {
				None
			};
			let storage = if storage_role {
				let depot = config.depot();
				let root = depot
					.log_directory
					.as_ref()
					.context("storage services require storage.log_directory")?;
				let mut log_shard = depot.log_shard.clone();
				if let Some(budget) = &disk_budget {
					log_shard.max_disk_bytes = Some(budget.log_disk_bytes);
				}
				let limits = depot_worker::bootstrap::log_shard_config(&log_shard)?;
				let directory = Arc::new(
					depot_worker::bootstrap::StorageDirectory::open(env.clone(), root, &limits)
						.await?,
				);
				// A process opens at most one storage directory.
				let _ = retained_directory.set(directory.clone());
				Some(directory)
			} else {
				None
			};
			let identity = match &storage {
				Some(storage) => storage.identity.clone(),
				None => rivet_topology::worker::WorkerIdentity::new(&mut env.rng("depot.identity")),
			};

			let ctx = rivet_engine_context::EngineContext::new_with_identity(
				env,
				shutdown,
				config.clone(),
				adapters,
				identity,
			)
			.await?;

			// Peer requests and membership reconciliation share the elected regional owner.
			let epoxy = Arc::new(epoxy_services::EpoxyServices::new(ctx.clone()));
			if let Some(peer) = services
				.iter_mut()
				.find(|service| service.name == "api_peer")
			{
				let host = epoxy.clone();
				peer.run = Arc::new(move |config, ctx| {
					Box::pin(peer::serve(config, ctx, Some(host.clone())))
				});
			}

			// Execute command
			// cli.command
			// 	.execute(config, run_config)
			// 	.await
			// 	.inspect_err(|err| {
			// 		rivet_telemetry::capture_error(err);
			// 	})
			let result = run_config::start(
				config,
				ctx,
				services,
				storage.clone(),
				Some(epoxy),
				!operator_command,
			)
			.await
			.inspect_err(|err| {
				rivet_telemetry::capture_error(err);
			});
			// The directory cannot be reused until its services have stopped.
			drop(storage);
			result?;
			if let Some(result) = command_result {
				let output = result.await.context("operator command did not finish")??;
				print_command_output(&output)?;
			}
			Ok(())
		},
	)
	.await;
	signals.shutdown().await?;
	#[cfg(all(target_os = "linux", feature = "heap-profile"))]
	heap_dumps.shutdown().await;
	result
}

/// Operator output is a machine-readable result on stdout, separate from diagnostic tracing.
#[allow(
	clippy::disallowed_macros,
	reason = "the CLI result is explicitly requested stdout output"
)]
fn print_command_output(output: &serde_json::Value) -> Result<()> {
	println!("{}", serde_json::to_string_pretty(output)?);
	Ok(())
}

/// Operator CLI processes keep only API-peer transport, select the stateless role and skip
/// storage sizing/directory ownership and host metrics before constructing clients.
fn select_command_services(
	command: Option<&SubCommand>,
	services: &mut Vec<rivet_service_manager::Service>,
) -> bool {
	let operator = matches!(command, Some(SubCommand::Depot(_) | SubCommand::Auth(_)));
	if operator {
		services.retain(|service| service.name == "api_peer");
	}
	operator
}
