//! The `/migration/v2/` routes that delete actors V2 parked after migrating them to V3.
//!
//! A parked record stays after its actor is deleted, so the parked records are the list of what
//! a migration moved out of V2 and how far its deletion got. A deletion run lists a namespace's
//! parked actors, deletes each one, and saves its progress in the namespace's delete cursor so a
//! later run resumes where it stopped.
//!
//! **Delete** destroys a parked actor through its own workflow, so V2's destroy steps clear its
//! state, indexes, KV and SQLite. A parked workflow is silenced and has no listen rows, but a
//! signal sent to it while parked still waits as a pending signal, and gasoline's listen handles
//! the oldest pending signal across every name it listens for. Delete therefore sends the destroy
//! signal while the workflow is still silenced, silences every other pending signal, and only
//! then, in one transaction that sees no other pending signal, writes the destroy listen row,
//! clears the workflow's silence and wakes it. A signal sent after that sorts after the destroy,
//! and the destroy ends the workflow. Every step is idempotent, so a caller retries a failed
//! delete with the same request.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use axum::body::Bytes;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use futures_util::TryStreamExt;
use gas::db::{Database, debug::DatabaseDebug};
use pegboard::keys;
use rivet_util::Id;
use universaldb::{
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};
use v2_migration_protocol as proto;

use super::{ParkedQuery, authorize, decode_body, ensure_local, facts, keys as mkeys};
use crate::{ctx::ApiCtx, errors};

const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 500;
const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);

/// Times a delete silences newly arrived signals before it gives up and leaves the retry to the
/// caller. Each round only finds signals sent during the previous one.
const MAX_ARM_ROUNDS: usize = 8;

/// `GET /migration/v2/namespaces/{namespace}/parked`: one page of the namespace's parked actors
/// in this datacenter, deleted ones included.
///
/// Parked records are keyed by actor id alone, so a page scans `limit` records of every namespace
/// and returns the ones in this namespace. A page can therefore be short or empty while `next` is
/// set.
pub async fn list(ctx: ApiCtx, namespace: String, query: ParkedQuery) -> Result<Vec<u8>> {
	let namespace_id = authorize(&ctx, &namespace).await?;

	let limit = query.limit.unwrap_or(DEFAULT_LIMIT);
	if limit == 0 || limit > MAX_LIMIT {
		return Err(errors::Migration::InvalidRequest {
			message: format!("limit must be between 1 and {MAX_LIMIT}"),
		}
		.build());
	}
	let after = match &query.after {
		Some(after) => Some(decode_cursor(after)?),
		None => None,
	};

	let udb = ctx.pools().udb()?;

	// The scan and the namespace reads run in separate transactions, so a page of records never
	// shares one transaction's time limit.
	let (rows, last_key, has_more) = udb
		.txn("migration_parked_list_scan", |tx| {
			let after = after.clone();
			async move {
				let start = Instant::now();
				let (range_start, range_end) = mkeys::parked_subspace().range();
				// The cursor is the last scanned key, so the page starts just after it.
				let begin = match after {
					Some(mut after) => {
						after.push(0x00);
						after
					}
					None => range_start,
				};

				let mut stream = tx.get_ranges_keyvalues(
					universaldb::RangeOption {
						limit: Some(limit),
						mode: StreamingMode::Iterator,
						..(begin, range_end).into()
					},
					Snapshot,
				);

				let mut rows = Vec::new();
				let mut last_key = None;
				let mut scanned = 0;
				let mut timed_out = false;

				while let Some(entry) = stream.try_next().await? {
					scanned += 1;
					last_key = Some(entry.key().to_vec());

					match mkeys::parked_actor_id(entry.key()) {
						Some(actor_id) => {
							rows.push((entry.key().to_vec(), actor_id, entry.value().to_vec()))
						}
						None => tracing::error!("failed to unpack parked record key, skipping"),
					}

					if start.elapsed() > EARLY_TXN_TIMEOUT {
						tracing::warn!("timed out scanning parked records");
						timed_out = true;
						break;
					}
				}

				Ok((rows, last_key, timed_out || scanned == limit))
			}
		})
		.await?;

	// Reads each record's namespace in as many transactions as it takes. Each one resumes at the
	// first record the previous one did not read.
	let mut actors = Vec::new();
	let mut read_count = 0;
	while read_count < rows.len() {
		let (chunk, chunk_read_count) = udb
			.txn("migration_parked_list_namespaces", |tx| {
				let pending = &rows[read_count..];
				async move {
					let start = Instant::now();
					let pb = tx.with_subspace(keys::subspace());
					let mut chunk = Vec::new();
					let mut chunk_read_count = 0;

					for (raw_key, actor_id, raw_value) in pending {
						chunk_read_count += 1;

						let actor_namespace_id = pb
							.read_opt(&keys::actor::NamespaceIdKey::new(*actor_id), Snapshot)
							.await?;
						if actor_namespace_id == Some(namespace_id) {
							let record = facts::decode_parked(raw_value)?;
							chunk.push(proto::ParkedActor {
								actor_id: actor_id.as_bytes(),
								v3_actor_id: record.v3_actor_id,
								generation: record.generation,
								cutover_ts: record.cutover_ts,
								parked_at: record.parked_at,
								delete_requested_at: record.delete_requested_at,
								deleted_at: record.deleted_at,
								cursor: raw_key.clone(),
							});
						}

						if start.elapsed() > EARLY_TXN_TIMEOUT {
							tracing::warn!("timed out reading parked record namespaces");
							break;
						}
					}

					Ok((chunk, chunk_read_count))
				}
			})
			.await?;

		actors.extend(chunk);
		read_count += chunk_read_count;
	}

	let next = if has_more { last_key } else { None };
	let response = proto::ParkedResponse { actors, next };

	proto::encode(&response)
}

