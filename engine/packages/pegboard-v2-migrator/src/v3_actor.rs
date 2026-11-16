//! The migrator's calls on a V3 actor's state machine: begin, activate, discard and recall a
//! hold, and read the actor back to learn what applied.
//!
//! Every call addresses the actor at one placement: the volume group its in-flight entry was
//! admitted for, or `None` for the legacy unprefixed placement. The caller passes the placement it
//! persisted, never the namespace's current standing, because an entry begun before its namespace
//! gained a group keeps its hold at the legacy placement until the backfill moves the actor.
//!
//! Every call is idempotent by migration id: the actor state machine applies an event only while
//! the actor is in the phase and hold the event names, so a retry after a lost reply changes
//! nothing and the read that follows answers as for the first attempt.

use anyhow::{Context, Result};
use pegboard::{
	commands::{ActorEvent, ActorMigrateFromV2Begin, ActorRead, is_volume_gone},
	keys::partition::ActorVolume,
	wire as pb,
};
use rivet_engine_context::EngineContext;
use rivet_keys::keys::codec::GroupId;
use rivet_util::Id;
use std::sync::Arc;

use crate::migrator_machine::RECOVERY_MIGRATION_ID;

/// How a begin left the V3 actor.
pub enum Begun {
	/// The actor is held by this migration. Its incarnation's database volume is addressed by
	/// `create_unix_ms`.
	Held { create_unix_ms: i64 },
	/// The id holds an actor outside any migration: a key that exists natively in V3.
	NativeActor,
	/// Another migration holds it, or it is being destroyed. Nothing was written.
	Elsewhere,
}

/// Holds the V3 actor `v3_actor_id` at `group` for `migration_id`, creating its state when it has
/// none.
pub async fn begin(
	ctx: &EngineContext,
	group: Option<GroupId>,
	v3_actor_id: Id,
	migration_id: u64,
	init: pb::ActorCreateData,
	create_unix_ms: i64,
) -> Result<Begun> {
	let response = ctx
		.depot()
		.command::<ActorMigrateFromV2Begin>(
			&ActorVolume::in_namespace(group, v3_actor_id),
			pb::MigrateFromV2Begin {
				migration_id,
				init,
				create_unix_ms,
			},
		)
		.await?;
	Ok(match response.phase {
		pb::ActorPhase::MigratingFromV2(held) if held.migration_id == migration_id => Begun::Held {
			create_unix_ms: response.create_unix_ms,
		},
		// A begin takes a delegated key over, so it never answers `DelegatedToV2`.
		pb::ActorPhase::MigratingFromV2(_)
		| pb::ActorPhase::Destroying(_)
		| pb::ActorPhase::Destroyed
		| pb::ActorPhase::DelegatedToV2 => Begun::Elsewhere,
		pb::ActorPhase::Initializing(_)
		| pb::ActorPhase::Starting(_)
		| pb::ActorPhase::Running(_)
		| pb::ActorPhase::Stopping(_)
		| pb::ActorPhase::Sleeping(_) => Begun::NativeActor,
	})
}

/// What a read of the V3 actor at `group` found. `None` is an actor with no state volume there.
pub async fn read(
	ctx: &EngineContext,
	group: Option<GroupId>,
	v3_actor_id: Id,
) -> Result<Option<pb::ActorReadResponse>> {
	match ctx
		.depot()
		.command::<ActorRead>(&ActorVolume::in_namespace(group, v3_actor_id), ())
		.await
	{
		Ok(read) => Ok(Some(read)),
		Err(error) if is_volume_gone(&error) => Ok(None),
		Err(error) => Err(error),
	}
}

/// Sends one migration event to the actor at `group`. Returns false when the actor has no state
/// volume there.
pub async fn send(
	ctx: &EngineContext,
	group: Option<GroupId>,
	v3_actor_id: Id,
	event: pb::MigrateFromV2,
) -> Result<bool> {
	let sent = ctx
		.depot()
		.command::<ActorEvent>(
			&ActorVolume::in_namespace(group, v3_actor_id),
			pb::ActorEvent::MigrateFromV2(event),
		)
		.await;
	match sent {
		Ok(_) => Ok(true),
		Err(error) if is_volume_gone(&error) => Ok(false),
		Err(error) => Err(error),
	}
}

/// Whether the actor is held under exactly `migration_id`.
pub fn held_as(read: &pb::ActorReadResponse, migration_id: u64) -> bool {
	matches!(
		&read.phase,
		Some(pb::ActorPhase::MigratingFromV2(held)) if held.migration_id == migration_id
	)
}

/// The key reservation of an actor held for a migration.
pub enum Reservation {
	/// Reserved, or the actor has no key to reserve.
	Reserved,
	/// The actor state machine is still reserving it.
	Pending,
	/// Another datacenter owns the key.
	OwnedElsewhere,
	/// The reservation ran out of attempts and never settles.
	Exhausted,
	/// The actor is not held by `migration_id`.
	NotHeld,
}

