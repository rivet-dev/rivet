//! Compaction coordinator and worker entry points.

mod shard;
mod worker;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{self, MissedTickBehavior};

use crate::engine::SqliteEngine;

type WorkerFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
type SpawnWorker = Arc<dyn Fn(String, Arc<SqliteEngine>) -> WorkerFuture + Send + Sync + 'static>;

const DEFAULT_REAP_INTERVAL: Duration = Duration::from_millis(100);

pub struct CompactionCoordinator {
	rx: mpsc::UnboundedReceiver<String>,
	engine: Arc<SqliteEngine>,
	workers: HashMap<String, JoinHandle<()>>,
	spawn_worker: SpawnWorker,
	reap_interval: Duration,
}

impl CompactionCoordinator {
	pub fn new(rx: mpsc::UnboundedReceiver<String>, engine: Arc<SqliteEngine>) -> Self {
		Self::with_worker(rx, engine, DEFAULT_REAP_INTERVAL, |actor_id, engine| {
			Box::pin(default_compaction_worker(actor_id, engine))
		})
	}

	pub async fn run(rx: mpsc::UnboundedReceiver<String>, engine: Arc<SqliteEngine>) {
		Self::new(rx, engine).run_loop().await;
	}

	fn with_worker<F>(
		rx: mpsc::UnboundedReceiver<String>,
		engine: Arc<SqliteEngine>,
		reap_interval: Duration,
		spawn_worker: F,
	) -> Self
	where
		F: Fn(String, Arc<SqliteEngine>) -> WorkerFuture + Send + Sync + 'static,
	{
		Self {
			rx,
			engine,
			workers: HashMap::new(),
			spawn_worker: Arc::new(spawn_worker),
			reap_interval,
		}
	}

	async fn run_loop(mut self) {
		let mut reap_interval = time::interval(self.reap_interval);
		reap_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

		loop {
			tokio::select! {
				maybe_actor_id = self.rx.recv() => {
					match maybe_actor_id {
						Some(actor_id) => self.spawn_worker_if_needed(actor_id),
						None => {
							self.reap_finished_workers();
							self.abort_workers();
							break;
						}
					}
				}
				_ = reap_interval.tick() => self.reap_finished_workers(),
			}
		}
	}

	fn spawn_worker_if_needed(&mut self, actor_id: String) {
		if self
			.workers
			.get(&actor_id)
			.is_some_and(|handle| !handle.is_finished())
		{
			return;
		}

		self.workers.remove(&actor_id);

		let worker = (self.spawn_worker)(actor_id.clone(), Arc::clone(&self.engine));
		let handle = tokio::spawn(worker);
		self.workers.insert(actor_id, handle);
	}

	fn reap_finished_workers(&mut self) {
		self.workers.retain(|_, handle| !handle.is_finished());
	}

	fn abort_workers(&mut self) {
		for (_, handle) in self.workers.drain() {
			handle.abort();
		}
	}
}

async fn default_compaction_worker(actor_id: String, engine: Arc<SqliteEngine>) {
	if std::env::var("RIVET_SQLITE_DISABLE_COMPACTION").is_ok() {
		tracing::debug!(%actor_id, "sqlite compaction disabled by environment");
		return;
	}
	if let Err(err) = engine.compact_default_batch(&actor_id).await {
		tracing::warn!(?err, %actor_id, "sqlite compaction worker failed");
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use parking_lot::Mutex;
	use std::collections::VecDeque;
	use tokio::sync::{Notify, mpsc};
	use tokio::time::{Duration, timeout};

	use super::CompactionCoordinator;
	use crate::engine::SqliteEngine;
	use crate::test_utils::test_db;

	#[tokio::test]
	async fn sending_same_actor_id_twice_only_spawns_one_worker() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let engine = std::sync::Arc::new(engine);
		let (tx, rx) = mpsc::unbounded_channel();
		let (spawned_tx, mut spawned_rx) = mpsc::unbounded_channel();
		let release = std::sync::Arc::new(Notify::new());

		let coordinator = tokio::spawn(
			CompactionCoordinator::with_worker(rx, engine, Duration::from_millis(10), {
				let release = std::sync::Arc::clone(&release);
				move |actor_id, _engine| {
					let spawned_tx = spawned_tx.clone();
					let release = std::sync::Arc::clone(&release);
					Box::pin(async move {
						let _ = spawned_tx.send(actor_id);
						release.notified().await;
					})
				}
			})
			.run_loop(),
		);

		tx.send("actor-a".to_string())?;
		assert_eq!(spawned_rx.recv().await, Some("actor-a".to_string()));

		tx.send("actor-a".to_string())?;
		assert!(
			timeout(Duration::from_millis(50), spawned_rx.recv())
				.await
				.is_err()
		);

		release.notify_waiters();
		drop(tx);
		coordinator.await?;

		Ok(())
	}

	#[tokio::test]
	async fn sending_actor_again_after_worker_completes_spawns_new_worker() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let engine = std::sync::Arc::new(engine);
		let (tx, rx) = mpsc::unbounded_channel();
		let (spawned_tx, mut spawned_rx) = mpsc::unbounded_channel();
		let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
		let releases = std::sync::Arc::new(Mutex::new(VecDeque::from(vec![
			std::sync::Arc::new(Notify::new()),
			std::sync::Arc::new(Notify::new()),
		])));

		let first_release = {
			let releases = releases.lock();
			std::sync::Arc::clone(&releases[0])
		};
		let second_release = {
			let releases = releases.lock();
			std::sync::Arc::clone(&releases[1])
		};

		let coordinator = tokio::spawn(
			CompactionCoordinator::with_worker(rx, engine, Duration::from_millis(10), {
				let releases = std::sync::Arc::clone(&releases);
				move |actor_id, _engine| {
					let spawned_tx = spawned_tx.clone();
					let completed_tx = completed_tx.clone();
					let release = releases
						.lock()
						.pop_front()
						.expect("each spawned worker should have a release gate");

					Box::pin(async move {
						let _ = spawned_tx.send(actor_id.clone());
						release.notified().await;
						let _ = completed_tx.send(actor_id);
					})
				}
			})
			.run_loop(),
		);

		tx.send("actor-a".to_string())?;
		assert_eq!(spawned_rx.recv().await, Some("actor-a".to_string()));

		first_release.notify_waiters();
		assert_eq!(completed_rx.recv().await, Some("actor-a".to_string()));

		tx.send("actor-a".to_string())?;
		assert_eq!(
			timeout(Duration::from_millis(50), spawned_rx.recv()).await?,
			Some("actor-a".to_string())
		);

		second_release.notify_waiters();
		assert_eq!(completed_rx.recv().await, Some("actor-a".to_string()));

		drop(tx);
		coordinator.await?;

		Ok(())
	}
}
