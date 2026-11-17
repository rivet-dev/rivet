//! `cutover`, `confirm` and `restore`: the fence that hands an actor from V2 to V3 and back.
//!
//! **Cutover** writes only `DestroyTsKey = cutoverTs`. V2's gateway reads `DestroyTsKey`
//! serializably and answers `actor.not_found` without waking, so no new wake starts. `cutoverTs`
//! is the idempotency key: a retry sees `DestroyTsKey == cutoverTs` and answers `alreadyCut`.
//!
//! **Confirm** runs once a delay longer than V2's guard timeouts has passed, so any wake that read
//! the actor before the cutover has landed. Such a wake is visible in one of the checked keys: it
//! moved the generation or armed a retry deadline, holds the workflow's worker, or left a pending
//! signal. If every check holds, the same transaction **parks** the actor: it moves the workflow's
//! deadline and its signal listen set out of gasoline's wake index into a parked record, so no
//! worker pulls the workflow again and no direct signal wakes it. If any check fails, it clears
//! the cutover ("un-cut") and V2 keeps serving the actor.
//!
//! **Restore** writes the wake rows back from the parked record, clears `DestroyTsKey` and deletes
//! the record, so the actor is an ordinary sleeping V2 actor again.

use anyhow::{Context, Result};
use axum::body::Bytes;
use futures_util::TryStreamExt;
use pegboard::keys;
use rivet_util::Id;
use universaldb::{options::StreamingMode, utils::IsolationLevel::Serializable};
use v2_migration_protocol as proto;

use super::{authorize, decode_body, ensure_local, ensure_namespace, facts, keys as mkeys};
use crate::{ctx::ApiCtx, errors};

pub async fn cutover(ctx: ApiCtx, actor_id: Id, namespace: String, body: Bytes) -> Result<Vec<u8>> {
	ensure_local(&ctx, actor_id)?;
	let namespace_id = authorize(&ctx, &namespace).await?;
	let request = decode_body::<proto::CutoverRequest>(&body)?;

	let hibernating_since_ts = facts::hibernating_since_ts(
		ctx.config()
			.pegboard()
			.hibernating_request_eligible_threshold(),
	);
	let udb = ctx.pools().udb()?;
	let response = udb
		.txn("migration_cutover", |tx| {
			let request = request.clone();
			async move {
				let actor = facts::read(&tx, actor_id, hibernating_since_ts, Serializable).await?;
				ensure_namespace(actor.as_ref(), namespace_id)?;
				let actor = actor.context("actor checked above")?;

				if let Some(destroy_ts) = actor.destroy_ts {
					return Ok(if destroy_ts == request.cutover_ts {
						proto::CutoverResponse::AlreadyCut
					} else {
						proto::CutoverResponse::Destroyed
					});
				}

				if actor.generation != request.generation {
					return Ok(proto::CutoverResponse::Woke);
				}

				// A cutover that a confirm or restore already undid is a late duplicate of an
				// abandoned attempt. Cutting again would hide the actor with nobody to finish it.
				if read_i64(&tx, &mkeys::uncut(actor_id))
					.await?
					.is_some_and(|uncut| request.cutover_ts <= uncut)
				{
					return Ok(proto::CutoverResponse::Woke);
				}

				let ineligible = if actor.version == 1 {
					Some(proto::IneligibleReason::ActorV1)
				} else if actor.sleep_ts.is_none() {
					Some(proto::IneligibleReason::NotSleeping)
				} else if actor.allocated {
					Some(proto::IneligibleReason::Allocated)
				} else if actor.hibernating {
					Some(proto::IneligibleReason::Hibernating)
				} else if facts::sqlite_v1(&tx, namespace_id, actor_id, Serializable).await? {
					Some(proto::IneligibleReason::SqliteV1)
				} else {
					None
				};
				if let Some(reason) = ineligible {
					return Ok(proto::CutoverResponse::Ineligible(proto::Ineligible {
						reason,
					}));
				}

				tx.with_subspace(keys::subspace()).write(
					&keys::actor::DestroyTsKey::new(actor_id),
					request.cutover_ts,
				)?;

				Ok(proto::CutoverResponse::Cut)
			}
		})
		.await?;

	tracing::info!(%actor_id, cutover_ts = request.cutover_ts, response = ?response, "migration cutover");
	proto::encode(&response)
}

