use std::{ops::Deref, path::Path, result::Result::Ok, sync::Arc};

use ::tokio::fs;
use anyhow::*;
use foundationdb as fdb;
use pegboard_config::isolate_runner::Config;
use service_discovery::ServiceDiscovery;

// TODO: Copied from rivet_pools
#[derive(Clone)]
pub struct FdbPool {
	db: Arc<fdb::Database>,
	_sd: Option<Arc<ServiceDiscovery>>,
	// Prevent dropping temp file
	_connection_file: Arc<tempfile::NamedTempFile>,
}

impl Deref for FdbPool {
	type Target = Arc<fdb::Database>;

	fn deref(&self) -> &Self::Target {
		&self.db
	}
}

#[tracing::instrument(skip(config))]
pub async fn setup_fdb_pool(config: &Config) -> Result<FdbPool> {
	let temp_file = tempfile::NamedTempFile::new()?;
	let temp_path = temp_file.path().to_path_buf();

	let fdb_config = config.foundationdb.clone();

	let sd = match &fdb_config.addresses {
		pegboard_config::Addresses::Dynamic { fetch_endpoint } => {
			let sd = ServiceDiscovery::new(fetch_endpoint.clone());

			// Initial fetch
			let servers = sd.fetch().await.context("failed to fetch services")?;
			let joined = servers
				.into_iter()
				.filter_map(|server| server.lan_ip)
				.map(|lan_ip| format!("{lan_ip}:4500"))
				.collect::<Vec<_>>()
				.join(",");
			write_connection_file(&fdb_config, &temp_path, &joined).await?;

			sd.start(move |servers| {
				let temp_path = temp_path.clone();
				let fdb_config = fdb_config.clone();
				async move {
					let joined = servers
						.into_iter()
						.filter_map(|server| server.lan_ip)
						.map(|lan_ip| format!("{lan_ip}:4500"))
						.collect::<Vec<_>>()
						.join(",");

					write_connection_file(&fdb_config, &temp_path, &joined).await?;

					anyhow::Ok(())
				}
			});

			Some(sd)
		}
		pegboard_config::Addresses::Static(addresses) => {
			let joined = addresses.join(",");
			write_connection_file(&fdb_config, &temp_path, &joined).await?;

			None
		}
	};

	// Start network
	fdb_util::init(temp_file.path());

	let fdb_handle = fdb_util::handle(&temp_file.path())?;

	tracing::debug!(config_file_path=%temp_file.path().display(), "fdb started");

	Ok(FdbPool {
		db: Arc::new(fdb_handle),
		_sd: sd,
		_connection_file: Arc::new(temp_file),
	})
}

async fn write_connection_file(
	fdb_config: &pegboard_config::FoundationDb,
	temp_path: &Path,
	joined: &str,
) -> Result<(), std::io::Error> {
	let connection = format!(
		"{cluster_description}:{cluster_id}@{joined}",
		cluster_description = fdb_config.cluster_description,
		cluster_id = fdb_config.cluster_id,
	);

	fs::write(temp_path, connection.as_bytes()).await?;

	Ok(())
}

pub mod tokio {
	use anyhow::*;
	use deno_core::unsync::MaskFutureAsSend;

	// Copied from deno-runtime tokio_util.rs
	fn create_basic_runtime() -> Result<::tokio::runtime::Runtime> {
		let event_interval = 61;
		let global_queue_interval = 31;
		let max_io_events_per_tick = 1024;

		::tokio::runtime::Builder::new_current_thread()
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
	pub fn create_and_run_current_thread<F, R>(future: F) -> Result<R>
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
}