/// `GET /migration/v2/namespaces/{namespace}/parked/delete-cursor`.
pub async fn get_delete_cursor(ctx: ApiCtx, namespace: String) -> Result<Vec<u8>> {
	let namespace_id = authorize(&ctx, &namespace).await?;

	let udb = ctx.pools().udb()?;
	let cursor = udb
		.txn("migration_delete_cursor_get", |tx| async move {
			let raw = tx
				.get(&mkeys::delete_cursor(namespace_id), Serializable)
				.await?;
			Ok(raw.map(|raw| raw.to_vec()).filter(|raw| !raw.is_empty()))
		})
		.await?;

	proto::encode(&proto::DeleteCursor { cursor })
}

/// `PUT /migration/v2/namespaces/{namespace}/parked/delete-cursor`. An unset cursor clears it.
pub async fn put_delete_cursor(ctx: ApiCtx, namespace: String, body: Bytes) -> Result<Vec<u8>> {
	let namespace_id = authorize(&ctx, &namespace).await?;
	authorize_delete(&ctx, &namespace).await?;
	let request = decode_body::<proto::DeleteCursor>(&body)?;
	if let Some(cursor) = &request.cursor
		&& mkeys::parked_actor_id(cursor).is_none()
	{
		return Err(errors::Migration::InvalidRequest {
			message: "cursor is not a parked actor cursor".to_string(),
		}
		.build());
	}

	let udb = ctx.pools().udb()?;
	udb.txn("migration_delete_cursor_put", |tx| {
		let cursor = request.cursor.clone();
		async move {
			let key = mkeys::delete_cursor(namespace_id);
			match cursor {
				Some(cursor) => tx.set(&key, &cursor),
				None => tx.clear(&key),
			}
			Ok(())
		}
	})
	.await?;

	proto::encode(&request)
}

/// What one delete request found in the parked record.
enum DeleteStep {
	NotParked,
	Deleted,
	/// The destroy is armed: the workflow is awake or already destroying the actor.
	Armed,
	/// A delete was requested but the workflow is still parked, so the destroy still has to be
	/// armed.
	Arm {
		workflow_id: Id,
		workflow_name: String,
	},
}

/// `POST /migration/v2/actors/{actor_id}/delete`.
pub async fn delete(ctx: ApiCtx, actor_id: Id, namespace: String, body: Bytes) -> Result<Vec<u8>> {
	ensure_local(&ctx, actor_id)?;
	let namespace_id = authorize(&ctx, &namespace).await?;
	authorize_delete(&ctx, &namespace).await?;
	let request = decode_body::<proto::DeleteRequest>(&body)?;

	let step = request_delete(&ctx, actor_id, namespace_id, &request).await?;
	let response = match step {
		DeleteStep::NotParked => proto::DeleteResponse::NotParked,
		DeleteStep::Deleted => proto::DeleteResponse::Deleted,
		DeleteStep::Armed => proto::DeleteResponse::Deleting,
		DeleteStep::Arm {
			workflow_id,
			workflow_name,
		} => {
			arm_destroy(&ctx, actor_id, workflow_id, &workflow_name).await?;
			proto::DeleteResponse::Deleting
		}
	};

	proto::encode(&response)
}

