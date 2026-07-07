use anyhow::{Context, Result};

use crate::ActorContext;
use crate::actor::connection::decode_persisted_connection;
use crate::actor::internal_storage;
use crate::actor::keys::{
	CONN_PREFIX, INSPECTOR_TOKEN_KEY, KV_PREFIX, LAST_PUSHED_ALARM_KEY, PERSIST_DATA_KEY,
	QUEUE_MESSAGES_PREFIX, QUEUE_METADATA_KEY, QUEUE_STORAGE_PREFIX, TRACES_STORAGE_PREFIX,
	WORKFLOW_STORAGE_PREFIX, decode_queue_message_key,
};
use crate::actor::queue::{decode_queue_message, decode_queue_metadata};
use crate::actor::state::{decode_last_pushed_alarm, decode_persisted_actor};
use crate::types::ListOpts;

const KV_IMPORT_STATE_META_KEY: &str = "kv_import_state";
const KV_IMPORT_STATE_IMPORTING: &str = "importing";
const KV_IMPORT_STATE_DONE: &str = "done";

/// Page size for legacy KV scans. The engine caps listings without an explicit
/// limit at 16,384 keys, so unpaginated scans would silently truncate large
/// actors and freeze a partial import as the source of truth. Scans hold their
/// cursor in memory per the migration spec.
const LEGACY_SCAN_PAGE_LIMIT: u32 = 256;
/// Upper bound for cursor continuation scans. Every legacy key starts with a
/// reserved low prefix byte (1 through 7), so a single 0xff byte sorts after
/// all of them.
const LEGACY_SCAN_END: &[u8] = &[0xff];

