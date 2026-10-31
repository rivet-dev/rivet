//! The testing VFS: real SQLite over a pass-through client, as the production VFS will be,
//! whose misses go to the harness [`Server`].
//!
//! Historical golden runs retain consumed pages optimistically; acceptance pressure runs opt
//! into native whole-page handoff consumption. Both modes keep the transaction's dirty
//! pages, and a buffer of the pages the server sent. A read SQLite makes is served from the
//! dirty pages, then the buffer, and otherwise costs one round trip. Every read is recorded as
//! an access, which is the trace the oracle strategy and the replay use.
#![allow(
	clippy::disallowed_methods,
	reason = "the testing VFS is a test harness with no runtime environment"
)]

use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::Arc;

use bytes::Bytes;
use libsqlite3_sys::*;
use parking_lot::Mutex;

use super::server::Server;
use super::store::PAGE;
use crate::page::Pgno;
use crate::prefetcher::Source;

/// One page read SQLite made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Access {
	pub pgno: Pgno,
	/// Served by the client's buffer or the transaction's own writes.
	pub hit: bool,
	/// The round trip this read belongs to, or the last one before it.
	pub request_id: u64,
}

/// A recorded event of the client, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
	Read(Access),
	/// A commit of these pages, leaving the file this many pages long.
	Commit {
		pages: Vec<Pgno>,
		size_pages: u32,
	},
	/// The database closed and opened again.
	Reopen,
}

#[derive(Debug, Clone)]
struct Buffered {
	bytes: Bytes,
	source: Source,
	read: bool,
}

/// Counts of one run, per source where it matters.
#[derive(Debug, Default, Clone)]
pub struct ClientCounts {
	pub round_trips: u64,
	pub reads: u64,
	pub hits: u64,
	/// Pages each source sent.
	pub sent: BTreeMap<&'static str, u64>,
	/// Pages each source sent that SQLite never read before they left the buffer.
	pub unread: BTreeMap<&'static str, u64>,
	pub commits: u64,
}

impl ClientCounts {
	pub fn false_hits(&self) -> u64 {
		self.unread.values().sum()
	}

	pub fn prefetched(&self) -> u64 {
		self.sent
			.iter()
			.filter(|(source, _)| **source != Source::Requested.as_str())
			.map(|(_, pages)| pages)
			.sum()
	}
}

pub struct Client {
	pub server: Server,
	pub native_handoff: bool,
	buffer: BTreeMap<Pgno, Buffered>,
	dirty: BTreeMap<Pgno, Bytes>,
	in_atomic: bool,
	size_pages: u32,
	pending_size: Option<u32>,
	opened: bool,
	pub counts: ClientCounts,
	pub events: Vec<Event>,
	reads: usize,
	/// Every page a reply handed SQLite is checked against the store's bytes.
	pub verify_bytes: bool,
}

impl Client {
	pub fn new(server: Server) -> Self {
		let size_pages = server.store.size_pages();
		Self {
			server,
			native_handoff: false,
			buffer: BTreeMap::new(),
			dirty: BTreeMap::new(),
			in_atomic: false,
			size_pages,
			pending_size: None,
			opened: false,
			counts: ClientCounts::default(),
			events: Vec::new(),
			reads: 0,
			verify_bytes: true,
		}
	}

	/// The file's size as SQLite sees it: the committed size, or the transaction's truncation,
	/// extended by any page the transaction wrote past it.
	fn file_pages(&self) -> u32 {
		let size = self.pending_size.unwrap_or(self.size_pages);
		let written = self.dirty.keys().next_back().copied().unwrap_or(0);
		size.max(written)
	}

	/// A page's current bytes without counting a read, for a write that covers part of it.
	fn current_page(&self, pgno: Pgno) -> Vec<u8> {
		if let Some(bytes) = self.dirty.get(&pgno) {
			return bytes.to_vec();
		}
		if let Some(buffered) = self.buffer.get(&pgno) {
			return buffered.bytes.to_vec();
		}
		self.server
			.store
			.page(pgno)
			.map(|page| page.to_vec())
			.unwrap_or_else(|| vec![0; PAGE])
	}

	/// Starts a new opening: the buffer goes, as the actor's next start lands elsewhere.
	pub fn reopen(&mut self) {
		self.drain_buffer();
		self.opened = false;
		self.size_pages = self.server.store.size_pages();
		self.server.open();
		self.events.push(Event::Reopen);
	}

