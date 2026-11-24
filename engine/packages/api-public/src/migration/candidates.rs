//! `GET /migration/v1/namespaces/{namespace}/candidates`: one page of the namespace's active
//! actors with every fact the migrator selects on.
//!
//! V2 applies no eligibility rule. It returns raw facts so policy such as the minimum sleep age
//! or the largest copy stays in the migrator's configuration.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use futures_util::{StreamExt, TryStreamExt};
use pegboard::keys;
use rivet_util::Id;
use universaldb::{options::StreamingMode, prelude::*, utils::IsolationLevel::Snapshot};
use v2_migration_protocol as proto;

use super::{CandidatesQuery, authorize, ensure_local, ensure_namespace, facts};
use crate::{ctx::ApiCtx, errors};

const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 500;
/// Actors whose facts are read concurrently within one transaction.
const READ_CONCURRENCY: usize = 32;
const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);

pub async fn candidates(ctx: ApiCtx, namespace: String, query: CandidatesQuery) -> Result<Vec<u8>> {
	let namespace_id = authorize(&ctx, &namespace).await?;

	let limit = query.limit.unwrap_or(DEFAULT_LIMIT);
	if limit == 0 || limit > MAX_LIMIT {
		return Err(errors::Migration::InvalidRequest {
			message: format!("limit must be between 1 and {MAX_LIMIT}"),
		}
		.build());
	}

	let after = match &query.after {
		Some(after) => Some(URL_SAFE_NO_PAD.decode(after).map_err(|_| {
			errors::Migration::InvalidRequest {
				message: "after is not a valid cursor".to_string(),
			}
			.build()
		})?),
		None => None,
	};

	// The scan includes sleeping actors and covers every actor name, or only `name` when set.
	let active_subspace =
		keys::subspace().subspace(&(NAMESPACE, namespace_id, ACTOR, BY_NAME, ACTIVE));
	if let Some(after) = &after {
		if !after.starts_with(active_subspace.bytes()) {
			return Err(errors::Migration::InvalidRequest {
				message: "after does not belong to this namespace".to_string(),
			}
			.build());
		}
	}
	let scanned_subspace = match &query.name {
		Some(name) => active_subspace.subspace(&(name.as_str(),)),
		None => active_subspace.clone(),
	};
	let (range_start, range_end) = scanned_subspace.range();

	// A cursor is a position in the namespace's whole index, so one written by an unfiltered
	// listing or under another name still resumes a filtered one: a cursor before the name's
	// rows starts at its first row, and one past them leaves nothing to list.
	let after = match after {
		Some(after) if after < range_start => None,
		Some(after) if after >= range_end => {
			return proto::encode(&proto::CandidatesResponse {
				candidates: Vec::new(),
				next: None,
			});
		}
		after => after,
	};

	let hibernating_since_ts = facts::hibernating_since_ts(
		ctx.config()
			.pegboard()
			.hibernating_request_eligible_threshold(),
	);
	let udb = ctx.pools().udb()?;

	// The index scan and the facts reads run in separate transactions, so a page of actors never
	// shares one transaction's time limit.
	let (index_rows, last_key, has_more) = udb
		.txn("migration_candidates_index", |tx| {
			let after = after.clone();
			let range_start = range_start.clone();
			let range_end = range_end.clone();
			async move {
				let start = Instant::now();
				let tx = tx.with_subspace(keys::subspace());

				// The cursor is the last returned key, so the page starts just after it.
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

				let mut index_rows = Vec::new();
				let mut last_key = None;
				let mut scanned = 0;
				let mut timed_out = false;

				while let Some(entry) = stream.try_next().await? {
					scanned += 1;
					last_key = Some(entry.key().to_vec());

					match tx.unpack::<keys::ns::ActiveActorKey>(entry.key()) {
						Ok(key) => index_rows.push((entry.key().to_vec(), key.actor_id)),
						Err(err) => {
							tracing::error!(
								?err,
								"failed to unpack active actor index key, skipping"
							);
						}
					}

					if start.elapsed() > EARLY_TXN_TIMEOUT {
						tracing::warn!("timed out scanning migration candidates");
						timed_out = true;
						break;
					}
				}

				Ok((index_rows, last_key, timed_out || scanned == limit))
			}
		})
		.await?;

	// Reads facts in as many transactions as it takes. Each one resumes at the first actor the
	// previous one did not read.
	let mut candidates = Vec::new();
	let mut read_count = 0;
	while read_count < index_rows.len() {
		let (chunk, chunk_read_count) = udb
			.txn("migration_candidates_facts", |tx| {
				let pending = &index_rows[read_count..];
				async move {
					let start = Instant::now();
					let mut stream = futures_util::stream::iter(pending.iter().cloned())
						.map(|(raw_key, actor_id)| {
							let tx = tx.clone();
							async move {
								let candidate =
									read_candidate(&tx, actor_id, raw_key, hibernating_since_ts)
										.await;
								(actor_id, candidate)
							}
						})
						.buffered(READ_CONCURRENCY);

					let mut chunk = Vec::new();
					let mut chunk_read_count = 0;
					while let Some((actor_id, candidate)) = stream.next().await {
						chunk_read_count += 1;

						// One actor that fails to read must not fail the page, or the migrator
						// would retry the same cursor forever. A skipped actor is listed again on
						// the next pass.
						match candidate {
							Ok(Some(candidate)) => chunk.push(candidate),
							Ok(None) => {}
							Err(err) => {
								tracing::error!(
									?actor_id,
									?err,
									"failed to read migration candidate, skipping"
								);
							}
						}

						if start.elapsed() > EARLY_TXN_TIMEOUT {
							tracing::warn!("timed out reading migration candidate facts");
							break;
						}
					}

					Ok((chunk, chunk_read_count))
				}
			})
			.await?;

		candidates.extend(chunk);
		read_count += chunk_read_count;
	}

	let next = if has_more { last_key } else { None };
	let response = proto::CandidatesResponse { candidates, next };

	let next = response
		.next
		.as_ref()
		.map(|next| URL_SAFE_NO_PAD.encode(next));
	tracing::debug!(
		count = response.candidates.len(),
		has_next = next.is_some(),
		"listed migration candidates"
	);

	proto::encode(&response)
}

