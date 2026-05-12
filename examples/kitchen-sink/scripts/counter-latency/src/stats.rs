// Worker health + concurrent stats counters. Mirrors the global state at
// the top of scripts/counter-latency.ts.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WorkerHealth {
	Pending,
	Healthy,
	Warning,
	Ended,
}

pub struct ConcurrentStats {
	pub connects: AtomicI64,
	pub reconnects: AtomicI64,
	pub first_messages: AtomicI64,
	pub connect_errors: AtomicI64,
	pub websocket_errors: AtomicI64,
	pub disconnects: AtomicI64,
	pub message_gaps: AtomicI64,
	pub slow_sql: AtomicI64,
	pub unclean_failures_or_disconnects: AtomicI64,
}

impl ConcurrentStats {
	pub fn new() -> Self {
		Self {
			connects: AtomicI64::new(0),
			reconnects: AtomicI64::new(0),
			first_messages: AtomicI64::new(0),
			connect_errors: AtomicI64::new(0),
			websocket_errors: AtomicI64::new(0),
			disconnects: AtomicI64::new(0),
			message_gaps: AtomicI64::new(0),
			slow_sql: AtomicI64::new(0),
			unclean_failures_or_disconnects: AtomicI64::new(0),
		}
	}
}

pub struct State {
	pub stats: ConcurrentStats,
	pub workers_started: AtomicI64,
	pub stopping: AtomicBool,
	pub worker_health: Mutex<HashMap<u32, WorkerHealth>>,
}

impl State {
	pub fn new() -> Self {
		Self {
			stats: ConcurrentStats::new(),
			workers_started: AtomicI64::new(0),
			stopping: AtomicBool::new(false),
			worker_health: Mutex::new(HashMap::new()),
		}
	}

	pub fn set_worker_health(&self, worker: u32, state: WorkerHealth) {
		self.worker_health.lock().unwrap().insert(worker, state);
	}

	pub fn flag_worker_warning(&self, worker: u32) {
		let mut map = self.worker_health.lock().unwrap();
		if let Some(WorkerHealth::Healthy) = map.get(&worker) {
			map.insert(worker, WorkerHealth::Warning);
		}
	}

	pub fn count_worker_health(&self) -> (i64, i64, i64, i64) {
		let map = self.worker_health.lock().unwrap();
		let mut pending = 0i64;
		let mut healthy = 0i64;
		let mut warning = 0i64;
		let mut ended = 0i64;
		for s in map.values() {
			match s {
				WorkerHealth::Pending => pending += 1,
				WorkerHealth::Healthy => healthy += 1,
				WorkerHealth::Warning => warning += 1,
				WorkerHealth::Ended => ended += 1,
			}
		}
		(pending, healthy, warning, ended)
	}

	pub fn workers_started(&self) -> i64 {
		self.workers_started.load(Ordering::Relaxed)
	}

	pub fn set_workers_started(&self, n: i64) {
		self.workers_started.store(n, Ordering::Relaxed);
	}

	pub fn stopping(&self) -> bool {
		self.stopping.load(Ordering::Relaxed)
	}

	pub fn set_stopping(&self) {
		self.stopping.store(true, Ordering::Relaxed);
	}
}
