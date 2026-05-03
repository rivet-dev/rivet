use std::ffi::{CStr, CString};
use std::ptr;

use anyhow::{Context, Result, bail};
use libsqlite3_sys::{
	SQLITE_BLOB, SQLITE_FLOAT, SQLITE_INTEGER, SQLITE_NULL, SQLITE_OK, SQLITE_ROW, SQLITE_TEXT,
	sqlite3, sqlite3_backup_finish, sqlite3_backup_init, sqlite3_backup_step, sqlite3_close,
	sqlite3_column_blob, sqlite3_column_bytes, sqlite3_column_count, sqlite3_column_double,
	sqlite3_column_int64, sqlite3_column_text, sqlite3_column_type, sqlite3_errmsg, sqlite3_exec,
	sqlite3_finalize, sqlite3_open, sqlite3_prepare_v2, sqlite3_step,
};

use super::workload::LogicalOp;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OracleCommitSemantics {
	PreCommitFailure,
	Success,
	AmbiguousPostCommit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AmbiguousOracleOutcome {
	Old,
	New,
	Invalid,
}

impl AmbiguousOracleOutcome {
	pub(crate) fn as_str(self) -> &'static str {
		match self {
			AmbiguousOracleOutcome::Old => "old",
			AmbiguousOracleOutcome::New => "new",
			AmbiguousOracleOutcome::Invalid => "invalid",
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OracleVerification {
	Matched,
	Ambiguous(AmbiguousOracleOutcome),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CanonicalDump {
	entries: Vec<String>,
}

pub(crate) struct NativeSqliteOracle {
	db: *mut sqlite3,
	applied_ops: Vec<(OracleCommitSemantics, LogicalOp)>,
	pending_ambiguous: Option<PendingAmbiguousOp>,
}

unsafe impl Send for NativeSqliteOracle {}

struct PendingAmbiguousOp {
	op: LogicalOp,
	old_dump: CanonicalDump,
	new_dump: CanonicalDump,
}

impl NativeSqliteOracle {
	pub(crate) fn open() -> Result<Self> {
		let name = CString::new(":memory:")?;
		let mut db = ptr::null_mut();
		let rc = unsafe { sqlite3_open(name.as_ptr(), &mut db) };
		if rc != SQLITE_OK {
			let message = sqlite_error_message(db);
			if !db.is_null() {
				unsafe {
					sqlite3_close(db);
				}
			}
			bail!("native sqlite oracle open failed with code {rc}: {message}");
		}

		Ok(Self {
			db,
			applied_ops: Vec::new(),
			pending_ambiguous: None,
		})
	}

	pub(crate) fn apply_sql(&mut self, sql: &str) -> Result<()> {
		self.ensure_no_pending_ambiguous("apply SQL")?;
		sqlite_exec_result(self.db, sql)
	}

	pub(crate) fn apply_logical_op(
		&mut self,
		op: LogicalOp,
		semantics: OracleCommitSemantics,
	) -> Result<()> {
		match semantics {
			OracleCommitSemantics::PreCommitFailure => {}
			OracleCommitSemantics::Success => {
				self.ensure_no_pending_ambiguous("apply a successful logical op")?;
				op.apply(self.db)?;
			}
			OracleCommitSemantics::AmbiguousPostCommit => self.snapshot_ambiguous_op(&op)?,
		}
		self.applied_ops.push((semantics, op));
		Ok(())
	}

	pub(crate) fn verify_matches(&mut self, db: *mut sqlite3) -> Result<OracleVerification> {
		Self::verify_integrity(db).context("depot-backed sqlite integrity check failed")?;
		self.verify_oracle_integrity()
			.context("native sqlite oracle integrity check failed")?;

		let actual = canonical_dump(db).context("failed to dump depot-backed sqlite database")?;
		if let Some(pending) = self.pending_ambiguous.take() {
			if actual == pending.old_dump {
				return Ok(OracleVerification::Ambiguous(AmbiguousOracleOutcome::Old));
			}

			if actual == pending.new_dump {
				pending.op.apply(self.db)?;
				return Ok(OracleVerification::Ambiguous(AmbiguousOracleOutcome::New));
			}

			bail!(
				"native sqlite ambiguous oracle mismatch\nactual:\n{}\nold expected:\n{}\nnew expected:\n{}",
				actual.render(),
				pending.old_dump.render(),
				pending.new_dump.render()
			);
		}

		let expected =
			canonical_dump(self.db).context("failed to dump native sqlite oracle database")?;
		if actual != expected {
			bail!(
				"native sqlite oracle mismatch\nactual:\n{}\nexpected:\n{}",
				actual.render(),
				expected.render()
			);
		}

		Ok(OracleVerification::Matched)
	}

	pub(crate) fn verify_oracle_integrity(&self) -> Result<()> {
		Self::verify_integrity(self.db)
	}

	pub(crate) fn verify_integrity(db: *mut sqlite3) -> Result<()> {
		let quick = query_rows(db, "PRAGMA quick_check;")?;
		if quick != vec![vec![CanonicalValue::Text("ok".to_string())]] {
			bail!("sqlite quick_check failed: {quick:?}");
		}

		let integrity = query_rows(db, "PRAGMA integrity_check;")?;
		if integrity != vec![vec![CanonicalValue::Text("ok".to_string())]] {
			bail!("sqlite integrity_check failed: {integrity:?}");
		}

		let foreign_keys = query_rows(db, "PRAGMA foreign_key_check;")?;
		if !foreign_keys.is_empty() {
			bail!("sqlite foreign_key_check failed: {foreign_keys:?}");
		}

		Ok(())
	}

	fn snapshot_ambiguous_op(&mut self, op: &LogicalOp) -> Result<()> {
		self.ensure_no_pending_ambiguous("record another ambiguous logical op")?;
		let old_dump =
			canonical_dump(self.db).context("failed to dump old native sqlite oracle state")?;
		let clone = clone_database(self.db)?;
		let result = (|| {
			op.apply(clone)?;
			canonical_dump(clone).context("failed to dump new native sqlite oracle state")
		})();
		unsafe {
			sqlite3_close(clone);
		}

		self.pending_ambiguous = Some(PendingAmbiguousOp {
			op: op.clone(),
			old_dump,
			new_dump: result?,
		});
		Ok(())
	}

	fn ensure_no_pending_ambiguous(&self, action: &str) -> Result<()> {
		if self.pending_ambiguous.is_some() {
			bail!("cannot {action} while an ambiguous oracle operation is unclassified");
		}
		Ok(())
	}
}

impl Drop for NativeSqliteOracle {
	fn drop(&mut self) {
		if !self.db.is_null() {
			unsafe {
				sqlite3_close(self.db);
			}
			self.db = ptr::null_mut();
		}
	}
}

pub(crate) fn canonical_dump(db: *mut sqlite3) -> Result<CanonicalDump> {
	let mut entries = Vec::new();
	for row in query_rows(
		db,
		"SELECT type, name, tbl_name, sql \
		 FROM sqlite_schema \
		 WHERE name NOT LIKE 'sqlite_%' AND sql IS NOT NULL \
		 ORDER BY type, name, tbl_name, sql;",
	)? {
		entries.push(format!("schema|{}", render_row(&row)));
	}

	let table_names = query_rows(
		db,
		"SELECT name FROM sqlite_schema \
		 WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
		 ORDER BY name;",
	)?;
	for row in table_names {
		let Some(CanonicalValue::Text(table_name)) = row.first() else {
			bail!("sqlite_schema returned non-text table name: {row:?}");
		};
		let columns = table_columns(db, table_name)?;
		entries.push(format!("table|{table_name}|columns|{}", columns.join(",")));
		if columns.is_empty() {
			continue;
		}

		let column_list = columns
			.iter()
			.map(|column| quote_ident(column))
			.collect::<Vec<_>>()
			.join(", ");
		let order_by = columns
			.iter()
			.map(|column| quote_ident(column))
			.collect::<Vec<_>>()
			.join(", ");
		let sql = format!(
			"SELECT {column_list} FROM {} ORDER BY {order_by};",
			quote_ident(table_name)
		);
		for data_row in query_rows(db, &sql)? {
			entries.push(format!("row|{table_name}|{}", render_row(&data_row)));
		}
	}

	Ok(CanonicalDump { entries })
}

impl CanonicalDump {
	pub(crate) fn render(&self) -> String {
		self.entries.join("\n")
	}
}

fn clone_database(source: *mut sqlite3) -> Result<*mut sqlite3> {
	let name = CString::new(":memory:")?;
	let main = CString::new("main")?;
	let mut clone = ptr::null_mut();
	let rc = unsafe { sqlite3_open(name.as_ptr(), &mut clone) };
	if rc != SQLITE_OK {
		let message = sqlite_error_message(clone);
		if !clone.is_null() {
			unsafe {
				sqlite3_close(clone);
			}
		}
		bail!("native sqlite oracle clone open failed with code {rc}: {message}");
	}

	let backup = unsafe { sqlite3_backup_init(clone, main.as_ptr(), source, main.as_ptr()) };
	if backup.is_null() {
		let message = sqlite_error_message(clone);
		unsafe {
			sqlite3_close(clone);
		}
		bail!("native sqlite oracle backup init failed: {message}");
	}

	let step_rc = unsafe { sqlite3_backup_step(backup, -1) };
	let finish_rc = unsafe { sqlite3_backup_finish(backup) };
	if step_rc != libsqlite3_sys::SQLITE_DONE || finish_rc != SQLITE_OK {
		let message = sqlite_error_message(clone);
		unsafe {
			sqlite3_close(clone);
		}
		bail!(
			"native sqlite oracle backup failed with step code {step_rc} and finish code {finish_rc}: {message}"
		);
	}

	Ok(clone)
}

#[derive(Clone, Debug, PartialEq)]
enum CanonicalValue {
	Null,
	Integer(i64),
	Float(f64),
	Text(String),
	Blob(Vec<u8>),
}

fn table_columns(db: *mut sqlite3, table_name: &str) -> Result<Vec<String>> {
	let sql = format!("PRAGMA table_info({});", quote_ident(table_name));
	query_rows(db, &sql)?
		.into_iter()
		.map(|row| match row.get(1) {
			Some(CanonicalValue::Text(name)) => Ok(name.clone()),
			_ => bail!("PRAGMA table_info returned malformed row: {row:?}"),
		})
		.collect()
}

fn query_rows(db: *mut sqlite3, sql: &str) -> Result<Vec<Vec<CanonicalValue>>> {
	let c_sql = CString::new(sql)?;
	let mut stmt = ptr::null_mut();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	if rc != SQLITE_OK {
		bail!(
			"{sql} prepare failed with code {rc}: {}",
			sqlite_error_message(db)
		);
	}
	if stmt.is_null() {
		return Ok(Vec::new());
	}

	let result = (|| {
		let mut rows = Vec::new();
		loop {
			match unsafe { sqlite3_step(stmt) } {
				SQLITE_ROW => rows.push(read_row(stmt)),
				libsqlite3_sys::SQLITE_DONE => break,
				step_rc => {
					bail!(
						"{sql} step failed with code {step_rc}: {}",
						sqlite_error_message(db)
					);
				}
			}
		}
		Ok(rows)
	})();

	unsafe {
		sqlite3_finalize(stmt);
	}

	result
}

fn read_row(stmt: *mut libsqlite3_sys::sqlite3_stmt) -> Vec<CanonicalValue> {
	let column_count = unsafe { sqlite3_column_count(stmt) };
	(0..column_count)
		.map(|index| read_value(stmt, index))
		.collect()
}

fn read_value(stmt: *mut libsqlite3_sys::sqlite3_stmt, index: i32) -> CanonicalValue {
	match unsafe { sqlite3_column_type(stmt, index) } {
		SQLITE_INTEGER => CanonicalValue::Integer(unsafe { sqlite3_column_int64(stmt, index) }),
		SQLITE_FLOAT => CanonicalValue::Float(unsafe { sqlite3_column_double(stmt, index) }),
		SQLITE_TEXT => {
			let text = unsafe { sqlite3_column_text(stmt, index) };
			if text.is_null() {
				CanonicalValue::Null
			} else {
				let len = unsafe { sqlite3_column_bytes(stmt, index) } as usize;
				let bytes = unsafe { std::slice::from_raw_parts(text.cast::<u8>(), len) };
				CanonicalValue::Text(String::from_utf8_lossy(bytes).into_owned())
			}
		}
		SQLITE_BLOB => {
			let blob = unsafe { sqlite3_column_blob(stmt, index) };
			if blob.is_null() {
				CanonicalValue::Null
			} else {
				let len = unsafe { sqlite3_column_bytes(stmt, index) } as usize;
				let bytes = unsafe { std::slice::from_raw_parts(blob.cast::<u8>(), len) };
				CanonicalValue::Blob(bytes.to_vec())
			}
		}
		SQLITE_NULL => CanonicalValue::Null,
		other => CanonicalValue::Text(format!("UNKNOWN({other})")),
	}
}

fn render_row(row: &[CanonicalValue]) -> String {
	row.iter().map(render_value).collect::<Vec<_>>().join("|")
}

fn render_value(value: &CanonicalValue) -> String {
	match value {
		CanonicalValue::Null => "null".to_string(),
		CanonicalValue::Integer(value) => format!("integer:{value}"),
		CanonicalValue::Float(value) => format!("float:{:016X}", value.to_bits()),
		CanonicalValue::Text(value) => format!("text:{}", escape_text(value)),
		CanonicalValue::Blob(value) => format!("blob:{}", hex_upper(value)),
	}
}

fn escape_text(value: &str) -> String {
	value
		.chars()
		.flat_map(|ch| match ch {
			'\\' => "\\\\".chars().collect::<Vec<_>>(),
			'\n' => "\\n".chars().collect(),
			'\r' => "\\r".chars().collect(),
			'\t' => "\\t".chars().collect(),
			'|' => "\\|".chars().collect(),
			ch => vec![ch],
		})
		.collect()
}

fn quote_ident(value: &str) -> String {
	format!("\"{}\"", value.replace('"', "\"\""))
}

fn hex_upper(bytes: &[u8]) -> String {
	const HEX: &[u8; 16] = b"0123456789ABCDEF";
	let mut out = String::with_capacity(bytes.len() * 2);
	for byte in bytes {
		out.push(HEX[(byte >> 4) as usize] as char);
		out.push(HEX[(byte & 0x0f) as usize] as char);
	}
	out
}

fn sqlite_exec_result(db: *mut sqlite3, sql: &str) -> Result<()> {
	let c_sql = CString::new(sql)?;
	let rc = unsafe { sqlite3_exec(db, c_sql.as_ptr(), None, ptr::null_mut(), ptr::null_mut()) };
	if rc != SQLITE_OK {
		bail!("{sql} failed with code {rc}: {}", sqlite_error_message(db));
	}
	Ok(())
}

fn sqlite_error_message(db: *mut sqlite3) -> String {
	unsafe {
		if db.is_null() {
			"unknown sqlite error".to_string()
		} else {
			CStr::from_ptr(sqlite3_errmsg(db))
				.to_string_lossy()
				.into_owned()
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn oracle_commit_semantics_apply_only_durable_ops() -> Result<()> {
		let mut oracle = NativeSqliteOracle::open()?;
		oracle.apply_sql("CREATE TABLE kv (k TEXT PRIMARY KEY, v BLOB NOT NULL);")?;
		oracle.apply_logical_op(
			LogicalOp::Put {
				key: "skipped".to_string(),
				value: vec![0],
			},
			OracleCommitSemantics::PreCommitFailure,
		)?;
		oracle.apply_logical_op(
			LogicalOp::Put {
				key: "kept".to_string(),
				value: vec![1, 2],
			},
			OracleCommitSemantics::Success,
		)?;
		oracle.apply_logical_op(
			LogicalOp::Put {
				key: "ambiguous".to_string(),
				value: vec![3, 4],
			},
			OracleCommitSemantics::AmbiguousPostCommit,
		)?;

		let dump = canonical_dump(oracle.db)?.render();
		assert!(dump.contains("row|kv|text:kept|blob:0102"));
		assert!(!dump.contains("ambiguous"));
		assert!(!dump.contains("skipped"));
		Ok(())
	}

	#[test]
	fn ambiguous_oracle_classifies_old_and_new_outcomes() -> Result<()> {
		let mut old_oracle = NativeSqliteOracle::open()?;
		old_oracle.apply_sql("CREATE TABLE kv (k TEXT PRIMARY KEY, v BLOB NOT NULL);")?;
		old_oracle.apply_logical_op(
			LogicalOp::Put {
				key: "ambiguous".to_string(),
				value: vec![3, 4],
			},
			OracleCommitSemantics::AmbiguousPostCommit,
		)?;
		assert_eq!(
			old_oracle.verify_matches(old_oracle.db)?,
			OracleVerification::Ambiguous(AmbiguousOracleOutcome::Old)
		);

		let mut new_oracle = NativeSqliteOracle::open()?;
		new_oracle.apply_sql("CREATE TABLE kv (k TEXT PRIMARY KEY, v BLOB NOT NULL);")?;
		let actual = clone_database(new_oracle.db)?;
		new_oracle.apply_logical_op(
			LogicalOp::Put {
				key: "ambiguous".to_string(),
				value: vec![3, 4],
			},
			OracleCommitSemantics::AmbiguousPostCommit,
		)?;
		(LogicalOp::Put {
			key: "ambiguous".to_string(),
			value: vec![3, 4],
		})
		.apply(actual)?;
		assert_eq!(
			new_oracle.verify_matches(actual)?,
			OracleVerification::Ambiguous(AmbiguousOracleOutcome::New)
		);
		unsafe {
			sqlite3_close(actual);
		}
		let dump = canonical_dump(new_oracle.db)?.render();
		assert!(dump.contains("row|kv|text:ambiguous|blob:0304"));
		Ok(())
	}

	#[test]
	fn canonical_dump_orders_schema_rows_and_typed_values() -> Result<()> {
		let mut oracle = NativeSqliteOracle::open()?;
		oracle.apply_sql(
			"CREATE TABLE b (k TEXT PRIMARY KEY, v BLOB, n INTEGER, r REAL, z TEXT); \
			 CREATE TABLE a (id INTEGER PRIMARY KEY, label TEXT); \
			 INSERT INTO b (k, v, n, r, z) VALUES ('z', x'CAFE', 7, 1.5, 'pipe|tab\t'); \
			 INSERT INTO b (k, v, n, r, z) VALUES ('a', NULL, -1, 2.0, NULL); \
			 INSERT INTO a (id, label) VALUES (2, 'two'), (1, 'one');",
		)?;

		let dump = canonical_dump(oracle.db)?.render();
		let row_a_one = dump
			.find("row|a|integer:1|text:one")
			.context("missing a row one")?;
		let row_a_two = dump
			.find("row|a|integer:2|text:two")
			.context("missing a row two")?;
		let row_b_a = dump
			.find("row|b|text:a|null|integer:-1|float:4000000000000000|null")
			.context("missing b row a")?;
		let row_b_z = dump
			.find("row|b|text:z|blob:CAFE|integer:7|float:3FF8000000000000|text:pipe\\|tab\\t")
			.context("missing b row z")?;
		assert!(row_a_one < row_a_two);
		assert!(row_a_two < row_b_a);
		assert!(row_b_a < row_b_z);
		Ok(())
	}

	#[test]
	fn integrity_helpers_reject_foreign_key_violations() -> Result<()> {
		let mut oracle = NativeSqliteOracle::open()?;
		oracle.apply_sql(
			"PRAGMA foreign_keys = OFF; \
			 CREATE TABLE parent (id INTEGER PRIMARY KEY); \
			 CREATE TABLE child (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES parent(id)); \
			 INSERT INTO child (id, parent_id) VALUES (1, 99);",
		)?;

		let err = NativeSqliteOracle::verify_integrity(oracle.db)
			.expect_err("foreign_key_check should reject the orphan row");
		assert!(err.to_string().contains("foreign_key_check failed"));
		Ok(())
	}
}