/// `GET /migration/v1/actors/{actor_id}/candidate`: the facts of one actor, as a listing would
/// return them, for a migration an operator requested by id. An actor outside the namespace is
/// `migration.actor_not_found`, as a missing one is.
pub async fn candidate(ctx: ApiCtx, actor_id: Id, namespace: String) -> Result<Vec<u8>> {
	ensure_local(&ctx, actor_id)?;
	let namespace_id = authorize(&ctx, &namespace).await?;

	let hibernating_since_ts = facts::hibernating_since_ts(
		ctx.config()
			.pegboard()
			.hibernating_request_eligible_threshold(),
	);
	let udb = ctx.pools().udb()?;
	let candidate = udb
		.txn("migration_candidate", |tx| async move {
			let actor = facts::read(&tx, actor_id, hibernating_since_ts, Snapshot).await?;
			ensure_namespace(actor.as_ref(), namespace_id)?;
			let actor = actor.context("actor checked above")?;
			candidate_of(&tx, actor_id, actor, Vec::new()).await
		})
		.await?;

	proto::encode(&candidate)
}

/// The candidate behind one index row. `cursor` is the row's raw key, which a listing resumes
/// right after.
async fn read_candidate(
	tx: &universaldb::Transaction,
	actor_id: Id,
	cursor: Vec<u8>,
	hibernating_since_ts: i64,
) -> Result<Option<proto::Candidate>> {
	let Some(facts) = facts::read(tx, actor_id, hibernating_since_ts, Snapshot).await? else {
		return Ok(None);
	};
	Ok(Some(candidate_of(tx, actor_id, facts, cursor).await?))
}

async fn candidate_of(
	tx: &universaldb::Transaction,
	actor_id: Id,
	facts: facts::ActorFacts,
	cursor: Vec<u8>,
) -> Result<proto::Candidate> {
	let (sqlite_v1, parked, sqlite_head, kv_bytes_estimate) = tokio::try_join!(
		facts::sqlite_v1(tx, facts.namespace_id, actor_id, Snapshot),
		facts::is_parked(tx, actor_id),
		facts::read_sqlite_head(tx, facts.namespace_id, actor_id, Snapshot),
		pegboard::actor_kv::estimate_kv_size(tx, actor_id),
	)?;

	Ok(proto::Candidate {
		actor_id: actor_id.as_bytes(),
		workflow_id: facts.workflow_id.as_bytes(),
		name: facts.name,
		key: facts.key,
		pool_name: facts.pool_name,
		create_ts: facts.create_ts,
		version: u8::try_from(facts.version).unwrap_or(u8::MAX),
		generation: facts.generation,
		sleep_ts: facts.sleep_ts,
		allocated: facts.allocated,
		destroy_ts: facts.destroy_ts,
		wake_deadline_ts: facts.wake_deadline_ts,
		hibernating: facts.hibernating,
		sqlite_v1,
		parked,
		db_size_pages: sqlite_head.map_or(0, |head| head.db_size_pages),
		kv_bytes_estimate: u64::try_from(kv_bytes_estimate).unwrap_or(0),
		cursor,
	})
}