	/// Counts what is still unread in the buffer and empties it.
	pub fn drain_buffer(&mut self) {
		for (_, buffered) in std::mem::take(&mut self.buffer) {
			if !buffered.read {
				*self
					.counts
					.unread
					.entry(buffered.source.as_str())
					.or_default() += 1;
			}
		}
	}

	fn read_page(&mut self, pgno: Pgno, partial: bool) -> Option<Bytes> {
		let result = self.read_page_inner(pgno);
		// Matches native take_buffered_page: SQLite owns a whole non-header page after
		// handoff; partial reads and page1 leave the shared buffer entry in place.
		if self.native_handoff && pgno != 1 && !partial {
			self.buffer.remove(&pgno);
		}
		result
	}

	fn read_page_inner(&mut self, pgno: Pgno) -> Option<Bytes> {
		self.counts.reads += 1;
		let position = self.reads;
		self.reads += 1;
		let request_id = self.counts.round_trips;
		if let Some(bytes) = self.dirty.get(&pgno).cloned() {
			self.counts.hits += 1;
			self.events.push(Event::Read(Access {
				pgno,
				hit: true,
				request_id,
			}));
			self.client_hit(pgno, &bytes);
			return Some(bytes);
		}
		if let Some(buffered) = self.buffer.get_mut(&pgno) {
			buffered.read = true;
			let bytes = buffered.bytes.clone();
			self.counts.hits += 1;
			self.events.push(Event::Read(Access {
				pgno,
				hit: true,
				request_id,
			}));
			self.client_hit(pgno, &bytes);
			return Some(bytes);
		}
		if pgno > self.size_pages {
			return None;
		}
		self.counts.round_trips += 1;
		let request_id = self.counts.round_trips;
		self.events.push(Event::Read(Access {
			pgno,
			hit: false,
			request_id,
		}));
		let opening = !self.opened;
		self.opened = true;
		let buffer = &self.buffer;
		let reply = self.server.get_pages(&[pgno], opening, position, &|page| {
			buffer.contains_key(&page)
		});
		let mut requested = None;
		for (page, bytes, source) in reply {
			let Some(bytes) = bytes else {
				continue;
			};
			if page == pgno && source == Source::Requested {
				self.verify(page, &bytes, source);
				*self.counts.sent.entry(source.as_str()).or_default() += 1;
				requested = Some(bytes.clone());
				self.insert(page, bytes, source, true);
				continue;
			}
			self.receive(page, bytes, source);
		}
		requested
	}

	fn verify(&self, page: Pgno, bytes: &Bytes, source: Source) {
		if self.verify_bytes {
			assert_eq!(
				Some(bytes),
				self.server.store.page(page).as_ref(),
				"page {page} from {source:?} differs from the file"
			);
		}
	}

	/// Buffers a page a reply carried that SQLite did not ask for.
	fn receive(&mut self, page: Pgno, bytes: Bytes, source: Source) {
		self.verify(page, &bytes, source);
		*self.counts.sent.entry(source.as_str()).or_default() += 1;
		if self.dirty.contains_key(&page) {
			*self.counts.unread.entry(source.as_str()).or_default() += 1;
			return;
		}
		self.insert(page, bytes, source, false);
	}

	/// Shows a client-side read-ahead rule a read served without a round trip, and buffers
	/// what it fetches without SQLite waiting. That fetch is a request of its own.
	fn client_hit(&mut self, pgno: Pgno, bytes: &Bytes) {
		let buffer = &self.buffer;
		let reply = self
			.server
			.client_hit(pgno, bytes, &|page| buffer.contains_key(&page));
		if reply.is_empty() {
			return;
		}
		self.counts.round_trips += 1;
		for (page, bytes, source) in reply {
			if let Some(bytes) = bytes {
				self.receive(page, bytes, source);
			}
		}
	}

	/// Replays one recorded read.
	pub fn replay_read(&mut self, pgno: Pgno) {
		let _ = self.read_page(pgno, false);
	}

	/// Replays one recorded commit, with zeroed pages.
	pub fn replay_commit(&mut self, pages: &[Pgno], size_pages: u32) {
		for &pgno in pages {
			self.write_page(pgno, Bytes::from(vec![0; PAGE]));
		}
		self.pending_size = Some(size_pages);
		let _ = self.commit();
	}

