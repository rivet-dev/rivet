use std::{
	io::{BufReader, Write},
	net::Ipv4Addr,
	os::fd::FromRawFd,
	path::{Path, PathBuf},
	rc::Rc,
	result::Result::{Err, Ok},
	sync::{mpsc as smpsc, Arc},
};

use anyhow::*;
use deno_core::{unsync::MaskFutureAsSend, v8::CreateParams, ModuleSpecifier, StaticModuleLoader};
use deno_runtime::{
	deno_fs::InMemoryFs,
	deno_io::{Stdio, StdioPipe},
	deno_permissions::{NetListenDescriptor, Permissions, PermissionsContainer, UnaryPermission},
	permissions::RuntimePermissionDescriptorParser,
	worker::{MainWorker, MainWorkerTerminateHandle, WorkerOptions, WorkerServiceOptions},
};
use nix::{libc, unistd::pipe};
use tokio::{fs, sync::mpsc};
use uuid::Uuid;

use crate::{config::Config, ext, log_shipper, utils};

pub fn run(
	actors_path: PathBuf,
	actor_id: Uuid,
	terminate_tx: mpsc::Sender<MainWorkerTerminateHandle>,
) -> Result<()> {
	let actor_path = actors_path.join(actor_id.to_string());

	// Write PID to file
	std::fs::write(
		actor_path.join("pid"),
		std::process::id().to_string().as_bytes(),
	)?;

	// Read config
	let config_data = std::fs::read_to_string(actor_path.join("config.json"))
		.context("Failed to read config file")?;
	let config =
		serde_json::from_str::<Config>(&config_data).context("Failed to parse config file")?;

	let (shutdown_tx, shutdown_rx) = smpsc::sync_channel(1);

	// Start log shipper
	let (msg_tx, log_shipper_thread) = if let Some(vector_socket_addr) = &config.vector_socket_addr
	{
		let (msg_tx, msg_rx) = smpsc::sync_channel::<log_shipper::ReceivedMessage>(
			log_shipper::MAX_BUFFER_BYTES / log_shipper::MAX_LINE_BYTES,
		);
		let log_shipper = log_shipper::LogShipper {
			actor_id,
			shutdown_rx,
			msg_rx,
			vector_socket_addr: vector_socket_addr.clone(),
			owner: config.owner.clone(),
		};
		let log_shipper_thread = log_shipper.spawn();

		(Some(msg_tx), Some(log_shipper_thread))
	} else {
		(None, None)
	};

	// Run the isolate
	let exit_code = match create_and_run_current_thread(run_inner(
		actor_id,
		actor_path.clone(),
		terminate_tx,
		msg_tx.clone(),
		config,
	))? {
		Result::Ok(exit_code) => exit_code,
		Err(err) => {
			tracing::error!(?actor_id, "Run isolate failed: {err:?}");
			log_shipper::send_message(
				actor_id,
				&msg_tx,
				None,
				log_shipper::StreamType::StdErr,
				format!("Aborting"),
			);

			1
		}
	};

	// Shutdown all threads
	match shutdown_tx.send(()) {
		Result::Ok(_) => {
			tracing::info!(?actor_id, "Sent shutdown signal");
		}
		Err(err) => {
			tracing::error!(?actor_id, "Failed to send shutdown signal: {err:?}");
		}
	}

	// Wait for log shipper to finish
	drop(msg_tx);
	if let Some(log_shipper_thread) = log_shipper_thread {
		match log_shipper_thread.join() {
			Result::Ok(_) => {}
			Err(err) => {
				tracing::error!(?actor_id, "Log shipper failed: {err:?}")
			}
		}
	}

	// Write exit code
	std::fs::write(
		actor_path.join("exit-code"),
		exit_code.to_string().as_bytes(),
	)?;

	Ok(())
}

