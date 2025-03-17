use std::{
	fs::File,
	io::{BufReader, Write},
	net::Ipv4Addr,
	os::fd::FromRawFd,
	path::{Path, PathBuf},
	rc::Rc,
	result::Result::{Err, Ok},
	sync::{mpsc as smpsc, Arc},
	thread::JoinHandle,
};

use anyhow::*;
use deno_core::{
	error::JsError, v8, v8::CreateParams, ModuleId, ModuleSpecifier, StaticModuleLoader,
};
use deno_runtime::{
	deno_fs::InMemoryFs,
	deno_io::{Stdio, StdioPipe},
	deno_permissions::{
		self, NetListenDescriptor, Permissions, PermissionsContainer, UnaryPermission,
	},
	permissions::RuntimePermissionDescriptorParser,
	worker::{MainWorker, MainWorkerTerminateHandle, WorkerOptions, WorkerServiceOptions},
};
use nix::{libc, unistd::pipe};
use pegboard::protocol;
use pegboard_actor_kv::ActorKv;
use pegboard_config::isolate_runner as config;
use tokio::{fs, sync::mpsc};
use utils::FdbPool;
use uuid::Uuid;

use crate::{ext, log_shipper, metadata::JsMetadata, utils};

pub fn run(
	config: config::Config,
	fdb_pool: FdbPool,
	actor_id: Uuid,
	generation: u32,
	terminate_tx: mpsc::Sender<MainWorkerTerminateHandle>,
) -> Result<()> {
	let actor_path = config.actors_path.join(format!("{actor_id}-{generation}"));

	// Write PID to file
	std::fs::write(
		actor_path.join("pid"),
		std::process::id().to_string().as_bytes(),
	)?;

	// Read config
	let config_data = std::fs::read_to_string(actor_path.join("config.json"))
		.context("Failed to read config file")?;
	let actor_config = serde_json::from_str::<config::actor::Config>(&config_data)
		.context("Failed to parse config file")?;

	let (shutdown_tx, shutdown_rx) = smpsc::sync_channel(1);

	// Start log shipper
	let (msg_tx, log_shipper_thread) =
		if let Some(vector_socket_addr) = &actor_config.vector_socket_addr {
			let (msg_tx, msg_rx) = smpsc::sync_channel::<log_shipper::ReceivedMessage>(
				log_shipper::MAX_BUFFER_BYTES / log_shipper::MAX_LINE_BYTES,
			);
			let log_shipper = log_shipper::LogShipper {
				actor_id,
				shutdown_rx,
				msg_rx,
				vector_socket_addr: vector_socket_addr.clone(),
			};
			let log_shipper_thread = log_shipper.spawn();

			(Some(msg_tx), Some(log_shipper_thread))
		} else {
			(None, None)
		};

	// Run the isolate
	let exit_code = match utils::tokio::create_and_run_current_thread(run_inner(
		fdb_pool,
		actor_path.clone(),
		actor_id,
		generation,
		terminate_tx,
		msg_tx.clone(),
		actor_config,
	))? {
		Result::Ok(exit_code) => exit_code,
		Err(err) => {
			tracing::error!(?actor_id, ?generation, "Run isolate failed: {err:?}");
			log_shipper::send_message(
				actor_id,
				&msg_tx,
				None,
				log_shipper::StreamType::StdErr,
				"Fatal error. Aborting.".into(),
			);

			Some(1)
		}
	};

	// Shutdown all threads
	match shutdown_tx.send(()) {
		Result::Ok(_) => {
			tracing::info!(?actor_id, ?generation, "Sent shutdown signal");
		}
		Err(err) => {
			tracing::error!(
				?actor_id,
				?generation,
				"Failed to send shutdown signal: {err:?}"
			);
		}
	}

	// Wait for log shipper to finish
	drop(msg_tx);
	if let Some(log_shipper_thread) = log_shipper_thread {
		match log_shipper_thread.join() {
			Result::Ok(_) => {}
			Err(err) => {
				tracing::error!(?actor_id, ?generation, "Log shipper failed: {err:?}")
			}
		}
	}

	// Write exit code. None is written as no bytes
	if let Some(code) = exit_code {
		std::fs::write(actor_path.join("exit-code"), code.to_string().as_bytes())?;
	} else {
		std::fs::write(actor_path.join("exit-code"), &[])?;
	}

	Ok(())
}