/// Records the delete request in the parked record, or reports how far an earlier one got. Marks
/// the actor deleted once its workflow finished.
async fn request_delete(
	ctx: &ApiCtx,
	actor_id: Id,
	namespace_id: Id,
	request: &proto::DeleteRequest,
) -> Result<DeleteStep> {
	let udb = ctx.pools().udb()?;
	udb.txn("migration_delete_request", |tx| async move {
		let pb = tx.with_subspace(keys::subspace());
		let actor_namespace_id = pb
			.read_opt(&keys::actor::NamespaceIdKey::new(actor_id), Serializable)
			.await?;
		if actor_namespace_id != Some(namespace_id) {
			return Err(errors::Migration::ActorNotFound.build());
		}

		let Some(raw) = tx.get(&mkeys::parked(actor_id), Serializable).await? else {
			return Ok(DeleteStep::NotParked);
		};
		let mut record = facts::decode_parked(&raw)?;
		if request.v3_actor_id[..] != record.v3_actor_id[..] {
			return Err(errors::Migration::InvalidRequest {
				message: "the parked actor was migrated to a different V3 actor".to_string(),
			}
			.build());
		}
		if record.deleted_at.is_some() {
			return Ok(DeleteStep::Deleted);
		}

		let workflow_id = Id::from_slice(&record.workflow_id).context("parked workflow id")?;
		let workflow_name = record.workflow_name.clone();

		if record.delete_requested_at.is_none() {
			// Only an actor still held by its cutover is parked. The destroy rewrites the
			// destroy timestamp, so this holds only before the first delete request.
			let destroy_ts = pb
				.read_opt(&keys::actor::DestroyTsKey::new(actor_id), Serializable)
				.await?;
			if destroy_ts != Some(record.cutover_ts) {
				bail!(
					"parked actor {actor_id} has destroy timestamp {destroy_ts:?}, not its cutover {}",
					record.cutover_ts
				);
			}

			record.delete_requested_at = Some(rivet_util::timestamp::now());
			tx.set(&mkeys::parked(actor_id), &facts::encode_parked(record)?);
			return Ok(DeleteStep::Arm {
				workflow_id,
				workflow_name,
			});
		}

		// The silence stays until the destroy is armed, so a silenced workflow still needs it.
		if tx
			.get(&mkeys::silence_ts(workflow_id), Serializable)
			.await?
			.is_some()
		{
			return Ok(DeleteStep::Arm {
				workflow_id,
				workflow_name,
			});
		}

		if first_row(&tx, &mkeys::output_subspace(workflow_id)).await? {
			record.deleted_at = Some(rivet_util::timestamp::now());
			tx.set(&mkeys::parked(actor_id), &facts::encode_parked(record)?);
			return Ok(DeleteStep::Deleted);
		}

		Ok(DeleteStep::Armed)
	})
	.await
}

/// Sends the destroy signal to the silenced workflow, silences every other pending signal, then
/// wakes the workflow with the destroy as its only pending signal.
async fn arm_destroy(
	ctx: &ApiCtx,
	actor_id: Id,
	workflow_id: Id,
	workflow_name: &str,
) -> Result<()> {
	let destroy_name = <pegboard::workflows::actor2::Destroy as gas::signal::Signal>::NAME;
	let db = gas::db::DatabaseKv::new(ctx.config().clone(), ctx.pools().clone()).await?;

	if pending_signals(ctx, workflow_id)
		.await?
		.iter()
		.all(|(signal_name, _)| signal_name != destroy_name)
	{
		ctx.signal(pegboard::workflows::actor2::Destroy {})
			.to_workflow_id(workflow_id)
			.send()
			.await
			.context("send destroy signal to parked actor workflow")?;
	}

	for _ in 0..MAX_ARM_ROUNDS {
		let others = pending_signals(ctx, workflow_id)
			.await?
			.into_iter()
			.filter(|(signal_name, _)| signal_name != destroy_name)
			.map(|(_, signal_id)| signal_id)
			.collect::<Vec<_>>();
		if !others.is_empty() {
			tracing::info!(
				?actor_id,
				count = others.len(),
				"silencing signals sent to a parked actor before deleting it"
			);
			DatabaseDebug::silence_signals(&*db, others).await?;
		}

		if wake_for_destroy(ctx, workflow_id, workflow_name, destroy_name).await? {
			return Ok(());
		}
	}

	bail!("signals kept arriving for parked actor {actor_id}; retry the delete")
}

