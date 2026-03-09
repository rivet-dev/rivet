//! Load test binary for the gasoline workflow engine with Postgres backend.
//!
//! This binary simulates the patterns used in real actor workflows: loops with signal listening,
//! activities that read/write UDB, and external signal publishing. It is designed to stress test
//! the Postgres-backed UDB and workflow engine to reproduce corruption/freezing issues.
//!
//! Usage:
//!   RIVET_TEST_DATABASE=postgres gasoline-load-test --mode worker
//!   RIVET_TEST_DATABASE=postgres gasoline-load-test --mode bombarder --workflow-count 50

mod workflows;

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use clap::Parser;
use gas::prelude::*;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "gasoline-load-test")]
struct Args {
	/// Mode: "worker" runs the workflow worker, "bombarder" dispatches workflows and sends signals,
	/// "standalone" runs both worker and bombarder in the same process
	#[arg(long, default_value = "standalone")]
	mode: String,

	/// Number of workflows to dispatch (bombarder/standalone mode)
	#[arg(long, default_value = "20")]
	workflow_count: usize,

	/// Number of signals to send per workflow (bombarder/standalone mode)
	#[arg(long, default_value = "10")]
	signals_per_workflow: usize,

	/// Delay between signal sends in milliseconds
	#[arg(long, default_value = "50")]
	signal_delay_ms: u64,

	/// Number of concurrent signal senders
	#[arg(long, default_value = "5")]
	concurrency: usize,

	/// Test ID to share state between worker and bombarder processes. If not set, a new one is
	/// generated.
	#[arg(long)]
	test_id: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
	// Set up logging
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.with_ansi(true)
		.init();

	let args = Args::parse();

	let test_id: uuid::Uuid = if let Some(id) = &args.test_id {
		id.parse().context("invalid test_id UUID")?
	} else {
		uuid::Uuid::new_v4()
	};

	tracing::info!(?test_id, mode = %args.mode, "starting gasoline load test");

	match args.mode.as_str() {
		"worker" => run_worker(test_id).await,
		"bombarder" => run_bombarder(test_id, &args).await,
		"standalone" => run_standalone(test_id, &args).await,
		_ => anyhow::bail!("unknown mode: {}", args.mode),
	}
}

fn build_registry() -> Result<Registry> {
	let mut reg = Registry::new();
	reg.register_workflow::<workflows::LoopSignalWorkflow>()?;
	reg.register_workflow::<workflows::BusyLoopWorkflow>()?;
	reg.register_workflow::<workflows::SignalChainWorkflow>()?;
	Ok(reg)
}

async fn setup_test_deps(test_id: uuid::Uuid) -> Result<rivet_test_deps::TestDeps> {
	rivet_test_deps::TestDeps::new_with_test_id(test_id).await
}

async fn run_worker(test_id: uuid::Uuid) -> Result<()> {
	let reg = build_registry()?;
	let mut test_deps = setup_test_deps(test_id).await?;
	test_deps.dont_stop_docker_containers_on_drop();

	let config = test_deps.config().clone();
	let pools = test_deps.pools().clone();

	let db = gas::db::DatabaseKv::new(config.clone(), pools.clone()).await?;
	let worker = Worker::new(reg.handle(), db, config, pools);

	tracing::info!("worker started, waiting for workflows");
	worker.start(None).await?;

	Ok(())
}

async fn run_bombarder(test_id: uuid::Uuid, args: &Args) -> Result<()> {
	let mut test_deps = setup_test_deps(test_id).await?;
	test_deps.dont_stop_docker_containers_on_drop();

	let config = test_deps.config().clone();
	let pools = test_deps.pools().clone();

	let db = gas::db::DatabaseKv::new(config.clone(), pools.clone()).await?;

	bombarder_logic(db, config, args).await
}

