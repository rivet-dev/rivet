//! The migrator's calls on a V3 actor's state machine: begin, activate, discard and recall a
//! hold, and read the actor back to learn what applied.
//!
//! Every call is idempotent by migration id: the actor state machine applies an event only while
//! the actor is in the phase and hold the event names, so a retry after a lost reply changes
//! nothing and the read that follows answers as for the first attempt.

use anyhow::Result;
use pegboard::{
	commands::{ActorEvent, ActorMigrateFromV2Begin, ActorRead, is_volume_gone},
	keys::partition::ActorVolume,
	wire as pb,
};
use rivet_engine_context::EngineContext;
use rivet_util::Id;

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

/// Holds the V3 actor `v3_actor_id` for `migration_id`, creating its state when it has none.
pub async fn begin(
	ctx: &EngineContext,
	v3_actor_id: Id,
	migration_id: u64,
	init: pb::ActorCreateData,
	create_unix_ms: i64,
) -> Result<Begun> {
	let response = ctx
		.depot()
		.command::<ActorMigrateFromV2Begin>(
			&ActorVolume::new(v3_actor_id).volume_key(),
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

/// What a read of the V3 actor found. `None` is an actor with no state volume.
pub async fn read(ctx: &EngineContext, v3_actor_id: Id) -> Result<Option<pb::ActorReadResponse>> {
	match ctx
		.depot()
		.command::<ActorRead>(&ActorVolume::new(v3_actor_id).volume_key(), ())
		.await
	{
		Ok(read) => Ok(Some(read)),
		Err(error) if is_volume_gone(&error) => Ok(None),
		Err(error) => Err(error),
	}
}

/// Sends one migration event. Returns false when the actor has no state volume.
pub async fn send(ctx: &EngineContext, v3_actor_id: Id, event: pb::MigrateFromV2) -> Result<bool> {
	let sent = ctx
		.depot()
		.command::<ActorEvent>(
			&ActorVolume::new(v3_actor_id).volume_key(),
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
	v3_actor_id: Id,
	migration_id: u64,
) -> Result<Reservation> {
	let read = read(ctx, v3_actor_id).await?;
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

/// How an activation or discard left the actor.
pub enum Settled {
	/// The step applied, now or by an earlier attempt.
	Applied,
	/// The actor is still held under this migration, so the step did not apply. Retry it.
	StillHeld,
	/// Another migration holds the actor.
	Elsewhere,
}

/// Activates the actor held under `migration_id` with V2's alarm. The hold ends only through an
/// activation or a discard, and after V2 confirmed nothing discards it, so an actor that is no
/// longer held was activated by an earlier attempt; it may since have run, slept, been recalled
/// by a recovery or been destroyed.
pub async fn activate(
	ctx: &EngineContext,
	v3_actor_id: Id,
	migration_id: u64,
	alarm_unix_ms: Option<i64>,
) -> Result<Settled> {
	send(
		ctx,
		v3_actor_id,
		pb::MigrateFromV2::MigrateFromV2Activate(pb::MigrateFromV2Activate {
			migration_id,
			alarm_unix_ms,
		}),
	)
	.await?;
	let read = read(ctx, v3_actor_id).await?;
	Ok(match read.as_ref().and_then(|read| read.phase.as_ref()) {
		Some(pb::ActorPhase::MigratingFromV2(held)) if held.migration_id == migration_id => {
			Settled::StillHeld
		}
		Some(pb::ActorPhase::MigratingFromV2(held))
			if held.migration_id != RECOVERY_MIGRATION_ID =>
		{
			Settled::Elsewhere
		}
		Some(_) | None => Settled::Applied,
	})
}

/// Discards the hold under `migration_id`: the actor is destroyed, which deletes the imported
/// database volume. An actor held elsewhere, or not held at all, is left alone.
///
/// The actor is read first, because a begin the storage host refused can leave a state volume
/// with no actor in it, and an event sent to that volume fails instead of finding nothing held.
pub async fn discard(ctx: &EngineContext, v3_actor_id: Id, migration_id: u64) -> Result<Settled> {
	match read(ctx, v3_actor_id).await? {
		Some(read) if held_as(&read, migration_id) => {}
		Some(_) | None => return Ok(Settled::Applied),
	}

	let sent = send(
		ctx,
		v3_actor_id,
		pb::MigrateFromV2::MigrateFromV2Discard(pb::MigrateFromV2Discard { migration_id }),
	)
	.await?;
	if !sent {
		return Ok(Settled::Applied);
	}
	let read = read(ctx, v3_actor_id).await?;
	Ok(match read {
		Some(read) if held_as(&read, migration_id) => Settled::StillHeld,
		Some(_) | None => Settled::Applied,
	})
}
