//! Agent-os SQLite persistence: schema + idempotent migration.
//!
//! The agent-os actor persists all of its durable state — signed preview
//! tokens, sessions, session events, and (eventually) the filesystem — to its
//! per-actor SQLite database via `ctx.sql()` / `ctx.db_*`. There is no actor-KV
//! state for agent-os; SQLite is the single source of truth. The schema is
//! ported from the (deleted) TS `agent-os/actor/db.ts` `migrateAgentOsTables`.

use std::io::Cursor;
use std::time::{SystemTime, UNIX_EPOCH};

use agent_os_client::SidecarJsBridgeCall;
use anyhow::{Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rivetkit::Ctx;
use serde_json::{Map as JsonMap, Value as JsonValue, json};

use crate::actor::AgentOsActor;

/// All agent-os persistence tables + indexes. Every statement is
/// `IF NOT EXISTS`, so this is idempotent and safe to run on every actor start.
pub const MIGRATION_SQL: &str = "\
CREATE TABLE IF NOT EXISTS agent_os_preview_tokens (
	token TEXT PRIMARY KEY,
	port INTEGER NOT NULL,
	created_at INTEGER NOT NULL,
	expires_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_preview_tokens_expires_at
	ON agent_os_preview_tokens(expires_at);
CREATE TABLE IF NOT EXISTS agent_os_fs_entries (
	path TEXT PRIMARY KEY,
	is_directory INTEGER NOT NULL DEFAULT 0,
	content BLOB,
	mode INTEGER NOT NULL DEFAULT 33188,
	uid INTEGER NOT NULL DEFAULT 0,
	gid INTEGER NOT NULL DEFAULT 0,
	size INTEGER NOT NULL DEFAULT 0,
	atime_ms INTEGER NOT NULL,
	mtime_ms INTEGER NOT NULL,
	ctime_ms INTEGER NOT NULL,
	birthtime_ms INTEGER NOT NULL,
	symlink_target TEXT,
	nlink INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_fs_entries_parent
	ON agent_os_fs_entries(path);
CREATE TABLE IF NOT EXISTS agent_os_sessions (
	session_id TEXT PRIMARY KEY,
	agent_type TEXT NOT NULL,
	capabilities TEXT NOT NULL,
	agent_info TEXT,
	created_at INTEGER NOT NULL,
	-- Original create-time cwd + env (env as a JSON object), threaded into the
	-- fallback `session/new` at resume so the rehydrated session keeps the same
	-- working dir + environment instead of defaulting (spec §12b item 3).
	cwd TEXT,
	env TEXT
);
CREATE TABLE IF NOT EXISTS agent_os_session_events (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	session_id TEXT NOT NULL,
	seq INTEGER NOT NULL,
	event TEXT NOT NULL,
	created_at INTEGER NOT NULL,
	FOREIGN KEY (session_id) REFERENCES agent_os_sessions(session_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_session_events_session_seq
	ON agent_os_session_events(session_id, seq);
";

/// Run the agent-os schema migration against the actor's SQLite database.
/// Idempotent; intended to be called once at the top of the actor run loop.
pub async fn migrate_actor(ctx: &Ctx<AgentOsActor>) -> Result<()> {
	ctx.db_exec(MIGRATION_SQL).await?;
	Ok(())
}

// ---------------------------------------------------------------------------
// SQLite helpers shared by the persistence-backed actions (previews, sessions).
//
// The actor `ctx.db_*` API takes parameters as a CBOR-encoded JSON array and
// returns rows as CBOR-encoded JSON objects (column name -> value).
// ---------------------------------------------------------------------------

/// Encode positional bind params as the CBOR JSON array the `db_*` API expects.
fn cbor_params(values: &[JsonValue]) -> Result<Vec<u8>> {
	let mut buf = Vec::new();
	ciborium::into_writer(&JsonValue::Array(values.to_vec()), &mut buf)?;
	Ok(buf)
}

/// Decode a `db_query` CBOR result into object rows (column -> value).
fn decode_rows(bytes: &[u8]) -> Result<Vec<serde_json::Map<String, JsonValue>>> {
	if bytes.is_empty() {
		return Ok(Vec::new());
	}
	let value: JsonValue = ciborium::from_reader(Cursor::new(bytes))?;
	Ok(match value {
		JsonValue::Array(rows) => rows
			.into_iter()
			.filter_map(|row| match row {
				JsonValue::Object(map) => Some(map),
				_ => None,
			})
			.collect(),
		_ => Vec::new(),
	})
}

/// Run a parameterized query and return the decoded object rows.
pub(crate) async fn query_rows(
	ctx: &Ctx<AgentOsActor>,
	sql: &str,
	params: &[JsonValue],
) -> Result<Vec<serde_json::Map<String, JsonValue>>> {
	let encoded = cbor_params(params)?;
	let bytes = ctx.db_query(sql, Some(encoded.as_slice())).await?;
	decode_rows(&bytes)
}

/// Run a parameterized statement that returns no rows (INSERT/UPDATE/DELETE).
pub(crate) async fn run_stmt(
	ctx: &Ctx<AgentOsActor>,
	sql: &str,
	params: &[JsonValue],
) -> Result<()> {
	let encoded = cbor_params(params)?;
	ctx.db_run(sql, Some(encoded.as_slice())).await?;
	Ok(())
}

const SQLITE_VFS_MOUNT_ID: &str = "rivetkit-agent-os-root";
const DEFAULT_FILE_MODE: i64 = 0o100644;
const DEFAULT_DIR_MODE: i64 = 0o040755;
const DEFAULT_SYMLINK_MODE: i64 = 0o120777;

// ---------------------------------------------------------------------------
// Session-event capture + transcript reconstruction (session-resume, spec §5/§7).
//
// `agent_os_session_events` is the canonical append-only conversation log,
// keyed by `external_session_id`. `seq` is INTERNAL ordering only: it is never
// surfaced to a client as a cursor/recovery state, so the "ACP session events
// are live-only" invariant holds — this is consumer-side durable persistence,
// not a replay buffer on the live `onSessionEvent` API.
// ---------------------------------------------------------------------------

/// Append one captured event to `agent_os_session_events` under the stable
/// `external_session_id`, allocating the next per-session `seq` (`MAX(seq)+1`).
///
/// `event_json` is the raw JSON text of either an inbound ACP `session/update`
/// notification (captured via `on_session_event`) or a synthetic outbound
/// `user_prompt` event recorded in `send_prompt` before the prompt streams.
pub async fn insert_session_event(
	ctx: &Ctx<AgentOsActor>,
	external_session_id: &str,
	event_json: &str,
) -> Result<()> {
	// Allocate the next per-session `seq` ATOMICALLY inside the INSERT. The capture
	// pump (a spawned tokio task) and the prompt action both call this and run
	// concurrently on the runtime, so a separate `SELECT MAX(seq)` followed by an
	// INSERT would race: both reads could observe the same max and write a duplicate
	// `seq` (there is no UNIQUE(session_id, seq) constraint). SQLite serializes
	// writers, so computing `MAX(seq)+1` in a sub-SELECT within the INSERT is atomic
	// against other inserts and cannot duplicate. `MAX(seq)` over an empty set is
	// SQL NULL → `COALESCE(..., 0)` gives the first event seq 0. `seq` is internal
	// ordering only, never a client-facing cursor (live-only invariant).
	run_stmt(
		ctx,
		"INSERT INTO agent_os_session_events (session_id, seq, event, created_at) \
		 SELECT ?, \
		        COALESCE((SELECT MAX(seq) + 1 FROM agent_os_session_events WHERE session_id = ?), 0), \
		        ?, ?",
		&[
			json!(external_session_id),
			json!(external_session_id),
			json!(event_json),
			json!(now_ms()),
		],
	)
	.await
}

/// Render the persisted event log for `external_session_id` to a Markdown
/// transcript and write it to a guest-readable path via the VM filesystem
/// callback, returning that path (spec §7).
///
/// The file is a disposable on-demand render of the canonical
/// `agent_os_session_events` rows: it is overwritten fresh each resume and is
/// **idempotent** (two reconstructs of the same rows produce identical bytes,
/// no append-duplication). The path is handed to the sidecar resume request so
/// a fallback agent can read prior context with its file tools.
pub async fn reconstruct_transcript_to_file(
	ctx: &Ctx<AgentOsActor>,
	external_session_id: &str,
) -> Result<String> {
	let rows = query_rows(
		ctx,
		"SELECT event FROM agent_os_session_events WHERE session_id = ? ORDER BY seq",
		&[json!(external_session_id)],
	)
	.await?;
	let events: Vec<JsonValue> = rows
		.into_iter()
		.filter_map(|mut row| {
			row.remove("event")
				.and_then(|v| match v {
					JsonValue::String(raw) => Some(raw),
					_ => None,
				})
				.and_then(|raw| serde_json::from_str::<JsonValue>(&raw).ok())
		})
		.collect();

	let markdown = render_transcript_markdown(external_session_id, &events);

	let path = format!("/root/.agentos/threads/{external_session_id}.md");
	// Ensure the parent directory chain exists (mkdir -p), then overwrite the
	// transcript fresh. Both go through the same sqlite_vfs callback the sidecar
	// uses, so the bytes are visible to the guest agent.
	create_dir(ctx, "/root/.agentos/threads", DEFAULT_DIR_MODE, true).await?;
	// The callback stores base64 file content (see `handle_sqlite_vfs_call`).
	write_file(ctx, &path, BASE64.encode(markdown), DEFAULT_FILE_MODE).await?;
	Ok(path)
}

/// Render captured ACP events to a role-labeled Markdown transcript. Pure /
/// deterministic so reconstruction is idempotent.
fn render_transcript_markdown(external_session_id: &str, events: &[JsonValue]) -> String {
	let mut out = String::new();
	out.push_str(&format!("# Session transcript: {external_session_id}\n"));

	for event in events {
		// Synthetic outbound prompt event (recorded in `send_prompt`).
		if event.get("method").and_then(JsonValue::as_str) == Some("user_prompt") {
			if let Some(text) = event
				.get("params")
				.and_then(|p| p.get("text"))
				.and_then(JsonValue::as_str)
			{
				out.push_str(&format!("\n## User\n\n{text}\n"));
			}
			continue;
		}

		// Inbound `session/update` notifications: the conversation content a
		// transcript needs (`update.sessionUpdate` discriminator).
		let Some(update) = event.get("params").and_then(|p| p.get("update")) else {
			continue;
		};
		let kind = update
			.get("sessionUpdate")
			.and_then(JsonValue::as_str)
			.unwrap_or("");
		match kind {
			"agent_message_chunk" | "agent_thought_chunk" => {
				if let Some(text) = update
					.get("content")
					.and_then(|c| c.get("text"))
					.and_then(JsonValue::as_str)
				{
					if kind == "agent_thought_chunk" {
						out.push_str(&format!("\n## Assistant (thinking)\n\n{text}\n"));
					} else {
						out.push_str(&format!("\n## Assistant\n\n{text}\n"));
					}
				}
			}
			"tool_call" | "tool_call_update" => {
				let title = update
					.get("title")
					.and_then(JsonValue::as_str)
					.or_else(|| update.get("kind").and_then(JsonValue::as_str))
					.unwrap_or("tool call");
				let status = update
					.get("status")
					.and_then(JsonValue::as_str)
					.unwrap_or("");
				out.push_str(&format!("\n### Tool call: {title}"));
				if !status.is_empty() {
					out.push_str(&format!(" ({status})"));
				}
				out.push('\n');
				// Render any textual tool output content.
				if let Some(content) = update.get("content").and_then(JsonValue::as_array) {
					for item in content {
						if let Some(text) = item
							.get("content")
							.and_then(|c| c.get("text"))
							.and_then(JsonValue::as_str)
							.or_else(|| item.get("text").and_then(JsonValue::as_str))
						{
							out.push_str(&format!("\n```\n{text}\n```\n"));
						}
					}
				}
			}
			_ => {}
		}
	}

	out
}

/// Native sqlite_vfs callback used by the Agent OS sidecar when the root is
/// backed by Rivet actor SQLite. The sidecar speaks base64 at the bridge
/// boundary; this actor stores that base64 text in `agent_os_fs_entries.content`
/// because the current `ctx.db_*` binder cannot bind raw BLOB parameters.
pub async fn handle_sqlite_vfs_call(
	ctx: &Ctx<AgentOsActor>,
	call: SidecarJsBridgeCall,
) -> std::result::Result<Option<JsonValue>, String> {
	if call.mount_id != SQLITE_VFS_MOUNT_ID {
		return Err(format!("ENOENT unknown sqlite_vfs mount {}", call.mount_id));
	}

	handle_sqlite_vfs_call_inner(ctx, call)
		.await
		.map_err(|error| error.to_string())
}

async fn handle_sqlite_vfs_call_inner(
	ctx: &Ctx<AgentOsActor>,
	call: SidecarJsBridgeCall,
) -> Result<Option<JsonValue>> {
	ensure_fs_root(ctx).await?;
	let args = call.args;
	match call.operation.as_str() {
		"readFile" => Ok(Some(json!(read_file(ctx, required_path(&args)?).await?))),
		"writeFile" => {
			write_file(
				ctx,
				required_path(&args)?,
				required_string(&args, "content")?,
				optional_i64(&args, "mode").unwrap_or(DEFAULT_FILE_MODE),
			)
			.await?;
			Ok(None)
		}
		"createFileExclusive" => {
			create_file_exclusive(
				ctx,
				required_path(&args)?,
				required_string(&args, "content")?,
				optional_i64(&args, "mode").unwrap_or(DEFAULT_FILE_MODE),
			)
			.await?;
			Ok(None)
		}
		"readDir" => Ok(Some(JsonValue::Array(
			read_dir(ctx, required_path(&args)?)
				.await?
				.into_iter()
				.map(JsonValue::String)
				.collect(),
		))),
		"readDirWithTypes" => Ok(Some(JsonValue::Array(
			read_dir_entries(ctx, required_path(&args)?)
				.await?
				.into_iter()
				.map(|entry| {
					json!({
						"name": entry.name,
						"isDirectory": entry.is_directory,
						"isSymbolicLink": entry.symlink_target.is_some(),
					})
				})
				.collect(),
		))),
		"createDir" => {
			create_dir(
				ctx,
				required_path(&args)?,
				optional_i64(&args, "mode").unwrap_or(DEFAULT_DIR_MODE),
				false,
			)
			.await?;
			Ok(None)
		}
		"mkdir" => {
			let recursive = args
				.get("recursive")
				.and_then(JsonValue::as_bool)
				.unwrap_or(false);
			create_dir(
				ctx,
				required_path(&args)?,
				optional_i64(&args, "mode").unwrap_or(DEFAULT_DIR_MODE),
				recursive,
			)
			.await?;
			Ok(None)
		}
		"exists" => Ok(Some(json!(
			lookup_entry(ctx, required_path(&args)?).await?.is_some()
		))),
		"access" => {
			lookup_entry_required(ctx, required_path(&args)?).await?;
			Ok(None)
		}
		"stat" => Ok(Some(stat_json(
			lookup_entry_required(ctx, required_path(&args)?).await?,
		))),
		"lstat" => Ok(Some(stat_json(
			lookup_entry_required(ctx, required_path(&args)?).await?,
		))),
		"open" => Ok(Some(stat_json(
			lookup_entry_required(ctx, required_path(&args)?).await?,
		))),
		"removeFile" => {
			remove_file(ctx, required_path(&args)?).await?;
			Ok(None)
		}
		"removeDir" => {
			remove_dir(ctx, required_path(&args)?).await?;
			Ok(None)
		}
		"rename" => {
			rename_entry(
				ctx,
				required_string(&args, "oldPath")?,
				required_string(&args, "newPath")?,
			)
			.await?;
			Ok(None)
		}
		"realpath" => Ok(Some(json!(normalize_path(required_path(&args)?)?))),
		"symlink" => {
			symlink_entry(
				ctx,
				required_string(&args, "target")?,
				required_string(&args, "path")?,
			)
			.await?;
			Ok(None)
		}
		"readLink" => Ok(Some(json!(read_link(ctx, required_path(&args)?).await?))),
		"link" => {
			link_entry(
				ctx,
				required_string(&args, "oldPath")?,
				required_string(&args, "newPath")?,
			)
			.await?;
			Ok(None)
		}
		"chmod" => {
			update_one_field(
				ctx,
				required_path(&args)?,
				"mode",
				json!(required_i64(&args, "mode")?),
			)
			.await?;
			Ok(None)
		}
		"chown" => {
			update_owner(
				ctx,
				required_path(&args)?,
				required_i64(&args, "uid")?,
				required_i64(&args, "gid")?,
			)
			.await?;
			Ok(None)
		}
		"utimes" => {
			update_times(
				ctx,
				required_path(&args)?,
				required_i64(&args, "atimeMs")?,
				required_i64(&args, "mtimeMs")?,
			)
			.await?;
			Ok(None)
		}
		"truncate" => {
			truncate_file(ctx, required_path(&args)?, required_len(&args)?).await?;
			Ok(None)
		}
		"pread" => Ok(Some(json!(
			pread_file(
				ctx,
				required_path(&args)?,
				required_i64(&args, "offset")?,
				required_len(&args)?,
			)
			.await?
		))),
		operation => bail!("ENOSYS unsupported sqlite_vfs operation {operation}"),
	}
}

#[derive(Clone, Debug)]
struct FsEntry {
	path: String,
	name: String,
	is_directory: bool,
	content: Option<String>,
	mode: i64,
	uid: i64,
	gid: i64,
	size: i64,
	atime_ms: i64,
	mtime_ms: i64,
	ctime_ms: i64,
	birthtime_ms: i64,
	symlink_target: Option<String>,
	nlink: i64,
}

impl FsEntry {
	fn from_row(mut row: JsonMap<String, JsonValue>) -> Result<Self> {
		let path = string_col(&mut row, "path")?;
		Ok(Self {
			name: basename(&path),
			path,
			is_directory: int_col(&mut row, "is_directory")? != 0,
			content: optional_content_col(&mut row, "content")?,
			mode: int_col(&mut row, "mode")?,
			uid: int_col(&mut row, "uid")?,
			gid: int_col(&mut row, "gid")?,
			size: int_col(&mut row, "size")?,
			atime_ms: int_col(&mut row, "atime_ms")?,
			mtime_ms: int_col(&mut row, "mtime_ms")?,
			ctime_ms: int_col(&mut row, "ctime_ms")?,
			birthtime_ms: int_col(&mut row, "birthtime_ms")?,
			symlink_target: optional_string_col(&mut row, "symlink_target")?,
			nlink: int_col(&mut row, "nlink")?,
		})
	}
}

fn now_ms() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_millis() as i64
}

async fn ensure_fs_root(ctx: &Ctx<AgentOsActor>) -> Result<()> {
	let now = now_ms();
	run_stmt(
		ctx,
		"INSERT OR IGNORE INTO agent_os_fs_entries
			(path, is_directory, content, mode, uid, gid, size, atime_ms, mtime_ms, ctime_ms, birthtime_ms, symlink_target, nlink)
			VALUES (?, 1, NULL, ?, 0, 0, 0, ?, ?, ?, ?, NULL, 2)",
		&[
			json!("/"),
			json!(DEFAULT_DIR_MODE),
			json!(now),
			json!(now),
			json!(now),
			json!(now),
		],
	)
	.await
}

async fn lookup_entry(ctx: &Ctx<AgentOsActor>, path: &str) -> Result<Option<FsEntry>> {
	let path = normalize_path(path)?;
	let rows = query_rows(
		ctx,
		"SELECT path, is_directory, content, mode, uid, gid, size, atime_ms, mtime_ms, ctime_ms, birthtime_ms, symlink_target, nlink
			FROM agent_os_fs_entries WHERE path = ?",
		&[json!(path)],
	)
	.await?;
	rows.into_iter().next().map(FsEntry::from_row).transpose()
}

async fn lookup_entry_required(ctx: &Ctx<AgentOsActor>, path: &str) -> Result<FsEntry> {
	lookup_entry(ctx, path)
		.await?
		.ok_or_else(|| anyhow!("ENOENT no such file or directory: {}", path))
}

async fn ensure_parent_dir(ctx: &Ctx<AgentOsActor>, path: &str) -> Result<()> {
	let Some(parent) = parent_path(path) else {
		return Ok(());
	};
	let parent = lookup_entry_required(ctx, &parent).await?;
	if !parent.is_directory {
		bail!("ENOTDIR parent is not a directory: {}", parent.path);
	}
	Ok(())
}

async fn read_file(ctx: &Ctx<AgentOsActor>, path: &str) -> Result<String> {
	let entry = lookup_entry_required(ctx, path).await?;
	if entry.is_directory {
		bail!("EISDIR is a directory: {}", entry.path);
	}
	Ok(entry.content.unwrap_or_default())
}

async fn write_file(ctx: &Ctx<AgentOsActor>, path: &str, content: String, mode: i64) -> Result<()> {
	let path = normalize_path(path)?;
	ensure_parent_dir(ctx, &path).await?;
	let size = decoded_len(&content)?;
	let now = now_ms();
	if let Some(existing) = lookup_entry(ctx, &path).await? {
		if existing.is_directory {
			bail!("EISDIR is a directory: {path}");
		}
		run_stmt(
			ctx,
			"UPDATE agent_os_fs_entries
				SET is_directory = 0, content = ?, mode = ?, size = ?, mtime_ms = ?, ctime_ms = ?, symlink_target = NULL, nlink = 1
				WHERE path = ?",
			&[
				json!(content),
				json!(mode),
				json!(size),
				json!(now),
				json!(now),
				json!(path),
			],
		)
		.await?;
		return Ok(());
	}
	insert_entry(ctx, &path, false, Some(content), mode, size, None, 1, now).await
}

async fn create_file_exclusive(
	ctx: &Ctx<AgentOsActor>,
	path: &str,
	content: String,
	mode: i64,
) -> Result<()> {
	let path = normalize_path(path)?;
	if lookup_entry(ctx, &path).await?.is_some() {
		bail!("EEXIST file exists: {path}");
	}
	ensure_parent_dir(ctx, &path).await?;
	let size = decoded_len(&content)?;
	insert_entry(
		ctx,
		&path,
		false,
		Some(content),
		mode,
		size,
		None,
		1,
		now_ms(),
	)
	.await
}

async fn create_dir(ctx: &Ctx<AgentOsActor>, path: &str, mode: i64, recursive: bool) -> Result<()> {
	let path = normalize_path(path)?;
	if path == "/" {
		return Ok(());
	}
	if let Some(existing) = lookup_entry(ctx, &path).await? {
		if recursive && existing.is_directory {
			return Ok(());
		}
		bail!("EEXIST file exists: {path}");
	}
	if recursive {
		let mut parents = Vec::new();
		let mut cursor = parent_path(&path);
		while let Some(parent) = cursor {
			if parent == "/" {
				break;
			}
			parents.push(parent.clone());
			cursor = parent_path(&parent);
		}
		parents.reverse();
		for parent in parents {
			if let Some(existing) = lookup_entry(ctx, &parent).await? {
				if !existing.is_directory {
					bail!("ENOTDIR parent is not a directory: {}", existing.path);
				}
				continue;
			}
			insert_entry(ctx, &parent, true, None, mode, 0, None, 2, now_ms()).await?;
		}
	} else {
		ensure_parent_dir(ctx, &path).await?;
	}
	insert_entry(ctx, &path, true, None, mode, 0, None, 2, now_ms()).await
}

async fn read_dir(ctx: &Ctx<AgentOsActor>, path: &str) -> Result<Vec<String>> {
	Ok(read_dir_entries(ctx, path)
		.await?
		.into_iter()
		.map(|entry| entry.name)
		.collect())
}

async fn read_dir_entries(ctx: &Ctx<AgentOsActor>, path: &str) -> Result<Vec<FsEntry>> {
	let path = normalize_path(path)?;
	let entry = lookup_entry_required(ctx, &path).await?;
	if !entry.is_directory {
		bail!("ENOTDIR not a directory: {path}");
	}
	let prefix = if path == "/" {
		"/".to_owned()
	} else {
		format!("{path}/")
	};
	let rows = query_rows(
		ctx,
		"SELECT path, is_directory, content, mode, uid, gid, size, atime_ms, mtime_ms, ctime_ms, birthtime_ms, symlink_target, nlink
			FROM agent_os_fs_entries WHERE path LIKE ? AND path != ? ORDER BY path",
		&[json!(format!("{prefix}%")), json!(path)],
	)
	.await?;
	rows.into_iter()
		.map(FsEntry::from_row)
		.filter_map(|entry| match entry {
			Ok(entry) if parent_path(&entry.path).as_deref() == Some(path.as_str()) => {
				Some(Ok(entry))
			}
			Ok(_) => None,
			Err(error) => Some(Err(error)),
		})
		.collect()
}

async fn remove_file(ctx: &Ctx<AgentOsActor>, path: &str) -> Result<()> {
	let entry = lookup_entry_required(ctx, path).await?;
	if entry.is_directory {
		bail!("EISDIR is a directory: {}", entry.path);
	}
	run_stmt(
		ctx,
		"DELETE FROM agent_os_fs_entries WHERE path = ?",
		&[json!(entry.path)],
	)
	.await
}

async fn remove_dir(ctx: &Ctx<AgentOsActor>, path: &str) -> Result<()> {
	let entry = lookup_entry_required(ctx, path).await?;
	if !entry.is_directory {
		bail!("ENOTDIR not a directory: {}", entry.path);
	}
	if entry.path == "/" {
		bail!("EBUSY cannot remove root directory");
	}
	if !read_dir_entries(ctx, &entry.path).await?.is_empty() {
		bail!("ENOTEMPTY directory not empty: {}", entry.path);
	}
	run_stmt(
		ctx,
		"DELETE FROM agent_os_fs_entries WHERE path = ?",
		&[json!(entry.path)],
	)
	.await
}

async fn rename_entry(ctx: &Ctx<AgentOsActor>, old_path: String, new_path: String) -> Result<()> {
	let old_path = normalize_path(&old_path)?;
	let new_path = normalize_path(&new_path)?;
	if old_path == "/" {
		bail!("EBUSY cannot rename root directory");
	}
	let entry = lookup_entry_required(ctx, &old_path).await?;
	ensure_parent_dir(ctx, &new_path).await?;
	if entry.is_directory && new_path.starts_with(&format!("{old_path}/")) {
		bail!("EINVAL cannot move directory into itself");
	}
	if let Some(existing) = lookup_entry(ctx, &new_path).await? {
		if existing.is_directory && !read_dir_entries(ctx, &existing.path).await?.is_empty() {
			bail!("ENOTEMPTY target directory not empty: {}", existing.path);
		}
		run_stmt(
			ctx,
			"DELETE FROM agent_os_fs_entries WHERE path = ?",
			&[json!(existing.path)],
		)
		.await?;
	}
	let old_prefix = format!("{old_path}/");
	let new_prefix = format!("{new_path}/");
	let rows = query_rows(
		ctx,
		"SELECT path FROM agent_os_fs_entries WHERE path = ? OR path LIKE ? ORDER BY path",
		&[json!(old_path), json!(format!("{old_prefix}%"))],
	)
	.await?;
	for row in rows {
		let path = row
			.get("path")
			.and_then(JsonValue::as_str)
			.ok_or_else(|| anyhow!("sqlite_vfs rename row missing path"))?;
		let next_path = if path == old_path {
			new_path.clone()
		} else {
			format!("{new_prefix}{}", &path[old_prefix.len()..])
		};
		run_stmt(
			ctx,
			"UPDATE agent_os_fs_entries SET path = ?, ctime_ms = ? WHERE path = ?",
			&[json!(next_path), json!(now_ms()), json!(path)],
		)
		.await?;
	}
	Ok(())
}

async fn symlink_entry(ctx: &Ctx<AgentOsActor>, target: String, path: String) -> Result<()> {
	let path = normalize_path(&path)?;
	if lookup_entry(ctx, &path).await?.is_some() {
		bail!("EEXIST file exists: {path}");
	}
	ensure_parent_dir(ctx, &path).await?;
	insert_entry(
		ctx,
		&path,
		false,
		None,
		DEFAULT_SYMLINK_MODE,
		target.len() as i64,
		Some(target),
		1,
		now_ms(),
	)
	.await
}

async fn read_link(ctx: &Ctx<AgentOsActor>, path: &str) -> Result<String> {
	let entry = lookup_entry_required(ctx, path).await?;
	entry
		.symlink_target
		.ok_or_else(|| anyhow!("EINVAL not a symbolic link: {}", entry.path))
}

async fn link_entry(ctx: &Ctx<AgentOsActor>, old_path: String, new_path: String) -> Result<()> {
	let old_path = normalize_path(&old_path)?;
	let new_path = normalize_path(&new_path)?;
	if lookup_entry(ctx, &new_path).await?.is_some() {
		bail!("EEXIST file exists: {new_path}");
	}
	ensure_parent_dir(ctx, &new_path).await?;
	let entry = lookup_entry_required(ctx, &old_path).await?;
	if entry.is_directory {
		bail!("EPERM cannot hard-link directory: {old_path}");
	}
	insert_entry(
		ctx,
		&new_path,
		false,
		entry.content,
		entry.mode,
		entry.size,
		entry.symlink_target,
		1,
		now_ms(),
	)
	.await?;
	update_one_field(ctx, &old_path, "nlink", json!(entry.nlink + 1)).await
}

async fn update_owner(ctx: &Ctx<AgentOsActor>, path: &str, uid: i64, gid: i64) -> Result<()> {
	let path = normalize_path(path)?;
	lookup_entry_required(ctx, &path).await?;
	run_stmt(
		ctx,
		"UPDATE agent_os_fs_entries SET uid = ?, gid = ?, ctime_ms = ? WHERE path = ?",
		&[json!(uid), json!(gid), json!(now_ms()), json!(path)],
	)
	.await
}

async fn update_times(
	ctx: &Ctx<AgentOsActor>,
	path: &str,
	atime_ms: i64,
	mtime_ms: i64,
) -> Result<()> {
	let path = normalize_path(path)?;
	lookup_entry_required(ctx, &path).await?;
	run_stmt(
		ctx,
		"UPDATE agent_os_fs_entries SET atime_ms = ?, mtime_ms = ?, ctime_ms = ? WHERE path = ?",
		&[
			json!(atime_ms),
			json!(mtime_ms),
			json!(now_ms()),
			json!(path),
		],
	)
	.await
}

async fn truncate_file(ctx: &Ctx<AgentOsActor>, path: &str, len: i64) -> Result<()> {
	if len < 0 {
		bail!("EINVAL negative truncate length");
	}
	let entry = lookup_entry_required(ctx, path).await?;
	if entry.is_directory {
		bail!("EISDIR is a directory: {}", entry.path);
	}
	let mut bytes = decode_content(entry.content.as_deref().unwrap_or_default())?;
	bytes.resize(len as usize, 0);
	let content = BASE64.encode(bytes);
	run_stmt(
		ctx,
		"UPDATE agent_os_fs_entries SET content = ?, size = ?, mtime_ms = ?, ctime_ms = ? WHERE path = ?",
		&[
			json!(content),
			json!(len),
			json!(now_ms()),
			json!(now_ms()),
			json!(entry.path),
		],
	)
	.await
}

async fn pread_file(ctx: &Ctx<AgentOsActor>, path: &str, offset: i64, len: i64) -> Result<String> {
	if offset < 0 || len < 0 {
		bail!("EINVAL negative pread offset or length");
	}
	let entry = lookup_entry_required(ctx, path).await?;
	if entry.is_directory {
		bail!("EISDIR is a directory: {}", entry.path);
	}
	let bytes = decode_content(entry.content.as_deref().unwrap_or_default())?;
	let start = (offset as usize).min(bytes.len());
	let end = start.saturating_add(len as usize).min(bytes.len());
	Ok(BASE64.encode(&bytes[start..end]))
}

async fn update_one_field(
	ctx: &Ctx<AgentOsActor>,
	path: &str,
	field: &str,
	value: JsonValue,
) -> Result<()> {
	let path = normalize_path(path)?;
	lookup_entry_required(ctx, &path).await?;
	let sql = match field {
		"mode" => "UPDATE agent_os_fs_entries SET mode = ?, ctime_ms = ? WHERE path = ?",
		"nlink" => "UPDATE agent_os_fs_entries SET nlink = ?, ctime_ms = ? WHERE path = ?",
		_ => bail!("EINVAL unsupported update field {field}"),
	};
	run_stmt(ctx, sql, &[value, json!(now_ms()), json!(path)]).await
}

async fn insert_entry(
	ctx: &Ctx<AgentOsActor>,
	path: &str,
	is_directory: bool,
	content: Option<String>,
	mode: i64,
	size: i64,
	symlink_target: Option<String>,
	nlink: i64,
	now: i64,
) -> Result<()> {
	run_stmt(
		ctx,
		"INSERT INTO agent_os_fs_entries
			(path, is_directory, content, mode, uid, gid, size, atime_ms, mtime_ms, ctime_ms, birthtime_ms, symlink_target, nlink)
			VALUES (?, ?, ?, ?, 0, 0, ?, ?, ?, ?, ?, ?, ?)",
		&[
			json!(path),
			json!(if is_directory { 1 } else { 0 }),
			content.map_or(JsonValue::Null, JsonValue::String),
			json!(mode),
			json!(size),
			json!(now),
			json!(now),
			json!(now),
			json!(now),
			symlink_target.map_or(JsonValue::Null, JsonValue::String),
			json!(nlink),
		],
	)
	.await
}

fn stat_json(entry: FsEntry) -> JsonValue {
	json!({
		"dev": 0,
		"ino": stable_ino(&entry.path),
		"mode": entry.mode,
		"nlink": entry.nlink,
		"uid": entry.uid,
		"gid": entry.gid,
		"rdev": 0,
		"size": entry.size,
		"blocks": if entry.size == 0 { 0 } else { (entry.size + 511) / 512 },
		"atimeMs": entry.atime_ms,
		"mtimeMs": entry.mtime_ms,
		"ctimeMs": entry.ctime_ms,
		"birthtimeMs": entry.birthtime_ms,
		"atimeNsec": (entry.atime_ms % 1000) * 1_000_000,
		"mtimeNsec": (entry.mtime_ms % 1000) * 1_000_000,
		"ctimeNsec": (entry.ctime_ms % 1000) * 1_000_000,
		"birthtimeNsec": (entry.birthtime_ms % 1000) * 1_000_000,
		"isDirectory": entry.is_directory,
		"isSymbolicLink": entry.symlink_target.is_some(),
	})
}

fn normalize_path(path: &str) -> Result<String> {
	if path.is_empty() {
		bail!("ENOENT empty path");
	}
	let mut parts = Vec::new();
	for part in path.split('/') {
		if part.is_empty() || part == "." {
			continue;
		}
		if part == ".." {
			parts.pop();
			continue;
		}
		parts.push(part);
	}
	if parts.is_empty() {
		Ok("/".to_owned())
	} else {
		Ok(format!("/{}", parts.join("/")))
	}
}

fn parent_path(path: &str) -> Option<String> {
	if path == "/" {
		return None;
	}
	let path = path.trim_end_matches('/');
	let index = path.rfind('/')?;
	if index == 0 {
		Some("/".to_owned())
	} else {
		Some(path[..index].to_owned())
	}
}

fn basename(path: &str) -> String {
	if path == "/" {
		return "/".to_owned();
	}
	path.rsplit('/').next().unwrap_or(path).to_owned()
}

fn required_path(args: &JsonValue) -> Result<&str> {
	required_string_ref(args, "path")
}

fn required_string(args: &JsonValue, key: &str) -> Result<String> {
	Ok(required_string_ref(args, key)?.to_owned())
}

fn required_string_ref<'a>(args: &'a JsonValue, key: &str) -> Result<&'a str> {
	args.get(key)
		.and_then(JsonValue::as_str)
		.ok_or_else(|| anyhow!("EINVAL missing string arg {key}"))
}

fn optional_i64(args: &JsonValue, key: &str) -> Option<i64> {
	args.get(key).and_then(JsonValue::as_i64)
}

fn required_i64(args: &JsonValue, key: &str) -> Result<i64> {
	optional_i64(args, key).ok_or_else(|| anyhow!("EINVAL missing integer arg {key}"))
}

fn required_len(args: &JsonValue) -> Result<i64> {
	optional_i64(args, "len")
		.or_else(|| optional_i64(args, "length"))
		.ok_or_else(|| anyhow!("EINVAL missing integer arg length"))
}

fn decoded_len(content: &str) -> Result<i64> {
	Ok(decode_content(content)?.len() as i64)
}

fn decode_content(content: &str) -> Result<Vec<u8>> {
	BASE64
		.decode(content)
		.map_err(|error| anyhow!("EINVAL invalid base64 file content: {error}"))
}

fn string_col(row: &mut JsonMap<String, JsonValue>, key: &str) -> Result<String> {
	row.remove(key)
		.and_then(|value| value.as_str().map(str::to_owned))
		.ok_or_else(|| anyhow!("sqlite_vfs row missing string column {key}"))
}

fn optional_string_col(row: &mut JsonMap<String, JsonValue>, key: &str) -> Result<Option<String>> {
	match row.remove(key) {
		Some(JsonValue::Null) | None => Ok(None),
		Some(JsonValue::String(value)) => Ok(Some(value)),
		Some(value) => bail!("sqlite_vfs row column {key} expected string/null, got {value:?}"),
	}
}

fn optional_content_col(row: &mut JsonMap<String, JsonValue>, key: &str) -> Result<Option<String>> {
	match row.remove(key) {
		Some(JsonValue::Null) | None => Ok(None),
		Some(JsonValue::String(value)) => Ok(Some(value)),
		Some(JsonValue::Array(bytes)) => {
			let raw = bytes
				.into_iter()
				.map(|value| {
					value
						.as_u64()
						.and_then(|byte| u8::try_from(byte).ok())
						.ok_or_else(|| {
							anyhow!("sqlite_vfs blob column {key} contains non-byte value")
						})
				})
				.collect::<Result<Vec<_>>>()?;
			Ok(Some(String::from_utf8(raw)?))
		}
		Some(value) => {
			bail!("sqlite_vfs row column {key} expected blob/string/null, got {value:?}")
		}
	}
}

fn int_col(row: &mut JsonMap<String, JsonValue>, key: &str) -> Result<i64> {
	row.remove(key)
		.and_then(|value| value.as_i64())
		.ok_or_else(|| anyhow!("sqlite_vfs row missing integer column {key}"))
}

fn stable_ino(path: &str) -> u64 {
	let mut hash = 0xcbf29ce484222325u64;
	for byte in path.as_bytes() {
		hash ^= u64::from(*byte);
		hash = hash.wrapping_mul(0x100000001b3);
	}
	hash
}

#[cfg(test)]
mod sqlite_vfs_callback_tests {
	use super::*;

	#[test]
	fn path_normalization_is_absolute_and_stays_within_root() {
		assert_eq!(normalize_path("/a/./b").unwrap(), "/a/b");
		assert_eq!(normalize_path("a/../b").unwrap(), "/b");
		assert_eq!(normalize_path("/../../").unwrap(), "/");
	}

	#[test]
	fn content_column_accepts_blob_rows_from_sqlite() {
		let mut row = JsonMap::new();
		row.insert(
			"content".to_owned(),
			JsonValue::Array(
				b"aGVsbG8="
					.iter()
					.map(|byte| JsonValue::from(*byte))
					.collect(),
			),
		);
		assert_eq!(
			optional_content_col(&mut row, "content").unwrap(),
			Some("aGVsbG8=".to_owned())
		);
	}

	#[test]
	fn transcript_render_is_role_labeled_and_idempotent() {
		let events = vec![
			json!({ "method": "user_prompt", "params": { "text": "hello there" } }),
			json!({
				"method": "session/update",
				"params": { "update": {
					"sessionUpdate": "agent_message_chunk",
					"content": { "type": "text", "text": "hi back" }
				}}
			}),
			json!({
				"method": "session/update",
				"params": { "update": {
					"sessionUpdate": "tool_call",
					"title": "read_file",
					"status": "completed",
					"content": [{ "content": { "type": "text", "text": "file body" } }]
				}}
			}),
		];
		let first = render_transcript_markdown("sess-1", &events);
		let second = render_transcript_markdown("sess-1", &events);
		// Idempotent: identical bytes on repeated render.
		assert_eq!(first, second);
		assert!(first.contains("# Session transcript: sess-1"));
		assert!(first.contains("## User\n\nhello there"));
		assert!(first.contains("## Assistant\n\nhi back"));
		assert!(first.contains("### Tool call: read_file (completed)"));
		assert!(first.contains("file body"));
	}

	#[test]
	fn stat_shape_uses_callback_camel_case_fields() {
		let entry = FsEntry {
			path: "/file".to_owned(),
			name: "file".to_owned(),
			is_directory: false,
			content: Some(BASE64.encode("hello")),
			mode: DEFAULT_FILE_MODE,
			uid: 1,
			gid: 2,
			size: 5,
			atime_ms: 1001,
			mtime_ms: 2002,
			ctime_ms: 3003,
			birthtime_ms: 4004,
			symlink_target: None,
			nlink: 1,
		};
		let value = stat_json(entry);
		assert_eq!(value["isDirectory"], JsonValue::Bool(false));
		assert_eq!(value["isSymbolicLink"], JsonValue::Bool(false));
		assert_eq!(value["size"], JsonValue::from(5));
		assert_eq!(value["mtimeNsec"], JsonValue::from(2_000_000));
	}
}