async fn run_standalone(test_id: uuid::Uuid, args: &Args) -> Result<()> {
	let reg = build_registry()?;
	let test_deps = setup_test_deps(test_id).await?;

	let config = test_deps.config().clone();
	let pools = test_deps.pools().clone();

	let db = gas::db::DatabaseKv::new(config.clone(), pools.clone()).await?;

	// Start worker in background
	let worker = Worker::new(reg.handle(), db.clone(), config.clone(), pools.clone());
	let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());
	let worker_handle = tokio::spawn(async move {
		if let Err(err) = worker.start(Some(shutdown_rx)).await {
			tracing::error!(?err, "worker error");
		}
	});

	// Give worker time to start
	tokio::time::sleep(Duration::from_secs(1)).await;

	// Run bombarder
	let result = bombarder_logic(db, config, args).await;

	// Shutdown
	tracing::info!("bombarder complete, shutting down worker");
	let _ = shutdown_tx.send(());
	let _ = tokio::time::timeout(Duration::from_secs(30), worker_handle).await;

	result
}

async fn bombarder_logic(
	db: Arc<dyn gas::db::Database + Sync>,
	config: rivet_config::Config,
	args: &Args,
) -> Result<()> {
	let ray_id = rivet_util::Id::new_v1(config.dc_label());
	let workflow_count = args.workflow_count;
	let signals_per_workflow = args.signals_per_workflow;
	let signal_delay = Duration::from_millis(args.signal_delay_ms);
	let concurrency = args.concurrency;

	tracing::info!(
		workflow_count,
		signals_per_workflow,
		?signal_delay,
		concurrency,
		"starting bombarder"
	);

	// Phase 1: Dispatch LoopSignal workflows (these loop and listen for signals)
	let mut loop_signal_ids = Vec::with_capacity(workflow_count);
	for i in 0..workflow_count {
		let workflow_id = rivet_util::Id::new_v1(config.dc_label());
		let input = serde_json::value::to_raw_value(&workflows::LoopSignalInput {
			max_iterations: signals_per_workflow,
		})?;

		let dispatched_id = db
			.dispatch_workflow(
				ray_id,
				workflow_id,
				"loop_signal",
				None,
				&input,
				false,
			)
			.await?;

		loop_signal_ids.push(dispatched_id);

		if (i + 1) % 10 == 0 {
			tracing::info!(count = i + 1, "dispatched loop_signal workflows");
		}
	}

	tracing::info!(
		count = loop_signal_ids.len(),
		"all loop_signal workflows dispatched"
	);

	// Phase 2: Dispatch BusyLoop workflows (pure loop stress, no signals)
	let busy_count = workflow_count / 2;
	let mut busy_ids = Vec::with_capacity(busy_count);
	for _ in 0..busy_count {
		let workflow_id = rivet_util::Id::new_v1(config.dc_label());
		let input = serde_json::value::to_raw_value(&workflows::BusyLoopInput {
			iterations: 100,
		})?;

		let dispatched_id = db
			.dispatch_workflow(ray_id, workflow_id, "busy_loop", None, &input, false)
			.await?;

		busy_ids.push(dispatched_id);
	}

	tracing::info!(count = busy_ids.len(), "dispatched busy_loop workflows");

	// Phase 3: Dispatch SignalChain workflows (these forward signals to each other)
	let chain_count = workflow_count / 4;
	let mut chain_ids = Vec::with_capacity(chain_count);
	for _ in 0..chain_count {
		let workflow_id = rivet_util::Id::new_v1(config.dc_label());
		let input = serde_json::value::to_raw_value(&workflows::SignalChainInput {
			chain_length: 5,
		})?;

		let dispatched_id = db
			.dispatch_workflow(
				ray_id,
				workflow_id,
				"signal_chain",
				None,
				&input,
				false,
			)
			.await?;

		chain_ids.push(dispatched_id);
	}

	tracing::info!(count = chain_ids.len(), "dispatched signal_chain workflows");

	// Give workflows time to start and begin listening
	tokio::time::sleep(Duration::from_secs(2)).await;

	// Phase 4: Bombard loop_signal workflows with signals
	tracing::info!("starting signal bombardment");

	let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
	let mut signal_handles = Vec::new();
	let total_signals = Arc::new(std::sync::atomic::AtomicUsize::new(0));
	let signal_errors = Arc::new(std::sync::atomic::AtomicUsize::new(0));

	for workflow_id in &loop_signal_ids {
		let db = db.clone();
		let config = config.clone();
		let semaphore = semaphore.clone();
		let workflow_id = *workflow_id;
		let total_signals = total_signals.clone();
		let signal_errors = signal_errors.clone();

		let handle = tokio::spawn(async move {
			for i in 0..signals_per_workflow {
				let _permit = semaphore.acquire().await.unwrap();

				let signal_id = rivet_util::Id::new_v1(config.dc_label());
				let body = serde_json::value::to_raw_value(
					&workflows::PingSignal {
						iteration: i,
						payload: format!("signal-{i}-for-{workflow_id}"),
					},
				)
				.unwrap();

				match db
					.publish_signal(ray_id, workflow_id, signal_id, "ping", &body)
					.await
				{
					Ok(()) => {
						total_signals
							.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
					}
					Err(err) => {
						signal_errors
							.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
						tracing::error!(?err, %workflow_id, iteration = i, "failed to publish signal");
					}
				}

				tokio::time::sleep(signal_delay).await;
			}
		});

		signal_handles.push(handle);
	}

	// Also send trigger signals to chain workflows
	for workflow_id in &chain_ids {
		let db = db.clone();
		let config = config.clone();
		let workflow_id = *workflow_id;

		let handle = tokio::spawn(async move {
			let signal_id = rivet_util::Id::new_v1(config.dc_label());
			let body =
				serde_json::value::to_raw_value(&workflows::TriggerSignal {}).unwrap();

			if let Err(err) = db
				.publish_signal(ray_id, workflow_id, signal_id, "trigger", &body)
				.await
			{
				tracing::error!(?err, %workflow_id, "failed to publish trigger signal");
			}
		});

		signal_handles.push(handle);
	}

	// Wait for all signals to be sent
	for handle in signal_handles {
		let _ = handle.await;
	}

	let total = total_signals.load(std::sync::atomic::Ordering::Relaxed);
	let errors = signal_errors.load(std::sync::atomic::Ordering::Relaxed);
	tracing::info!(total_signals = total, signal_errors = errors, "signal bombardment complete");

	// Phase 5: Monitor workflow completion
	tracing::info!("monitoring workflow completion");

	let all_ids: Vec<_> = loop_signal_ids
		.iter()
		.chain(busy_ids.iter())
		.chain(chain_ids.iter())
		.copied()
		.collect();

	let start = std::time::Instant::now();
	let timeout = Duration::from_secs(120);
	let mut last_report = std::time::Instant::now();

	loop {
		if start.elapsed() > timeout {
			tracing::error!("timeout waiting for workflows to complete");
			break;
		}

		let workflows = db.get_workflows(all_ids.clone()).await?;

		let completed = workflows.iter().filter(|w| w.is_complete()).count();
		let dead = workflows.iter().filter(|w| w.is_dead()).count();
		let active = workflows.len() - completed - dead;

		if last_report.elapsed() > Duration::from_secs(5) {
			tracing::info!(
				total = workflows.len(),
				completed,
				active,
				dead,
				elapsed = ?start.elapsed(),
				"workflow status"
			);
			last_report = std::time::Instant::now();
		}

		if completed == workflows.len() {
			tracing::info!(
				elapsed = ?start.elapsed(),
				"all workflows completed successfully"
			);
			return Ok(());
		}

		if dead > 0 {
			tracing::error!(
				dead,
				"found dead workflows (no wake condition, not completed)"
			);

			// Report the dead workflows
			for wf in &workflows {
				if wf.is_dead() {
					tracing::error!(workflow_id = %wf.workflow_id, "dead workflow");
				}
			}
		}

		tokio::time::sleep(Duration::from_secs(2)).await;
	}

	// Final report
	let workflows = db.get_workflows(all_ids.clone()).await?;
	let completed = workflows.iter().filter(|w| w.is_complete()).count();
	let dead = workflows.iter().filter(|w| w.is_dead()).count();

	tracing::error!(
		total = workflows.len(),
		completed,
		dead,
		active = workflows.len() - completed - dead,
		"final workflow status after timeout"
	);

	if completed < workflows.len() {
		anyhow::bail!(
			"not all workflows completed: {}/{} completed, {} dead",
			completed,
			workflows.len(),
			dead,
		);
	}

	Ok(())
}
