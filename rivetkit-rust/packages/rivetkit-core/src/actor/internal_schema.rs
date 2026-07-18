use anyhow::{Context, Result, bail};

use crate::sqlite::{BindParam, ColumnValue, SqliteBatchStatement, SqliteDb};

pub(crate) const INTERNAL_SCHEMA_VERSION: i64 = 7;

const SCHEMA_VERSION_KEY: &str = "schema_version";

// `_rivet_meta` is the bootstrap root created before the numbered migrations.
// `schema_version` cannot live in a table created by those migrations, and
// `kv_import_state` must survive clearing partially imported runtime tables so
// interrupted imports can be detected and retried. This is not a general-
// purpose runtime KV store; its text accessors are migration bookkeeping only.
// W[bootstrap + import bookkeeping only | point upsert | <100 B | 1-page map]
const CREATE_META_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS _rivet_meta (
    key   TEXT PRIMARY KEY,
    value BLOB NOT NULL
) STRICT, WITHOUT ROWID
"#;

const MIGRATIONS: &[&[&str]] = &[
	&[
		// W[queue_next_id per enqueue; alarm per head-change; token once | point UPDATE of one column | <100 B | single-row: all runtime singletons on one leaf]
		r#"
CREATE TABLE _rivet_runtime (
    id                INTEGER PRIMARY KEY CHECK (id = 1),
    last_pushed_alarm INTEGER,
    inspector_token   TEXT,
    queue_next_id     INTEGER NOT NULL DEFAULT 1
) STRICT
"#,
		// W[once at init | single INSERT | input <=256 KiB | COLD: never rewritten; overflow chain isolated from hot state]
		r#"
CREATE TABLE _rivet_actor (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    has_initialized INTEGER NOT NULL,
    input           BLOB
) STRICT
"#,
		// W[debounced save ~1/s + immediate at shutdown | UPDATE state | <=256 KiB | HOT: sole column, so saves dirty only state pages]
		r#"
CREATE TABLE _rivet_actor_state (
    id    INTEGER PRIMARY KEY CHECK (id = 1),
    state BLOB NOT NULL
) STRICT
"#,
	],
	&[
		// W[per schedule/cancel/fire, immediate | point insert/delete | <200 B | replaces full actor blob rewrite with one row]
		r#"
CREATE TABLE _rivet_schedule_events (
    event_id   TEXT PRIMARY KEY,
    trigger_at INTEGER NOT NULL,
    action     TEXT NOT NULL,
    args       BLOB
) STRICT, WITHOUT ROWID
"#,
		r#"
CREATE INDEX _rivet_schedule_events_trigger_at
    ON _rivet_schedule_events (trigger_at)
"#,
	],
	&[
		// W[once per connect, DELETE on disconnect | whole row | up to 256 KiB | COLD: immutable per conn, separate from hot message index]
		r#"
CREATE TABLE _rivet_conns (
    conn_id         TEXT PRIMARY KEY,
    parameters      BLOB NOT NULL,
    gateway_id      BLOB NOT NULL,
    request_id      BLOB NOT NULL,
    request_path    TEXT NOT NULL,
    request_headers BLOB NOT NULL
) STRICT, WITHOUT ROWID
"#,
		// W[dirty per WS message, written debounced ~1/s; rewritten at sleep | point UPDATE | ~100-300 B | HOT: compact conn state rows]
		r#"
CREATE TABLE _rivet_conn_state (
    conn_id              TEXT PRIMARY KEY,
    state                BLOB NOT NULL,
    server_message_index INTEGER NOT NULL,
    subscriptions        BLOB NOT NULL
) STRICT, WITHOUT ROWID
"#,
	],
	&[
		// W[per enqueue plus queue_next_id; batch DELETE on receive/ack | append/delete, never rewritten | body <=256 KiB | INTEGER PK avoids hidden index]
		r#"
CREATE TABLE _rivet_queue (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    body       BLOB NOT NULL,
    created_at INTEGER NOT NULL
) STRICT
"#,
	],
	&[
		// W[per workflow step flush | keyed upsert + range delete | values <=256 KiB | verbatim fdb-tuple keys in one clustered tree]
		r#"
CREATE TABLE _rivet_wf_kv (
    key   BLOB PRIMARY KEY,
    value BLOB NOT NULL
) STRICT, WITHOUT ROWID
"#,
	],
	&[
		// W[per c.kv op (deprecated) | keyed upsert/delete/range | values <=128 KiB | verbatim raw KV key bytes]
		r#"
CREATE TABLE _rivet_user_kv (
    key   BLOB PRIMARY KEY,
    value BLOB NOT NULL
) STRICT, WITHOUT ROWID
"#,
	],
	&[
		// W[dirty per client WS message, written with other conn state | point UPDATE | 2 B logical value | preserves the client ack cursor across hibernation]
		r#"
ALTER TABLE _rivet_conn_state
    ADD COLUMN client_message_index INTEGER NOT NULL DEFAULT 0
"#,
	],
];

