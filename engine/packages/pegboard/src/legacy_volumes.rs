//! Legacy: the fallback from a grouped actor volume to its unprefixed volume. Volumes created
//! before namespaces had groups live at unprefixed keys; new behavior keys every volume under its
//! namespace's group. An open in a namespace whose backfill is not done tries the grouped volume
//! first and, when it does not exist, the unprefixed one. Removed in 3.1 with the backfill.

use crate::{
	NamespaceStanding,
	commands::ActorRead,
	keys::partition::{ActorVolume, LegacyActorVolume},
	wire,
};
use anyhow::Result;
use rivet_engine_context::EngineContext;
use rivet_keys::keys::codec::GroupId;
use rivet_util::Id;

/// Where a resolve found an actor's volumes, with the state volume read it made there, so the
/// caller does not read the record again.
#[derive(Clone, Debug)]
pub struct ResolvedActor {
	/// The group the actor's volumes are keyed in, `None` for the unprefixed shape.
	pub group: Option<GroupId>,
	/// What the resolve learned about the state volume in `group`.
	pub read: ResolvedRead,
}

/// What a resolve learned about the actor's state volume in the group it answered.
#[derive(Clone, Debug)]
pub enum ResolvedRead {
	/// Nothing: the namespace's backfill is done, so the resolve answered its group with no read.
	Unread,
	/// The state volume in the answered group is missing or deleted, so the actor reads as not
	/// found there.
	Gone,
	/// The state volume's read in the answered group.
	Found(wire::ActorReadResponse),
}

impl ResolvedRead {
	/// The state volume read of `actor_id` in `group`, the group it was resolved in: the
	/// resolve's own read when it made one, otherwise one read now. `None` when the volume is
	/// missing or deleted.
	pub async fn into_read(
		self,
		depot: &depot_client::Client,
		group: Option<GroupId>,
		actor_id: Id,
	) -> Result<Option<wire::ActorReadResponse>> {
		match self {
			ResolvedRead::Found(read) => Ok(Some(read)),
			ResolvedRead::Gone => Ok(None),
			ResolvedRead::Unread => {
				let volume = ActorVolume::in_namespace(group, actor_id);
				match depot.command::<ActorRead>(&volume, ()).await {
					Ok(read) => Ok(Some(read)),
					Err(error) if crate::commands::is_volume_gone(&error) => Ok(None),
					Err(error) => Err(error),
				}
			}
		}
	}
}

/// Legacy: where the volumes of `actor_id` in `namespace` are keyed: the namespace's group, or
/// `None` for an actor created before namespaces had groups, whose state and database volumes
/// are still unprefixed. Taken only while the namespace's backfill is not done, or after it for
/// an actor it lists as skipped ([`NamespaceStanding::legacy_fallback`]); otherwise it answers
/// the namespace's group with no read.
///
/// **Temporary hot-path cost:** while the backfill is not done, the resolve reads the grouped
/// state volume, and for an actor not yet moved the unprefixed one too. The read it answers with
/// is returned, so a caller that needs the record reuses it: a grouped actor then costs no extra
/// read, and an actor not yet moved one. The backfill bounds how long this lasts.
///
/// The unprefixed key space is shared by every namespace from before groups, so a record found
/// there counts only when it names this namespace; a foreign or missing one answers the
/// namespace's group, where the actor reads as not found. An actor found there has its name
/// recorded for the backfill.
pub async fn legacy_resolve_actor(
	ctx: &EngineContext,
	namespace: &NamespaceStanding,
	actor_id: Id,
) -> Result<ResolvedActor> {
	let fallback = namespace.legacy_fallback();
	let found = legacy_resolve_actor_in(
		&ctx.depot(),
		namespace.group_id(),
		fallback.namespace_for(actor_id),
		actor_id,
	)
	.await?;
	// A finished backfill walks no more names.
	if !namespace.backfill_done() {
		legacy_record_found_name(ctx, &namespace.name, &found).await;
	}
	Ok(found)
}

