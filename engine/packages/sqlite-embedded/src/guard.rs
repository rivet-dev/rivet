//! Resource and statement guards for a connection that runs SQL sent by an untrusted peer.
//!
//! An engine that executes an Envoy's SQL on its own threads bounds the work of each command,
//! the database's size, the largest value and each statement's result size, and refuses statements that would change the
//! VFS's journal, page or locking assumptions or open another database file.
use std::{
	ffi::CStr,
	os::raw::{c_char, c_int, c_void},
	sync::atomic::{AtomicU64, Ordering},
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

/// A guard installed on one open connection. It must outlive the connection, because SQLite
/// keeps a pointer to it for the progress callback.
pub(crate) struct InstalledGuard {
	config: ConnectionGuard,
	progress_callbacks: AtomicU64,
}

impl InstalledGuard {
	pub(crate) fn new(config: ConnectionGuard) -> Box<Self> {
		Box::new(Self {
			config,
			progress_callbacks: AtomicU64::new(0),
		})
	}

	/// Sets the size cap, then installs the length limit, the progress handler and the
	/// authorizer. The size cap is set first because the authorizer then refuses to change it.
	pub(crate) fn install(&self, db: *mut sqlite3) -> Result<()> {
		crate::vfs::sqlite_exec(
			db,
			&format!("PRAGMA max_page_count = {};", self.config.max_page_count),
		)
		.map_err(anyhow::Error::msg)?;
		unsafe {
			sqlite3_limit(db, SQLITE_LIMIT_LENGTH, self.config.max_length_bytes);
			sqlite3_progress_handler(
				db,
				PROGRESS_INTERVAL,
				Some(progress),
				self as *const Self as *mut c_void,
			);
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
}

extern "C" fn progress(context: *mut c_void) -> c_int {
	let guard = unsafe { &*(context as *const InstalledGuard) };
	let callbacks = guard.progress_callbacks.fetch_add(1, Ordering::Relaxed) + 1;
	c_int::from(callbacks > guard.config.max_progress_callbacks)
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