pub async fn run_inner(
	fdb_pool: FdbPool,
	actor_path: PathBuf,
	actor_id: Uuid,
	generation: u32,
	terminate_tx: mpsc::Sender<MainWorkerTerminateHandle>,
	msg_tx: Option<smpsc::SyncSender<log_shipper::ReceivedMessage>>,
	actor_config: config::actor::Config,
) -> Result<Option<i32>> {
	tracing::info!(?actor_id, ?generation, "starting isolate");

	// Init KV store (create or open)
	let mut kv = ActorKv::new((&*fdb_pool).clone(), actor_id);
	kv.init().await?;

	tracing::info!(?actor_id, ?generation, "isolate kv initialized");

	// Should match the path from `Actor::make_fs` in manager/src/actor/setup.rs
	let index = actor_path.join("fs").join("index.js");

	// Load index.js
	let index_script_content = match fs::read_to_string(&index).await {
		Ok(c) => c,
		Err(err) => {
			tracing::error!(?err, "Failed to load {}", index.display());

			log_shipper::send_message(
				actor_id,
				&msg_tx,
				None,
				log_shipper::StreamType::StdErr,
				"Failed to load /index.js".into(),
			);

			return Ok(Some(1));
		}
	};

	// Load script into a static module loader. No dynamic scripts can be loaded this way.
	let index_module = ModuleSpecifier::from_file_path(Path::new("/index.js"))
		.map_err(|_| anyhow!("invalid file name"))?;
	let loader = StaticModuleLoader::new([(index_module.clone(), index_script_content)]);

	// TODO(RVT-4192): Replace with a custom fs that only reads from actor_path/fs
	let fs = Arc::new(InMemoryFs::default());

	// Build permissions
	let permission_desc_parser = Arc::new(RuntimePermissionDescriptorParser::new(fs.clone()));
	let mut permissions = Permissions::none_without_prompt();

	// Outbound traffic
	permissions.net = UnaryPermission::allow_all();
	// Sockets
	let loopback = Ipv4Addr::new(0, 0, 0, 0);
	permissions.net_listen = Permissions::new_unary::<NetListenDescriptor>(
		Some(
			actor_config
				.ports
				.iter()
				.map(|port| {
					NetListenDescriptor::from_ipv4(
						loopback,
						Some(port.target),
						match port.protocol {
							protocol::TransportProtocol::Tcp => deno_permissions::Protocol::Tcp,
							protocol::TransportProtocol::Udp => deno_permissions::Protocol::Udp,
						},
					)
				})
				.collect(),
		),
		None,
		false,
	);
	// We use a custom in-memory env
	permissions.env = UnaryPermission::allow_all();

	// Create pipes
	let (stdout_read_fd, stdout_write_fd) = pipe()?;
	let (stderr_read_fd, stderr_write_fd) = pipe()?;

	// SAFETY: These are created by pipes
	let stdout_reader = unsafe { std::fs::File::from_raw_fd(stdout_read_fd) };
	let stdout_writer = unsafe { std::fs::File::from_raw_fd(stdout_write_fd) };
	let stderr_reader = unsafe { std::fs::File::from_raw_fd(stderr_read_fd) };
	let stderr_writer = unsafe { std::fs::File::from_raw_fd(stderr_write_fd) };
	let mut stderr_writer2 = stderr_writer.try_clone()?;

	let isolate_stdout = BufReader::new(stdout_reader);
	let isolate_stderr = BufReader::new(stderr_reader);

	// Ship stdout & stderr logs
	let stdout_handle = log_shipper::ship_logs(
		actor_id,
		msg_tx.clone(),
		log_shipper::StreamType::StdOut,
		isolate_stdout,
	);
	let stderr_handle = log_shipper::ship_logs(
		actor_id,
		msg_tx.clone(),
		log_shipper::StreamType::StdErr,
		isolate_stderr,
	);

	// Build worker. If this errors its likely a problem with the runtime and not user input
	let mut worker = MainWorker::try_bootstrap_from_options(
		index_module.clone(),
		WorkerServiceOptions {
			module_loader: Rc::new(loader),
			permissions: PermissionsContainer::new(permission_desc_parser, permissions),
			blob_store: Default::default(),
			broadcast_channel: Default::default(),
			feature_checker: Default::default(),
			node_services: Default::default(),
			npm_process_state_provider: Default::default(),
			root_cert_store_provider: Default::default(),
			fetch_dns_resolver: Default::default(),
			shared_array_buffer_store: Default::default(),
			compiled_wasm_module_store: Default::default(),
			v8_code_cache: Default::default(),
			fs,
		},
		WorkerOptions {
			extensions: vec![
				ext::kv::rivet_kv::init_ops_and_esm(kv),
				ext::runtime::rivet_runtime::init_ops_and_esm(),
			],
			// Configure memory limits
			create_params: {
				fn floor_align(value: usize, alignment: usize) -> usize {
					value & !(alignment - 1)
				}

				// Memory must be aligned with PAGESIZE
				let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.try_into()?;
				let mem = floor_align(actor_config.resources.memory.try_into()?, page_size);
				let mem_max = floor_align(actor_config.resources.memory_max.try_into()?, page_size);

				Some(CreateParams::default().heap_limits(mem, mem_max))
			},
			stdio: Stdio {
				// TODO: Make this read from /dev/null instead
				stdin: StdioPipe::inherit(),
				stdout: StdioPipe::file(stdout_writer),
				stderr: StdioPipe::file(stderr_writer),
			},
			env: actor_config.env,
			..Default::default()
		},
	)?;

	// Send terminate handle to watcher task
	terminate_tx.send(worker.terminate_handle().clone()).await?;
	drop(terminate_tx);

	// First step preloads the module. This can throw a JS error from certain syntax.
	match worker.preload_main_module(&index_module).await {
		Ok(module_id) => {
			tracing::info!(?actor_id, ?generation, "Isolate ready");

			// Second step evaluates the module but does not run it (because its sync).
			let res = worker.evaluate_module_sync(module_id);

			if worker.is_terminated() {
				tracing::info!(?actor_id, ?generation, "Isolate terminated");
			} else {
				if let Err(err) = res {
					tracing::info!(?actor_id, ?generation, "Isolate evaluation failed");

					runtime_error(&mut stderr_writer2, &mut worker, err)?;
				} else {
					// Call `start`
					match handle_entrypoint(
						actor_config.metadata.deserialize()?,
						&mut worker,
						module_id,
					) {
						Ok(()) => {
							// Third step runs event loop until stopped. We do this even after an error in
							// case a beforeunload event handler was registered.
							loop {
								let res = worker.run_event_loop(Default::default()).await;

								if worker.is_terminated() {
									tracing::info!(?actor_id, ?generation, "Isolate terminated");
									break;
								}

								if let Err(err) = res {
									tracing::info!(
										?actor_id,
										?generation,
										"Isolate execution failed"
									);

									runtime_error(&mut stderr_writer2, &mut worker, err)?;
								}

								// We dispatch the beforeunload event then run the event loop again
								match worker.dispatch_beforeunload_event() {
									Ok(web_continue) => {
										if !web_continue {
											break;
										}
									}
									Err(err) => {
										tracing::info!(
											?actor_id,
											?generation,
											"Dispatch beforeunload event failed"
										);

										runtime_error(&mut stderr_writer2, &mut worker, err)?;

										break;
									}
								}
							}
						}
						Err(err) => runtime_error(&mut stderr_writer2, &mut worker, err)?,
					}
				}
			}
		}
		Err(err) => {
			tracing::info!(?actor_id, ?generation, "Isolate preload failed");

			match err.downcast::<JsError>() {
				// JS error
				Ok(err) => runtime_error(&mut stderr_writer2, &mut worker, err.into())?,
				Err(err) => {
					// Also JS error
					if deno_core::error::get_custom_error_class(&err).is_some() {
						runtime_error(&mut stderr_writer2, &mut worker, err)?;
					}
					// Fatal error
					else {
						return Err(err);
					}
				}
			}
		}
	}

	// For good measure
	worker.v8_isolate().terminate_execution();

	tracing::info!(?actor_id, ?generation, "Isolate complete");

	let exit_code = if worker.is_terminated() {
		None
	} else {
		Some(worker.exit_code())
	};

	// Drop worker and writer so the stdout and stderr pipes close
	drop(worker);

	wait_logs_complete(
		actor_id,
		generation,
		stderr_writer2,
		stdout_handle,
		stderr_handle,
	)?;

	Ok(exit_code)
}