/// Legacy: records the actor name of `found` for the backfill when the resolve found the actor
/// at its unprefixed volume, so the backfill walks that name even when no envoy record names it.
/// Removed in 3.1 with the backfill.
pub async fn legacy_record_found_name(ctx: &EngineContext, namespace: &str, found: &ResolvedActor) {
	if found.group.is_some() {
		return;
	}
	let ResolvedRead::Found(read) = &found.read else {
		return;
	};
	let Some(data) = &read.data else {
		return;
	};
	crate::legacy_backfill::legacy_record_actor_name(ctx, namespace, &data.name).await;
}

/// Legacy: [`legacy_resolve_actor`] from a namespace's parts as a connection resolved them: its
/// group, and its name when it falls back ([`NamespaceStanding::legacy_fallback`]), `None`
/// otherwise. Removed in 3.1 with the backfill.
pub async fn legacy_resolve_actor_in(
	depot: &depot_client::Client,
	group: Option<GroupId>,
	fallback_namespace: Option<&str>,
	actor_id: Id,
) -> Result<ResolvedActor> {
	let Some(namespace) = fallback_namespace else {
		return Ok(ResolvedActor {
			group,
			read: ResolvedRead::Unread,
		});
	};

	let grouped = ActorVolume::in_namespace(group, actor_id);
	match legacy_grouped_read(depot.command::<ActorRead>(&grouped, ()).await, actor_id)? {
		GroupedRead::MovedAway(copy) => {
			let legacy = legacy_unprefixed_actor_read(depot, namespace, actor_id).await?;
			// The metric counts the shape answered: the legacy actor, or the copy in the group.
			return Ok(match legacy_settle_moved_away(copy, legacy) {
				MovedAwayRead::Legacy(read) => {
					record_legacy_read(LegacyReadResult::Legacy);
					ResolvedActor {
						group: None,
						read: ResolvedRead::Found(read),
					}
				}
				MovedAwayRead::Copy(read) => {
					record_legacy_read(LegacyReadResult::Grouped);
					ResolvedActor {
						group,
						read: ResolvedRead::Found(read),
					}
				}
			});
		}
		GroupedRead::Found(read) => {
			record_legacy_read(LegacyReadResult::Grouped);
			return Ok(ResolvedActor {
				group,
				read: ResolvedRead::Found(read),
			});
		}
		GroupedRead::Gone => {
			record_legacy_read(LegacyReadResult::NotFound);
			return Ok(ResolvedActor {
				group,
				read: ResolvedRead::Gone,
			});
		}
		GroupedRead::Missing => {}
	}

	legacy_resolve_ungrouped_actor(depot, group, namespace, actor_id).await
}

/// Legacy: the rest of [`legacy_resolve_actor_in`] once the grouped state volume of `actor_id`
/// in `group` is known never to have existed, for a caller whose own command there found it
/// missing: reads the unprefixed state volume alone, and answers it when its record names
/// `namespace`, the grouped shape otherwise. Taken only while the namespace's backfill is not
/// done. A destroyed record is answered as found, so a lookup by id reads it as destroyed; unlike
/// [`crate::actor_key::legacy_resolve_keyed_actor`], which skips it so a create of the key goes to
/// the new id. Removed in 3.1 with the backfill.
pub async fn legacy_resolve_ungrouped_actor(
	depot: &depot_client::Client,
	group: Option<GroupId>,
	namespace: &str,
	actor_id: Id,
) -> Result<ResolvedActor> {
	match legacy_unprefixed_actor_read(depot, namespace, actor_id).await? {
		Some(read) => {
			record_legacy_read(LegacyReadResult::Legacy);
			Ok(ResolvedActor {
				group: None,
				read: ResolvedRead::Found(read),
			})
		}
		None => {
			record_legacy_read(LegacyReadResult::NotFound);
			Ok(ResolvedActor {
				group,
				read: ResolvedRead::Gone,
			})
		}
	}
}

