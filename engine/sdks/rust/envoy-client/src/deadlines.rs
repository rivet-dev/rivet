//! Per-request deadlines for the envoy loop. Each in-flight KV, SQLite and remote SQLite
//! request, each retired actor's unacknowledged events, and each recorded HTTP request
//! cancellation has one deadline here, so the
//! loop sleeps until the earliest one and visits only due entries instead of sweeping
//! every request on an interval.

use std::collections::{BTreeSet, HashMap};

use crate::envoy::EnvoyContext;
use crate::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeadlineKey {
	Kv(u32),
	Sqlite(u32),
	RemoteSqlite(u32),
	/// A retired actor generation whose unacknowledged events are kept for replay.
	TerminalEvents(String, u32),
	/// A recorded HTTP request cancellation, kept for its TTL.
	HttpCancellation(crate::tunnel::HttpRequestCancellationKey),
}

/// A deadline queue indexed by key. Each key has at most one deadline. A due key is only
/// a hint: the handler checks its entry's own timestamp, so a stale deadline is harmless.
#[derive(Default)]
pub struct Deadlines {
	queue: BTreeSet<(Instant, DeadlineKey)>,
	at: HashMap<DeadlineKey, Instant>,
}

impl Deadlines {
	/// Sets `key`'s deadline, replacing an earlier or later one.
	pub fn schedule(&mut self, key: DeadlineKey, at: Instant) {
		if let Some(old) = self.at.insert(key.clone(), at) {
			self.queue.remove(&(old, key.clone()));
		}
		self.queue.insert((at, key));
	}

	pub fn cancel(&mut self, key: &DeadlineKey) {
		if let Some(old) = self.at.remove(key) {
			self.queue.remove(&(old, key.clone()));
		}
	}

	pub fn next_at(&self) -> Option<Instant> {
		self.queue.first().map(|(at, _)| *at)
	}

	/// Removes and returns every key due at `now`, in deadline order.
	pub fn pop_due(&mut self, now: Instant) -> Vec<DeadlineKey> {
		let mut due = Vec::new();
		while let Some((at, _)) = self.queue.first() {
			if *at > now {
				break;
			}
			let (_, key) = self.queue.pop_first().expect("checked first entry");
			self.at.remove(&key);
			due.push(key);
		}
		due
	}

	pub fn len(&self) -> usize {
		self.at.len()
	}

	pub fn is_empty(&self) -> bool {
		self.at.is_empty()
	}

	pub fn contains(&self, key: &DeadlineKey) -> bool {
		self.at.contains_key(key)
	}
}

/// Sleeps until `at`, or forever when there is no deadline.
pub async fn sleep_until_deadline(at: Option<Instant>) {
	match at {
		Some(at) => crate::utils::sleep(at.saturating_duration_since(Instant::now())).await,
		None => std::future::pending::<()>().await,
	}
}

/// Expires every request and retired event set whose deadline has passed.
pub fn expire_due(ctx: &mut EnvoyContext) {
	expire_due_at(ctx, Instant::now());
}

/// `expire_due` at an explicit time, for callers that drive time themselves.
pub fn expire_due_at(ctx: &mut EnvoyContext, now: Instant) {
	for key in ctx.deadlines.pop_due(now) {
		match key {
			DeadlineKey::Kv(request_id) => crate::kv::expire_kv_request(ctx, request_id, now),
			DeadlineKey::Sqlite(request_id) => {
				crate::sqlite::expire_sqlite_request(ctx, request_id, now)
			}
			DeadlineKey::RemoteSqlite(request_id) => {
				crate::sqlite::expire_remote_sqlite_request(ctx, request_id, now)
			}
			DeadlineKey::TerminalEvents(actor_id, generation) => {
				crate::events::expire_terminal_events(ctx, actor_id, generation, now)
			}
			DeadlineKey::HttpCancellation(key) => {
				crate::tunnel::expire_http_request_cancellation(ctx, key, now)
			}
		}
	}
}
