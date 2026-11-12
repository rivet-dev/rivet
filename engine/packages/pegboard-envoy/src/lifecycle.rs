//! Translates envoy lifecycle events into actor state machine events.
use anyhow::{Context, Result};
use pegboard::{keys::partition::ActorVolume, wire};
use rivet_engine_context::EngineContext;
use rivet_envoy_protocol as p;
use rivet_keys::keys::codec::GroupId;
use rivet_util_id::Id;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Maps one envoy event to the actor event it drives. The envoy protocol's epoch is the
/// actor's epoch.
pub(crate) fn actor_event(event: &p::EventWrapper) -> wire::ActorEvent {
	let kind = match &event.inner {
		p::Event::EventActorIntent(intent) => match intent.intent {
			p::ActorIntent::ActorIntentSleep => wire::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent,
			p::ActorIntent::ActorIntentStop => wire::ToEnvoyEventKind::ToEnvoyEventKindStopIntent,
		},
		p::Event::EventActorStateUpdate(update) => match &update.state {
			p::ActorState::ActorStateRunning => wire::ToEnvoyEventKind::ToEnvoyEventKindRunning,
			p::ActorState::ActorStateStopped(stopped) => {
				wire::ToEnvoyEventKind::ToEnvoyEventKindStopped(wire::ToEnvoyEventKindStopped {
					ok: match stopped.code {
						p::StopCode::Ok => true,
						p::StopCode::Error => false,
					},
					message: stopped
						.message
						.as_deref()
						.map(truncate_message)
						.map(Into::into),
				})
			}
		},
		p::Event::EventActorSetAlarm(alarm) => {
			wire::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(wire::ToEnvoyEventKindSetAlarm {
				alarm_unix_ms: alarm.alarm_ts,
			})
		}
	};

	wire::ActorEvent::ToEnvoyEvent(wire::ToEnvoyEvent {
		epoch: event.checkpoint.generation,
		kind,
	})
}

/// The longest stop message kept in the actor's state, in bytes.
const MAX_STOP_MESSAGE_BYTES: usize = 1024;

/// Truncates a stop message to [`MAX_STOP_MESSAGE_BYTES`] on a character boundary, so a large
/// error string cannot grow every later actor state write.
pub(crate) fn truncate_message(message: &str) -> String {
	let mut end = message.len().min(MAX_STOP_MESSAGE_BYTES);
	while !message.is_char_boundary(end) {
		end -= 1;
	}
	message[..end].to_owned()
}

/// Whether the event reports that its epoch stopped.
pub(crate) fn is_stopped(event: &p::EventWrapper) -> bool {
	matches!(
		&event.inner,
		p::Event::EventActorStateUpdate(p::EventActorStateUpdate {
			state: p::ActorState::ActorStateStopped(_)
		})
	)
}

/// Legacy: the connection's namespace had no group at connect and has one now, so this
/// connection's scope is stale and it must close. The envoy reconnects with the grouped scope and
/// replays the event, which stays unacknowledged. Removed in 3.1 with the backfill.
#[derive(Debug)]
pub(crate) struct LegacyNamespaceMoved;

impl std::fmt::Display for LegacyNamespaceMoved {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("envoy namespace moved into its volume group after connect")
	}
}

impl std::error::Error for LegacyNamespaceMoved {}

