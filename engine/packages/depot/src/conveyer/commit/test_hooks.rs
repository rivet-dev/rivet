#[cfg(debug_assertions)]
use std::sync::Arc;

#[cfg(debug_assertions)]
use parking_lot::Mutex;
#[cfg(debug_assertions)]
use tokio::sync::Notify;

#[cfg(debug_assertions)]
static PAUSE_AFTER_TRUNCATE_CLEANUP: Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>> =
	Mutex::new(None);

#[cfg(debug_assertions)]
pub struct PauseGuard {
	slot: &'static Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>>,
}

#[cfg(debug_assertions)]
pub fn pause_after_truncate_cleanup(database_id: &str) -> (PauseGuard, Arc<Notify>, Arc<Notify>) {
	let reached = Arc::new(Notify::new());
	let release = Arc::new(Notify::new());
	*PAUSE_AFTER_TRUNCATE_CLEANUP.lock() = Some((
		database_id.to_string(),
		Arc::clone(&reached),
		Arc::clone(&release),
	));

	(
		PauseGuard {
			slot: &PAUSE_AFTER_TRUNCATE_CLEANUP,
		},
		reached,
		release,
	)
}

#[cfg(debug_assertions)]
pub(super) async fn maybe_pause_after_truncate_cleanup(database_id: &str) {
	let hook = PAUSE_AFTER_TRUNCATE_CLEANUP
		.lock()
		.as_ref()
		.filter(|(hook_database_id, _, _)| hook_database_id == database_id)
		.map(|(_, reached, release)| (Arc::clone(reached), Arc::clone(release)));

	if let Some((reached, release)) = hook {
		reached.notify_waiters();
		release.notified().await;
	}
}

#[cfg(not(debug_assertions))]
pub(super) async fn maybe_pause_after_truncate_cleanup(_database_id: &str) {}

#[cfg(debug_assertions)]
impl Drop for PauseGuard {
	fn drop(&mut self) {
		*self.slot.lock() = None;
	}
}
