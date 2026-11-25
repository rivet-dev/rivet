//! Deletes V2's copy of the actors V2 parked after they migrated here, behind
//! `rivet-engine migrate-from-v2 delete-parked`.
//!
//! V2 destroys each one through its own actor workflow, which clears its KV and SQLite data, and
//! marks its parked record deleted. The run walks V2's parked listing one datacenter at a time and
//! resumes from the cursor V2 keeps for the namespace, which it moves past the actors it finished.
//! Deleting an actor's V2 copy cannot be undone, so before the first delete request this side
//! checks that the actor has left the migration here and, when
//! `migrate_from_v2.backup` is set, that its backup is complete. An actor that fails
//! a check is skipped and reported, and the saved cursor stops before it, so a later run checks it
//! again.

use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::{StreamExt, stream};
use migrate_from_v2_protocol as proto;
use pegboard::wire as pb;
use rivet_engine_context::EngineContext;
use rivet_util::Id;
use serde::Serialize;

use crate::{backup, recover::legacy_moved_keyed_copy, v2_client::V2Client, v3_actor};

/// How often a requested delete is asked about while V2's workflow destroys the actor.
const DELETE_POLL: Duration = Duration::from_secs(2);
/// How long one actor's destroy may run before the run reports it as still deleting and moves on.
/// A later run asks about it again.
const DELETE_TIMEOUT: Duration = Duration::from_secs(10 * 60);

pub struct DeleteOptions {
	/// Actors deleted at once.
	pub concurrency: usize,
	/// The V2 datacenters to delete in, or every configured one.
	pub datacenter_labels: Option<Vec<u16>>,
}

#[derive(Debug, Default, Serialize)]
pub struct DatacenterSummary {
	pub datacenter_label: u16,
	/// Actors this run deleted.
	pub deleted: u64,
	/// Actors an earlier run deleted.
	pub already_deleted: u64,
	/// Actors not deleted, with why.
	pub skipped: Vec<SkippedActor>,
	/// The cursor V2 keeps after this run ended before an actor that is not deleted.
	pub cursor_held: bool,
}

#[derive(Debug, Serialize)]
pub struct SkippedActor {
	pub v2_actor_id: String,
	pub v3_actor_id: String,
	pub reason: String,
}

/// What became of one parked actor.
enum Deletion {
	Deleted,
	AlreadyDeleted,
	Skipped(String),
}

pub async fn delete_parked(
	ctx: &EngineContext,
	options: DeleteOptions,
) -> Result<Vec<DatacenterSummary>> {
	let migration = ctx
		.config()
		.dynamic()
		.migrate_from_v2()
		.cloned()
		.context("migrate_from_v2 is not configured")?;
	let v2 = V2Client::new(ctx.external_http(), &migration)?;
	let datacenter_labels = match options.datacenter_labels {
		Some(labels) => labels,
		None => v2.datacenter_labels(),
	};
	let concurrency = options.concurrency.max(1);
	let page_limit = usize::try_from(migration.parked_page_limit())
		.context("migrate_from_v2.parked_page_limit does not fit in usize")?;

	let mut summaries = Vec::new();
	for datacenter_label in datacenter_labels {
		let summary =
			delete_in_datacenter(ctx, &v2, datacenter_label, concurrency, page_limit).await?;
		summaries.push(summary);
	}
	Ok(summaries)
}

/// Deletes every parked actor of one datacenter, from the saved cursor to the end of the listing.
async fn delete_in_datacenter(
	ctx: &EngineContext,
	v2: &V2Client,
	datacenter_label: u16,
	concurrency: usize,
	page_limit: usize,
) -> Result<DatacenterSummary> {
	let mut summary = DatacenterSummary {
		datacenter_label,
		..Default::default()
	};
	let saved = v2.delete_cursor(datacenter_label).await?.cursor;
	let mut after = saved.as_ref().map(|cursor| cursor.to_vec());
	// The saved cursor moves only past a run of finished actors, so a resume never skips one.
	let mut cursor_advancing = true;

	loop {
		let page = v2
			.parked(datacenter_label, after.as_deref(), page_limit)
			.await?;
		let mut finished_through = None;

		// `buffered` yields in listing order, so the finished run is known as results arrive.
		let attempts = page.actors.into_iter().map(|actor| {
			let ctx = ctx.clone();
			let v2 = v2.clone();
			async move {
				let deletion = delete_one(&ctx, &v2, &actor).await;
				(actor, deletion)
			}
		});
		let mut deletions = stream::iter(attempts).buffered(concurrency);
		while let Some((actor, deletion)) = deletions.next().await {
			let v2_actor_id = Id::from_slice(&actor.actor_id)?;
			let v3_actor_id = Id::from_slice(&actor.v3_actor_id)?;
			let finished = match deletion {
				Ok(Deletion::Deleted) => {
					summary.deleted += 1;
					true
				}
				Ok(Deletion::AlreadyDeleted) => {
					summary.already_deleted += 1;
					true
				}
				Ok(Deletion::Skipped(reason)) => {
					summary.skipped.push(SkippedActor {
						v2_actor_id: v2_actor_id.to_string(),
						v3_actor_id: v3_actor_id.to_string(),
						reason,
					});
					false
				}
				Err(error) => {
					summary.skipped.push(SkippedActor {
						v2_actor_id: v2_actor_id.to_string(),
						v3_actor_id: v3_actor_id.to_string(),
						reason: format!("{error:#}"),
					});
					false
				}
			};
			if !finished {
				cursor_advancing = false;
			}
			if cursor_advancing {
				finished_through = Some(actor.cursor.clone());
			}
		}

		if let Some(cursor) = finished_through {
			v2.set_delete_cursor(
				datacenter_label,
				&proto::DeleteCursor {
					cursor: Some(cursor),
				},
			)
			.await?;
		}
		tracing::info!(
			datacenter_label,
			deleted = summary.deleted,
			already_deleted = summary.already_deleted,
			skipped = summary.skipped.len(),
			"deleted a page of parked V2 actors"
		);

		match page.next {
			Some(next) => after = Some(next.to_vec()),
			None => break,
		}
	}

	summary.cursor_held = !cursor_advancing;
	Ok(summary)
}