/// Legacy: reads the unprefixed state volume of `actor_id` and judges it for `namespace`
/// ([`legacy_unprefixed_read`]), counting nothing. Removed in 3.1 with the backfill.
async fn legacy_unprefixed_actor_read(
	depot: &depot_client::Client,
	namespace: &str,
	actor_id: Id,
) -> Result<Option<wire::ActorReadResponse>> {
	let legacy = LegacyActorVolume::new(actor_id).volume_key();
	legacy_unprefixed_read(namespace, depot.command::<ActorRead>(&legacy, ()).await)
}

/// Legacy: the group of [`legacy_resolve_actor_in`], for a caller that does not read the record.
/// Removed in 3.1 with the backfill.
pub async fn legacy_resolve_actor_volume_in(
	depot: &depot_client::Client,
	group: Option<GroupId>,
	fallback_namespace: Option<&str>,
	actor_id: Id,
) -> Result<Option<GroupId>> {
	Ok(
		legacy_resolve_actor_in(depot, group, fallback_namespace, actor_id)
			.await?
			.group,
	)
}

/// Legacy: what a fallback's read of the grouped state volume decides. Removed in 3.1 with the
/// backfill.
#[derive(Debug)]
pub(crate) enum GroupedRead {
	/// The grouped volume exists; the actor is there.
	Found(wire::ActorReadResponse),
	/// The grouped volume holds a destroying or destroyed copy the backfill made of the legacy
	/// actor read for, which a failed move or a give-up abandoned ([`legacy_abandoned_copy`]):
	/// the legacy actor may still be the actor, so
	/// the unprefixed volume is read, and the copy is answered only when the legacy actor is
	/// destroyed too (see [`legacy_settle_moved_away`]).
	MovedAway(wire::ActorReadResponse),
	/// The grouped volume was deleted: a grouped actor existed and was destroyed, so the actor is
	/// not found there and the unprefixed volume is not read.
	Gone,
	/// The grouped volume never existed, so the unprefixed one is read.
	Missing,
}

/// Legacy: judges a fallback's read of the grouped state volume for the legacy actor
/// `legacy_id`. A volume that never existed falls through to the unprefixed read, and so does an
/// abandoned copy the backfill made of `legacy_id`, since every abandoned move leaves one; a
/// deleted volume answers the grouped shape, where the actor reads as not found instead of
/// failing. The legacy behavior answered any existing grouped record. Removed in 3.1 with the
/// backfill.
pub(crate) fn legacy_grouped_read(
	result: Result<wire::ActorReadResponse>,
	legacy_id: Id,
) -> Result<GroupedRead> {
	match result {
		Ok(read) if legacy_moved_away(&read, legacy_id) => Ok(GroupedRead::MovedAway(read)),
		Ok(read) => Ok(GroupedRead::Found(read)),
		Err(error) if depot_client::groups::is_volume_not_found(&error) => Ok(GroupedRead::Missing),
		Err(error) if crate::commands::is_volume_gone(&error) => Ok(GroupedRead::Gone),
		Err(error) => Err(error),
	}
}

/// Legacy: whether `read` is a copy the backfill made of the legacy actor `legacy_id` and then
/// abandoned ([`legacy_abandoned_copy`]). Removed in 3.1 with the backfill.
pub fn legacy_moved_away(read: &wire::ActorReadResponse, legacy_id: Id) -> bool {
	let (Some(phase), Some(data)) = (&read.phase, &read.data) else {
		return false;
	};
	legacy_abandoned_copy(phase, &read.legacy_move, data, legacy_id)
}