/// Reads the key reservation of the actor held under `migration_id`. Activation is refused until
/// it is reserved.
pub async fn reservation(
	ctx: &EngineContext,
	group: Option<GroupId>,
	v3_actor_id: Id,
	migration_id: u64,
) -> Result<Reservation> {
	let read = read(ctx, group, v3_actor_id).await?;
	let Some(pb::ActorPhase::MigratingFromV2(held)) = read.and_then(|read| read.phase) else {
		return Ok(Reservation::NotHeld);
	};
	if held.migration_id != migration_id {
		return Ok(Reservation::NotHeld);
	}
	let Some(reserve_epoxy) = held.reserve_epoxy else {
		return Ok(Reservation::Reserved);
	};
	Ok(match reserve_epoxy.state.phase {
		pb::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved => Reservation::Reserved,
		// A failed attempt is retried by the actor state machine.
		pb::ReserveEpoxyPhase::ReserveEpoxyPhasePending
		| pb::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed => Reservation::Pending,
		// A hold's reservation never asks V2, so it never finds the key there.
		pb::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(_)
		| pb::ReserveEpoxyPhase::ReserveEpoxyPhaseInV2 => Reservation::OwnedElsewhere,
		pb::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached => Reservation::Exhausted,
	})
}

/// How an activation left the actor.
#[derive(Debug, PartialEq, Eq)]
pub enum Activation {
	/// The activation applied, now or by an earlier attempt. `legacy_namespace` is the namespace
	/// the actor's record names, which an actor at the legacy placement always has.
	Applied { legacy_namespace: Option<String> },
	/// The activation applied at the legacy placement, but the actor's legacy index entry is not
	/// written yet: the actor state machine writes it after the activation, as a retried effect.
	/// The step is read again until it is, because the namespace's backfill lists the actors it
	/// moves from those entries and must not start while one is missing.
	IndexPending,
	/// The actor is still held under this migration, so the step did not apply. Retry it.
	StillHeld,
	/// Another migration holds the actor.
	Elsewhere,
}

/// How a discard left the actor.
#[derive(Debug)]
pub enum Settled {
	/// The step applied, now or by an earlier attempt.
	Applied,
	/// The actor is still held under this migration, so the step did not apply. Retry it.
	StillHeld,
	/// Another migration holds the actor.
	Elsewhere,
}

/// Activates the actor held under `migration_id` at `group` with V2's alarm. The hold ends only
/// through an activation or a discard, and after V2 confirmed nothing discards it, so an actor that
/// is no longer held was activated by an earlier attempt; it may since have run, slept, been
/// recalled by a recovery or been destroyed.
///
/// The actor is addressed where its entry placed it, so it must be there: an actor with no state
/// at that placement after the activation was sent is a failure, never an activation that applied.
pub async fn activate(
	ctx: &EngineContext,
	group: Option<GroupId>,
	v3_actor_id: Id,
	migration_id: u64,
	alarm_unix_ms: Option<i64>,
) -> Result<Activation> {
	let sent = send(
		ctx,
		group,
		v3_actor_id,
		pb::MigrateFromV2::MigrateFromV2Activate(pb::MigrateFromV2Activate {
			migration_id,
			alarm_unix_ms,
		}),
	)
	.await?;
	if !sent {
		return Err(hold_vanished(group, v3_actor_id, "activation"));
	}
	let read = read(ctx, group, v3_actor_id).await?;
	activation_settled(read.as_ref(), group, v3_actor_id, migration_id)
}

/// What the read that follows an activation of `migration_id` says. `Applied` needs a record at
/// the addressed placement that is no longer held by this migration; a missing volume or a
/// volume with no actor in it means the hold vanished, which is a [`HoldVanished`] error the
/// caller retries a bounded number of times, since claiming the activation applied would record
/// a cutover that never happened.
///
/// At the legacy placement an activated actor that still lives also needs its legacy index entry
/// written, or the read answers `IndexPending`. The namespace's backfill starts only once the
/// migrator holds no legacy-placed entry, and its walk finds actors only through those entries,
/// so an entry that finished before the index write landed could let the walk pass the actor's
/// bucket and finish without moving it. A read with no index write phase is waited out the same
/// way: every read is sealed at the fleet's read floor, so the field is missing while any storage
/// worker reads only `pegboard.actor` v3 or older (or while the floor is unknown), and legacy
/// activations then wait until every storage worker reads v4, which only holds the entry longer.
/// A recovery's hold, a destroy and a delegation to V2 need no entry here:
/// the recovery owns the actor, and the others leave nothing to move. A grouped actor is listed
/// through its group, which the backfill does not walk, so it never waits.
pub fn activation_settled(
	read: Option<&pb::ActorReadResponse>,
	group: Option<GroupId>,
	v3_actor_id: Id,
	migration_id: u64,
) -> Result<Activation> {
	let Some((read, phase)) = read.and_then(|read| read.phase.as_ref().map(|phase| (read, phase)))
	else {
		return Err(hold_vanished(group, v3_actor_id, "activation"));
	};
	let lives = match phase {
		pb::ActorPhase::MigratingFromV2(held) if held.migration_id == migration_id => {
			return Ok(Activation::StillHeld);
		}
		pb::ActorPhase::MigratingFromV2(held) if held.migration_id != RECOVERY_MIGRATION_ID => {
			return Ok(Activation::Elsewhere);
		}
		pb::ActorPhase::MigratingFromV2(_)
		| pb::ActorPhase::Destroying(_)
		| pb::ActorPhase::Destroyed
		| pb::ActorPhase::DelegatedToV2 => false,
		pb::ActorPhase::Initializing(_)
		| pb::ActorPhase::Starting(_)
		| pb::ActorPhase::Running(_)
		| pb::ActorPhase::Stopping(_)
		| pb::ActorPhase::Sleeping(_) => true,
	};

	let index_written = matches!(
		read.legacy_write_index_phase,
		Some(pb::WriteIndexPhase::WriteIndexPhaseSuccess)
	);
	if group.is_none() && lives && !index_written {
		return Ok(Activation::IndexPending);
	}

	let legacy_namespace = read
		.data
		.as_ref()
		.and_then(|data| data.legacy_namespace.as_ref())
		.map(|namespace| namespace.to_string());
	Ok(Activation::Applied { legacy_namespace })
}

