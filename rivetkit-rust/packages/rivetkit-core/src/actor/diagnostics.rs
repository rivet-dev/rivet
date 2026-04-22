use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use scc::HashMap as SccHashMap;

const WARNING_WINDOW: Duration = Duration::from_secs(30);
const WARNING_LIMIT: usize = 3;

// Forced-sync: warning windows are updated from synchronous diagnostics paths.
static GLOBAL_WARNINGS: OnceLock<SccHashMap<String, Arc<Mutex<WarningWindow>>>> = OnceLock::new();
static ACTOR_WARNINGS: OnceLock<SccHashMap<String, Arc<Mutex<WarningWindow>>>> = OnceLock::new();

#[derive(Debug)]
pub(crate) struct ActorDiagnostics {
	actor_id: String,
	warnings: SccHashMap<String, Arc<Mutex<WarningWindow>>>,
}

impl ActorDiagnostics {
	pub(crate) fn new(actor_id: impl Into<String>) -> Self {
		Self {
			actor_id: actor_id.into(),
			warnings: SccHashMap::new(),
		}
	}

	pub(crate) fn record(&self, kind: &'static str) -> Option<WarningSuppression> {
		let per_actor = record_limited_warning(&self.warnings, kind.to_owned(), Instant::now());
		let global = record_limited_warning(global_warnings(), kind.to_owned(), Instant::now());

		if per_actor.emit && global.emit {
			Some(WarningSuppression {
				actor_id: self.actor_id.clone(),
				per_actor_suppressed: per_actor.suppressed,
				global_suppressed: global.suppressed,
			})
		} else {
			None
		}
	}
}

pub(crate) fn record_actor_warning(
	actor_id: &str,
	kind: &'static str,
) -> Option<WarningSuppression> {
	let actor_key = format!("{actor_id}:{kind}");
	let per_actor = record_limited_warning(actor_warnings(), actor_key, Instant::now());
	let global = record_limited_warning(global_warnings(), kind.to_owned(), Instant::now());

	if per_actor.emit && global.emit {
		Some(WarningSuppression {
			actor_id: actor_id.to_owned(),
			per_actor_suppressed: per_actor.suppressed,
			global_suppressed: global.suppressed,
		})
	} else {
		None
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WarningSuppression {
	pub(crate) actor_id: String,
	pub(crate) per_actor_suppressed: u64,
	pub(crate) global_suppressed: u64,
}

#[derive(Debug)]
struct WarningDecision {
	emit: bool,
	suppressed: u64,
}

#[derive(Debug)]
struct WarningWindow {
	started_at: Instant,
	emitted: usize,
	suppressed: u64,
}

impl WarningWindow {
	fn new(now: Instant) -> Self {
		Self {
			started_at: now,
			emitted: 0,
			suppressed: 0,
		}
	}

	fn record(&mut self, now: Instant) -> WarningDecision {
		if now.duration_since(self.started_at) >= WARNING_WINDOW {
			let suppressed = self.suppressed;
			self.started_at = now;
			self.emitted = 1;
			self.suppressed = 0;
			return WarningDecision {
				emit: true,
				suppressed,
			};
		}

		if self.emitted < WARNING_LIMIT {
			self.emitted += 1;
			WarningDecision {
				emit: true,
				suppressed: 0,
			}
		} else {
			self.suppressed += 1;
			WarningDecision {
				emit: false,
				suppressed: 0,
			}
		}
	}
}

fn record_limited_warning(
	warnings: &SccHashMap<String, Arc<Mutex<WarningWindow>>>,
	key: String,
	now: Instant,
) -> WarningDecision {
	let window = warnings
		.read_sync(&key, |_, window| window.clone())
		.unwrap_or_else(|| {
			let window = Arc::new(Mutex::new(WarningWindow::new(now)));
			let _ = warnings.insert_sync(key, window.clone());
			window
		});

	window.lock().record(now)
}

fn global_warnings() -> &'static SccHashMap<String, Arc<Mutex<WarningWindow>>> {
	GLOBAL_WARNINGS.get_or_init(SccHashMap::new)
}

fn actor_warnings() -> &'static SccHashMap<String, Arc<Mutex<WarningWindow>>> {
	ACTOR_WARNINGS.get_or_init(SccHashMap::new)
}
