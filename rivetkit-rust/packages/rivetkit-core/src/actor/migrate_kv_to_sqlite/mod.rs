use anyhow::{Context, Result, bail};

use std::time::Duration;

use crate::ActorContext;
use crate::actor::connection::decode_persisted_connection;
use crate::actor::internal_storage;
use crate::actor::internal_storage::KV_IMPORT_STATE_META_KEY;
use crate::actor::keys::{
	CONN_PREFIX, INSPECTOR_TOKEN_KEY, KV_PREFIX, LAST_PUSHED_ALARM_KEY, PERSIST_DATA_KEY,
	QUEUE_MESSAGES_PREFIX, QUEUE_METADATA_KEY, QUEUE_STORAGE_PREFIX, TRACES_PREFIX,
	TRACES_STORAGE_PREFIX, WORKFLOW_STORAGE_PREFIX, decode_queue_message_key,
};
use crate::actor::metrics::startup_phase::StartupPhase;
use crate::actor::queue::{decode_queue_message, decode_queue_metadata};
use crate::actor::state::{decode_last_pushed_alarm, decode_persisted_actor};
use crate::sqlite::{BindParam, SqliteBatchStatement};
use crate::time::Instant;
use crate::types::ListOpts;

const KV_IMPORT_STATE_IMPORTING: &str = "importing";
pub(crate) const KV_IMPORT_STATE_DONE: &str = "done";

/// Page size for legacy KV scans. The engine caps listings without an explicit
/// limit at 16,384 keys, so unpaginated scans would silently truncate large
/// actors and freeze a partial import as the source of truth. Scans hold their
/// cursor in memory per the migration spec.
const LEGACY_SCAN_PAGE_LIMIT: u32 = 256;
const IMPORT_PROGRESS_RECORD_INTERVAL: usize = 1_024;
const IMPORT_PROGRESS_BYTE_INTERVAL: usize = 16 * 1024 * 1024;
/// Upper bound for cursor continuation scans. Every legacy key starts with a
/// reserved low prefix byte (1 through 7), so a single 0xff byte sorts after
/// all of them.
const LEGACY_SCAN_END: &[u8] = &[0xff];

/// What `ensure_startup_storage` did: whether the inspector token and its mirror are both
/// durable, and how long each of its steps took.
pub(crate) struct StartupStorage {
	pub(crate) inspector_token_initialized: bool,
	/// This call bootstrapped a fresh actor, so its storage holds exactly the bootstrap rows:
	/// the legacy import is done and no actor snapshot exists. The rows are staged to commit
	/// with the database's first transaction.
	pub(crate) bootstrapped: bool,
	/// The start command showed the actor's storage was never written: this is its first
	/// generation.
	pub(crate) first_generation: bool,
	/// The steps that ran, in order, for `rivetkit_actor_startup_phase_duration_seconds`.
	pub(crate) phases: Vec<(StartupPhase, Duration)>,
	/// The inspector token a fresh bootstrap staged, which still needs its KV mirror. The start
	/// writes the mirror once the bootstrap is durable.
	pub(crate) inspector_mirror_token: Option<String>,
}