/// Discards the hold under `migration_id` at `group`: the actor is destroyed, which deletes the
/// imported database volume. An actor held elsewhere, or not held at all, is left alone.
///
/// The actor is read first, because a begin the storage host refused can leave a state volume
/// with no actor in it, and an event sent to that volume fails instead of finding nothing held;
/// an actor not held there, or not there at all, was never held at this placement, so there is
/// nothing to discard. Once the discard is sent, the actor must still be there.
pub async fn discard(
	ctx: &EngineContext,
	group: Option<GroupId>,
	v3_actor_id: Id,
	migration_id: u64,
) -> Result<Settled> {
	match read(ctx, group, v3_actor_id).await? {
		Some(read) if held_as(&read, migration_id) => {}
		Some(_) | None => return Ok(Settled::Applied),
	}

	let sent = send(
		ctx,
		group,
		v3_actor_id,
		pb::MigrateFromV2::MigrateFromV2Discard(pb::MigrateFromV2Discard { migration_id }),
	)
	.await?;
	if !sent {
		return Err(hold_vanished(group, v3_actor_id, "discard"));
	}
	let read = read(ctx, group, v3_actor_id).await?;
	discard_settled(read.as_ref(), group, v3_actor_id, migration_id)
}

/// What the read that follows a sent discard of `migration_id` says: still held, or applied by a
/// record that is no longer held by it. A missing record means the hold vanished, which is an
/// error, as for [`activation_settled`].
pub fn discard_settled(
	read: Option<&pb::ActorReadResponse>,
	group: Option<GroupId>,
	v3_actor_id: Id,
	migration_id: u64,
) -> Result<Settled> {
	let Some(read) = read.filter(|read| read.phase.is_some()) else {
		return Err(hold_vanished(group, v3_actor_id, "discard"));
	};
	Ok(match held_as(read, migration_id) {
		true => Settled::StillHeld,
		false => Settled::Applied,
	})
}

/// A hold that is gone from the placement its entry names after a step was sent. It means
/// something other than the migrator removed the actor's state, such as a delete of the target
/// namespace's volumes, or the entry names the wrong placement; either way no step applied there.
/// The migrator retries it a bounded number of times, then leaves the actor for an operator.
#[derive(Debug)]
pub struct HoldVanished {
	message: String,
}

impl std::fmt::Display for HoldVanished {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.message)
	}
}

impl std::error::Error for HoldVanished {}

/// Whether `error` is a [`HoldVanished`].
pub fn is_hold_vanished(error: &anyhow::Error) -> bool {
	error.downcast_ref::<HoldVanished>().is_some()
}

/// The [`HoldVanished`] error of the actor at `group` after `step` was sent.
fn hold_vanished(group: Option<GroupId>, v3_actor_id: Id, step: &str) -> anyhow::Error {
	let placement = match group {
		Some(_) => "its volume group",
		None => "the legacy unprefixed placement",
	};
	tracing::error!(
		%v3_actor_id,
		grouped = group.is_some(),
		step,
		"the held V3 actor has no state at the placement its migration entry names; the step did not apply"
	);
	anyhow::Error::new(HoldVanished {
		message: format!(
			"the V3 actor {v3_actor_id} has no state at {placement} after the {step} was sent; its hold vanished"
		),
	})
}

/// The target namespace's standing. A migration placed by it lands as a fresh create in that
/// namespace would: in the namespace's group when it has one, ungrouped otherwise. Only an
/// admission and the copy's placement check read it; every later call addresses the placement the
/// entry persisted.
pub async fn target(ctx: &EngineContext) -> Result<Arc<pegboard::NamespaceStanding>> {
	let dynamic = ctx.config().dynamic();
	let migration = dynamic
		.migrate_from_v2()
		.context("migrate_from_v2 is not configured, so the migrator has no target namespace")?;
	pegboard::catalog::namespace_standing(ctx, migration.target_namespace()).await
}