/// Delivers one envoy event to the actor state machine. Returns `false` when the delivery
/// budget ran out, leaving the event unacknowledged so the envoy replays it after reconnecting.
/// An event for an actor that is gone, or whose record is not in the connection's namespace, is
/// dropped and acknowledged, since no delivery could ever apply it. Fails with
/// [`LegacyNamespaceMoved`], leaving the event unacknowledged, when the connection's namespace
/// moved into its group after connect.
///
/// `legacy_foreign_logged` is the connection's flag for its first refused foreign event: only
/// that one logs a warning, so an envoy that keeps naming another namespace's actors cannot flood
/// the logs.
///
/// Replaying is safe because the state machine applies an envoy event only to the epoch it
/// names and only in the phase it expects, so a second delivery of a committed event is a no-op.
pub(crate) async fn deliver(
	ctx: &EngineContext,
	scope: &crate::routes::EnvoyScope,
	legacy_foreign_logged: &AtomicBool,
	actor_id: &str,
	event: p::EventWrapper,
) -> Result<bool> {
	let id = Id::parse(actor_id).context("invalid envoy event actor id")?;
	let fallback = scope.legacy_fallback_now(ctx).await;
	let store = DepotEventStore {
		ctx,
		scope,
		fallback,
		actor_id: id,
		event: actor_event(&event),
	};
	let budget = Duration::from_millis(ctx.config().pegboard().envoy_event_delivery_timeout_ms());
	let falls_back = fallback.namespace_for(id).is_some();
	let delivery = deliver_within(ctx.env().as_ref(), budget, scope, falls_back, &store).await;

	match delivery {
		Delivery::Delivered => Ok(true),
		Delivery::Gone => {
			tracing::debug!(
				%actor_id,
				epoch = event.checkpoint.generation,
				"actor event names an actor that is gone; dropping it"
			);
			Ok(true)
		}
		Delivery::LegacyForeign => {
			if legacy_foreign_logged.swap(true, Ordering::Relaxed) {
				tracing::debug!(
					%actor_id,
					namespace = %scope.namespace,
					epoch = event.checkpoint.generation,
					"actor event names an actor outside the envoy's namespace; refusing it"
				);
			} else {
				tracing::warn!(
					%actor_id,
					namespace = %scope.namespace,
					epoch = event.checkpoint.generation,
					"actor event names an actor outside the envoy's namespace; refusing it, the envoy may be misconfigured, later refusals on this connection log at debug"
				);
			}
			Ok(true)
		}
		Delivery::LegacyNamespaceMoved => {
			tracing::info!(
				%actor_id,
				namespace = %scope.namespace,
				epoch = event.checkpoint.generation,
				"actor event reached a connection opened before its namespace moved into its group; closing it so the envoy reconnects and replays"
			);
			Err(LegacyNamespaceMoved.into())
		}
		Delivery::Exhausted => {
			tracing::warn!(
				%actor_id,
				epoch = event.checkpoint.generation,
				index = event.checkpoint.index,
				"actor event delivery exhausted its budget; storage may be overloaded"
			);
			Ok(false)
		}
	}
}

/// How a delivery ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Delivery {
	/// The actor's state machine took the event.
	Delivered,
	/// The actor's state volume is deleted or never existed, so the actor is gone.
	Gone,
	/// Legacy: the actor's unprefixed record names another namespace than the connection's.
	/// Removed in 3.1 with the backfill.
	LegacyForeign,
	/// Legacy: the connection's namespace had no group at connect, the actor's unprefixed state
	/// volume is absent, and the namespace has a group now, so the actor may live in it. The
	/// event is not acknowledged and the connection closes. Removed in 3.1 with the backfill.
	LegacyNamespaceMoved,
	/// The budget ran out with the event still undelivered.
	Exhausted,
}

/// Where a delivery sends the event next.
#[derive(Clone, Copy, Debug)]
enum EventTarget {
	/// Legacy: the connection's namespace had no group at connect, so the actor's volume is
	/// unprefixed and only its record's legacy namespace says whose it is: the record is read
	/// first. Removed in 3.1 with the backfill.
	LegacyUngrouped,
	/// Legacy: the namespace's backfill is not done, so the grouped volume is tried first and
	/// the actor resolved from what that send answers. Removed in 3.1 with the backfill.
	Unresolved,
	/// The group the actor's state volume is in.
	Resolved(Option<GroupId>),
}

/// How an actor's state volume was found absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Absence {
	/// The volume was deleted: an actor existed there and was destroyed.
	Deleted,
	/// The volume never existed, or holds no record.
	Missing,
}

impl Absence {
	/// The absence a failed command reports, or `None` for any other failure.
	fn of(error: &anyhow::Error) -> Option<Absence> {
		if is_volume_deleted(error) {
			Some(Absence::Deleted)
		} else if depot_client::groups::is_volume_not_found(error) {
			Some(Absence::Missing)
		} else {
			None
		}
	}
}

