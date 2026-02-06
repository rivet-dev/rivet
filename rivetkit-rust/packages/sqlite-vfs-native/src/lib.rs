use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::ptr;

use libsqlite3_sys as sqlite;
use napi::bindgen_prelude::Buffer;
use napi::{Env, Error, JsFunction, JsObject, Result, Status};
use napi_derive::napi;
use tempfile::TempDir;

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
const MIN_SAFE_INTEGER: i64 = -9_007_199_254_740_991;

#[napi]
pub struct NativeDatabase {
	db: *mut sqlite::sqlite3,
	_temp_dir: TempDir,
	path: PathBuf,
	closed: bool,
}

unsafe impl Send for NativeDatabase {}

impl Drop for NativeDatabase {
	fn drop(&mut self) {
		let _ = self.close_internal();
	}
}

#[napi]
impl NativeDatabase {
	#[napi(constructor)]
	pub fn new(bytes: Option<Buffer>) -> Result<Self> {
		let temp_dir = TempDir::new().map_err(|err| {
			Error::new(Status::GenericFailure, format!("temp dir error: {err}"))
		})?;
		let path = temp_dir.path().join("db.sqlite");
		if let Some(data) = bytes {
			std::fs::write(&path, &data).map_err(|err| {
				Error::new(Status::GenericFailure, format!("write db error: {err}"))
			})?;
		}

		let mut db = ptr::null_mut();
		let c_path = CString::new(path.to_string_lossy().as_bytes()).map_err(|_| {
			Error::new(Status::InvalidArg, "invalid database path")
		})?;

		let flags = sqlite::SQLITE_OPEN_READWRITE | sqlite::SQLITE_OPEN_CREATE;
		let rc = unsafe { sqlite::sqlite3_open_v2(c_path.as_ptr(), &mut db, flags, ptr::null()) };
		if rc != sqlite::SQLITE_OK {
			return Err(sqlite_error(db, "sqlite3_open_v2"));
		}

		exec_pragma(db, "PRAGMA journal_mode=DELETE;")?;
		exec_pragma(db, "PRAGMA synchronous=FULL;")?;

		Ok(Self {
			db,
			_temp_dir: temp_dir,
			path,
			closed: false,
		})
	}

	#[napi]
	pub fn exec(&mut self, env: Env, sql: String, callback: Option<JsFunction>) -> Result<()> {
		if self.db.is_null() || self.closed {
			return Err(Error::new(Status::GenericFailure, "database is closed"));
		}

		let c_sql = CString::new(sql).map_err(|_| {
			Error::new(Status::InvalidArg, "sql contains null byte")
		})?;
		let mut tail = c_sql.as_ptr();

		loop {
			let mut stmt: *mut sqlite::sqlite3_stmt = ptr::null_mut();
			let rc = unsafe { sqlite::sqlite3_prepare_v2(self.db, tail, -1, &mut stmt, &mut tail) };
			if rc != sqlite::SQLITE_OK {
				if !stmt.is_null() {
					unsafe {
						sqlite::sqlite3_finalize(stmt);
					}
				}
				return Err(sqlite_error(self.db, "sqlite3_prepare_v2"));
			}

			if stmt.is_null() {
				if unsafe { *tail } == 0 {
					break;
				}
				continue;
			}

			let column_count = unsafe { sqlite::sqlite3_column_count(stmt) };
			let column_names = collect_column_names(stmt, column_count);

			loop {
				let step_rc = unsafe { sqlite::sqlite3_step(stmt) };
				if step_rc == sqlite::SQLITE_ROW {
					if let Some(cb) = &callback {
						let row = build_row(&env, stmt, column_count)?;
						let columns = build_columns(&env, &column_names)?;
						cb.call(None, &[row.into_unknown(), columns.into_unknown()])?;
					}
				} else if step_rc == sqlite::SQLITE_DONE {
					break;
				} else {
					unsafe {
						sqlite::sqlite3_finalize(stmt);
					}
					return Err(sqlite_error(self.db, "sqlite3_step"));
				}
			}

			unsafe {
				sqlite::sqlite3_finalize(stmt);
			}

			if unsafe { *tail } == 0 {
				break;
			}
		}

		Ok(())
	}

	#[napi]
	pub fn export(&self) -> Result<Buffer> {
		if self.db.is_null() || self.closed {
			return Err(Error::new(Status::GenericFailure, "database is closed"));
		}
		let bytes = std::fs::read(&self.path).map_err(|err| {
			Error::new(Status::GenericFailure, format!("read db error: {err}"))
		})?;
		Ok(Buffer::from(bytes))
	}

	#[napi]
	pub fn close(&mut self) -> Result<()> {
		self.close_internal()
	}
}