	fn insert(&mut self, pgno: Pgno, bytes: Bytes, source: Source, read: bool) {
		let old = self.buffer.insert(
			pgno,
			Buffered {
				bytes,
				source,
				read,
			},
		);
		if let Some(old) = old.filter(|old| !old.read) {
			*self.counts.unread.entry(old.source.as_str()).or_default() += 1;
		}
	}

	fn write_page(&mut self, pgno: Pgno, bytes: Bytes) {
		self.dirty.insert(pgno, bytes);
	}

	fn commit(&mut self) -> bool {
		if self.dirty.is_empty() && self.pending_size.is_none() {
			return true;
		}
		let size = self.file_pages();
		let dirty = std::mem::take(&mut self.dirty);
		self.pending_size = None;
		self.counts.commits += 1;
		self.events.push(Event::Commit {
			pages: dirty.keys().copied().collect(),
			size_pages: size,
		});
		if !self.server.commit(&dirty, size) {
			return false;
		}
		self.size_pages = size;
		// The client keeps what it wrote, as the production client's committed buffer does.
		for (pgno, bytes) in dirty {
			if pgno <= size {
				let old = self.buffer.insert(
					pgno,
					Buffered {
						bytes,
						source: Source::Requested,
						read: true,
					},
				);
				if let Some(old) = old.filter(|old| !old.read) {
					*self.counts.unread.entry(old.source.as_str()).or_default() += 1;
				}
			}
		}
		self.buffer.retain(|pgno, _| *pgno <= size);
		true
	}

	fn rollback(&mut self) {
		self.dirty.clear();
		self.pending_size = None;
	}
}

/// A registered testing VFS. Dropping it unregisters it; close connections first.
pub struct TestVfs {
	vfs: *mut sqlite3_vfs,
	_name: CString,
	state: *mut VfsState,
	pub client: Arc<Mutex<Client>>,
}

// SAFETY: the raw pointers are owned by this handle and only used by SQLite on the thread that
// runs the connection; the client is behind a mutex.
unsafe impl Send for TestVfs {}

struct VfsState {
	client: Arc<Mutex<Client>>,
	main_name: String,
	aux: Mutex<BTreeMap<String, Arc<Mutex<Vec<u8>>>>>,
	methods: sqlite3_io_methods,
}

#[repr(C)]
struct File {
	base: sqlite3_file,
	state: *const VfsState,
	/// `None` for the main database file.
	aux: Option<Arc<Mutex<Vec<u8>>>>,
	aux_name: Option<String>,
	delete_on_close: bool,
}

static NEXT_VFS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl TestVfs {
	/// Registers a VFS whose file `main.db` is served by `client`.
	pub fn register(client: Client) -> Self {
		let id = NEXT_VFS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		let name = CString::new(format!("sqlite-prefetch-test-{id}")).expect("valid name");
		let client = Arc::new(Mutex::new(client));
		let mut methods: sqlite3_io_methods = unsafe { std::mem::zeroed() };
		methods.iVersion = 1;
		methods.xClose = Some(io_close);
		methods.xRead = Some(io_read);
		methods.xWrite = Some(io_write);
		methods.xTruncate = Some(io_truncate);
		methods.xSync = Some(io_sync);
		methods.xFileSize = Some(io_file_size);
		methods.xLock = Some(io_lock);
		methods.xUnlock = Some(io_lock);
		methods.xCheckReservedLock = Some(io_check_reserved_lock);
		methods.xFileControl = Some(io_file_control);
		methods.xSectorSize = Some(io_sector_size);
		methods.xDeviceCharacteristics = Some(io_device_characteristics);
		let state = Box::into_raw(Box::new(VfsState {
			client: client.clone(),
			main_name: "main.db".to_string(),
			aux: Mutex::new(BTreeMap::new()),
			methods,
		}));
		let mut vfs: sqlite3_vfs = unsafe { std::mem::zeroed() };
		vfs.iVersion = 1;
		vfs.szOsFile = std::mem::size_of::<File>() as c_int;
		vfs.mxPathname = 512;
		vfs.zName = name.as_ptr();
		vfs.pAppData = state.cast::<c_void>();
		vfs.xOpen = Some(vfs_open);
		vfs.xDelete = Some(vfs_delete);
		vfs.xAccess = Some(vfs_access);
		vfs.xFullPathname = Some(vfs_full_pathname);
		vfs.xRandomness = Some(vfs_randomness);
		vfs.xSleep = Some(vfs_sleep);
		vfs.xCurrentTime = Some(vfs_current_time);
		vfs.xGetLastError = Some(vfs_get_last_error);
		let vfs = Box::into_raw(Box::new(vfs));
		let rc = unsafe { sqlite3_vfs_register(vfs, 0) };
		assert_eq!(rc, SQLITE_OK, "registering the testing VFS");
		Self {
			vfs,
			_name: name,
			state,
			client,
		}
	}

	pub fn name(&self) -> &str {
		self._name.to_str().expect("ascii name")
	}

	/// Opens a connection to the database with the production VFS's pragmas.
	pub fn connect(&self, cache_pages: u32) -> rusqlite::Connection {
		let connection = rusqlite::Connection::open_with_flags_and_vfs(
			"main.db",
			rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
			self.name(),
		)
		.expect("open through the testing VFS");
		connection
			.execute_batch(&format!(
				"PRAGMA page_size = 4096;
				PRAGMA journal_mode = DELETE;
				PRAGMA synchronous = NORMAL;
				PRAGMA temp_store = MEMORY;
				PRAGMA auto_vacuum = NONE;
				PRAGMA locking_mode = EXCLUSIVE;
				PRAGMA cache_size = {cache_pages};"
			))
			.expect("configure the connection");
		connection
	}
}