/// Legacy: what [`EventStore::legacy_ungrouped_record`] found at the actor's unprefixed state
/// volume. Removed in 3.1 with the backfill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LegacyUngroupedRecord {
	/// A record whose legacy namespace is the connection's.
	Owned,
	/// A record of another namespace, or one that names none.
	Foreign,
	/// No record.
	Absent(Absence),
}

/// The storage operations a delivery makes, apart from its order and budget.
trait EventStore {
	/// Sends the event to the actor's state volume in `group`.
	async fn send(&self, group: Option<GroupId>) -> Result<()>;

	/// Legacy: resolves the actor once its grouped state volume was found missing, reading the
	/// unprefixed one. Removed in 3.1 with the backfill.
	async fn resolve_ungrouped(&self) -> Result<pegboard::legacy_volumes::ResolvedActor>;

	/// Resolves the group of the actor's volumes after a send failed for another reason.
	async fn resolve(&self) -> Result<Option<GroupId>>;

	/// Legacy: reads the actor's unprefixed record to tell whose it is. Removed in 3.1 with the
	/// backfill.
	async fn legacy_ungrouped_record(&self) -> Result<LegacyUngroupedRecord>;

	/// Legacy: reads the connection's namespace uncached and answers its group now, for a
	/// connection whose namespace had none at connect. Answers `None` without a read while older
	/// binaries are known to run, since no namespace moves into its group then. Removed in 3.1
	/// with the backfill.
	async fn legacy_namespace_group(&self) -> Result<Option<GroupId>>;
}