/// Wakes the workflow when the destroy is its only pending signal, in one transaction that a new
/// pending signal conflicts with. Returns `false` when another signal arrived first.
async fn wake_for_destroy(
	ctx: &ApiCtx,
	workflow_id: Id,
	workflow_name: &str,
	destroy_name: &str,
) -> Result<bool> {
	let udb = ctx.pools().udb()?;
	udb.txn("migration_delete_arm", |tx| async move {
		let pending = tx
			.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&mkeys::pending_signal_subspace(workflow_id)).into()
				},
				Serializable,
			)
			.map_ok(|entry| entry.key().to_vec())
			.try_collect::<Vec<_>>()
			.await?;
		let mut has_destroy = false;
		for raw_key in &pending {
			let (signal_name, _ts, _signal_id) = mkeys::pending_signal_subspace(workflow_id)
				.unpack::<(String, i64, Id)>(raw_key)
				.context("unpack pending signal key")?;
			if signal_name != destroy_name {
				return Ok(false);
			}
			has_destroy = true;
		}
		if !has_destroy {
			bail!("the destroy signal of workflow {workflow_id} is no longer pending");
		}

		let listen_row = mkeys::wake_signal_subspace(workflow_id).pack(&(destroy_name,));
		tx.set(&listen_row, &[]);
		tx.clear(&mkeys::silence_ts(workflow_id));
		tx.set(
			&mkeys::immediate_wake_row(workflow_name, workflow_id, rivet_util::timestamp::now()),
			&mkeys::wake_row_nonce(),
		);

		Ok(true)
	})
	.await
}

/// The workflow's pending signals as `(name, signal id)`.
async fn pending_signals(ctx: &ApiCtx, workflow_id: Id) -> Result<Vec<(String, Id)>> {
	let udb = ctx.pools().udb()?;
	udb.txn("migration_delete_pending_signals", |tx| async move {
		let subspace = mkeys::pending_signal_subspace(workflow_id);
		let keys = tx
			.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&subspace).into()
				},
				Serializable,
			)
			.map_ok(|entry| entry.key().to_vec())
			.try_collect::<Vec<_>>()
			.await?;

		keys.iter()
			.map(|raw_key| {
				let (signal_name, _ts, signal_id) = subspace
					.unpack::<(String, i64, Id)>(raw_key)
					.context("unpack pending signal key")?;
				Ok((signal_name, signal_id))
			})
			.collect()
	})
	.await
}

/// Deleting destroys actors, so it also needs a token that may delete every actor in the
/// namespace.
async fn authorize_delete(ctx: &ApiCtx, namespace: &str) -> Result<()> {
	ctx.auth(
		rivet_auth::AccessNamespaceScope::Name(namespace.to_string()),
		rivet_auth::ResourceKind::Actor,
		rivet_auth::TargetScope::Any,
		rivet_auth::OperationKind::Delete,
	)
	.await
}

/// A cursor is any key in the parked subspace, not only a readable parked record key: a page that
/// ends on a record key it skipped as unreadable returns that key as `next`.
fn decode_cursor(cursor: &str) -> Result<Vec<u8>> {
	let raw = URL_SAFE_NO_PAD.decode(cursor).map_err(|_| {
		errors::Migration::InvalidRequest {
			message: "after is not a valid cursor".to_string(),
		}
		.build()
	})?;
	if !raw.starts_with(mkeys::parked_subspace().bytes()) {
		return Err(errors::Migration::InvalidRequest {
			message: "after is not a parked actor cursor".to_string(),
		}
		.build());
	}
	Ok(raw)
}

async fn first_row(
	tx: &universaldb::Transaction,
	subspace: &universaldb::utils::Subspace,
) -> Result<bool> {
	let mut stream = tx.get_ranges_keyvalues(
		universaldb::RangeOption {
			limit: Some(1),
			mode: StreamingMode::Small,
			..subspace.into()
		},
		Serializable,
	);

	Ok(stream.try_next().await?.is_some())
}