/// Bootstraps a fresh actor's framework rows together before any user code runs.
/// `inspector_token_initialized` is true only when the inspector token and its mirror are both
/// durable. Nonempty legacy actors retain the resumable import path below.
///
/// When the start command showed the actor's storage was never written, the actor is fresh
/// without reading the schema or probing legacy KV. If that report is ever wrong, the
/// transaction carrying the bootstrap fails as a whole because its migrations create tables
/// that already exist, so no existing row is overwritten and the start fails instead.
pub(crate) async fn ensure_startup_storage(ctx: &ActorContext) -> Result<StartupStorage> {
	let schema = ctx.sql();
	let mut phases = Vec::new();

	let first_generation = ctx.take_new_actor_storage();
	// Whether the schema probe found the internal schema, when it ran.
	let mut found_schema = false;
	let fresh = if first_generation {
		tracing::debug!(
			actor_id = %ctx.actor_id(),
			"new actor storage, skipping the schema check and legacy kv probe"
		);
		true
	} else {
		let started = Instant::now();
		let has_schema = internal_storage::schema::has_internal_schema(schema).await?;
		phases.push((StartupPhase::SchemaCheck, started.elapsed()));
		found_schema = has_schema;

		if has_schema {
			false
		} else {
			let started = Instant::now();
			let empty = legacy_bootstrap_range_empty(ctx).await?;
			phases.push((StartupPhase::KvProbe, started.elapsed()));
			empty
		}
	};

	if fresh {
		let token = crate::inspector::auth::new_inspector_token();
		let mut statements = internal_storage::schema::bootstrap_statements()?;
		statements.push(SqliteBatchStatement {
			sql: internal_storage::UPSERT_META_TEXT_SQL.to_owned(),
			params: Some(vec![
				BindParam::Text(KV_IMPORT_STATE_META_KEY.to_owned()),
				BindParam::Blob(KV_IMPORT_STATE_DONE.as_bytes().to_vec()),
			]),
		});
		if let Some(token) = &token {
			statements.push(SqliteBatchStatement {
				sql: internal_storage::UPSERT_INSPECTOR_TOKEN_SQL.to_owned(),
				params: Some(vec![
					BindParam::Null,
					BindParam::Text(token.clone()),
					BindParam::Integer(1),
				]),
			});
		}
		// The bootstrap commits with the database's first transaction, normally the actor's
		// first state save, instead of in a commit of its own. SQLite holds the authoritative
		// token, and the KV mirror only serves it to the dashboard, so the start writes the
		// mirror in the background after the bootstrap is durable. A bootstrap without its
		// mirror gets the mirror repaired by the next start's inspector init.
		schema.defer_bootstrap(statements);
		let started = Instant::now();
		let open = if first_generation {
			schema.open_new().await
		} else {
			schema.open().await
		};
		let open_elapsed = started.elapsed();
		open.context("open fresh actor database")?;
		phases.push((StartupPhase::DatabaseOpen, open_elapsed));
		return Ok(StartupStorage {
			inspector_token_initialized: true,
			bootstrapped: true,
			first_generation,
			phases,
			inspector_mirror_token: token,
		});
	}
	internal_storage::schema::ensure_probed_internal_schema(schema, found_schema).await?;
	Ok(StartupStorage {
		inspector_token_initialized: false,
		bootstrapped: false,
		first_generation,
		phases,
		inspector_mirror_token: None,
	})
}

/// One bounded read covers every legacy namespace that can affect the import.
/// Start below the first reserved key so even older exclusive-start range
/// implementations include actor state at [1]. Trace-only actors remain empty.
async fn legacy_bootstrap_range_empty(ctx: &ActorContext) -> Result<bool> {
	Ok(ctx
		.legacy_kv()
		.list_range(
			&[],
			&TRACES_PREFIX,
			ListOpts {
				reverse: false,
				limit: Some(1),
			},
		)
		.await
		.context("probe legacy actor storage before bootstrap")?
		.is_empty())
}

pub(crate) async fn import_core_state_if_needed(ctx: &ActorContext) -> Result<()> {
	let result = import_core_state_if_needed_inner(ctx).await;
	if let Err(error) = &result {
		tracing::warn!(
			actor_id = %ctx.actor_id(),
			phase = "legacy_kv_import",
			%error,
			"legacy kv to sqlite import failed"
		);
	}
	result
}