// Reads the `start` function from the default export of index.js and calls it.
fn handle_entrypoint(
	actor_metadata: protocol::ActorMetadata,
	worker: &mut MainWorker,
	index_module_id: ModuleId,
) -> Result<()> {
	let mm = worker.js_runtime.module_map();
	let scope = &mut worker.js_runtime.handle_scope();

	// Get index.js mod
	let g_ns = mm.get_module_namespace(scope, index_module_id)?;
	let ns = v8::Local::new(scope, g_ns);

	// Get default export
	let default_export_name = v8::String::new(scope, "default").context("v8 primitive")?;
	let default_export = ns
		.get(scope, default_export_name.into())
		.context("default export")?
		.to_object(scope)
		.context(
			"Missing default export at index.js. Try: export default { start(ctx) { ... } }",
		)?;

	// Get `start` export
	let start_export_name = v8::String::new(scope, "start").context("v8 primitive")?;
	let start_export = default_export
		.get(scope, start_export_name.into())
		.context("Invalid `start` function in default export")?;

	// Parse `start` as function
	let start_func = v8::Local::<v8::Function>::try_from(start_export)
		.context("Invalid `start` function in default export")?;

	// Get rivet ns
	let rivet_ns_module_id = mm
		.get_id(
			&"ext:rivet_runtime/90_rivet_ns.js",
			deno_core::RequestedModuleType::None,
		)
		.context("ns should be loaded")?;
	let rivet_g_ns = mm.get_module_namespace(scope, rivet_ns_module_id)?;
	let rivet_ns = v8::Local::new(scope, rivet_g_ns);

	// Get deep freeze function
	let deep_freeze_name = v8::String::new(scope, "deepFreeze").context("v8 primitive")?;
	let deep_freeze = rivet_ns
		.get(scope, deep_freeze_name.into())
		.context("deepFreeze")?;
	let deep_freeze = v8::Local::<v8::Function>::try_from(deep_freeze).context("deepFreeze")?;

	// Get actor context from ns
	let ctx_export_name = v8::String::new(scope, "ACTOR_CONTEXT").context("v8 primitive")?;
	let ctx_export = rivet_ns
		.get(scope, ctx_export_name.into())
		.context("runtime export")?
		.to_object(scope)
		.context("ns is object")?;

	// Serialize metadata
	let metadata = JsMetadata::from_actor(actor_metadata, scope)?;
	let metadata = deno_core::serde_v8::to_v8(scope, metadata)?;

	// Add metadata
	let metadata_key = v8::String::new(scope, "metadata")
		.context("v8 primitive")?
		.into();
	ctx_export.set(scope, metadata_key, metadata);

	// Freeze ctx
	let frozen_ctx = deep_freeze
		.call(scope, rivet_ns.into(), &[ctx_export.into()])
		.context("deepFreeze call")?;

	// Call `start` function
	let res = start_func.call(scope, default_export.into(), &[frozen_ctx]);

	// Make sure `start` function async
	match res {
		Some(promise) if promise.is_promise() => {}
		_ => bail!("`start` function must be async"),
	}

	Ok(())
}