pub(crate) async fn import_core_state_if_needed(ctx: &ActorContext) -> Result<()> {
	match internal_storage::load_meta_text(ctx.sql(), KV_IMPORT_STATE_META_KEY)
		.await
		.context("probe internal sqlite kv import state")?
		.as_deref()
	{
		Some(KV_IMPORT_STATE_DONE) => return Ok(()),
		Some(KV_IMPORT_STATE_IMPORTING) => {
			tracing::warn!(
				actor_id = %ctx.actor_id(),
				"retrying interrupted legacy kv to sqlite import"
			);
			internal_storage::clear_imported_storage(ctx.sql())
				.await
				.context("clear interrupted legacy kv to sqlite import")?;
		}
		Some(state) => {
			tracing::warn!(
				actor_id = %ctx.actor_id(),
				state,
				"retrying legacy kv to sqlite import from unknown state"
			);
			internal_storage::clear_imported_storage(ctx.sql())
				.await
				.context("clear unknown legacy kv to sqlite import state")?;
		}
		None => {}
	}

	let legacy_core_values = load_legacy_core_values(ctx).await?;
	if legacy_core_values.iter().all(Option::is_none) && legacy_prefixes_empty(ctx).await? {
		internal_storage::persist_meta_text(ctx.sql(), KV_IMPORT_STATE_META_KEY, KV_IMPORT_STATE_DONE)
			.await
			.context("mark empty legacy core actor kv import complete")?;
		return Ok(());
	}

	internal_storage::persist_meta_text(
		ctx.sql(),
		KV_IMPORT_STATE_META_KEY,
		KV_IMPORT_STATE_IMPORTING,
	)
	.await
	.context("mark legacy kv to sqlite import started")?;

	let has_actor_snapshot = internal_storage::load_actor_snapshot(ctx.sql())
		.await
		.context("probe internal actor sqlite snapshot before kv import")?
		.is_some();
	let has_inspector_token = internal_storage::load_inspector_token(ctx.sql())
		.await
		.context("probe internal sqlite inspector token before kv import")?
		.is_some();

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
		.and_then(|bytes| match decode_last_pushed_alarm(&bytes) {
			Ok(value) => value,
			Err(error) => {
				tracing::warn!(
					actor_id = %ctx.actor_id(),
					?error,
					"skipping corrupt legacy last pushed alarm during sqlite import"
				);
				None
			}
		});
	let inspector_token = values.next().flatten().and_then(|bytes| {
		String::from_utf8(bytes)
			.map_err(|error| {
				tracing::warn!(
					actor_id = %ctx.actor_id(),
					?error,
					"skipping non-utf8 legacy inspector token during sqlite import"
				);
			})
			.ok()
	});
	let queue_metadata = values
		.next()
		.flatten()
		.map(|bytes| decode_queue_metadata(&bytes))
		.transpose()
		.map_err(|error| {
			tracing::warn!(
				actor_id = %ctx.actor_id(),
				?error,
				"skipping corrupt legacy queue metadata during sqlite import"
			);
			error
		})
		.ok()
		.flatten();

	if !has_actor_snapshot {
		if let Some(actor) = actor {
			internal_storage::persist_actor_snapshot(ctx.sql(), &actor)
				.await
				.context("import legacy actor snapshot into sqlite")?;
		}
	}
	if last_pushed_alarm.is_some() {
		internal_storage::persist_last_pushed_alarm(ctx.sql(), last_pushed_alarm)
			.await
			.context("import legacy last pushed alarm into sqlite")?;
	}
	if !has_inspector_token {
		if let Some(token) = inspector_token {
			internal_storage::persist_inspector_token(ctx.sql(), &token)
				.await
				.context("import legacy inspector token into sqlite")?;
		}
	}

	let mut conn_scan = LegacyPrefixScan::new(ctx, &CONN_PREFIX);
	while let Some(page) = conn_scan
		.next_page()
		.await
		.context("list legacy connection kv records for sqlite import")?
	{
		for (_key, value) in page {
			match decode_persisted_connection(&value) {
				Ok(connection) => {
					internal_storage::persist_connection_snapshot(ctx.sql(), &connection)
						.await
						.with_context(|| {
							format!(
								"import legacy hibernatable connection '{}' into sqlite",
								connection.id
							)
						})?;
				}
				Err(error) => {
					tracing::warn!(
						actor_id = %ctx.actor_id(),
						?error,
						"skipping corrupt legacy hibernatable connection during sqlite import"
					);
				}
			}
		}
	}

	let mut queue_next_id = queue_metadata
		.as_ref()
		.map(|metadata| metadata.next_id)
		.unwrap_or(1)
		.max(1);
	let mut queue_scan = LegacyPrefixScan::new(ctx, &QUEUE_MESSAGES_PREFIX);
	while let Some(page) = queue_scan
		.next_page()
		.await
		.context("list legacy queue kv records for sqlite import")?
	{
		for (key, value) in page {
			let id = match decode_queue_message_key(&key) {
				Ok(id) => id,
				Err(error) => {
					tracing::warn!(
						actor_id = %ctx.actor_id(),
						?error,
						"skipping legacy queue message with invalid key during sqlite import"
					);
					continue;
				}
			};
			match decode_queue_message(&value) {
				Ok(message) => {
					if message.failure_count.is_some()
						|| message.available_at.is_some()
						|| message.in_flight.is_some()
						|| message.in_flight_at.is_some()
					{
						// The retry fields have no SQLite representation; the
						// message imports as immediately deliverable. The
						// original record stays readable in the frozen legacy
						// KV, so no extra backup copy is written.
						tracing::warn!(
							actor_id = %ctx.actor_id(),
							message_id = id,
							"importing legacy queue message with dropped retry fields"
						);
					}
					queue_next_id = queue_next_id.max(id.saturating_add(1));
					internal_storage::persist_queue_message(ctx.sql(), id, queue_next_id, &message)
						.await
						.with_context(|| {
							format!("import legacy queue message {id} into sqlite")
						})?;
				}
				Err(error) => {
					tracing::warn!(
						actor_id = %ctx.actor_id(),
						message_id = id,
						?error,
						"skipping corrupt legacy queue message during sqlite import"
					);
				}
			}
		}
	}
	internal_storage::persist_queue_next_id(ctx.sql(), queue_next_id)
		.await
		.context("import legacy queue next id into sqlite")?;

	let mut workflow_scan = LegacyPrefixScan::new(ctx, &WORKFLOW_STORAGE_PREFIX);
	while let Some(page) = workflow_scan
		.next_page()
		.await
		.context("list legacy workflow kv records for sqlite import")?
	{
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
	}

	// Legacy user KV keys are stored verbatim. The TypeScript runtime reads and
	// writes `[4]`-prefixed keys and the Rust runtime uses raw keys; both pass
	// keys through unchanged, so stripping the prefix here would make every
	// migrated TypeScript `c.kv` entry unreachable.
	let mut user_scan = LegacyPrefixScan::new(ctx, &KV_PREFIX);
	while let Some(page) = user_scan
		.next_page()
		.await
		.context("list legacy actor kv records for user-kv sqlite import")?
	{
		let entries = page
			.into_iter()
			.filter(|(key, _)| !should_skip_user_kv_import_key(key))
			.collect::<Vec<_>>();
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
	}

	internal_storage::persist_meta_text(ctx.sql(), KV_IMPORT_STATE_META_KEY, KV_IMPORT_STATE_DONE)
		.await
		.context("mark legacy core actor kv import complete")?;

	Ok(())
}

async fn load_legacy_core_values(ctx: &ActorContext) -> Result<Vec<Option<Vec<u8>>>> {
	let keys = [
		PERSIST_DATA_KEY,
		LAST_PUSHED_ALARM_KEY,
		INSPECTOR_TOKEN_KEY.as_slice(),
		QUEUE_METADATA_KEY.as_slice(),
	];

	ctx.kv_internal()
		.batch_get(&keys)
		.await
		.context("load legacy core actor kv records for sqlite import")
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
	ctx.kv_internal()
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
						.kv_internal()
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

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../../../tests/migrate_kv_to_sqlite.rs"]
pub(crate) mod tests;
