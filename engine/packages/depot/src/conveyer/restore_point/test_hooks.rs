#[cfg(debug_assertions)]
use std::sync::Arc;

#[cfg(debug_assertions)]
use parking_lot::Mutex;
#[cfg(debug_assertions)]
use tokio::sync::Notify;

#[cfg(debug_assertions)]
static PAUSE_AFTER_RESOLVE: Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>> = Mutex::new(None);
#[cfg(debug_assertions)]
static FAIL_AFTER_RESTORE_ROLLBACK: Mutex<Option<String>> = Mutex::new(None);

#[cfg(debug_assertions)]
pub struct PauseGuard {
	slot: &'static Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>>,
}

#[cfg(debug_assertions)]
pub struct FailureGuard {
	slot: &'static Mutex<Option<String>>,
}

#[cfg(debug_assertions)]
pub fn pause_after_resolve(database_id: &str) -> (PauseGuard, Arc<Notify>, Arc<Notify>) {
	let reached = Arc::new(Notify::new());
	let release = Arc::new(Notify::new());
	*PAUSE_AFTER_RESOLVE.lock() = Some((
		database_id.to_string(),
		Arc::clone(&reached),
		Arc::clone(&release),
	));

	(
		PauseGuard {
			slot: &PAUSE_AFTER_RESOLVE,
		},
		reached,
		release,
	)
}

#[cfg(debug_assertions)]
pub fn fail_after_restore_rollback(database_id: &str) -> FailureGuard {
	*FAIL_AFTER_RESTORE_ROLLBACK.lock() = Some(database_id.to_string());

	FailureGuard {
		slot: &FAIL_AFTER_RESTORE_ROLLBACK,
	}
}

#[cfg(debug_assertions)]
pub(super) async fn maybe_pause_after_resolve(database_id: &str) {
	let hook = {
		let mut slot = PAUSE_AFTER_RESOLVE.lock();
		if slot
			.as_ref()
			.is_some_and(|(hook_database_id, _, _)| hook_database_id == database_id)
		{
			slot.take().map(|(_, reached, release)| (reached, release))
		} else {
			None
		}
	};

	if let Some((reached, release)) = hook {
		reached.notify_waiters();
		release.notified().await;
	}
}

#[cfg(debug_assertions)]
pub(super) fn maybe_fail_after_restore_rollback(database_id: &str) -> anyhow::Result<()> {
	let should_fail = {
		let mut slot = FAIL_AFTER_RESTORE_ROLLBACK.lock();
		if slot
			.as_ref()
			.is_some_and(|hook_database_id| hook_database_id == database_id)
		{
			slot.take();
			true
		} else {
			false
		}
	};

	if should_fail {
		anyhow::bail!("injected failure after sqlite restore rollback");
	}

	Ok(())
}

#[cfg(not(debug_assertions))]
pub(super) async fn maybe_pause_after_resolve(_database_id: &str) {}

#[cfg(not(debug_assertions))]
pub(super) fn maybe_fail_after_restore_rollback(_database_id: &str) -> anyhow::Result<()> {
	Ok(())
}

#[cfg(debug_assertions)]
impl Drop for PauseGuard {
	fn drop(&mut self) {
		*self.slot.lock() = None;
	}
}

#[cfg(debug_assertions)]
impl Drop for FailureGuard {
	fn drop(&mut self) {
		*self.slot.lock() = None;
	}
}