impl Drop for TestVfs {
	fn drop(&mut self) {
		unsafe {
			sqlite3_vfs_unregister(self.vfs);
			drop(Box::from_raw(self.vfs));
			drop(Box::from_raw(self.state));
		}
	}
}

/// Runs a callback, turning a panic into an I/O error instead of unwinding into SQLite. A
/// failed byte check panics on purpose: the test reads the stored message.
fn guarded(fallback: c_int, f: impl FnOnce() -> c_int) -> c_int {
	match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
		Ok(rc) => rc,
		Err(panic) => {
			let message = panic
				.downcast_ref::<String>()
				.cloned()
				.or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
				.unwrap_or_default();
			PANICS.lock().push(message);
			fallback
		}
	}
}

/// Panics caught inside VFS callbacks, for the test to report.
pub static PANICS: Mutex<Vec<String>> = Mutex::new(Vec::new());

unsafe fn file<'a>(p_file: *mut sqlite3_file) -> &'a mut File {
	unsafe { &mut *(p_file as *mut File) }
}

unsafe fn state<'a>(file: &File) -> &'a VfsState {
	unsafe { &*file.state }
}

unsafe extern "C" fn vfs_open(
	p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	p_file: *mut sqlite3_file,
	flags: c_int,
	p_out_flags: *mut c_int,
) -> c_int {
	guarded(SQLITE_CANTOPEN, || unsafe {
		let state = &*((*p_vfs).pAppData as *const VfsState);
		let name = match z_name.is_null() {
			true => None,
			false => Some(CStr::from_ptr(z_name).to_string_lossy().into_owned()),
		};
		let file = p_file as *mut File;
		let aux = match name.as_deref() {
			Some(name) if name == state.main_name => None,
			Some(name) => Some(
				state
					.aux
					.lock()
					.entry(name.to_string())
					.or_default()
					.clone(),
			),
			None => Some(Arc::new(Mutex::new(Vec::new()))),
		};
		std::ptr::write(
			file,
			File {
				base: sqlite3_file {
					pMethods: &state.methods,
				},
				state,
				aux_name: name.filter(|name| *name != state.main_name),
				aux,
				delete_on_close: flags & SQLITE_OPEN_DELETEONCLOSE != 0,
			},
		);
		if !p_out_flags.is_null() {
			*p_out_flags = flags;
		}
		SQLITE_OK
	})
}

unsafe extern "C" fn vfs_delete(
	p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	_sync_dir: c_int,
) -> c_int {
	guarded(SQLITE_IOERR_DELETE, || unsafe {
		let state = &*((*p_vfs).pAppData as *const VfsState);
		let name = CStr::from_ptr(z_name).to_string_lossy().into_owned();
		state.aux.lock().remove(&name);
		SQLITE_OK
	})
}