pub async fn confirm(ctx: ApiCtx, actor_id: Id, namespace: String, body: Bytes) -> Result<Vec<u8>> {
	ensure_local(&ctx, actor_id)?;
	let namespace_id = authorize(&ctx, &namespace).await?;
	let request = decode_body::<proto::ConfirmRequest>(&body)?;
	let v3_actor_id = Id::from_slice(&request.v3_actor_id).map_err(|_| {
		errors::Migration::InvalidRequest {
			message: "v3ActorId is not an actor id".to_string(),
		}
		.build()
	})?;

	let pull_ahead_ms = pull_ahead_ms(&ctx);
	let hibernating_since_ts = facts::hibernating_since_ts(
		ctx.config()
			.pegboard()
			.hibernating_request_eligible_threshold(),
	);
	let udb = ctx.pools().udb()?;
	let response = udb
		.txn("migration_confirm", |tx| {
			let request = request.clone();
			async move {
				let actor = facts::read(&tx, actor_id, hibernating_since_ts, Serializable).await?;
				ensure_namespace(actor.as_ref(), namespace_id)?;
				let actor = actor.context("actor checked above")?;

				if let Some(raw) = tx.get(&mkeys::parked(actor_id), Serializable).await? {
					let parked = facts::decode_parked(&raw)?;
					if parked.cutover_ts == request.cutover_ts {
						return Ok(proto::ConfirmResponse::AlreadyConfirmed);
					}
					return Err(errors::Migration::CutoverMismatch {
						parked_cutover_ts: parked.cutover_ts,
						cutover_ts: request.cutover_ts,
					}
					.build());
				}

				match actor.destroy_ts {
					None => {
						return Ok(proto::ConfirmResponse::ConfirmWoke(proto::ConfirmWoke {
							reason: proto::WokeReason::NotCut,
						}));
					}
					Some(destroy_ts) if destroy_ts != request.cutover_ts => {
						return Ok(proto::ConfirmResponse::Destroyed);
					}
					Some(_) => {}
				}

				if let Some(reason) = woke_reason(pull_ahead_ms, &tx, &actor, &request).await? {
					// Un-cut: only our own cutover value is cleared, never a real destroy.
					tx.with_subspace(keys::subspace())
						.delete(&keys::actor::DestroyTsKey::new(actor_id));
					write_uncut(&tx, actor_id, request.cutover_ts).await?;
					return Ok(proto::ConfirmResponse::ConfirmWoke(proto::ConfirmWoke {
						reason,
					}));
				}

				park(&tx, &actor, &request, v3_actor_id).await?;
				Ok(proto::ConfirmResponse::Confirmed)
			}
		})
		.await?;

	tracing::info!(%actor_id, cutover_ts = request.cutover_ts, response = ?response, "migration confirm");
	proto::encode(&response)
}