fn runtime_error(stderr_writer: &mut File, worker: &mut MainWorker, err: Error) -> Result<()> {
	// Write final error to stderr
	stderr_writer.write_all(err.to_string().as_bytes())?;

	// Update error code if not already errored
	if worker.exit_code() == 0 {
		worker.set_exit_code(1);
	}

	Ok(())
}

/// Waits for logs to be written and log shipper threads to complete.
fn wait_logs_complete(
	actor_id: Uuid,
	generation: u32,
	mut stderr_writer2: File,
	stdout_handle: JoinHandle<()>,
	stderr_handle: JoinHandle<()>,
) -> Result<()> {
	stderr_writer2.flush()?;
	drop(stderr_writer2);

	// Wait for threads to finish
	match stdout_handle.join() {
		Result::Ok(_) => {}
		Err(err) => {
			tracing::error!(?actor_id, ?generation, "stdout thread panicked: {err:?}");
		}
	}
	match stderr_handle.join() {
		Result::Ok(_) => {}
		Err(err) => {
			tracing::error!(?actor_id, ?generation, "stderr thread panicked: {err:?}");
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use std::{net::SocketAddr, path::Path, result::Result::Ok};

	use anyhow::*;
	use deno_runtime::worker::MainWorkerTerminateHandle;
	use foundationdb as fdb;
	use pegboard::protocol;
	use pegboard_config::isolate_runner as config;
	use tracing_subscriber::prelude::*;
	use uuid::Uuid;

	use super::run_inner;
	use crate::utils;

	// TODO: Currently requires an fdb container to be running already
	#[tokio::test]
	async fn test_isolate() -> Result<()> {
		tracing_subscriber::registry()
			.with(
				tracing_logfmt::builder()
					.with_ansi_color(true)
					.layer()
					.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
			)
			.init();

		let tmp_dir = tempfile::TempDir::new().unwrap();
		let actors_path = tmp_dir.path().join("actors");
		let actor_id = Uuid::nil();
		let generation = 0;

		let fs_path = actors_path
			.join(format!("{actor_id}-{generation}"))
			.join("fs");
		std::fs::create_dir_all(&fs_path)?;

		std::fs::copy(
			Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/index.js"),
			fs_path.join("index.js"),
		)?;
		std::fs::write(tmp_dir.path().join("fdb.cluster"), "fdb:fdb@127.0.0.1:4500")?;

		let config = config::Config {
			// Not important
			actors_path: Path::new("").to_path_buf(),
			manager_ws_addr: SocketAddr::from(([0, 0, 0, 0], 0)),
		};

		deno_core::v8_set_flags(vec![
			// Binary name
			"UNUSED_BUT_NECESSARY_ARG0".into(),
			// Disable eval
			"--disallow-code-generation-from-strings".into(),
		]);

		// Start FDB network thread
		fdb_util::init(&tmp_dir.path().join("fdb.cluster"));

		// For receiving the terminate handle
		let (terminate_tx, _terminate_rx) =
			tokio::sync::mpsc::channel::<MainWorkerTerminateHandle>(1);

		let actor_config = config::actor::Config {
			resources: config::actor::Resources {
				memory: 26843545600,
				memory_max: 26843545600,
			},
			ports: Default::default(),
			env: Default::default(),
			metadata: protocol::Raw::new(&protocol::ActorMetadata {
				actor: protocol::ActorMetadataActor {
					actor_id: Uuid::nil(),
					tags: [("foo".to_string(), "bar".to_string())]
						.into_iter()
						.collect(),
					create_ts: 0,
				},
				project: protocol::ActorMetadataProject {
					project_id: Uuid::nil(),
					slug: "foo".into(),
				},
				environment: protocol::ActorMetadataEnvironment {
					env_id: Uuid::nil(),
					slug: "foo".into(),
				},
				datacenter: protocol::ActorMetadataDatacenter {
					name_id: "local".to_string(),
					display_name: "Local".to_string(),
				},
				cluster: protocol::ActorMetadataCluster {
					cluster_id: Uuid::nil(),
				},
				build: protocol::ActorMetadataBuild {
					build_id: Uuid::nil(),
				},
			})
			.unwrap(),
			vector_socket_addr: Default::default(),
		};

		let exit_code = run_inner(
			config,
			actors_path.join(actor_id.to_string()).to_path_buf(),
			actor_id,
			generation,
			terminate_tx,
			None,
			actor_config,
		)
		.await?;

		ensure!(exit_code == 0);

		Ok(())
	}
}