impl NativeDatabase {
	fn close_internal(&mut self) -> Result<()> {
		if self.closed || self.db.is_null() {
			return Ok(());
		}

		let rc = unsafe { sqlite::sqlite3_close(self.db) };
		if rc != sqlite::SQLITE_OK {
			return Err(sqlite_error(self.db, "sqlite3_close"));
		}
		self.db = ptr::null_mut();
		self.closed = true;
		Ok(())
	}
}

fn sqlite_error(db: *mut sqlite::sqlite3, context: &str) -> Error {
	if db.is_null() {
		return Error::new(Status::GenericFailure, format!("{context}: sqlite error"));
	}
	unsafe {
		let msg = sqlite::sqlite3_errmsg(db);
		if msg.is_null() {
			return Error::new(Status::GenericFailure, format!("{context}: sqlite error"));
		}
		let c_str = CStr::from_ptr(msg);
		Error::new(
			Status::GenericFailure,
			format!("{context}: {}", c_str.to_string_lossy()),
		)
	}
}

fn exec_pragma(db: *mut sqlite::sqlite3, sql: &str) -> Result<()> {
	let c_sql = CString::new(sql).map_err(|_| {
		Error::new(Status::InvalidArg, "pragma contains null byte")
	})?;
	let mut err_msg: *mut i8 = ptr::null_mut();
	let rc = unsafe { sqlite::sqlite3_exec(db, c_sql.as_ptr(), None, ptr::null_mut(), &mut err_msg) };
	if rc != sqlite::SQLITE_OK {
		if !err_msg.is_null() {
			let msg = unsafe { CStr::from_ptr(err_msg) }.to_string_lossy().to_string();
			unsafe {
				sqlite::sqlite3_free(err_msg as *mut _);
			}
			return Err(Error::new(Status::GenericFailure, msg));
		}
		return Err(sqlite_error(db, "sqlite3_exec"));
	}
	Ok(())
}

fn collect_column_names(stmt: *mut sqlite::sqlite3_stmt, count: i32) -> Vec<String> {
	let mut names = Vec::with_capacity(count as usize);
	for i in 0..count {
		let name_ptr = unsafe { sqlite::sqlite3_column_name(stmt, i) };
		let name = if name_ptr.is_null() {
			String::new()
		} else {
			unsafe { CStr::from_ptr(name_ptr) }.to_string_lossy().to_string()
		};
		names.push(name);
	}
	names
}

fn build_columns(env: &Env, names: &[String]) -> Result<JsObject> {
	let mut columns = env.create_array_with_length(names.len())?;
	for (index, name) in names.iter().enumerate() {
		let js_name = env.create_string(name)?;
		columns.set_element(index as u32, js_name)?;
	}
	Ok(columns)
}

fn build_row(env: &Env, stmt: *mut sqlite::sqlite3_stmt, count: i32) -> Result<JsObject> {
	let mut row = env.create_array_with_length(count as usize)?;
	for i in 0..count {
		let value = match unsafe { sqlite::sqlite3_column_type(stmt, i) } {
			sqlite::SQLITE_INTEGER => {
				let v = unsafe { sqlite::sqlite3_column_int64(stmt, i) };
				if v > MAX_SAFE_INTEGER || v < MIN_SAFE_INTEGER {
					let bigint = env.create_bigint_from_i64(v)?;
					bigint.into_unknown()?
				} else {
					let num = env.create_int64(v)?;
					num.into_unknown()
				}
			}
			sqlite::SQLITE_FLOAT => {
				let v = unsafe { sqlite::sqlite3_column_double(stmt, i) };
				let num = env.create_double(v)?;
				num.into_unknown()
			}
			sqlite::SQLITE_TEXT => {
				let ptr = unsafe { sqlite::sqlite3_column_text(stmt, i) };
				if ptr.is_null() {
					env.get_null()?.into_unknown()
				} else {
					let bytes = unsafe { CStr::from_ptr(ptr as *const i8) }
						.to_string_lossy()
						.to_string();
					let js_str = env.create_string(&bytes)?;
					js_str.into_unknown()
				}
			}
			sqlite::SQLITE_BLOB => {
				let ptr = unsafe { sqlite::sqlite3_column_blob(stmt, i) } as *const u8;
				let len = unsafe { sqlite::sqlite3_column_bytes(stmt, i) } as usize;
				if ptr.is_null() || len == 0 {
					let buf = env.create_buffer(0)?;
					buf.into_unknown()
				} else {
					let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
					let buf = env.create_buffer_copy(slice)?;
					buf.into_raw().into_unknown()
				}
			}
			_ => env.get_null()?.into_unknown(),
		};
		row.set_element(i as u32, value)?;
	}
	Ok(row)
}
