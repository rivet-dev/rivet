use anyhow::{Result, bail};
use libsqlite3_sys::{
	SQLITE_OK, SQLITE_STATIC, sqlite3, sqlite3_bind_blob, sqlite3_bind_text, sqlite3_finalize,
	sqlite3_prepare_v2, sqlite3_step, sqlite3_stmt,
};
use std::ffi::CString;
use std::ptr;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LogicalOp {
	Put {
		key: String,
		value: Vec<u8>,
	},
	#[allow(dead_code)]
	Delete {
		key: String,
	},
	CreateHeavySchema,
	InsertHeavyBlob {
		id: i64,
		bucket: String,
		payload: Vec<u8>,
	},
	AddHeavyNoteColumn,
	SetHeavyNote {
		id: i64,
		note: String,
	},
	DeleteHeavyRange {
		min_id: i64,
		max_id: i64,
	},
	ExplicitRollbackInsert {
		id: i64,
		payload_len: usize,
	},
	Vacuum,
	#[allow(dead_code)]
	Sql(String),
}

impl LogicalOp {
	pub(crate) fn apply(&self, db: *mut sqlite3) -> Result<()> {
		match self {
			LogicalOp::Put { key, value } => put(db, key, value),
			LogicalOp::Delete { key } => delete(db, key),
			LogicalOp::CreateHeavySchema => create_heavy_schema(db),
			LogicalOp::InsertHeavyBlob {
				id,
				bucket,
				payload,
			} => insert_heavy_blob(db, *id, bucket, payload),
			LogicalOp::AddHeavyNoteColumn => add_heavy_note_column(db),
			LogicalOp::SetHeavyNote { id, note } => set_heavy_note(db, *id, note),
			LogicalOp::DeleteHeavyRange { min_id, max_id } => {
				delete_heavy_range(db, *min_id, *max_id)
			}
			LogicalOp::ExplicitRollbackInsert { id, payload_len } => {
				explicit_rollback_insert(db, *id, *payload_len)
			}
			LogicalOp::Vacuum => {
				super::super::sqlite_exec(db, "VACUUM;").map_err(anyhow::Error::msg)
			}
			LogicalOp::Sql(sql) => super::super::sqlite_exec(db, sql).map_err(anyhow::Error::msg),
		}
	}
}

fn put(db: *mut sqlite3, key: &str, value: &[u8]) -> Result<()> {
	let stmt = prepare(
		db,
		"INSERT INTO kv (k, v) VALUES (?, ?) \
		 ON CONFLICT(k) DO UPDATE SET v = excluded.v;",
	)?;
	bind_text(stmt, 1, key)?;
	bind_blob(stmt, 2, value)?;
	step_done(db, stmt)
}

fn delete(db: *mut sqlite3, key: &str) -> Result<()> {
	let stmt = prepare(db, "DELETE FROM kv WHERE k = ?;")?;
	bind_text(stmt, 1, key)?;
	step_done(db, stmt)
}

fn create_heavy_schema(db: *mut sqlite3) -> Result<()> {
	super::super::sqlite_exec(
		db,
		"CREATE TABLE heavy_items (\
			id INTEGER PRIMARY KEY, \
			bucket TEXT NOT NULL, \
			payload BLOB NOT NULL\
		); \
		CREATE INDEX heavy_items_bucket_idx ON heavy_items(bucket, id);",
	)
	.map_err(anyhow::Error::msg)
}

fn insert_heavy_blob(db: *mut sqlite3, id: i64, bucket: &str, payload: &[u8]) -> Result<()> {
	let stmt = prepare(
		db,
		"INSERT INTO heavy_items (id, bucket, payload) VALUES (?, ?, ?) \
		 ON CONFLICT(id) DO UPDATE SET \
			bucket = excluded.bucket, \
			payload = excluded.payload;",
	)?;
	bind_i64(stmt, 1, id)?;
	bind_text(stmt, 2, bucket)?;
	bind_blob(stmt, 3, payload)?;
	step_done(db, stmt)
}

