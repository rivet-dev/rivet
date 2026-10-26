//! Explicitly owned config runtime; constructing Config or EngineContext starts no background work.
use crate::cluster_config_machine::{
	ClusterConfigStateMachine,
	event::Event,
	executor::ConfigExecutor,
	snapshot::Snapshot,
	state::State,
	types::{Purpose, Settings},
};
use anyhow::{Context, Result};
use object_store::ObjectStore;
use rivet_config::Config;
use rivet_env::Env;
use rivet_state_machine_ephemeral::{Runner, RunnerHandle};
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};
use tokio::sync::watch;

/// Owns the runner and all its I/O. Dropping cancels; close also joins every task.
pub struct ConfigRuntime {
	runner: Option<RunnerHandle<ClusterConfigStateMachine>>,
	executor: Arc<ConfigExecutor>,
	alive: Arc<AtomicBool>,
}
impl ConfigRuntime {
	/// Start a per-process follower or bounded operation without requiring UPS, Topology or Depot.
	pub fn start(
		env: Arc<dyn Env>,
		store: Arc<dyn ObjectStore>,
		current: Config,
		purpose: Purpose,
		settings: Settings,
	) -> Result<Self> {
		let state = State::new(purpose, settings)?;
		let alive = Arc::new(AtomicBool::new(true));
		let mut executor = None;
		let tasks = env.task_scope();
		let runner = Runner::spawn_with_executor(env, state, |snapshot| {
			let created = Arc::new(ConfigExecutor::new(
				store,
				current,
				snapshot,
				alive.clone(),
				tasks,
			));
			executor = Some(created.clone());
			created
		});
		Ok(Self {
			runner: Some(runner),
			executor: executor.expect("executor constructed synchronously"),
			alive,
		})
	}
	/// Diagnostics/completion; application settings continue to use Config::dynamic_watch().
	pub fn subscribe(&self) -> watch::Receiver<Snapshot> {
		self.runner
			.as_ref()
			.expect("open config runtime")
			.snapshot
			.clone()
	}
	/// Requests an immediate read. The bounded runner queue coalesces repeated requests.
	pub fn refresh(&self) {
		let _ = self
			.runner
			.as_ref()
			.expect("open config runtime")
			.inbox
			.try_send(Event::RefreshRequest);
	}
	/// Wait for bootstrap/update completion; Follow intentionally runs until stopped.
	pub async fn completed(&self) -> Result<()> {
		let mut snapshot = self.subscribe();
		let result = snapshot
			.wait_for(|view| view.result.is_some())
			.await
			.context("config runner ended before completion")?
			.result
			.clone()
			.expect("completed config result");
		result.map_err(anyhow::Error::msg)
	}
	/// Cancel local dispatch and join work. A previously sent conditional PUT can still commit.
	pub async fn close(mut self) -> Result<()> {
		self.alive.store(false, Ordering::SeqCst);
		self.executor.cancel();
		let mut runner = self
			.runner
			.take()
			.context("config runtime already closed")?;
		if runner.inbox.send(Event::StopRequest).await.is_ok() {
			let _ = runner
				.snapshot
				.wait_for(|snapshot| snapshot.result.is_some())
				.await;
		}
		self.executor.shutdown().await;
		drop(runner.inbox);
		runner
			.stopped
			.await
			.context("config runner failed during shutdown")
	}
}
impl Drop for ConfigRuntime {
	fn drop(&mut self) {
		self.alive.store(false, Ordering::SeqCst);
		self.executor.cancel();
	}
}
/// Bootstrap and CLI operations share the same core but own bounded, short-lived runners.
pub(crate) async fn run_once(
	env: Arc<dyn Env>,
	store: Arc<dyn ObjectStore>,
	current: &Config,
	purpose: Purpose,
) -> Result<()> {
	let runtime = ConfigRuntime::start(env, store, current.clone(), purpose, Settings::default())?;
	let result = runtime.completed().await;
	let closed = runtime.close().await;
	result.and(closed)
}