async fn import_core_state_if_needed_inner(ctx: &ActorContext) -> Result<()> {
	let import_state = internal_storage::load_meta_text(ctx.sql(), KV_IMPORT_STATE_META_KEY)
		.await
		.context("probe internal sqlite kv import state")?;
	tracing::debug!(
		actor_id = %ctx.actor_id(),
		state = import_state.as_deref().unwrap_or("absent"),
		"legacy kv import state probed"
	);
	let is_retry = match import_state.as_deref() {
		Some(KV_IMPORT_STATE_DONE) => {
			tracing::debug!(actor_id = %ctx.actor_id(), result = "already_done", "legacy kv import probe completed");
			return Ok(());
		}
		Some(KV_IMPORT_STATE_IMPORTING) => {
			tracing::warn!(
				actor_id = %ctx.actor_id(),
				state = KV_IMPORT_STATE_IMPORTING,
				"retrying interrupted legacy kv to sqlite import"
			);
			true
		}
		Some(state) => {
			bail!("unknown legacy kv import state '{state}'")
		}
		None => false,
	};

	// Fresh actors have no legacy KV at all. Probe the entire actor namespace once
	// before issuing the individual key and prefix reads needed by existing actors.
	// Interrupted imports deliberately keep using the full migration path.
	let legacy_core_values = if !is_retry && legacy_actor_kv_empty(ctx).await? {
		None
	} else {
		let values = load_legacy_core_values(ctx).await?;
		if values.iter().all(Option::is_none) && legacy_prefixes_empty(ctx).await? {
			None
		} else {
			Some(values)
		}
	};

	let Some(legacy_core_values) = legacy_core_values else {
		tracing::debug!(actor_id = %ctx.actor_id(), result = "empty", "legacy kv import probe completed");
		internal_storage::persist_meta_text(
			ctx.sql(),
			KV_IMPORT_STATE_META_KEY,
			KV_IMPORT_STATE_DONE,
		)
		.await
		.context("mark empty legacy core actor kv import complete")?;
		return Ok(());
	};
	tracing::info!(actor_id = %ctx.actor_id(), result = "import_needed", "legacy kv import probe completed");

	if !is_retry {
		internal_storage::persist_meta_text(
			ctx.sql(),
			KV_IMPORT_STATE_META_KEY,
			KV_IMPORT_STATE_IMPORTING,
		)
		.await
		.context("mark legacy kv to sqlite import started")?;
	}
	tracing::info!(actor_id = %ctx.actor_id(), "legacy kv to sqlite import started");

	// The marker is committed before cleanup, so a crash during any bounded
	// delete or import phase deterministically retries from the frozen legacy
	// snapshot. This intentionally makes legacy KV the sole source of truth for
	// every non-empty first import as well as every retry.
	internal_storage::clear_imported_storage(ctx.sql(), ctx.actor_id())
		.await
		.context("clear destination before legacy kv to sqlite import")?;
	tracing::info!(
		actor_id = %ctx.actor_id(),
		reason = if is_retry { "interrupted" } else { "initial" },
		"cleared legacy kv import destination"
	);

	let core_bytes: usize = legacy_core_values.iter().flatten().map(Vec::len).sum();
	let mut values = legacy_core_values.into_iter();

	let actor = values
		.next()
		.flatten()
		.map(|bytes| decode_persisted_actor(&bytes))
		.transpose()
		.context("decode legacy persisted actor during sqlite import")?;
	let last_pushed_alarm = values
		.next()
		.flatten()
		.map(|bytes| decode_last_pushed_alarm(&bytes))
		.transpose()
		.context("decode legacy last pushed alarm during sqlite import")?
		.flatten();
	let inspector_token = values
		.next()
		.flatten()
		.map(String::from_utf8)
		.transpose()
		.context("decode legacy inspector token as utf-8 during sqlite import")?;
	let queue_metadata = values
		.next()
		.flatten()
		.map(|bytes| decode_queue_metadata(&bytes))
		.transpose()
		.context("decode legacy queue metadata during sqlite import")?;
	let core_record_count = usize::from(actor.is_some())
		+ usize::from(last_pushed_alarm.is_some())
		+ usize::from(inspector_token.is_some())
		+ usize::from(queue_metadata.is_some());

	if let Some(actor) = actor {
		internal_storage::import_legacy_actor_snapshot(ctx.sql(), &actor)
			.await
			.context("import legacy actor snapshot into sqlite")?;
	}
	if last_pushed_alarm.is_some() {
		internal_storage::persist_last_pushed_alarm(ctx.sql(), last_pushed_alarm)
			.await
			.context("import legacy last pushed alarm into sqlite")?;
	}
	if let Some(token) = inspector_token {
		internal_storage::persist_inspector_token(ctx.sql(), &token)
			.await
			.context("import legacy inspector token into sqlite")?;
	}
	tracing::info!(
		actor_id = %ctx.actor_id(),
		record_count = core_record_count,
		byte_count = core_bytes,
		"legacy actor snapshot subspace import completed"
	);

	let mut connection_count = 0usize;
	let mut connection_bytes = 0usize;
	let mut connection_progress = ImportProgress::default();
	let mut conn_scan = LegacyPrefixScan::new(ctx, &CONN_PREFIX);
	while let Some(page) = conn_scan
		.next_page()
		.await
		.context("list legacy connection kv records for sqlite import")?
	{
		let mut connections = Vec::with_capacity(page.len());
		for (_key, value) in page {
			connection_bytes = connection_bytes.saturating_add(value.len());
			connections.push(
				decode_persisted_connection(&value)
					.context("decode legacy hibernatable connection during sqlite import")?,
			);
		}
		internal_storage::persist_connection_snapshots(ctx.sql(), &connections)
			.await
			.context("import legacy hibernatable connection chunk into sqlite")?;
		connection_count += connections.len();
		connection_progress.log_if_due(ctx, "connections", connection_count, connection_bytes);
	}
	tracing::info!(actor_id = %ctx.actor_id(), record_count = connection_count, byte_count = connection_bytes, "legacy connection subspace import completed");

	let mut queue_next_id = queue_metadata
		.as_ref()
		.map(|metadata| metadata.next_id)
		.unwrap_or(1)
		.max(1);
	let mut queue_scan = LegacyPrefixScan::new(ctx, &QUEUE_MESSAGES_PREFIX);
	let mut queue_count = 0usize;
	let mut queue_bytes = 0usize;
	let mut queue_progress = ImportProgress::default();
	while let Some(page) = queue_scan
		.next_page()
		.await
		.context("list legacy queue kv records for sqlite import")?
	{
		let mut messages = Vec::with_capacity(page.len());
		for (key, value) in page {
			queue_bytes = queue_bytes.saturating_add(value.len());
			let id = decode_queue_message_key(&key)
				.context("decode legacy queue message key during sqlite import")?;
			let message = decode_queue_message(&value).with_context(|| {
				format!("decode legacy queue message {id} during sqlite import")
			})?;
			if message.failure_count.is_some()
				|| message.available_at.is_some()
				|| message.in_flight.is_some()
				|| message.in_flight_at.is_some()
			{
				bail!(
					"legacy queue message {id} contains retry, delay, or in-flight state that SQLite queue storage cannot preserve"
				);
			}
			queue_next_id = queue_next_id.max(id.saturating_add(1));
			messages.push((id, message));
		}
		internal_storage::persist_queue_messages(ctx.sql(), &messages)
			.await
			.context("import legacy queue message chunk into sqlite")?;
		queue_count += messages.len();
		queue_progress.log_if_due(ctx, "queue", queue_count, queue_bytes);
	}
	internal_storage::persist_queue_next_id(ctx.sql(), queue_next_id)
		.await
		.context("import legacy queue next id into sqlite")?;
	tracing::info!(actor_id = %ctx.actor_id(), record_count = queue_count, byte_count = queue_bytes, "legacy queue subspace import completed");

	let mut workflow_count = 0usize;
	let mut workflow_bytes = 0usize;
	let mut workflow_progress = ImportProgress::default();
	let mut workflow_scan = LegacyPrefixScan::new(ctx, &WORKFLOW_STORAGE_PREFIX);
	while let Some(page) = workflow_scan
		.next_page()
		.await
		.context("list legacy workflow kv records for sqlite import")?
	{
		workflow_count += page.len();
		workflow_bytes = workflow_bytes.saturating_add(
			page.iter()
				.map(|(key, value)| key.len() + value.len())
				.sum(),
		);
		// Chunk page writes so no import transaction exceeds the depot commit
		// size limit.
		for chunk in internal_storage::split_kv_tx_chunks(&page) {
			let chunk_refs = chunk
				.iter()
				.map(|(key, value)| (key.as_slice(), value.as_slice()))
				.collect::<Vec<_>>();
			internal_storage::workflow_kv_batch_put(ctx.sql(), &chunk_refs)
				.await
				.with_context(|| {
					format!(
						"import legacy workflow kv chunk with {} entries into sqlite",
						chunk.len()
					)
				})?;
		}
		workflow_progress.log_if_due(ctx, "workflow", workflow_count, workflow_bytes);
	}
	tracing::info!(actor_id = %ctx.actor_id(), record_count = workflow_count, byte_count = workflow_bytes, "legacy workflow kv subspace import completed");

	// Legacy user KV keys are stored verbatim. The TypeScript runtime reads and
	// writes `[4]`-prefixed keys and the Rust runtime uses raw keys; both pass
	// keys through unchanged, so stripping the prefix here would make every
	// migrated TypeScript `c.kv` entry unreachable.
	let mut user_scan = LegacyPrefixScan::new(ctx, &KV_PREFIX);
	let mut user_count = 0usize;
	let mut user_bytes = 0usize;
	let mut user_skipped_count = 0usize;
	let mut user_progress = ImportProgress::default();
	while let Some(page) = user_scan
		.next_page()
		.await
		.context("list legacy actor kv records for user-kv sqlite import")?
	{
		let page_count = page.len();
		let entries = page
			.into_iter()
			.filter(|(key, _)| !should_skip_user_kv_import_key(key))
			.collect::<Vec<_>>();
		user_skipped_count += page_count.saturating_sub(entries.len());
		user_count += entries.len();
		user_bytes = user_bytes.saturating_add(
			entries
				.iter()
				.map(|(key, value)| key.len() + value.len())
				.sum(),
		);
		for chunk in internal_storage::split_kv_tx_chunks(&entries) {
			let chunk_refs = chunk
				.iter()
				.map(|(key, value)| (key.as_slice(), value.as_slice()))
				.collect::<Vec<_>>();
			internal_storage::user_kv_batch_put(ctx.sql(), &chunk_refs)
				.await
				.with_context(|| {
					format!(
						"import legacy user kv chunk with {} entries into sqlite",
						chunk.len()
					)
				})?;
		}
		user_progress.log_if_due(ctx, "user_kv", user_count, user_bytes);
	}
	tracing::info!(actor_id = %ctx.actor_id(), record_count = user_count, byte_count = user_bytes, skipped_count = user_skipped_count, "legacy user kv subspace import completed");

	internal_storage::persist_meta_text(ctx.sql(), KV_IMPORT_STATE_META_KEY, KV_IMPORT_STATE_DONE)
		.await
		.context("mark legacy core actor kv import complete")?;
	tracing::info!(actor_id = %ctx.actor_id(), "legacy kv to sqlite import completed");

	Ok(())
}

