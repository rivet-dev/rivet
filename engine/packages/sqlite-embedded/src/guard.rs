//! Resource and statement guards for a connection that runs SQL sent by an untrusted peer.
//!
//! An engine that executes an Envoy's SQL on its own threads bounds the work of each command,
//! the database's size, the largest value and each statement's result size, and refuses statements that would change the
//! VFS's journal, page or locking assumptions or open another database file.
//!
//! Every connection, guarded or not, also gets a progress handler through [`CommandProgress`],
//! which lets an owner interrupt the command running on it. The handler is installed when the
//! connection opens rather than when an interrupt is wanted, for two reasons. SQLite computes a
//! statement's progress limit once when its step begins, so one long `sqlite3_step`, such as an
//! aggregate over a join that emits no rows, never calls a handler installed during it. And
//! calling `sqlite3_progress_handler` from the interrupting thread would block on the
//! connection's mutex until that step ends. The handler costs one atomic load per
//! `PROGRESS_INTERVAL` virtual machine instructions; point reads and small writes run fewer
//! instructions than that and never call it.
use std::{
	ffi::CStr,
	os::raw::{c_char, c_int, c_void},
	sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use anyhow::{Result, ensure};
use libsqlite3_sys::{
	SQLITE_ATTACH, SQLITE_DENY, SQLITE_LIMIT_LENGTH, SQLITE_OK, SQLITE_PRAGMA, sqlite3,
	sqlite3_limit, sqlite3_progress_handler, sqlite3_set_authorizer,
};

/// SQLite virtual machine instructions between two progress callbacks.
const PROGRESS_INTERVAL: c_int = 1000;

/// Pragmas a guarded connection must not set. Reading them stays allowed.
const DENIED_PRAGMA_WRITES: &[&str] = &[
	"temp_store",
	"temp_store_directory",
	"data_store_directory",
	"max_page_count",
	"page_size",
	"journal_mode",
	"synchronous",
	"locking_mode",
	"cache_spill",
	"auto_vacuum",
	"journal_size_limit",
];

#[derive(Clone, Copy, Debug)]
pub struct ConnectionGuard {
	/// The database's largest size in pages.
	pub max_page_count: u32,
	/// The largest string or blob in bytes.
	pub max_length_bytes: i32,
	/// Progress callbacks one command may take before SQLite interrupts it. Each callback is
	/// `PROGRESS_INTERVAL` virtual machine instructions.
	pub max_progress_callbacks: u64,
	/// The most logical bytes one statement's result may hold. Each value costs 8 bytes plus
	/// its text or blob length, and the statement stops as soon as its result passes the cap.
	pub max_result_bytes: u64,
}

impl Default for ConnectionGuard {
	fn default() -> Self {
		Self {
			max_page_count: 65_536,
			max_length_bytes: 24 * 1024 * 1024,
			max_progress_callbacks: 100_000,
			max_result_bytes: 24 * 1024 * 1024,
		}
	}
}

/// The progress handler's state for one connection: the guard's work budget, if a guard is
/// installed, and whether its owner interrupted the running command. SQLite keeps a pointer to
/// it for the progress callback, so it must outlive every statement the connection runs: the
/// owner drops it only once the connection is closed or its handler is removed with
/// [`Self::uninstall`].
pub(crate) struct CommandProgress {
	guard: Option<InstalledGuard>,
	/// Set once by [`Self::interrupt`] and never cleared, so every progress callback after it
	/// stops the command. Only an owner about to close the database sets it: a close, or a
	/// failed start just before it drops its held commits and closes. Every SQL command after it
	/// fails once it reaches a progress callback; commands that run no SQL, such as a discard of
	/// held commits, and short statements are unaffected.
	interrupted: AtomicBool,
}

impl CommandProgress {
	pub(crate) fn new(guard: Option<ConnectionGuard>) -> Box<Self> {
		Box::new(Self {
			guard: guard.map(InstalledGuard::new),
			interrupted: AtomicBool::new(false),
		})
	}

	/// Installs the guard, if any, and the progress handler.
	pub(crate) fn install(&self, db: *mut sqlite3) -> Result<()> {
		if let Some(guard) = &self.guard {
			guard.install(db)?;
		}
		// SAFETY: `self` is boxed, and its owner keeps it alive until the connection is closed
		// or the handler is removed with `uninstall`, as the type requires.
		unsafe {
			sqlite3_progress_handler(
				db,
				PROGRESS_INTERVAL,
				Some(progress),
				self as *const Self as *mut c_void,
			);
		}
		Ok(())
	}

	/// Removes the progress handler from `db`, for a connection that outlives this state, such
	/// as one handed to another thread to close.
	///
	/// # Safety
	///
	/// `db` must be a live connection that the caller owns, and no thread may run a statement
	/// on it, or otherwise use it, during the call.
	pub(crate) unsafe fn uninstall(db: *mut sqlite3) {
		// SAFETY: the caller guarantees `db` is live and idle, as this function requires.
		unsafe {
			sqlite3_progress_handler(db, 0, None, std::ptr::null_mut());
		}
	}

	pub(crate) fn guard(&self) -> Option<&InstalledGuard> {
		self.guard.as_ref()
	}

	/// Makes the running command fail with `SQLITE_INTERRUPT` at its next progress callback,
	/// within `PROGRESS_INTERVAL` virtual machine instructions of its next step. A command
	/// blocked in a VFS request stops once that request returns. Unlike
	/// `sqlite3_interrupt`, which SQLite clears when a statement starts with none active, this
	/// also stops a statement the command starts later, such as the next one in a batch.
	pub(crate) fn interrupt(&self) {
		self.interrupted.store(true, Ordering::Release);
	}

	/// Whether [`Self::interrupt`] ran.
	pub(crate) fn interrupted(&self) -> bool {
		self.interrupted.load(Ordering::Acquire)
	}

	/// Whether SQLite should stop the running command now.
	fn stops_command(&self) -> bool {
		if self.interrupted() {
			return true;
		}
		self.guard
			.as_ref()
			.is_some_and(|guard| guard.exhausted_budget())
	}
}

/// A guard installed on one open connection, owned by its [`CommandProgress`].
pub(crate) struct InstalledGuard {
	config: ConnectionGuard,
	progress_callbacks: AtomicU64,
}

impl InstalledGuard {
	fn new(config: ConnectionGuard) -> Self {
		Self {
			config,
			progress_callbacks: AtomicU64::new(0),
		}
	}

	/// Sets the size cap, then installs the length limit and the authorizer. The size cap is
	/// set first because the authorizer then refuses to change it. The work budget is checked by
	/// the [`CommandProgress`] progress handler.
	fn install(&self, db: *mut sqlite3) -> Result<()> {
		crate::vfs::sqlite_exec(
			db,
			&format!("PRAGMA max_page_count = {};", self.config.max_page_count),
		)
		.map_err(anyhow::Error::msg)?;
		unsafe {
			sqlite3_limit(db, SQLITE_LIMIT_LENGTH, self.config.max_length_bytes);
		}
		let rc = unsafe { sqlite3_set_authorizer(db, Some(authorize), std::ptr::null_mut()) };
		ensure!(
			rc == SQLITE_OK,
			"failed to install the SQLite authorizer: {rc}"
		);
		Ok(())
	}

	/// The result byte cap for each statement on this connection.
	pub(crate) fn max_result_bytes(&self) -> u64 {
		self.config.max_result_bytes
	}

	/// Starts a new command's work budget.
	pub(crate) fn reset(&self) {
		self.progress_callbacks.store(0, Ordering::Relaxed);
	}

	/// Counts one progress callback and reports whether the command used up its budget.
	fn exhausted_budget(&self) -> bool {
		let callbacks = self.progress_callbacks.fetch_add(1, Ordering::Relaxed) + 1;
		callbacks > self.config.max_progress_callbacks
	}
}

extern "C" fn progress(context: *mut c_void) -> c_int {
	let progress = unsafe { &*(context as *const CommandProgress) };
	c_int::from(progress.stops_command())
}

extern "C" fn authorize(
	_context: *mut c_void,
	action: c_int,
	first: *const c_char,
	second: *const c_char,
	_database: *const c_char,
	_trigger: *const c_char,
) -> c_int {
	let text = |value: *const c_char| -> Option<String> {
		if value.is_null() {
			return None;
		}
		Some(
			unsafe { CStr::from_ptr(value) }
				.to_string_lossy()
				.into_owned(),
		)
	};
	match action {
		// An empty filename attaches a private temporary database, which stays in process.
		SQLITE_ATTACH if text(first).is_some_and(|name| !name.is_empty()) => SQLITE_DENY,
		SQLITE_PRAGMA if second.is_null() => SQLITE_OK,
		SQLITE_PRAGMA => {
			let name = text(first).unwrap_or_default().to_ascii_lowercase();
			if DENIED_PRAGMA_WRITES.contains(&name.as_str()) {
				SQLITE_DENY
			} else {
				SQLITE_OK
			}
		}
		_ => SQLITE_OK,
	}
}
