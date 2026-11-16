//! Hands a migrated actor back to V2, which serves it again exactly as it was at migration.
//!
//! 1. Hold the V3 actor under [`RECOVERY_MIGRATION_ID`], so requests get the retryable
//!    `actor.migrating_from_v2` and nothing starts it. A sleeping actor is recalled. An actor a
//!    migration still holds, such as one the migrator left as needing recovery, is taken over:
//!    no migrator id reaches the recovery id, so its begin wins.
//! 2. Call V2 `restore`, which writes the parked wake entries back and clears the cutover.
//! 3. Discard the hold: the V3 actor is destroyed, which deletes its database volume, so routing
//!    falls back to V2.
//!
//! A keyed actor the namespace's backfill moved into its volume group lives at a new id there, so
//! a recovery by its legacy id stops before any step and names that id: holding nothing at the
//! legacy id and restoring V2 would leave V2 and the moved copy both serving the key.
//!
//! Every step is idempotent, so a recovery that failed partway is completed by calling it again.
//! Writes the actor made in V3 after it migrated are not carried back. When V2 holds no parked
//! copy, the V3 actor is the only one and it is activated again instead of destroyed.

use anyhow::{Context, Result, bail};
use migrate_from_v2_protocol as proto;
use pegboard::wire as pb;
use rivet_engine_context::EngineContext;
use rivet_util::Id;

use crate::{migrator_machine::RECOVERY_MIGRATION_ID, v2_client::V2Client, v3_actor};

#[derive(Debug, serde::Serialize)]
pub struct Recovered {
	pub v2_actor_id: String,
	pub v3_actor_id: String,
	/// `restored`, or `not_parked` when V2 had already been restored.
	pub v2: &'static str,
}