pub(crate) async fn ensure_internal_schema(db: &SqliteDb) -> Result<()> {
	db.execute(CREATE_META_TABLE, None)
		.await
		.context("create rivet internal schema metadata table")?;

	let current_version = read_schema_version(db).await?;
	if current_version > INTERNAL_SCHEMA_VERSION {
		bail!(
			"actor sqlite internal schema version {current_version} is newer than supported version {INTERNAL_SCHEMA_VERSION}"
		);
	}
	if current_version == INTERNAL_SCHEMA_VERSION {
		return Ok(());
	}

	apply_schema_ladder(db, current_version).await
}

async fn apply_schema_ladder(db: &SqliteDb, current_version: i64) -> Result<()> {
	let statements = migration_statements(current_version, INTERNAL_SCHEMA_VERSION)?;
	match db.execute_batch(statements).await {
		Ok(_) => Ok(()),
		Err(error) if is_commit_too_large(&error) => {
			for version in current_version + 1..=INTERNAL_SCHEMA_VERSION {
				db.execute_batch(migration_statements(version - 1, version)?)
					.await
					.with_context(|| format!("apply rivet internal schema migration v{version}"))?;
			}
			Ok(())
		}
		Err(error) => Err(error).context("apply rivet internal schema migrations"),
	}
}

fn migration_statements(from_version: i64, to_version: i64) -> Result<Vec<SqliteBatchStatement>> {
	if from_version < 0 || to_version < from_version || to_version > INTERNAL_SCHEMA_VERSION {
		bail!("invalid internal schema migration range {from_version}..={to_version}");
	}

	let mut statements = Vec::new();
	for migration in &MIGRATIONS[from_version as usize..to_version as usize] {
		for sql in *migration {
			statements.push(SqliteBatchStatement {
				sql: (*sql).to_owned(),
				params: None,
			});
		}
	}
	statements.push(SqliteBatchStatement {
		sql: "INSERT INTO _rivet_meta (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value".to_owned(),
		params: Some(vec![
			BindParam::Text(SCHEMA_VERSION_KEY.to_owned()),
			BindParam::Blob(encode_schema_version(to_version)),
		]),
	});
	Ok(statements)
}

async fn read_schema_version(db: &SqliteDb) -> Result<i64> {
	let result = db
		.query(
			"SELECT value FROM _rivet_meta WHERE key = ?",
			Some(vec![BindParam::Text(SCHEMA_VERSION_KEY.to_owned())]),
		)
		.await
		.context("read rivet internal schema version")?;

	let Some(row) = result.rows.first() else {
		return Ok(0);
	};
	let Some(ColumnValue::Blob(value)) = row.first() else {
		bail!("invalid rivet internal schema_version row");
	};
	decode_schema_version(value)
}

fn encode_schema_version(version: i64) -> Vec<u8> {
	version.to_le_bytes().to_vec()
}

fn decode_schema_version(value: &[u8]) -> Result<i64> {
	let bytes: [u8; 8] = value
		.try_into()
		.context("rivet internal schema_version must be an i64 little-endian blob")?;
	Ok(i64::from_le_bytes(bytes))
}

fn is_commit_too_large(error: &anyhow::Error) -> bool {
	error
		.chain()
		.any(|cause| cause.to_string().contains("commit_too_large"))
}

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../../tests/internal_schema.rs"]
pub(crate) mod tests;