fn add_heavy_note_column(db: *mut sqlite3) -> Result<()> {
	super::super::sqlite_exec(
		db,
		"ALTER TABLE heavy_items ADD COLUMN note TEXT; \
		 CREATE INDEX heavy_items_note_idx ON heavy_items(note, id);",
	)
	.map_err(anyhow::Error::msg)
}

fn set_heavy_note(db: *mut sqlite3, id: i64, note: &str) -> Result<()> {
	let stmt = prepare(db, "UPDATE heavy_items SET note = ? WHERE id = ?;")?;
	bind_text(stmt, 1, note)?;
	bind_i64(stmt, 2, id)?;
	step_done(db, stmt)
}

fn delete_heavy_range(db: *mut sqlite3, min_id: i64, max_id: i64) -> Result<()> {
	let stmt = prepare(db, "DELETE FROM heavy_items WHERE id BETWEEN ? AND ?;")?;
	bind_i64(stmt, 1, min_id)?;
	bind_i64(stmt, 2, max_id)?;
	step_done(db, stmt)
}

fn explicit_rollback_insert(db: *mut sqlite3, id: i64, payload_len: usize) -> Result<()> {
	super::super::sqlite_exec(db, "BEGIN IMMEDIATE;").map_err(anyhow::Error::msg)?;
	let result = (|| {
		let stmt = prepare(
			db,
			"INSERT INTO heavy_items (id, bucket, payload) VALUES (?, ?, zeroblob(?));",
		)?;
		bind_i64(stmt, 1, id)?;
		bind_text(stmt, 2, "rolled-back")?;
		bind_i64(stmt, 3, i64::try_from(payload_len)?)?;
		step_done(db, stmt)
	})();
	let rollback = super::super::sqlite_exec(db, "ROLLBACK;").map_err(anyhow::Error::msg);
	result?;
	rollback
}

fn prepare(db: *mut sqlite3, sql: &str) -> Result<*mut sqlite3_stmt> {
	let c_sql = CString::new(sql)?;
	let mut stmt = ptr::null_mut();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	if rc != SQLITE_OK {
		bail!(
			"{sql} prepare failed with code {rc}: {}",
			sqlite_error_message(db)
		);
	}
	Ok(stmt)
}

fn bind_text(stmt: *mut sqlite3_stmt, index: i32, value: &str) -> Result<()> {
	let rc = unsafe {
		sqlite3_bind_text(
			stmt,
			index,
			value.as_ptr().cast(),
			value.len() as i32,
			SQLITE_STATIC(),
		)
	};
	if rc != SQLITE_OK {
		bail!("sqlite text bind failed with code {rc}");
	}
	Ok(())
}

fn bind_blob(stmt: *mut sqlite3_stmt, index: i32, value: &[u8]) -> Result<()> {
	let rc = unsafe {
		sqlite3_bind_blob(
			stmt,
			index,
			value.as_ptr().cast(),
			value.len() as i32,
			SQLITE_STATIC(),
		)
	};
	if rc != SQLITE_OK {
		bail!("sqlite blob bind failed with code {rc}");
	}
	Ok(())
}

fn bind_i64(stmt: *mut sqlite3_stmt, index: i32, value: i64) -> Result<()> {
	let rc = unsafe { libsqlite3_sys::sqlite3_bind_int64(stmt, index, value) };
	if rc != SQLITE_OK {
		bail!("sqlite integer bind failed with code {rc}");
	}
	Ok(())
}

fn step_done(db: *mut sqlite3, stmt: *mut sqlite3_stmt) -> Result<()> {
	let rc = unsafe { sqlite3_step(stmt) };
	unsafe {
		sqlite3_finalize(stmt);
	}
	if rc != libsqlite3_sys::SQLITE_DONE {
		bail!(
			"sqlite step failed with code {rc}: {}",
			sqlite_error_message(db)
		);
	}
	Ok(())
}

fn sqlite_error_message(db: *mut sqlite3) -> String {
	let err = unsafe { libsqlite3_sys::sqlite3_errmsg(db) };
	if err.is_null() {
		return "unknown sqlite error".to_string();
	}
	unsafe { std::ffi::CStr::from_ptr(err) }
		.to_string_lossy()
		.into_owned()
}