async fn load_legacy_core_values(ctx: &ActorContext) -> Result<Vec<Option<Vec<u8>>>> {
	let keys = [
		PERSIST_DATA_KEY,
		LAST_PUSHED_ALARM_KEY,
		INSPECTOR_TOKEN_KEY.as_slice(),
		QUEUE_METADATA_KEY.as_slice(),
	];

	ctx.legacy_kv()
		.batch_get(&keys)
		.await
		.context("load legacy core actor kv records for sqlite import")
}

async fn legacy_actor_kv_empty(ctx: &ActorContext) -> Result<bool> {
	Ok(list_legacy_prefix_with_limit(ctx, &[], Some(1))
		.await
		.context("probe complete legacy actor kv for sqlite import")?
		.is_empty())
}

async fn legacy_prefixes_empty(ctx: &ActorContext) -> Result<bool> {
	for prefix in [
		CONN_PREFIX.as_slice(),
		QUEUE_MESSAGES_PREFIX.as_slice(),
		WORKFLOW_STORAGE_PREFIX.as_slice(),
		KV_PREFIX.as_slice(),
	] {
		let entries = list_legacy_prefix_with_limit(ctx, prefix, Some(1))
			.await
			.context("probe legacy actor kv prefix for sqlite import")?;
		if !entries.is_empty() {
			return Ok(false);
		}
	}

	Ok(true)
}