/// Deletes one parked actor's V2 copy, once it passes the checks, and waits for V2's workflow to
/// finish destroying it. A delete V2 already began is only waited on, since it cannot be undone.
async fn delete_one(
	ctx: &EngineContext,
	v2: &V2Client,
	actor: &proto::ParkedActor,
) -> Result<Deletion> {
	if actor.deleted_at.is_some() {
		return Ok(Deletion::AlreadyDeleted);
	}
	let v2_actor_id = Id::from_slice(&actor.actor_id)?;
	let v3_actor_id = Id::from_slice(&actor.v3_actor_id)?;

	if actor.delete_requested_at.is_none() {
		if let Some(reason) = refusal(ctx, v2, actor, v2_actor_id, v3_actor_id).await? {
			return Ok(Deletion::Skipped(reason));
		}
	}

	let request = proto::DeleteRequest {
		v3_actor_id: actor.v3_actor_id.clone(),
	};
	let clock = ctx.env().clock();
	let deadline_ms = clock
		.observe()?
		.boot_ms
		.saturating_add(DELETE_TIMEOUT.as_millis() as u64);
	loop {
		match v2.delete(v2_actor_id, &request).await? {
			proto::DeleteResponse::Deleted => return Ok(Deletion::Deleted),
			proto::DeleteResponse::NotParked => {
				return Ok(Deletion::Skipped(
					"V2 no longer holds the actor parked; it may have been recovered".into(),
				));
			}
			proto::DeleteResponse::Deleting => {}
		}
		if clock.observe()?.boot_ms >= deadline_ms {
			return Ok(Deletion::Skipped(
				"V2 is still destroying the actor; a later run checks it again".into(),
			));
		}
		rivet_env::tokio::time::sleep(clock.as_ref(), DELETE_POLL).await?;
	}
}

/// Why the actor's V2 copy must not be deleted yet, or `None` when it may be.
async fn refusal(
	ctx: &EngineContext,
	v2: &V2Client,
	actor: &proto::ParkedActor,
	v2_actor_id: Id,
	v3_actor_id: Id,
) -> Result<Option<String>> {
	if let Some(reason) = v3_refusal(ctx, v3_actor_id).await? {
		return Ok(Some(reason));
	}

	if let Some(store) = &ctx.object_stores().migrate_from_v2_backup {
		let complete = backup::is_complete(
			store.as_ref(),
			v2.namespace(),
			v2_actor_id,
			actor.generation,
		)
		.await?;
		if !complete {
			return Ok(Some(format!(
				"no complete backup of generation {} exists",
				actor.generation
			)));
		}
	}

	Ok(None)
}

/// Why the V3 actor does not yet own the data, or `None` when it does. An actor still held for
/// the migration or a recovery may yet go back to V2; one that is active, sleeping or destroyed
/// here no longer needs its V2 copy.
async fn v3_refusal(ctx: &EngineContext, v3_actor_id: Id) -> Result<Option<String>> {
	// Find the actor where it lives now. One migrated before its namespace had a volume group may
	// have been moved into the group by the namespace's backfill, under a new id when keyed.
	let target = v3_actor::target(ctx).await?;
	let mut v3_actor_id = v3_actor_id;
	if target.group_id().is_some()
		&& let Some(legacy) = v3_actor::read(ctx, None, v3_actor_id).await?
		&& let Some(moved_to) = legacy_moved_keyed_copy(&legacy, &target.name, v3_actor_id)
	{
		v3_actor_id = moved_to;
	}
	let group = pegboard::legacy_volumes::legacy_resolve_actor(ctx, &target, v3_actor_id)
		.await
		.context("resolve the V3 actor's placement")?
		.group;
	let read = v3_actor::read(ctx, group, v3_actor_id).await?;

	Ok(match read.as_ref().and_then(|read| read.phase.as_ref()) {
		None => Some(format!("V3 holds no actor {v3_actor_id}")),
		Some(pb::ActorPhase::MigratingFromV2(_)) => Some(format!(
			"the V3 actor {v3_actor_id} is still held for a migration or recovery"
		)),
		Some(pb::ActorPhase::DelegatedToV2) => Some(format!(
			"the V3 actor {v3_actor_id} is delegated back to V2"
		)),
		Some(
			pb::ActorPhase::Initializing(_)
			| pb::ActorPhase::Starting(_)
			| pb::ActorPhase::Running(_)
			| pb::ActorPhase::Stopping(_)
			| pb::ActorPhase::Sleeping(_)
			| pb::ActorPhase::Destroying(_)
			| pb::ActorPhase::Destroyed,
		) => None,
	})
}