/// Returns why the actor cannot be parked, or `None` if every confirm check holds.
async fn woke_reason(
	pull_ahead_ms: i64,
	tx: &universaldb::Transaction,
	actor: &facts::ActorFacts,
	request: &proto::ConfirmRequest,
) -> Result<Option<proto::WokeReason>> {
	if actor.generation != request.generation {
		return Ok(Some(proto::WokeReason::GenerationMoved));
	}
	if actor.sleep_ts != Some(request.sleep_ts) {
		return Ok(Some(proto::WokeReason::SleepMoved));
	}
	if actor.allocated {
		return Ok(Some(proto::WokeReason::Allocated));
	}
	if actor.hibernating {
		return Ok(Some(proto::WokeReason::Hibernating));
	}

	let workflow_id = actor.workflow_id;
	let worker_key = mkeys::worker_id(workflow_id);
	let lease_key = mkeys::lease(workflow_id);
	let silence_key = mkeys::silence_ts(workflow_id);
	let pending_signals = mkeys::pending_signal_subspace(workflow_id);
	let (worker, lease, silenced, pending_signal) = tokio::try_join!(
		tx.get(&worker_key, Serializable),
		tx.get(&lease_key, Serializable),
		tx.get(&silence_key, Serializable),
		first_row(tx, &pending_signals),
	)?;
	if worker.is_some() || lease.is_some() {
		return Ok(Some(proto::WokeReason::WorkerRunning));
	}
	// An operator silenced the workflow already; parking would lose that on restore.
	if silenced.is_some() {
		return Ok(Some(proto::WokeReason::Silenced));
	}
	if pending_signal {
		return Ok(Some(proto::WokeReason::PendingSignal));
	}

	if actor.wake_deadline_ts != request.wake_deadline_ts {
		return Ok(Some(proto::WokeReason::DeadlineMoved));
	}
	if let Some(deadline_ts) = actor.wake_deadline_ts {
		// A worker pulls deadlines up to its poll interval ahead and holds them in memory, so a
		// deadline inside that window may already belong to a worker that read it.
		let horizon_ms = request.horizon_ms.max(pull_ahead_ms);
		let horizon = rivet_util::timestamp::now().saturating_add(horizon_ms);
		if deadline_ts < horizon {
			return Ok(Some(proto::WokeReason::DeadlineSoon));
		}
	}

	Ok(None)
}

/// Moves the workflow's wake rows into the parked record, in the confirm transaction.
async fn park(
	tx: &universaldb::Transaction,
	actor: &facts::ActorFacts,
	request: &proto::ConfirmRequest,
	v3_actor_id: Id,
) -> Result<()> {
	let workflow_id = actor.workflow_id;
	let workflow_name = facts::read_workflow_name(tx, workflow_id, Serializable).await?;
	let wake_signals = read_wake_signals(tx, workflow_id).await?;

	if let Some(deadline_ts) = actor.wake_deadline_ts {
		tx.clear(&mkeys::wake_deadline(workflow_id));
		tx.clear(&mkeys::deadline_wake_row(
			&workflow_name,
			workflow_id,
			deadline_ts,
		));
	}
	tx.clear_subspace_range(&mkeys::wake_signal_subspace(workflow_id));
	let parked_at = rivet_util::timestamp::now();
	tx.set(&mkeys::silence_ts(workflow_id), &parked_at.to_be_bytes());

	let record = proto::ParkedRecord {
		generation: actor.generation,
		cutover_ts: request.cutover_ts,
		parked_at,
		v3_actor_id: v3_actor_id.as_bytes(),
		workflow_id: workflow_id.as_bytes(),
		workflow_name,
		wake_deadline_ts: actor.wake_deadline_ts,
		wake_signals,
	};
	tx.set(
		&mkeys::parked(actor.actor_id),
		&facts::encode_parked(record)?,
	);

	Ok(())
}