pub async fn run_inner(
	actor_id: Uuid,
	actor_path: PathBuf,
	terminate_tx: mpsc::Sender<MainWorkerTerminateHandle>,
	msg_tx: Option<smpsc::SyncSender<log_shipper::ReceivedMessage>>,
	config: Config,
) -> Result<i32> {
	tracing::info!(?actor_id, "Starting isolate");

	// Load script into a static module loader. No dynamic scripts can be loaded this way.
	let script_content = fs::read_to_string(actor_path.join("index.js"))
		.await
		.context("failed to load index.js")?;
	let main_module = ModuleSpecifier::from_file_path(Path::new("/index.js"))
		.map_err(|_| anyhow!("invalid file name"))?;
	let loader = StaticModuleLoader::new([(main_module.clone(), script_content)]);

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
			config
				.ports
				.iter()
				.map(|port| {
					NetListenDescriptor::from_ipv4(
						loopback,
						Some(port.target),
						port.protocol.into(),
					)
				})
				.collect(),
		),
		None,
		false,
	)?;
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

	// Build worker
	let mut worker = MainWorker::bootstrap_from_options(
		main_module.clone(),
		WorkerServiceOptions {
			module_loader: Rc::new(loader),
			permissions: PermissionsContainer::new(permission_desc_parser, permissions),
			blob_store: Default::default(),
			broadcast_channel: Default::default(),
			feature_checker: Default::default(),
			node_services: Default::default(),
			npm_process_state_provider: Default::default(),
			root_cert_store_provider: Default::default(),
			shared_array_buffer_store: Default::default(),
			compiled_wasm_module_store: Default::default(),
			v8_code_cache: Default::default(),
			fs,
		},
		WorkerOptions {
			extensions: vec![
				ext::kv::rivet_kv::init_ops_and_esm(utils::fdb_handle()?),
				ext::runtime::rivet_runtime::init_ops_and_esm(),
			],
			// Configure memory limits
			create_params: {
				fn floor_align(value: usize, alignment: usize) -> usize {
					value & !(alignment - 1)
				}

				// Memory must be aligned with PAGESIZE
				let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.try_into()?;
				let mem = floor_align(config.resources.memory.try_into()?, page_size);
				let mem_max = floor_align(config.resources.memory_max.try_into()?, page_size);

				Some(CreateParams::default().heap_limits(mem, mem_max))
			},
			stdio: Stdio {
				// TODO: Make this read from /dev/null instead
				stdin: StdioPipe::inherit(),
				stdout: StdioPipe::file(stdout_writer),
				stderr: StdioPipe::file(stderr_writer),
			},
			env: config.env,
			..Default::default()
		},
	);

	// Send terminate handle to main thread
	terminate_tx.send(worker.terminate_handle().clone()).await?;
	drop(terminate_tx);

	let module_id = worker.preload_main_module(&main_module).await?;

	tracing::info!(?actor_id, "Isolate ready");

	// First step evaluates the module and possibly runs it. Sometimes the event loop continues running and
	// sometimes it doesn't
	let res = worker.evaluate_module(module_id).await;
	if worker.is_terminated() {
		tracing::info!(?actor_id, "Isolate terminated");
	} else {
		if let Err(err) = res {
			tracing::info!(?actor_id, "Isolate execution failed");

			// Write error to stderr
			stderr_writer2.write_all(err.to_string().as_bytes())?;

			if worker.exit_code() == 0 {
				worker.set_exit_code(1);
			}
		}

		// Second step continues running the event loop until stopped
		loop {
			let res = worker.run_event_loop(Default::default()).await;

			if worker.is_terminated() {
				tracing::info!(?actor_id, "Isolate terminated");
				break;
			}

			if let Err(err) = res {
				tracing::info!(?actor_id, "Isolate execution failed");

				// Write final error to stderr
				stderr_writer2.write_all(err.to_string().as_bytes())?;

				if worker.exit_code() == 0 {
					worker.set_exit_code(1);
				}
			}

			// We dispatch the beforeunload event then run the event loop again
			match worker.dispatch_beforeunload_event() {
				Ok(web_continue) => {
					if !web_continue {
						break;
					}
				}
				Err(err) => {
					tracing::info!(?actor_id, "Dispatch beforeunload event failed");

					// Write final error to stderr
					stderr_writer2.write_all(err.to_string().as_bytes())?;

					if worker.exit_code() == 0 {
						worker.set_exit_code(1);
					}

					break;
				}
			}
		}
	}

	// For good measure
	worker.v8_isolate().terminate_execution();

	tracing::info!(?actor_id, "Isolate complete");

	let exit_code = worker.exit_code();

	// Drop worker and writer so the stdout and stderr pipes close
	drop(worker);

	stderr_writer2.flush()?;
	drop(stderr_writer2);

	// Wait for threads to finish
	match stdout_handle.join() {
		Result::Ok(_) => {}
		Err(err) => {
			tracing::error!(?actor_id, "stdout thread panicked: {err:?}");
		}
	}
	match stderr_handle.join() {
		Result::Ok(_) => {}
		Err(err) => {
			tracing::error!(?actor_id, "stderr thread panicked: {err:?}");
		}
	}

	Ok(exit_code)
}

// Copied from deno-runtime tokio_util.rs
fn create_basic_runtime() -> Result<tokio::runtime::Runtime> {
	let event_interval = 61;
	let global_queue_interval = 31;
	let max_io_events_per_tick = 1024;

	tokio::runtime::Builder::new_current_thread()
		.enable_io()
		.enable_time()
		.event_interval(event_interval)
		.global_queue_interval(global_queue_interval)
		.max_io_events_per_tick(max_io_events_per_tick)
		// This limits the number of threads for blocking operations (like for
		// synchronous fs ops) or CPU bound tasks like when we run dprint in
		// parallel for deno fmt.
		// The default value is 512, which is an unhelpfully large thread pool. We
		// don't ever want to have more than a couple dozen threads.
		.max_blocking_threads(32)
		.build()
		.map_err(Into::into)
}

// Copied from deno-runtime tokio_util.rs
#[inline(always)]
fn create_and_run_current_thread<F, R>(future: F) -> Result<R>
where
	F: std::future::Future<Output = R> + 'static,
	R: Send + 'static,
{
	let rt = create_basic_runtime()?;

	// Since this is the main future, we want to box it in debug mode because it tends to be fairly
	// large and the compiler won't optimize repeated copies. We also make this runtime factory
	// function #[inline(always)] to avoid holding the unboxed, unused future on the stack.

	#[cfg(debug_assertions)]
	// SAFETY: this is guaranteed to be running on a current-thread executor
	let future = Box::pin(unsafe { MaskFutureAsSend::new(future) });

	#[cfg(not(debug_assertions))]
	// SAFETY: this is guaranteed to be running on a current-thread executor
	let future = unsafe { MaskFutureAsSend::new(future) };

	let join_handle = rt.spawn(future);

	let r = rt.block_on(join_handle)?.into_inner();
	// Forcefully shutdown the runtime - we're done executing JS code at this
	// point, but there might be outstanding blocking tasks that were created and
	// latered "unrefed". They won't terminate on their own, so we're forcing
	// termination of Tokio runtime at this point.
	rt.shutdown_background();

	Ok(r)
}