/// Delivers the event through `store` within `budget`, retrying failed sends and resolves with
/// capped exponential backoff. The first send runs inside the budget too. `falls_back` is whether
/// the connection's legacy fallback, as the delivery read it once, covers the actor, so a grouped
/// scope sends to the grouped volume first and resolves the unprefixed one only if that is
/// missing.
///
/// A gone answer is final only from the scope's own group. In a grouped scope, a gone answer
/// from the unprefixed shape gets one more send to the grouped volume: the backfill writes the
/// grouped record before it removes the unprefixed one, so that send is authoritative.
///
/// Legacy: in a scope without a group, a gone answer is never final on its own. The connection's
/// scope was fixed at connect, so the namespace may have moved into its group since and the actor
/// may live there; the namespace is read once, uncached, and a group there ends the delivery with
/// [`Delivery::LegacyNamespaceMoved`]. Otherwise a deleted volume is final and a missing one is
/// retried within the budget, after which the connection closes and the envoy replays. Removed in
/// 3.1 with the backfill.
///
/// Without a group the actor's volume key carries no namespace, so an untrusted envoy could
/// name another namespace's legacy actor id; the record's legacy namespace is checked before
/// the event is sent. The state machine's epoch and phase checks already keep a stray event
/// from doing more than a matching envoy event could, so this only closes the cross-namespace
/// path.
async fn deliver_within<S: EventStore>(
	env: &dyn rivet_env::Env,
	budget: Duration,
	scope: &crate::routes::EnvoyScope,
	falls_back: bool,
	store: &S,
) -> Delivery {
	let clock = env.clock();
	let mut target = match (scope.group, falls_back) {
		(None, _) => EventTarget::LegacyUngrouped,
		(Some(_), true) => EventTarget::Unresolved,
		(Some(group), false) => EventTarget::Resolved(Some(group)),
	};
	// Legacy: the namespace group a scope without one read after an absent volume, once per
	// delivery. Removed in 3.1 with the backfill.
	let mut legacy_group_read: Option<Option<GroupId>> = None;
	// Legacy: whether the unprefixed actor already answered for an event a backfill copy refused,
	// either resolved gone or sent the event once, so the copy's next refusal ends the delivery.
	// Removed in 3.1 with the backfill.
	let mut legacy_unprefixed_answered = false;

	let attempts = async {
		let mut delay = Duration::from_millis(10);
		loop {
			let absence = match target {
				EventTarget::LegacyUngrouped => match store.legacy_ungrouped_record().await {
					Ok(LegacyUngroupedRecord::Owned) => {
						target = EventTarget::Resolved(None);
						continue;
					}
					Ok(LegacyUngroupedRecord::Foreign) => return Delivery::LegacyForeign,
					Ok(LegacyUngroupedRecord::Absent(absence)) => Ok(absence),
					Err(error) => Err(error),
				},
				EventTarget::Unresolved => match store.send(scope.group).await {
					Ok(()) => return Delivery::Delivered,
					Err(error) if is_volume_deleted(&error) => return Delivery::Gone,
					// Legacy: an abandoned copy of the legacy actor refused the event; the
					// unprefixed actor is resolved as for a missing grouped volume. Removed in
					// 3.1 with the backfill.
					Err(error)
						if depot_client::groups::is_volume_not_found(&error)
							|| legacy_is_moved_away(&error) =>
					{
						match store.resolve_ungrouped().await {
							Ok(found) => {
								legacy_unprefixed_answered |= matches!(
									found.read,
									pegboard::legacy_volumes::ResolvedRead::Gone
								);
								target = ungrouped_target(scope, found);
								continue;
							}
							Err(error) => Err(error),
						}
					}
					Err(error) => {
						tracing::debug!(
							?error,
							"actor event to the grouped volume failed; resolving the actor"
						);
						match store.resolve().await {
							Ok(group) => {
								target = EventTarget::Resolved(group);
								continue;
							}
							Err(error) => Err(error),
						}
					}
				},
				EventTarget::Resolved(group) => match store.send(group).await {
					Ok(()) => return Delivery::Delivered,
					// Legacy: a copy refused the event as another incarnation's. The unprefixed
					// actor with its id gets the event once, as after `Done`, when every
					// connection resolves to the group and a stopping legacy actor's report would
					// otherwise never reach it, but only when its record names this connection's
					// namespace: a group fork's child keeps its parent's move mark, so the refusal
					// alone does not prove the unprefixed actor is this namespace's. Once it
					// answered, or was resolved gone, no actor is left to take the event, as when
					// a destroyed actor ignores it. Removed in 3.1 with the backfill.
					Err(error) if legacy_is_moved_away(&error) => {
						match (group, legacy_unprefixed_answered) {
							(Some(_), false) => match store.legacy_ungrouped_record().await {
								Ok(LegacyUngroupedRecord::Owned) => {
									legacy_unprefixed_answered = true;
									target = EventTarget::Resolved(None);
									continue;
								}
								Ok(
									LegacyUngroupedRecord::Foreign
									| LegacyUngroupedRecord::Absent(_),
								) => return Delivery::Delivered,
								Err(error) => Err(error),
							},
							(Some(_), true) | (None, _) => return Delivery::Delivered,
						}
					}
					Err(error) => match (Absence::of(&error), scope.group, group) {
						(None, _, _) => Err(error),
						// The scope's own group answered: the actor is gone.
						(Some(_), Some(_), Some(_)) => return Delivery::Gone,
						// The unprefixed shape answered in a grouped scope: the grouped volume
						// gets one more send, whose answer is final.
						(Some(_), Some(own), None) => {
							target = EventTarget::Resolved(Some(own));
							continue;
						}
						(Some(absence), None, _) => Ok(absence),
					},
				},
			};

			// Legacy: an absent unprefixed volume in a scope without a group re-reads the
			// namespace before anything is final. A volume still missing then is retried, with
			// no error to report. Removed in 3.1 with the backfill.
			let failed = match absence {
				Err(error) => Some(error),
				Ok(absence) => {
					let group = match legacy_group_read {
						Some(group) => Ok(group),
						None => store.legacy_namespace_group().await,
					};
					target = EventTarget::LegacyUngrouped;
					match (group, absence) {
						(Ok(Some(_)), _) => return Delivery::LegacyNamespaceMoved,
						(Ok(None), Absence::Deleted) => return Delivery::Gone,
						(Ok(None), Absence::Missing) => {
							legacy_group_read = Some(None);
							None
						}
						(Err(error), _) => Some(error),
					}
				}
			};
			tracing::debug!(error = ?failed, "actor event attempt failed; retrying");
			if rivet_env::sleep(clock.as_ref(), delay).await.is_err() {
				return Delivery::Exhausted;
			}
			delay = (delay * 2).min(Duration::from_millis(500));
		}
	};

	match rivet_env::timeout(clock.as_ref(), budget, attempts).await {
		Ok(delivery) => delivery,
		Err(_elapsed) => Delivery::Exhausted,
	}
}