unsafe extern "C" fn vfs_access(
	p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	_flags: c_int,
	p_res_out: *mut c_int,
) -> c_int {
	guarded(SQLITE_IOERR_ACCESS, || unsafe {
		let state = &*((*p_vfs).pAppData as *const VfsState);
		let name = CStr::from_ptr(z_name).to_string_lossy().into_owned();
		let exists = match name == state.main_name {
			true => true,
			false => state
				.aux
				.lock()
				.get(&name)
				.is_some_and(|file| !file.lock().is_empty()),
		};
		*p_res_out = c_int::from(exists);
		SQLITE_OK
	})
}

unsafe extern "C" fn vfs_full_pathname(
	_p_vfs: *mut sqlite3_vfs,
	z_name: *const c_char,
	n_out: c_int,
	z_out: *mut c_char,
) -> c_int {
	guarded(SQLITE_IOERR, || unsafe {
		let name = CStr::from_ptr(z_name).to_bytes().to_vec();
		if name.len() + 1 > n_out as usize {
			return SQLITE_CANTOPEN;
		}
		std::ptr::copy_nonoverlapping(name.as_ptr().cast::<c_char>(), z_out, name.len());
		*z_out.add(name.len()) = 0;
		SQLITE_OK
	})
}

unsafe extern "C" fn vfs_randomness(
	_p_vfs: *mut sqlite3_vfs,
	n_byte: c_int,
	z_out: *mut c_char,
) -> c_int {
	unsafe {
		for index in 0..n_byte as usize {
			*z_out.add(index) = (index as u8).wrapping_mul(31) as c_char;
		}
	}
	n_byte
}

unsafe extern "C" fn vfs_sleep(_p_vfs: *mut sqlite3_vfs, microseconds: c_int) -> c_int {
	microseconds
}

unsafe extern "C" fn vfs_current_time(_p_vfs: *mut sqlite3_vfs, p_time_out: *mut f64) -> c_int {
	unsafe {
		*p_time_out = 2_460_000.5;
	}
	SQLITE_OK
}

unsafe extern "C" fn vfs_get_last_error(
	_p_vfs: *mut sqlite3_vfs,
	_n_byte: c_int,
	_z_err_msg: *mut c_char,
) -> c_int {
	0
}

unsafe extern "C" fn io_close(p_file: *mut sqlite3_file) -> c_int {
	guarded(SQLITE_IOERR_CLOSE, || unsafe {
		let file = file(p_file);
		if file.delete_on_close {
			if let Some(name) = &file.aux_name {
				state(file).aux.lock().remove(name);
			}
		}
		std::ptr::drop_in_place(file as *mut File);
		SQLITE_OK
	})
}

unsafe extern "C" fn io_read(
	p_file: *mut sqlite3_file,
	p_buf: *mut c_void,
	i_amt: c_int,
	i_ofst: sqlite3_int64,
) -> c_int {
	guarded(SQLITE_IOERR_READ, || unsafe {
		let file = file(p_file);
		let out = std::slice::from_raw_parts_mut(p_buf.cast::<u8>(), i_amt as usize);
		if let Some(aux) = &file.aux {
			let aux = aux.lock();
			let start = i_ofst as usize;
			out.fill(0);
			if start >= aux.len() {
				return SQLITE_IOERR_SHORT_READ;
			}
			let end = (start + out.len()).min(aux.len());
			out[..end - start].copy_from_slice(&aux[start..end]);
			return match end - start == out.len() {
				true => SQLITE_OK,
				false => SQLITE_IOERR_SHORT_READ,
			};
		}
		let mut client = state(file).client.lock();
		let mut short = false;
		let mut written = 0usize;
		let mut offset = i_ofst as usize;
		while written < out.len() {
			let pgno = (offset / PAGE) as Pgno + 1;
			let within = offset % PAGE;
			let take = (PAGE - within).min(out.len() - written);
			match client.read_page(pgno, i_ofst as usize % PAGE != 0 || out.len() % PAGE != 0) {
				Some(page) => {
					out[written..written + take].copy_from_slice(&page[within..within + take])
				}
				None => {
					out[written..written + take].fill(0);
					short = true;
				}
			}
			written += take;
			offset += take;
		}
		match short {
			true => SQLITE_IOERR_SHORT_READ,
			false => SQLITE_OK,
		}
	})
}

