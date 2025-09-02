use anyhow::Result;
use std::{ops::Deref, sync::Arc, time::Duration};
use tokio::{sync::watch, task::JoinHandle};

use crate::{ctx::test::TestCtx, db::debug::DatabaseDebug, prelude::*};

pub fn setup_logging() {
	// Set up logging
	let _ = tracing_subscriber::fmt()
		.with_env_filter("debug")
		.with_ansi(false)
		.with_test_writer()
		.try_init();
}

pub struct WorkflowTestCtx {
	ctx: TestCtx,
	debug_db: Arc<dyn DatabaseDebug>,
	shutdown_tx: watch::Sender<()>,
	pub test_deps: rivet_test_deps::TestDeps,
	worker_handle: Option<JoinHandle<Result<()>>>,
}

impl WorkflowTestCtx {
	pub async fn new(reg: Registry, test_deps: rivet_test_deps::TestDeps) -> Result<Self> {
		setup_logging();

		tracing::info!("setting up gasoline test environment");

		let config = test_deps.config().clone();
		let pools = test_deps.pools().clone();

		let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())
			.expect("failed to create cache");

		let db = db::DatabaseKv::from_pools(pools.clone()).await.unwrap();
		let debug_db =
			db::DatabaseKv::from_pools(pools.clone()).await.unwrap() as Arc<dyn DatabaseDebug>;

		// Create test context with custom components
		tracing::info!("creating test context");
		let ctx = TestCtx::new::<db::DatabaseKv>(
			"gasoline_test",
			db.clone(),
			config.clone(),
			pools.clone(),
			cache.clone(),
		)
		.await?;

		let worker = Worker::new(reg.handle(), db, config.clone(), pools);
		let (shutdown_tx, shutdown_rx) = watch::channel(());

		tracing::info!("starting workflow worker");
		let worker_handle = tokio::spawn(worker.start(Some(shutdown_rx)));

		// Give the worker time to start up
		tokio::time::sleep(Duration::from_millis(500)).await;

		tracing::info!("test environment setup complete");
		Ok(WorkflowTestCtx {
			ctx,
			debug_db,
			shutdown_tx,
			test_deps,
			worker_handle: Some(worker_handle),
		})
	}

	pub fn debug_db(&self) -> &dyn DatabaseDebug {
		&*self.debug_db
	}

	pub async fn shutdown(&mut self) -> Result<()> {
		if let Some(worker_handle) = self.worker_handle.take() {
			tracing::info!("stopping workflow worker");

			// Trigger shutdown
			self.shutdown_tx.send(())?;

			// Wait for the worker to finish its shutdown sequence
			//
			// This ensures that `Worker::shutdown` has been called successfully
			tracing::info!("waiting for workflow worker handle to finish");
			match worker_handle.await {
				Ok(result) => {
					if let Err(err) = result {
						tracing::warn!(?err, "worker stopped with error");
					}
				}
				Err(err) => {
					tracing::warn!(?err, "worker task join error");
				}
			}

			tracing::info!("workflow worker stopped");
		}
		Ok(())
	}
}

impl Deref for WorkflowTestCtx {
	type Target = TestCtx;

	fn deref(&self) -> &Self::Target {
		&self.ctx
	}
}

pub async fn setup(reg: Registry) -> Result<WorkflowTestCtx> {
	let test_deps = rivet_test_deps::TestDeps::new().await.unwrap();
	setup_with_deps(reg, test_deps).await
}

pub async fn setup_with_deps(
	reg: Registry,
	test_deps: rivet_test_deps::TestDeps,
) -> Result<WorkflowTestCtx> {
	WorkflowTestCtx::new(reg, test_deps).await
}