/// Legacy: whether the actor whose state has `phase` and `legacy_move` and whose record is `data`
/// is a copy the backfill made of the legacy actor `legacy_id` and then abandoned: destroying
/// or destroyed, and not destroyed by a request during its hold
/// (`LegacyMoveMarkCopyDestroyed`). Such a copy is not the actor; the legacy actor behind it
/// still may be. A copy still destroying counts, since an abandon's destroy lasts as long as the
/// copy's database delete, and a create that waited it out would otherwise shadow the live legacy
/// actor. A copy a request destroyed stays destroyed, since the backfill then destroys its legacy
/// actor too. The resolvers and Envoy event delivery share this one test. Removed in 3.1 with
/// the backfill.
pub fn legacy_abandoned_copy(
	phase: &wire::ActorPhase,
	legacy_move: &Option<wire::LegacyMoveMark>,
	data: &wire::ActorData,
	legacy_id: Id,
) -> bool {
	legacy_abandoned_copy_state(phase, legacy_move) && legacy_copy_of(data, legacy_id)
}

/// Legacy: the state half of [`legacy_abandoned_copy`], for a caller that holds the state and
/// reads the record only when this passes. Removed in 3.1 with the backfill.
pub fn legacy_abandoned_copy_state(
	phase: &wire::ActorPhase,
	legacy_move: &Option<wire::LegacyMoveMark>,
) -> bool {
	let ending = match phase {
		wire::ActorPhase::Destroying(_)
		| wire::ActorPhase::Destroyed
		| wire::ActorPhase::DelegatedToV2 => true,
		wire::ActorPhase::Initializing(_)
		| wire::ActorPhase::Starting(_)
		| wire::ActorPhase::Running(_)
		| wire::ActorPhase::Stopping(_)
		| wire::ActorPhase::Sleeping(_)
		| wire::ActorPhase::MigratingFromV2(_) => false,
	};
	ending && !legacy_destroyed_by_request(legacy_move)
}

/// Legacy: whether an Envoy event at `event_epoch` to the actor whose state is `state` and whose
/// record is `data` belongs to the legacy actor `legacy_id` rather than to this actor, a copy the
/// backfill made of it under the same id, which random-id actors keep. The copy refuses such an
/// event and delivery sends it to the legacy actor, whose stop report would otherwise be accepted
/// and ignored here, leaving the legacy actor stopping forever. That is so for an abandoned copy
/// ([`legacy_abandoned_copy`]), for a copy still held for its confirm, which never started, and
/// for any event whose epoch is not the copy's current one, since a confirmed copy's epochs are
/// above every epoch of the legacy actor. Never for a copy a request destroyed. Removed in 3.1
/// with the backfill.
pub fn legacy_copy_refuses_envoy_event(
	state: &wire::ActorState,
	event_epoch: u32,
	data: &wire::ActorData,
	legacy_id: Id,
) -> bool {
	legacy_copy_refuses_envoy_event_state(state, event_epoch) && legacy_copy_of(data, legacy_id)
}

/// Legacy: the state half of [`legacy_copy_refuses_envoy_event`], for a caller that reads the
/// record only when this passes, which an event at a live actor's current epoch never does.
/// Removed in 3.1 with the backfill.
pub fn legacy_copy_refuses_envoy_event_state(state: &wire::ActorState, event_epoch: u32) -> bool {
	let held = match &state.phase {
		wire::ActorPhase::Initializing(init) => init.legacy_move.is_some(),
		wire::ActorPhase::Starting(_)
		| wire::ActorPhase::Running(_)
		| wire::ActorPhase::Stopping(_)
		| wire::ActorPhase::Sleeping(_)
		| wire::ActorPhase::MigratingFromV2(_)
		| wire::ActorPhase::DelegatedToV2
		| wire::ActorPhase::Destroying(_)
		| wire::ActorPhase::Destroyed => false,
	};
	let other_incarnation = held || event_epoch != state.epoch;
	legacy_abandoned_copy_state(&state.phase, &state.legacy_move)
		|| (other_incarnation && !legacy_destroyed_by_request(&state.legacy_move))
}

