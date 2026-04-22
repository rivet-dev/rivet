use std::{
	collections::HashSet,
	path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json},
};
use rivet_api_types::actors::{
	delete,
	import_export::{
		ExportActorIdsSelector, ExportActorNamesSelector, ExportRequest, ExportResponse,
		ExportSelector, ImportRequest, ImportResponse,
	},
	list as list_types, list_names as list_names_types,
};
use rivet_api_util::{Method, request_remote_datacenter};
use rivet_envoy_protocol as ep;
use rivet_types::actors::{Actor, CrashPolicy};
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use tokio::{
	fs,
	io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
};

use crate::{
	actors::{list as list_routes, list_names as list_names_routes, utils},
	ctx::ApiCtx,
	errors,
};

const ARCHIVE_VERSION: u32 = 2;
const MIN_SUPPORTED_ARCHIVE_VERSION: u32 = 1;
const ACTOR_LIST_PAGE_SIZE: usize = 100;
const KV_BATCH_SIZE: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchiveManifestV1 {
	version: u32,
	generated_at: i64,
	source_cluster: Option<String>,
	source_namespace_id: Id,
	source_namespace_name: Option<String>,
	selector: ExportSelector,
	actor_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActorMetadataV1 {
	source_actor_id: Id,
	name: String,
	key: Option<String>,
	runner_name_selector: String,
	crash_policy: CrashPolicy,
	create_ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KvArchiveEntry {
	key: Vec<u8>,
	value: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SqliteArchiveEntry {
	key_suffix: Vec<u8>,
	value: Vec<u8>,
}

#[derive(Debug)]
enum SelectorVariant {
	All,
	ActorNames(Vec<String>),
	ActorIds(Vec<Id>),
}

enum ImportActorOutcome {
	Imported,
	Skipped(String),
}

/// Dangerous and intended for operational use.
#[utoipa::path(
	post,
	operation_id = "admin_actors_export",
	path = "/admin/actors/export",
	request_body(content = ExportRequest, content_type = "application/json"),
	responses(
		(status = 200, body = ExportResponse),
	),
	security(("bearer_auth" = [])),
)]
pub async fn export(
	Extension(ctx): Extension<ApiCtx>,
	Json(body): Json<ExportRequest>,
) -> Response {
	match export_inner(ctx, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

/// Dangerous and intended for operational use.
#[utoipa::path(
	post,
	operation_id = "admin_actors_import",
	path = "/admin/actors/import",
	request_body(content = ImportRequest, content_type = "application/json"),
	responses(
		(status = 200, body = ImportResponse),
	),
	security(("bearer_auth" = [])),
)]
pub async fn import(
	Extension(ctx): Extension<ApiCtx>,
	Json(body): Json<ImportRequest>,
) -> Response {
	match import_inner(ctx, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn export_inner(ctx: ApiCtx, body: ExportRequest) -> Result<ExportResponse> {
	ctx.auth().await?;

	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: body.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	let actors = resolve_selected_actors(&ctx, &body.namespace, &body.selector).await?;
	let export_id = format!("rivet-actor-export-{}", Id::new_v1(ctx.config().dc_label()));
	let temp_path = std::env::temp_dir().join(format!("{export_id}.tmp"));
	let final_path = std::env::temp_dir().join(&export_id);

	fs::create_dir_all(temp_path.join("actors")).await?;

	let export_res = async {
		write_json(
			&temp_path.join("manifest.json"),
			&ArchiveManifestV1 {
				version: ARCHIVE_VERSION,
				generated_at: rivet_util::timestamp::now(),
				source_cluster: None,
				source_namespace_id: namespace.namespace_id,
				source_namespace_name: Some(namespace.name.clone()),
				selector: body.selector.clone(),
				actor_count: 0,
			},
		)
		.await?;

		for actor in &actors {
			let actor_dir = temp_path.join("actors").join(actor.actor_id.to_string());
			fs::create_dir_all(&actor_dir).await?;

			write_json(
				&actor_dir.join("metadata.json"),
				&ActorMetadataV1 {
					source_actor_id: actor.actor_id,
					name: actor.name.clone(),
					key: actor.key.clone(),
					runner_name_selector: actor.runner_name_selector.clone(),
					crash_policy: actor.crash_policy,
					create_ts: actor.create_ts,
				},
			)
			.await?;

			export_actor_kv(&ctx, actor, &actor_dir.join("kv.bin")).await?;
			export_actor_sqlite_v2(&ctx, actor, &actor_dir.join("sqlite.bin")).await?;
		}

		write_json(
			&temp_path.join("manifest.json"),
			&ArchiveManifestV1 {
				version: ARCHIVE_VERSION,
				generated_at: rivet_util::timestamp::now(),
				source_cluster: None,
				source_namespace_id: namespace.namespace_id,
				source_namespace_name: Some(namespace.name),
				selector: body.selector,
				actor_count: actors.len(),
			},
		)
		.await?;

		Ok::<(), anyhow::Error>(())
	}
	.await;

	if let Err(err) = export_res {
		let _ = fs::remove_dir_all(&temp_path).await;
		return Err(err);
	}

	fs::rename(&temp_path, &final_path).await.with_context(|| {
		format!(
			"failed to finalize actor export archive at {}",
			final_path.display()
		)
	})?;

	Ok(ExportResponse {
		archive_path: final_path.to_string_lossy().into_owned(),
		actor_count: actors.len(),
	})
}

#[tracing::instrument(skip_all)]
async fn import_inner(ctx: ApiCtx, body: ImportRequest) -> Result<ImportResponse> {
	ctx.auth().await?;

	let target_namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: body.target_namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	let archive_path = PathBuf::from(&body.archive_path);
	let manifest: ArchiveManifestV1 = read_json(&archive_path.join("manifest.json")).await?;
	if manifest.version < MIN_SUPPORTED_ARCHIVE_VERSION || manifest.version > ARCHIVE_VERSION {
		return Err(errors::Validation::InvalidInput {
			message: format!(
				"unsupported actor archive version {}, supported range is {}..={}",
				manifest.version, MIN_SUPPORTED_ARCHIVE_VERSION, ARCHIVE_VERSION
			),
		}
		.build());
	}

	let actors_dir = archive_path.join("actors");
	if !fs::try_exists(&actors_dir).await? {
		return Err(errors::Validation::InvalidInput {
			message: format!(
				"archive is missing actors directory at {}",
				actors_dir.display()
			),
		}
		.build());
	}

	let mut imported_actors = 0;
	let mut skipped_actors = 0;
	let mut warnings = Vec::new();
	let mut dir_entries = fs::read_dir(&actors_dir).await?;

	while let Some(entry) = dir_entries.next_entry().await? {
		if !entry.file_type().await?.is_dir() {
			continue;
		}

		match import_actor_dir(
			&ctx,
			&body.target_namespace,
			target_namespace.namespace_id,
			entry.path(),
		)
		.await?
		{
			ImportActorOutcome::Imported => imported_actors += 1,
			ImportActorOutcome::Skipped(warning) => {
				tracing::warn!(warning = %warning, target_namespace = %body.target_namespace, "skipping imported actor");
				skipped_actors += 1;
				warnings.push(warning);
			}
		}
	}

	Ok(ImportResponse {
		imported_actors,
		skipped_actors,
		warnings,
	})
}

async fn import_actor_dir(
	ctx: &ApiCtx,
	target_namespace: &str,
	target_namespace_id: Id,
	actor_dir: PathBuf,
) -> Result<ImportActorOutcome> {
	let actor_folder = actor_dir
		.file_name()
		.map(|name| name.to_string_lossy().into_owned())
		.unwrap_or_else(|| actor_dir.display().to_string());
	let metadata_path = actor_dir.join("metadata.json");
	let kv_path = actor_dir.join("kv.bin");

	if !fs::try_exists(&metadata_path).await? {
		return Ok(ImportActorOutcome::Skipped(format!(
			"skipped malformed archive entry {actor_folder}: missing metadata.json"
		)));
	}
	if !fs::try_exists(&kv_path).await? {
		return Ok(ImportActorOutcome::Skipped(format!(
			"skipped malformed archive entry {actor_folder}: missing kv.bin"
		)));
	}

	let metadata: ActorMetadataV1 = match read_json(&metadata_path).await {
		Ok(metadata) => metadata,
		Err(err) => {
			return Ok(ImportActorOutcome::Skipped(format!(
				"skipped malformed archive entry {actor_folder}: failed to parse metadata.json: {err:#}"
			)));
		}
	};

	if actor_exists_with_name_and_key(
		ctx,
		target_namespace,
		&metadata.name,
		metadata.key.as_deref(),
	)
	.await?
	{
		return Ok(ImportActorOutcome::Skipped(format!(
			"skipped archive actor {} (name={}, key={:?}) because target namespace {} already has the same (name, key)",
			metadata.source_actor_id, metadata.name, metadata.key, target_namespace,
		)));
	}

	// Source actor IDs are retained in archive paths for provenance only.
	// Import must always generate new actor IDs because the target may be another namespace in the same cluster.
	let created_actor =
		create_imported_actor(ctx, target_namespace, target_namespace_id, &metadata).await?;

	let sqlite_path = actor_dir.join("sqlite.bin");
	let replay_res = async {
		replay_actor_kv(ctx, &created_actor, &kv_path).await?;
		// sqlite.bin is optional so v1 archives keep working.
		if fs::try_exists(&sqlite_path).await? {
			replay_actor_sqlite_v2(ctx, &created_actor, &sqlite_path).await?;
		}
		Ok::<(), anyhow::Error>(())
	}
	.await;

	match replay_res {
		Ok(()) => Ok(ImportActorOutcome::Imported),
		Err(err) => {
			match rollback_imported_actor(ctx, target_namespace, created_actor.actor_id).await {
				Ok(()) => Ok(ImportActorOutcome::Skipped(format!(
					"rolled back partial import for archive actor {} (name={}, key={:?}) in namespace {} after error: {err:#}",
					metadata.source_actor_id, metadata.name, metadata.key, target_namespace,
				))),
				Err(rollback_err) => Err(rollback_err).context(format!(
					"failed to roll back partial import for archive actor {} after import error: {err:#}",
					metadata.source_actor_id,
				)),
			}
		}
	}
}

async fn resolve_selected_actors(
	ctx: &ApiCtx,
	namespace: &str,
	selector: &ExportSelector,
) -> Result<Vec<Actor>> {
	match parse_selector(selector)? {
		SelectorVariant::All => collect_all_actors(ctx, namespace).await,
		SelectorVariant::ActorNames(names) => {
			let mut actors = Vec::new();
			let mut seen = HashSet::new();
			for name in names {
				for actor in collect_actors_for_name(ctx, namespace, &name).await? {
					if seen.insert(actor.actor_id) {
						actors.push(actor);
					}
				}
			}
			Ok(actors)
		}
		SelectorVariant::ActorIds(ids) => {
			let inner_ctx: rivet_api_builder::ApiCtx = ctx.clone().into();
			utils::fetch_actors_by_ids(&inner_ctx, ids, namespace.to_string(), Some(false), None)
				.await
		}
	}
}

fn parse_selector(selector: &ExportSelector) -> Result<SelectorVariant> {
	let variant_count = usize::from(selector.all.unwrap_or(false))
		+ usize::from(selector.actor_names.is_some())
		+ usize::from(selector.actor_ids.is_some());
	if variant_count != 1 {
		return Err(errors::Validation::InvalidInput {
			message: "export selector must set exactly one of `all`, `actor_names`, or `actor_ids`"
				.to_string(),
		}
		.build());
	}

	if selector.all == Some(true) {
		return Ok(SelectorVariant::All);
	}

	if let Some(ExportActorNamesSelector { names }) = &selector.actor_names {
		if names.is_empty() {
			return Err(errors::Validation::InvalidInput {
				message: "`actor_names.names` must not be empty".to_string(),
			}
			.build());
		}

		let mut deduped = Vec::new();
		let mut seen = HashSet::new();
		for name in names {
			if seen.insert(name.clone()) {
				deduped.push(name.clone());
			}
		}
		return Ok(SelectorVariant::ActorNames(deduped));
	}

	if let Some(ExportActorIdsSelector { ids }) = &selector.actor_ids {
		if ids.is_empty() {
			return Err(errors::Validation::InvalidInput {
				message: "`actor_ids.ids` must not be empty".to_string(),
			}
			.build());
		}

		let mut deduped = Vec::new();
		let mut seen = HashSet::new();
		for actor_id in ids {
			if seen.insert(*actor_id) {
				deduped.push(*actor_id);
			}
		}
		return Ok(SelectorVariant::ActorIds(deduped));
	}

	Err(errors::Validation::InvalidInput {
		message: "`all` must be true when used".to_string(),
	}
	.build())
}

async fn collect_all_actors(ctx: &ApiCtx, namespace: &str) -> Result<Vec<Actor>> {
	let mut actors = Vec::new();
	let mut names_cursor = None;

	loop {
		let names_res = list_names_routes::list_names_inner(
			// list_names_inner handles fanout and pagination for actor names across datacenters.
			ctx.clone(),
			list_names_types::ListNamesQuery {
				namespace: namespace.to_string(),
				limit: Some(ACTOR_LIST_PAGE_SIZE),
				cursor: names_cursor.clone(),
			},
		)
		.await?;

		let mut names = names_res.names.into_keys().collect::<Vec<_>>();
		names.sort();

		for name in names {
			actors.extend(collect_actors_for_name(ctx, namespace, &name).await?);
		}

		if names_res.pagination.cursor.is_none() {
			break;
		}
		names_cursor = names_res.pagination.cursor;
	}

	Ok(actors)
}

async fn collect_actors_for_name(ctx: &ApiCtx, namespace: &str, name: &str) -> Result<Vec<Actor>> {
	let mut actors = Vec::new();
	let mut cursor = None;

	loop {
		let res = list_routes::list_inner(
			// list_inner handles the cross-datacenter actor fanout for a specific actor name.
			ctx.clone(),
			list_types::ListQuery {
				namespace: namespace.to_string(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: Vec::new(),
				include_destroyed: Some(false),
				limit: Some(ACTOR_LIST_PAGE_SIZE),
				cursor: cursor.clone(),
			},
		)
		.await?;

		actors.extend(res.actors);

		if res.pagination.cursor.is_none() {
			break;
		}
		cursor = res.pagination.cursor;
	}

	Ok(actors)
}

async fn export_actor_kv(ctx: &ApiCtx, actor: &Actor, path: &Path) -> Result<()> {
	let file = fs::File::create(path).await?;
	let mut writer = BufWriter::new(file);
	let recipient = pegboard::actor_kv::Recipient {
		actor_id: actor.actor_id,
		namespace_id: actor.namespace_id,
		name: actor.name.clone(),
	};
	// KV keys are tuple-encoded with two wrapper bytes, so the largest legal raw key is
	// `MAX_KEY_SIZE - 2` bytes long.
	let max_end_key = vec![0xFF; pegboard::actor_kv::MAX_KEY_SIZE - 2];
	let mut after_key: Option<Vec<u8>> = None;

	loop {
		let previous_key = after_key.clone();
		// TODO: v1 does not quiesce actors before export. A future workflow should freeze or otherwise
		// quiesce actors before export to improve consistency.
		let query = if let Some(start) = previous_key.clone() {
			ep::KvListQuery::KvListRangeQuery(ep::KvListRangeQuery {
				start,
				end: max_end_key.clone(),
				exclusive: true,
			})
		} else {
			ep::KvListQuery::KvListAllQuery
		};
		let (keys, values, _) =
			pegboard::actor_kv::list(&*ctx.udb()?, &recipient, query, false, Some(KV_BATCH_SIZE))
				.await?;

		if keys.is_empty() {
			break;
		}

		let mut wrote_any = false;
		for (key, value) in keys.into_iter().zip(values.into_iter()).filter(|(key, _)| {
			previous_key
				.as_ref()
				.map(|prev| key != prev)
				.unwrap_or(true)
		}) {
			let payload = encode_kv_entry(&KvArchiveEntry {
				key: key.clone(),
				value,
			})?;
			writer.write_u32(payload.len().try_into()?).await?;
			writer.write_all(&payload).await?;
			after_key = Some(key);
			wrote_any = true;
		}

		if !wrote_any {
			break;
		}
	}

	writer.flush().await?;
	Ok(())
}

async fn create_imported_actor(
	ctx: &ApiCtx,
	target_namespace: &str,
	target_namespace_id: Id,
	metadata: &ActorMetadataV1,
) -> Result<Actor> {
	let inner_ctx: rivet_api_builder::ApiCtx = ctx.clone().into();
	let target_dc_label = utils::find_dc_for_actor_creation(
		&inner_ctx,
		target_namespace_id,
		target_namespace,
		&metadata.runner_name_selector,
		None,
	)
	.await?;
	let actor_id = Id::new_v1(target_dc_label);
	let query = rivet_api_peer::actors::import_create::ImportCreateQuery {
		namespace: target_namespace.to_string(),
	};
	let request = rivet_api_peer::actors::import_create::ImportCreateRequest {
		actor_id,
		name: metadata.name.clone(),
		key: metadata.key.clone(),
		runner_name_selector: metadata.runner_name_selector.clone(),
		crash_policy: metadata.crash_policy,
		create_ts: metadata.create_ts,
	};

	let response = if target_dc_label == ctx.config().dc_label() {
		rivet_api_peer::actors::import_create::create(ctx.clone().into(), (), query, request)
			.await?
	} else {
		request_remote_datacenter::<rivet_api_peer::actors::import_create::ImportCreateResponse>(
			ctx.config(),
			target_dc_label,
			"/actors/import-create",
			Method::POST,
			Some(&query),
			Some(&request),
		)
		.await?
	};

	Ok(response.actor)
}

async fn export_actor_sqlite_v2(ctx: &ApiCtx, actor: &Actor, path: &Path) -> Result<()> {
	let entries = pegboard::actor_sqlite_v2::export_actor(&*ctx.udb()?, actor.actor_id).await?;

	let file = fs::File::create(path).await?;
	let mut writer = BufWriter::new(file);

	for (key_suffix, value) in entries {
		let payload = encode_sqlite_entry(&SqliteArchiveEntry { key_suffix, value })?;
		writer.write_u32(payload.len().try_into()?).await?;
		writer.write_all(&payload).await?;
	}

	writer.flush().await?;
	Ok(())
}

async fn replay_actor_sqlite_v2(ctx: &ApiCtx, actor: &Actor, sqlite_path: &Path) -> Result<()> {
	let file = fs::File::open(sqlite_path).await?;
	let mut reader = BufReader::new(file);
	let mut entries = Vec::new();

	loop {
		let entry_len = match reader.read_u32().await {
			Ok(len) => usize::try_from(len)?,
			Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
			Err(err) => return Err(err.into()),
		};

		let mut payload = vec![0; entry_len];
		reader.read_exact(&mut payload).await?;
		let entry = decode_sqlite_entry(&payload)?;
		entries.push((entry.key_suffix, entry.value));
	}

	if !entries.is_empty() {
		pegboard::actor_sqlite_v2::import_actor(&*ctx.udb()?, actor.actor_id, entries).await?;
	}

	Ok(())
}

async fn replay_actor_kv(ctx: &ApiCtx, actor: &Actor, kv_path: &Path) -> Result<()> {
	let file = fs::File::open(kv_path).await?;
	let mut reader = BufReader::new(file);
	let recipient = pegboard::actor_kv::Recipient {
		actor_id: actor.actor_id,
		namespace_id: actor.namespace_id,
		name: actor.name.clone(),
	};
	let mut keys = Vec::new();
	let mut values = Vec::new();

	loop {
		let entry_len = match reader.read_u32().await {
			Ok(len) => usize::try_from(len)?,
			Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
			Err(err) => return Err(err.into()),
		};

		let mut payload = vec![0; entry_len];
		reader.read_exact(&mut payload).await?;
		let entry = decode_kv_entry(&payload)?;
		keys.push(entry.key);
		values.push(entry.value);

		if keys.len() >= KV_BATCH_SIZE {
			pegboard::actor_kv::put(&*ctx.udb()?, &recipient, keys, values).await?;
			keys = Vec::new();
			values = Vec::new();
		}
	}

	if !keys.is_empty() {
		pegboard::actor_kv::put(&*ctx.udb()?, &recipient, keys, values).await?;
	}

	Ok(())
}

async fn rollback_imported_actor(ctx: &ApiCtx, target_namespace: &str, actor_id: Id) -> Result<()> {
	if actor_id.label() == ctx.config().dc_label() {
		rivet_api_peer::actors::delete::delete(
			ctx.clone().into(),
			delete::DeletePath { actor_id },
			delete::DeleteQuery {
				namespace: target_namespace.to_string(),
			},
		)
		.await?;
	} else {
		request_remote_datacenter::<delete::DeleteResponse>(
			ctx.config(),
			actor_id.label(),
			&format!("/actors/{actor_id}"),
			Method::DELETE,
			Some(&delete::DeleteQuery {
				namespace: target_namespace.to_string(),
			}),
			Option::<&()>::None,
		)
		.await?;
	}

	Ok(())
}

async fn actor_exists_with_name_and_key(
	ctx: &ApiCtx,
	namespace: &str,
	name: &str,
	key: Option<&str>,
) -> Result<bool> {
	if let Some(key) = key {
		let res = list_routes::list_inner(
			ctx.clone(),
			list_types::ListQuery {
				namespace: namespace.to_string(),
				name: Some(name.to_string()),
				key: Some(key.to_string()),
				actor_ids: None,
				actor_id: Vec::new(),
				include_destroyed: Some(false),
				limit: Some(1),
				cursor: None,
			},
		)
		.await?;

		return Ok(!res.actors.is_empty());
	}

	let mut cursor = None;
	loop {
		let res = list_routes::list_inner(
			ctx.clone(),
			list_types::ListQuery {
				namespace: namespace.to_string(),
				name: Some(name.to_string()),
				key: None,
				actor_ids: None,
				actor_id: Vec::new(),
				include_destroyed: Some(false),
				limit: Some(ACTOR_LIST_PAGE_SIZE),
				cursor: cursor.clone(),
			},
		)
		.await?;

		if res.actors.iter().any(|actor| actor.key.is_none()) {
			return Ok(true);
		}

		if res.pagination.cursor.is_none() {
			return Ok(false);
		}
		cursor = res.pagination.cursor;
	}
}

async fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
	let bytes = serde_json::to_vec_pretty(value)?;
	fs::write(path, bytes).await?;
	Ok(())
}

async fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
	let bytes = fs::read(path).await?;
	Ok(serde_json::from_slice(&bytes)?)
}

fn encode_kv_entry(entry: &KvArchiveEntry) -> Result<Vec<u8>> {
	Ok(serde_bare::to_vec(entry)?)
}

fn decode_kv_entry(payload: &[u8]) -> Result<KvArchiveEntry> {
	Ok(serde_bare::from_slice(payload)?)
}

fn encode_sqlite_entry(entry: &SqliteArchiveEntry) -> Result<Vec<u8>> {
	Ok(serde_bare::to_vec(entry)?)
}

fn decode_sqlite_entry(payload: &[u8]) -> Result<SqliteArchiveEntry> {
	Ok(serde_bare::from_slice(payload)?)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn selector_requires_exactly_one_variant() {
		let err = parse_selector(&ExportSelector {
			all: Some(true),
			actor_names: Some(ExportActorNamesSelector {
				names: vec!["foo".to_string()],
			}),
			actor_ids: None,
		})
		.expect_err("selector with multiple variants should fail");

		assert!(
			err.to_string().contains("exactly one"),
			"unexpected selector validation error: {err:#}"
		);
	}

	#[test]
	fn selector_accepts_actor_ids() {
		let selector = parse_selector(&ExportSelector {
			all: None,
			actor_names: None,
			actor_ids: Some(ExportActorIdsSelector {
				ids: vec![Id::new_v1(1), Id::new_v1(1)],
			}),
		})
		.expect("selector with actor ids should be valid");

		match selector {
			SelectorVariant::ActorIds(ids) => assert_eq!(ids.len(), 2),
			_ => panic!("expected actor id selector"),
		}
	}

	#[test]
	fn kv_entry_round_trip() {
		let encoded = encode_kv_entry(&KvArchiveEntry {
			key: b"hello".to_vec(),
			value: b"world".to_vec(),
		})
		.expect("failed to encode kv entry");
		let decoded = decode_kv_entry(&encoded).expect("failed to decode kv entry");

		assert_eq!(decoded.key, b"hello");
		assert_eq!(decoded.value, b"world");
	}

	#[test]
	fn sqlite_entry_round_trip() {
		let encoded = encode_sqlite_entry(&SqliteArchiveEntry {
			key_suffix: b"/META".to_vec(),
			value: b"opaque-bytes".to_vec(),
		})
		.expect("failed to encode sqlite entry");
		let decoded = decode_sqlite_entry(&encoded).expect("failed to decode sqlite entry");

		assert_eq!(decoded.key_suffix, b"/META");
		assert_eq!(decoded.value, b"opaque-bytes");
	}
}