pub async fn recover(
	ctx: &EngineContext,
	v2_actor_id: Id,
	v3_actor_id: Option<Id>,
) -> Result<Recovered> {
	let migration = ctx
		.config()
		.dynamic()
		.migrate_from_v2()
		.cloned()
		.context("migrate_from_v2 is not configured")?;
	let v2 = V2Client::new(ctx.external_http(), &migration)?;
	let v3_actor_id = v3_actor_id.unwrap_or(v2_actor_id);

	// Find the V3 actor's placement once. An actor migrated before its namespace had a volume
	// group lives at the legacy unprefixed placement until the backfill moves it, and every step
	// below addresses the actor where this resolve found it.
	let target = v3_actor::target(ctx).await?;
	// A keyed actor the backfill moved keeps a destroyed legacy record that names its move, and
	// lives at a new id in the group. Nothing is held at the legacy id, so recovering there would
	// restore V2 beside the live copy; stop before any step and name the copy instead.
	if target.group_id().is_some()
		&& let Some(legacy) = v3_actor::read(ctx, None, v3_actor_id).await?
		&& let Some(moved_to) = legacy_moved_keyed_copy(&legacy, &target.name, v3_actor_id)
	{
		bail!(
			"the namespace's backfill moved the V3 actor {v3_actor_id} to {moved_to} in its volume group; nothing was recovered and V2 was not restored; recover it by that id with --v3-actor-id {moved_to}"
		);
	}
	let group = pegboard::legacy_volumes::legacy_resolve_actor(ctx, &target, v3_actor_id)
		.await
		.context("resolve the V3 actor's placement")?
		.group;

	// Hold the V3 actor first, so V2 and V3 never serve it at once.
	let read = v3_actor::read(ctx, group, v3_actor_id).await?;
	let phase = read.as_ref().and_then(|read| read.phase.as_ref());
	let recalled_now = match phase {
		Some(pb::ActorPhase::MigratingFromV2(held))
			if held.migration_id == RECOVERY_MIGRATION_ID =>
		{
			false
		}
		// A held actor never served in V3, so V2's copy is the authoritative one and taking the
		// hold over loses nothing. The data record names the incarnation the begin keeps.
		Some(pb::ActorPhase::MigratingFromV2(held)) => {
			let data = read
				.as_ref()
				.and_then(|read| read.data.clone())
				.context("a held V3 actor has no data record")?;
			tracing::info!(%v3_actor_id, migration_id = held.migration_id, "taking over a migration's hold to recover the actor");
			let begun = v3_actor::begin(
				ctx,
				group,
				v3_actor_id,
				RECOVERY_MIGRATION_ID,
				pb::ActorCreateData {
					actor_id: data.actor_id,
					pool_name: data.pool_name,
					name: data.name,
					key: data.key,
					input: None,
					legacy_namespace: data.legacy_namespace,
					fork_from: None,
					legacy_move: None,
				},
				data.create_unix_ms,
			)
			.await?;
			if !matches!(begun, v3_actor::Begun::Held { .. }) {
				bail!(
					"the V3 actor {v3_actor_id} left its hold during recovery; retry the recovery"
				);
			}
			false
		}
		Some(pb::ActorPhase::Sleeping(_)) => {
			v3_actor::send(
				ctx,
				group,
				v3_actor_id,
				pb::MigrateFromV2::MigrateFromV2Recall(pb::MigrateFromV2Recall {
					migration_id: RECOVERY_MIGRATION_ID,
				}),
			)
			.await?;
			let recalled = v3_actor::read(ctx, group, v3_actor_id).await?;
			if !recalled
				.as_ref()
				.is_some_and(|read| v3_actor::held_as(read, RECOVERY_MIGRATION_ID))
			{
				bail!("the V3 actor {v3_actor_id} woke during recovery; retry once it sleeps");
			}
			true
		}
		// A key delegated to V2 has no actor here to hold.
		Some(
			pb::ActorPhase::Destroying(_)
			| pb::ActorPhase::Destroyed
			| pb::ActorPhase::DelegatedToV2,
		)
		| None => false,
		Some(
			pb::ActorPhase::Initializing(_)
			| pb::ActorPhase::Starting(_)
			| pb::ActorPhase::Running(_)
			| pb::ActorPhase::Stopping(_),
		) => bail!("the V3 actor {v3_actor_id} is not sleeping; let it sleep before recovering it"),
	};
	let held = recalled_now || matches!(phase, Some(pb::ActorPhase::MigratingFromV2(_)));

	let restored = v2
		.restore(
			v2_actor_id,
			&proto::RestoreRequest {
				cutover_ts: None,
				v3_actor_id: Some(v3_actor_id.as_bytes().into()),
			},
		)
		.await
		.context("restore the actor in V2")?;

	// V2 holds no parked copy of this actor, so V3's is the only one: hand it back to V3 instead
	// of destroying it. A recovery that already restored V2 and stopped before the discard finds
	// the hold here and finishes the discard below.
	if let (proto::RestoreResponse::NotParked, true) = (&restored, recalled_now) {
		v3_actor::activate(ctx, group, v3_actor_id, RECOVERY_MIGRATION_ID, None).await?;
		bail!("V2 has no parked copy of {v2_actor_id}; the V3 actor {v3_actor_id} stays in V3");
	}

	if held {
		match v3_actor::discard(ctx, group, v3_actor_id, RECOVERY_MIGRATION_ID).await? {
			v3_actor::Settled::Applied | v3_actor::Settled::Elsewhere => {}
			v3_actor::Settled::StillHeld => {
				bail!("the V3 actor {v3_actor_id} is still held; retry the recovery")
			}
		}
	}

	let v2_result = match restored {
		proto::RestoreResponse::Restored => "restored",
		proto::RestoreResponse::NotParked => "not_parked",
	};
	tracing::info!(%v2_actor_id, %v3_actor_id, v2 = v2_result, "recovered a migrated actor to V2");
	Ok(Recovered {
		v2_actor_id: v2_actor_id.to_string(),
		v3_actor_id: v3_actor_id.to_string(),
		v2: v2_result,
	})
}

/// Legacy: the grouped id the backfill moved a keyed actor to, read from `read`, the legacy
/// unprefixed record of `legacy_id`: a record of `namespace` with a key that the move destroyed
/// (`LegacyMoveMarkMoved`, kept through `Destroying` and `Destroyed`). The backfill creates the
/// moved copy at the id the key takes in the group, which hashes the name and key alone. An
/// unkeyed actor keeps its id, so the resolve finds its copy and this answers `None`, as for any
/// other record. Removed in 3.1 with the backfill.
pub fn legacy_moved_keyed_copy(
	read: &pb::ActorReadResponse,
	namespace: &str,
	legacy_id: Id,
) -> Option<Id> {
	let data = read.data.as_ref()?;
	if data.legacy_namespace.as_deref() != Some(namespace) {
		return None;
	}
	let key = data.key.as_deref()?;
	let moved = matches!(
		read.legacy_move,
		Some(pb::LegacyMoveMark::LegacyMoveMarkMoved(_))
	);
	let destroyed = matches!(
		read.phase,
		Some(pb::ActorPhase::Destroying(_) | pb::ActorPhase::Destroyed)
	);
	if !(moved && destroyed) {
		return None;
	}
	Some(pegboard::actor_key::keyed_actor_id(
		legacy_id.label(),
		&data.name,
		key,
	))
}