pub async fn restore(ctx: ApiCtx, actor_id: Id, namespace: String, body: Bytes) -> Result<Vec<u8>> {
	ensure_local(&ctx, actor_id)?;
	let namespace_id = authorize(&ctx, &namespace).await?;
	let request = decode_body::<proto::RestoreRequest>(&body)?;

	let hibernating_since_ts = facts::hibernating_since_ts(
		ctx.config()
			.pegboard()
			.hibernating_request_eligible_threshold(),
	);
	let udb = ctx.pools().udb()?;
	let response = udb
		.txn("migration_restore", |tx| {
			let request = request.clone();
			async move {
				let actor = facts::read(&tx, actor_id, hibernating_since_ts, Serializable).await?;
				ensure_namespace(actor.as_ref(), namespace_id)?;

				let Some(raw) = tx.get(&mkeys::parked(actor_id), Serializable).await? else {
					return Ok(proto::RestoreResponse::NotParked);
				};
				let parked = facts::decode_parked(&raw)?;
				if request
					.cutover_ts
					.is_some_and(|cutover_ts| cutover_ts != parked.cutover_ts)
				{
					return Err(errors::Migration::CutoverMismatch {
						parked_cutover_ts: parked.cutover_ts,
						cutover_ts: request.cutover_ts.unwrap_or_default(),
					}
					.build());
				}

				if let Some(v3_actor_id) = &request.v3_actor_id
					&& v3_actor_id[..] != parked.v3_actor_id[..]
				{
					return Err(errors::Migration::InvalidRequest {
						message: "the parked actor was migrated to a different V3 actor"
							.to_string(),
					}
					.build());
				}

				let workflow_id =
					Id::from_slice(&parked.workflow_id).context("parked workflow id")?;
				tx.clear(&mkeys::silence_ts(workflow_id));
				for signal_name in &parked.wake_signals {
					let row =
						mkeys::wake_signal_subspace(workflow_id).pack(&(signal_name.as_str(),));
					tx.set(&row, &[]);
				}
				if let Some(deadline_ts) = parked.wake_deadline_ts {
					tx.set(
						&mkeys::wake_deadline(workflow_id),
						&deadline_ts.to_be_bytes(),
					);
					tx.set(
						&mkeys::deadline_wake_row(&parked.workflow_name, workflow_id, deadline_ts),
						&mkeys::wake_row_nonce(),
					);
				}

				// A signal sent while parked found no listen row, so nothing woke the workflow for
				// it. Wake it once now so it handles what is pending.
				if first_row(&tx, &mkeys::pending_signal_subspace(workflow_id)).await? {
					tx.set(
						&mkeys::immediate_wake_row(
							&parked.workflow_name,
							workflow_id,
							rivet_util::timestamp::now(),
						),
						&mkeys::wake_row_nonce(),
					);
				}

				// Only the cutover's own value is cleared, never a later real destroy.
				let pb = tx.with_subspace(keys::subspace());
				let destroy_key = keys::actor::DestroyTsKey::new(actor_id);
				if pb.read_opt(&destroy_key, Serializable).await? == Some(parked.cutover_ts) {
					pb.delete(&destroy_key);
				}
				tx.clear(&mkeys::parked(actor_id));
				// A late duplicate of the original cutover must not cut the restored actor.
				write_uncut(&tx, actor_id, parked.cutover_ts).await?;

				Ok(proto::RestoreResponse::Restored)
			}
		})
		.await?;

	tracing::info!(%actor_id, cutover_ts = ?request.cutover_ts, response = ?response, "migration restore");
	proto::encode(&response)
}

async fn read_wake_signals(tx: &universaldb::Transaction, workflow_id: Id) -> Result<Vec<String>> {
	let subspace = mkeys::wake_signal_subspace(workflow_id);
	tx.get_ranges_keyvalues(
		universaldb::RangeOption {
			mode: StreamingMode::WantAll,
			..(&subspace).into()
		},
		Serializable,
	)
	.map_err(anyhow::Error::from)
	.and_then(|row| {
		let subspace = subspace.clone();
		async move {
			let (signal_name,) = subspace
				.unpack::<(String,)>(row.key())
				.context("unpack wake signal row")?;
			Ok(signal_name)
		}
	})
	.try_collect()
	.await
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

/// Gasoline's pull-ahead window plus a margin: the shortest confirm horizon that is safe.
fn pull_ahead_ms(ctx: &ApiCtx) -> i64 {
	let poll_ms = ctx.config().runtime.worker_poll_interval().as_millis() as i64;
	poll_ms.saturating_add(5_000)
}

async fn read_i64(tx: &universaldb::Transaction, key: &[u8]) -> Result<Option<i64>> {
	let Some(raw) = tx.get(key, Serializable).await? else {
		return Ok(None);
	};
	let bytes: [u8; 8] = raw[..]
		.try_into()
		.context("migration value is not 8 bytes")?;
	Ok(Some(i64::from_be_bytes(bytes)))
}

/// Raises the actor's un-cut tombstone to `cutover_ts`.
async fn write_uncut(tx: &universaldb::Transaction, actor_id: Id, cutover_ts: i64) -> Result<()> {
	let key = mkeys::uncut(actor_id);
	let current = read_i64(tx, &key).await?;
	if current.is_none_or(|current| current < cutover_ts) {
		tx.set(&key, &cutover_ts.to_be_bytes());
	}
	Ok(())
}
