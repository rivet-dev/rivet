// Worker health + concurrent stats counters. Mirrors the global state at
// the top of scripts/counter-latency.ts.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WorkerHealth {
	/// WS handshake in progress.
	Connecting,
	/// WS open; ping phase in progress (waiting for pong id=1 or id=2).
	Pinging,
	/// Ping phase completed cleanly; worker is in steady state or has emitted its rolling rtt
	/// line.
	Connected,
	/// Worker was Connected but has been flagged slow — STALL fired, MESSAGE-GAP fired, or
	/// SLOW-SQL fired. Stays in this state for the remainder of the cycle.
	ConnectedSlow,
	/// Phase 1 failed, WS errored, or disconnect logged.
	Failed,
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
	pub stalls: AtomicI64,
	pub unclean_failures_or_disconnects: AtomicI64,
	pub agent2_queries: AtomicI64,
	pub agent2_reads: AtomicI64,
	pub agent2_mutations: AtomicI64,
	pub agent2_tx: AtomicI64,
	pub agent2_other: AtomicI64,
	pub agent2_rows: AtomicI64,
	pub agent2_query_errors: AtomicI64,
	pub agent2_slow_queries: AtomicI64,
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
			stalls: AtomicI64::new(0),
			unclean_failures_or_disconnects: AtomicI64::new(0),
			agent2_queries: AtomicI64::new(0),
			agent2_reads: AtomicI64::new(0),
			agent2_mutations: AtomicI64::new(0),
			agent2_tx: AtomicI64::new(0),
			agent2_other: AtomicI64::new(0),
			agent2_rows: AtomicI64::new(0),
			agent2_query_errors: AtomicI64::new(0),
			agent2_slow_queries: AtomicI64::new(0),
		}
	}
}

pub struct State {
	pub stats: ConcurrentStats,
	pub workers_started: AtomicI64,
	pub stopping: AtomicBool,
	pub worker_health: Mutex<HashMap<u32, WorkerHealth>>,
}

/// Per-state counts returned by `count_worker_health`, in the same order as the scoreboard column
/// labels.
pub struct HealthCounts {
	pub connecting: i64,
	pub pinging: i64,
	pub connected: i64,
	pub connected_slow: i64,
	pub failed: i64,
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

	pub fn drop_worker_health(&self, worker: u32) {
		self.worker_health.lock().unwrap().remove(&worker);
	}

	/// Promote a worker from `Connected` to `ConnectedSlow`. No-op if the worker is in any other
	/// state (we don't want to mark a failed worker as slow, nor downgrade a worker that's still
	/// in the ping phase).
	pub fn flag_worker_slow(&self, worker: u32) {
		let mut map = self.worker_health.lock().unwrap();
		if let Some(WorkerHealth::Connected) = map.get(&worker) {
			map.insert(worker, WorkerHealth::ConnectedSlow);
		}
	}

	pub fn count_worker_health(&self) -> HealthCounts {
		let map = self.worker_health.lock().unwrap();
		let mut counts = HealthCounts {
			connecting: 0,
			pinging: 0,
			connected: 0,
			connected_slow: 0,
			failed: 0,
		};
		for s in map.values() {
			match s {
				WorkerHealth::Connecting => counts.connecting += 1,
				WorkerHealth::Pinging => counts.pinging += 1,
				WorkerHealth::Connected => counts.connected += 1,
				WorkerHealth::ConnectedSlow => counts.connected_slow += 1,
				WorkerHealth::Failed => counts.failed += 1,
			}
		}
		counts
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