async fn list_legacy_prefix_with_limit(
	ctx: &ActorContext,
	prefix: &[u8],
	limit: Option<u32>,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	ctx.legacy_kv()
		.list_prefix(
			prefix,
			ListOpts {
				reverse: false,
				limit,
			},
		)
		.await
}

/// Paginated ascending scan over one legacy KV subspace. The first page uses a
/// prefix listing; continuation pages range-scan from the last seen key so the
/// scan never depends on the backend's default listing cap.
struct LegacyPrefixScan<'a> {
	ctx: &'a ActorContext,
	prefix: &'a [u8],
	cursor: Option<Vec<u8>>,
	done: bool,
}

impl<'a> LegacyPrefixScan<'a> {
	fn new(ctx: &'a ActorContext, prefix: &'a [u8]) -> Self {
		Self {
			ctx,
			prefix,
			cursor: None,
			done: false,
		}
	}

	async fn next_page(&mut self) -> Result<Option<Vec<(Vec<u8>, Vec<u8>)>>> {
		loop {
			if self.done {
				return Ok(None);
			}

			let raw_page = match &self.cursor {
				None => {
					list_legacy_prefix_with_limit(
						self.ctx,
						self.prefix,
						Some(LEGACY_SCAN_PAGE_LIMIT),
					)
					.await?
				}
				Some(cursor) => {
					self.ctx
						.legacy_kv()
						.list_range(
							cursor,
							LEGACY_SCAN_END,
							ListOpts {
								reverse: false,
								limit: Some(LEGACY_SCAN_PAGE_LIMIT),
							},
						)
						.await?
				}
			};

			let last_raw_key = raw_page.last().map(|(key, _)| key.clone());

			let mut entries = Vec::with_capacity(raw_page.len());
			for (key, value) in raw_page {
				// Backends differ on whether a range scan includes its start
				// key, so drop cursor re-reads instead of assuming either
				// bound semantic.
				if let Some(cursor) = &self.cursor {
					if key.as_slice() <= cursor.as_slice() {
						continue;
					}
				}
				if !key.starts_with(self.prefix) {
					self.done = true;
					break;
				}
				entries.push((key, value));
			}

			// Detect exhaustion purely from key progress. A page shorter than the
			// requested limit is not proof of exhaustion because backends may
			// cap a page below the request, which is exactly how unpaginated
			// scans truncated large actors.
			match last_raw_key {
				Some(key)
					if self
						.cursor
						.as_deref()
						.is_none_or(|cursor| key.as_slice() > cursor) =>
				{
					self.cursor = Some(key);
				}
				Some(_) | None => self.done = true,
			}

			if !entries.is_empty() {
				return Ok(Some(entries));
			}
		}
	}
}