/// Legacy: where a delivery sends next once the unprefixed volume was resolved: there when the
/// actor was found, otherwise to the grouped volume once more, since an actor absent in both
/// shapes may have just moved into the group. Removed in 3.1 with the backfill.
fn ungrouped_target(
	scope: &crate::routes::EnvoyScope,
	found: pegboard::legacy_volumes::ResolvedActor,
) -> EventTarget {
	match found.read {
		pegboard::legacy_volumes::ResolvedRead::Gone => EventTarget::Resolved(scope.group),
		pegboard::legacy_volumes::ResolvedRead::Found(_)
		| pegboard::legacy_volumes::ResolvedRead::Unread => EventTarget::Resolved(found.group),
	}
}

/// Legacy: whether an event was refused by an abandoned copy the backfill made of the legacy
/// actor with the same id ([`pegboard::validation::LegacyActorMovedAway`]). Removed in 3.1 with
/// the backfill.
fn legacy_is_moved_away(error: &anyhow::Error) -> bool {
	let extracted = rivet_error::RivetError::extract(error);
	extracted.group() == "actor" && extracted.code() == "legacy_moved_away"
}

/// Whether a command failed because its volume was deleted, which means the actor is gone.
fn is_volume_deleted(error: &anyhow::Error) -> bool {
	let extracted = rivet_error::RivetError::extract(error);
	extracted.group() == "depot" && extracted.code() == "volume_deleted"
}

/// The production [`EventStore`]: Depot commands on the actor's state volumes.
struct DepotEventStore<'a> {
	ctx: &'a EngineContext,
	scope: &'a crate::routes::EnvoyScope,
	/// Legacy: the connection's fallback as the delivery read it once at its start. Removed in
	/// 3.1 with the backfill.
	fallback: &'a pegboard::catalog::LegacyFallback,
	actor_id: Id,
	event: wire::ActorEvent,
}

impl EventStore for DepotEventStore<'_> {
	async fn send(&self, group: Option<GroupId>) -> Result<()> {
		self.ctx
			.depot()
			.command::<pegboard::commands::ActorEvent>(
				&ActorVolume::in_namespace(group, self.actor_id),
				self.event.clone(),
			)
			.await
			.map(|_response| ())
	}

	async fn resolve_ungrouped(&self) -> Result<pegboard::legacy_volumes::ResolvedActor> {
		let namespace = self
			.fallback
			.namespace_for(self.actor_id)
			.context("an ungrouped resolve needs the legacy fallback")?;
		let found = pegboard::legacy_volumes::legacy_resolve_ungrouped_actor(
			&self.ctx.depot(),
			self.scope.group,
			namespace,
			self.actor_id,
		)
		.await?;
		// A finished backfill, which still falls back for its skipped actors, walks no names.
		if self.fallback.every() {
			pegboard::legacy_volumes::legacy_record_found_name(&self.ctx, namespace, &found).await;
		}
		Ok(found)
	}

	async fn resolve(&self) -> Result<Option<GroupId>> {
		self.scope.actor_group(self.ctx, self.actor_id).await
	}

	async fn legacy_ungrouped_record(&self) -> Result<LegacyUngroupedRecord> {
		let read = self
			.ctx
			.depot()
			.command::<pegboard::commands::ActorRead>(
				&ActorVolume::in_namespace(None, self.actor_id),
				(),
			)
			.await;
		let read = match read {
			Ok(read) => read,
			Err(error) => match Absence::of(&error) {
				Some(absence) => return Ok(LegacyUngroupedRecord::Absent(absence)),
				None => return Err(error),
			},
		};
		let Some(data) = read.data else {
			return Ok(LegacyUngroupedRecord::Absent(Absence::Missing));
		};
		match pegboard::catalog::actor_in_namespace(None, &self.scope.namespace, &data) {
			true => Ok(LegacyUngroupedRecord::Owned),
			false => Ok(LegacyUngroupedRecord::Foreign),
		}
	}

	async fn legacy_namespace_group(&self) -> Result<Option<GroupId>> {
		pegboard::catalog::legacy_current_namespace_group(self.ctx, &self.scope.namespace).await
	}
}