unsafe extern "C" fn io_write(
	p_file: *mut sqlite3_file,
	p_buf: *const c_void,
	i_amt: c_int,
	i_ofst: sqlite3_int64,
) -> c_int {
	guarded(SQLITE_IOERR_WRITE, || unsafe {
		let file = file(p_file);
		let data = std::slice::from_raw_parts(p_buf.cast::<u8>(), i_amt as usize);
		if let Some(aux) = &file.aux {
			let mut aux = aux.lock();
			let start = i_ofst as usize;
			if aux.len() < start + data.len() {
				aux.resize(start + data.len(), 0);
			}
			aux[start..start + data.len()].copy_from_slice(data);
			return SQLITE_OK;
		}
		let mut client = state(file).client.lock();
		let mut offset = i_ofst as usize;
		let mut done = 0;
		while done < data.len() {
			let pgno = (offset / PAGE) as Pgno + 1;
			let within = offset % PAGE;
			let take = (PAGE - within).min(data.len() - done);
			let mut page = match take == PAGE {
				true => vec![0; PAGE],
				false => client.current_page(pgno),
			};
			page[within..within + take].copy_from_slice(&data[done..done + take]);
			client.write_page(pgno, Bytes::from(page));
			done += take;
			offset += take;
		}
		SQLITE_OK
	})
}

unsafe extern "C" fn io_truncate(p_file: *mut sqlite3_file, size: sqlite3_int64) -> c_int {
	guarded(SQLITE_IOERR_TRUNCATE, || unsafe {
		let file = file(p_file);
		if let Some(aux) = &file.aux {
			aux.lock().truncate(size as usize);
			return SQLITE_OK;
		}
		let mut client = state(file).client.lock();
		let pages = (size as usize / PAGE) as u32;
		client.dirty.retain(|pgno, _| *pgno <= pages);
		client.pending_size = Some(pages);
		SQLITE_OK
	})
}

unsafe extern "C" fn io_sync(p_file: *mut sqlite3_file, _flags: c_int) -> c_int {
	guarded(SQLITE_IOERR_FSYNC, || unsafe {
		let file = file(p_file);
		if file.aux.is_some() {
			return SQLITE_OK;
		}
		let mut client = state(file).client.lock();
		if client.in_atomic {
			return SQLITE_OK;
		}
		match client.commit() {
			true => SQLITE_OK,
			false => SQLITE_IOERR_FSYNC,
		}
	})
}

unsafe extern "C" fn io_file_size(p_file: *mut sqlite3_file, p_size: *mut sqlite3_int64) -> c_int {
	guarded(SQLITE_IOERR_FSTAT, || unsafe {
		let file = file(p_file);
		let size = match &file.aux {
			Some(aux) => aux.lock().len(),
			None => state(file).client.lock().file_pages() as usize * PAGE,
		};
		*p_size = size as sqlite3_int64;
		SQLITE_OK
	})
}

unsafe extern "C" fn io_lock(_p_file: *mut sqlite3_file, _level: c_int) -> c_int {
	SQLITE_OK
}

unsafe extern "C" fn io_check_reserved_lock(
	_p_file: *mut sqlite3_file,
	p_res_out: *mut c_int,
) -> c_int {
	unsafe {
		*p_res_out = 0;
	}
	SQLITE_OK
}

unsafe extern "C" fn io_file_control(
	p_file: *mut sqlite3_file,
	op: c_int,
	_p_arg: *mut c_void,
) -> c_int {
	guarded(SQLITE_IOERR, || unsafe {
		let file = file(p_file);
		if file.aux.is_some() {
			return SQLITE_NOTFOUND;
		}
		let mut client = state(file).client.lock();
		match op {
			SQLITE_FCNTL_BEGIN_ATOMIC_WRITE => {
				client.in_atomic = true;
				SQLITE_OK
			}
			SQLITE_FCNTL_COMMIT_ATOMIC_WRITE => {
				client.in_atomic = false;
				match client.commit() {
					true => SQLITE_OK,
					false => {
						client.rollback();
						SQLITE_IOERR
					}
				}
			}
			SQLITE_FCNTL_ROLLBACK_ATOMIC_WRITE => {
				client.in_atomic = false;
				client.rollback();
				SQLITE_OK
			}
			_ => SQLITE_NOTFOUND,
		}
	})
}

unsafe extern "C" fn io_sector_size(_p_file: *mut sqlite3_file) -> c_int {
	PAGE as c_int
}

unsafe extern "C" fn io_device_characteristics(p_file: *mut sqlite3_file) -> c_int {
	unsafe {
		match file(p_file).aux.is_some() {
			true => 0,
			false => SQLITE_IOCAP_BATCH_ATOMIC,
		}
	}
}