fn should_skip_user_kv_import_key(key: &[u8]) -> bool {
	key == PERSIST_DATA_KEY
		|| key == LAST_PUSHED_ALARM_KEY
		|| key == INSPECTOR_TOKEN_KEY.as_slice()
		|| key == QUEUE_METADATA_KEY.as_slice()
		|| !key.starts_with(&KV_PREFIX)
		|| key.starts_with(&CONN_PREFIX)
		|| key.starts_with(&QUEUE_STORAGE_PREFIX)
		|| key.starts_with(&WORKFLOW_STORAGE_PREFIX)
		|| key.starts_with(&TRACES_STORAGE_PREFIX)
}

#[derive(Default)]
struct ImportProgress {
	last_record_count: usize,
	last_byte_count: usize,
}

impl ImportProgress {
	fn log_if_due(
		&mut self,
		ctx: &ActorContext,
		phase: &'static str,
		record_count: usize,
		byte_count: usize,
	) {
		if record_count.saturating_sub(self.last_record_count) < IMPORT_PROGRESS_RECORD_INTERVAL
			&& byte_count.saturating_sub(self.last_byte_count) < IMPORT_PROGRESS_BYTE_INTERVAL
		{
			return;
		}
		self.last_record_count = record_count;
		self.last_byte_count = byte_count;
		tracing::info!(
			actor_id = %ctx.actor_id(),
			phase,
			record_count,
			byte_count,
			"legacy kv to sqlite import progress"
		);
	}
}

// Test shim keeps moved tests in crate-root tests/ with private-module access.