/// Legacy: whether a request destroyed the actor while the backfill held it as a copy. Removed
/// in 3.1 with the backfill.
fn legacy_destroyed_by_request(legacy_move: &Option<wire::LegacyMoveMark>) -> bool {
	match legacy_move {
		Some(wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed) => true,
		Some(
			wire::LegacyMoveMark::LegacyMoveMarkMoving(_)
			| wire::LegacyMoveMark::LegacyMoveMarkMoved(_)
			| wire::LegacyMoveMark::LegacyMoveMarkCleared(_),
		)
		| None => false,
	}
}

/// Legacy: whether `data` is the record of a copy the backfill made of the legacy actor
/// `legacy_id`. Removed in 3.1 with the backfill.
fn legacy_copy_of(data: &wire::ActorData, legacy_id: Id) -> bool {
	data.legacy_moved_from
		.as_ref()
		.is_some_and(|moved_from| moved_from.as_ref() == legacy_id.as_bytes().as_slice())
}

/// Legacy: what a fallback answers for an abandoned copy of a legacy actor. Removed in 3.1 with
/// the backfill.
#[derive(Debug)]
pub(crate) enum MovedAwayRead {
	/// The legacy actor is not destroyed, so it is still the actor.
	Legacy(wire::ActorReadResponse),
	/// The legacy actor is destroyed or missing too, so the abandoned copy is answered, as before.
	Copy(wire::ActorReadResponse),
}

/// Legacy: decides between the abandoned copy `copy` at the grouped volume and the legacy actor
/// `legacy`, its unprefixed read for this namespace: a legacy actor not yet destroyed is the
/// actor, since its move was abandoned; otherwise the copy is answered, so an actor that was
/// moved and later destroyed still reads as destroyed in its group. A legacy actor marked
/// `Moved` answers the copy too, even while it is still stopping: a move sets that mark only
/// once it keeps the copy, so the copy is the actor, and a request destroy of a confirmed copy
/// leaves it no `CopyDestroyed` mark. Removed in 3.1 with the backfill.
pub(crate) fn legacy_settle_moved_away(
	copy: wire::ActorReadResponse,
	legacy: Option<wire::ActorReadResponse>,
) -> MovedAwayRead {
	let Some(read) = legacy else {
		return MovedAwayRead::Copy(copy);
	};
	let moved = matches!(
		read.legacy_move,
		Some(wire::LegacyMoveMark::LegacyMoveMarkMoved(_))
	);
	let destroyed = matches!(read.phase, Some(wire::ActorPhase::Destroyed));
	match moved || destroyed {
		true => MovedAwayRead::Copy(copy),
		false => MovedAwayRead::Legacy(read),
	}
}

/// Legacy: judges a fallback's read of the unprefixed state volume for `namespace`: the read when
/// its record names `namespace`, and `None` for a missing or deleted volume, a record with no
/// data yet, or one of another namespace, since every namespace from before groups shares that
/// key space. Removed in 3.1 with the backfill.
pub(crate) fn legacy_unprefixed_read(
	namespace: &str,
	result: Result<wire::ActorReadResponse>,
) -> Result<Option<wire::ActorReadResponse>> {
	match result {
		Ok(read) => {
			let ours = read
				.data
				.as_ref()
				.is_some_and(|data| crate::catalog::actor_in_namespace(None, namespace, data));
			match ours {
				true => Ok(Some(read)),
				false => Ok(None),
			}
		}
		Err(error) if crate::commands::is_volume_gone(&error) => Ok(None),
		Err(error) => Err(error),
	}
}

/// The result label of [`crate::metrics::LEGACY_ACTOR_READS`].
#[derive(Clone, Copy, Debug)]
pub(crate) enum LegacyReadResult {
	Grouped,
	Legacy,
	NotFound,
}

/// Counts one fallback resolve by where it found the actor.
pub(crate) fn record_legacy_read(result: LegacyReadResult) {
	let label = match result {
		LegacyReadResult::Grouped => "grouped",
		LegacyReadResult::Legacy => "legacy",
		LegacyReadResult::NotFound => "not_found",
	};
	crate::metrics::LEGACY_ACTOR_READS
		.with_label_values(&[label])
		.inc();
}
