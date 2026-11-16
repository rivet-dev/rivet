use crate::commands::ActorRead;
use crate::keys::epoxy::{ActorKeyKey, LegacyActorKeyKey};
use crate::keys::partition::{ActorVolume, LegacyActorVolume};
use crate::legacy_volumes::{
	GroupedRead, LegacyReadResult, MovedAwayRead, ResolvedRead, legacy_grouped_read,
	legacy_settle_moved_away, legacy_unprefixed_read, record_legacy_read,
};
use crate::wire;
use anyhow::{Context, Result};
use bytes::Bytes;
use epoxy::ops::{
	kv::get::{self, ReadMode},
	propose::{
		CheckAndSetCommand, Command, CommandKind, ConsensusFailedReason, Input, Proposal,
		ProposalResult,
	},
};
use rivet_engine_context::EngineContext;
use rivet_keys::keys::{
	KeyAddress,
	codec::{FormalKey, GroupId, Placement, ValueCodec},
};
use rivet_util::Id;
use std::sync::Arc;

// Every `key` in this module is the canonical actor key that `rivetkit_shared_types::actor_key`
// encodes. Edges translate what clients send before calling in, because the reservation, the
// keyed id and the stored key compare the string byte for byte.

/// Decodes a reservation value. Both key shapes share [`ActorKeyKey`]'s codec.
fn decode_reservation(value: Bytes) -> Result<wire::EpoxyActorByKeyEntry> {
	<ActorKeyKey as FormalKey>::Codec::decode(value)
}

async fn get_reservation(ctx: &EngineContext, epoxy_key: &Bytes) -> Result<Option<Id>> {
	let output = ctx
		.epoxy()?
		.op(get::Input {
			key: epoxy_key.to_vec(),
			mode: ReadMode::LatestReachable,
		})
		.await?;
	let Some(value) = output.value.and_then(|committed| committed.value) else {
		return Ok(None);
	};

	let entry = decode_reservation(value)?;
	let reservation_id = Id::from_slice(&entry.reservation_id)?;

	Ok(Some(reservation_id))
}

pub(crate) enum ReserveResult {
	Reserved {},
	AlreadyReserved { reservation_id: Id },
}

/// Reserves `key` for `name` in the group `group` at `placement`, the keyed actor's placement.
/// A grouped reservation's id hashes the name and key alone. Without a group, which only a
/// namespace served from its legacy catalog row has while older binaries run, the legacy key and
/// id, both of which name `legacy_namespace`, are written, since only those are what the older
/// binaries read.
pub(crate) async fn reserve(
	ctx: &EngineContext,
	group: Option<GroupId>,
	legacy_namespace: Option<String>,
	placement: Placement,
	name: String,
	key: String,
) -> Result<ReserveResult> {
	let (digest, epoxy_key) = match group {
		Some(group) => (
			key_digest(b"rivet.actor.key.reservation.v1", &[], &name, &key),
			ActorKeyKey::new(group, placement, name, key).pack(),
		),
		None => {
			let namespace = legacy_namespace
				.context("an unprefixed reservation needs the actor's legacy namespace")?;
			(
				key_digest(
					b"rivet.actor.key.reservation.v1",
					namespace.as_bytes(),
					&name,
					&key,
				),
				LegacyActorKeyKey::new(namespace, name, key).pack(),
			)
		}
	};
	let mut bytes = [0; 16];
	bytes.copy_from_slice(&digest[..16]);
	let reservation_id = Id::v1(uuid::Uuid::from_bytes(bytes), ctx.config().dc_label());

	// Reservations are immutable. An existing key can be recreated under its original
	// reservation without proposing another conditional write. A missing read still takes
	// the compare-and-set path below, which resolves concurrent first reservations.
	match get_reservation(ctx, &epoxy_key).await {
		Ok(Some(existing)) => {
			return Ok(if existing == reservation_id {
				ReserveResult::Reserved {}
			} else {
				ReserveResult::AlreadyReserved {
					reservation_id: existing,
				}
			});
		}
		Ok(None) => {}
		Err(error) if depot_client::retry_after_ms(&error).is_some() => {
			// An interrupted creating write can leave the volume unreadable until a writer
			// reconciles it. The conditional proposal below preserves any existing winner.
			tracing::debug!(
				?error,
				"reservation read refused; reconciling with a conditional write"
			);
		}
		Err(error) => return Err(error),
	}

	// A process of the previous release reads a legacy reservation while a deploy rolls.
	let entry = wire::EpoxyActorByKeyEntry {
		reservation_id: reservation_id.as_bytes().into(),
	};
	let topology = ctx.topology();
	let value = crate::sealed::seal_stored::<rivet_data::versioned::EpoxyActorByKeyEntry>(
		crate::sealed::SealedValue::Reservation,
		&entry,
		topology.as_ref(),
		topology.config().lease.refresh_hint_interval_ms,
	)?
	.to_vec();

	let proposal_res = ctx
		.epoxy()?
		.propose(Input {
			proposal: Proposal {
				commands: vec![Command {
					kind: CommandKind::CheckAndSetCommand(CheckAndSetCommand {
						key: epoxy_key.to_vec(),
						expect_one_of: vec![None],
						new_value: Some(value),
					}),
				}],
			},
			mutable: false,
		})
		.await?;

	match proposal_res {
		ProposalResult::Committed => Ok(ReserveResult::Reserved {}),
		ProposalResult::ConsensusFailed {
			reason:
				ConsensusFailedReason::ExpectedValueDoesNotMatch {
					current_value: Some(current),
				},
		} => {
			let existing_reservation_id =
				Id::from_slice(&decode_reservation(current.into())?.reservation_id)?;

			// The reservation is per key and datacenter, so finding our own id means this
			// datacenter reserved the key before, as a recreation does. Another id names the
			// datacenter that owns it.
			if existing_reservation_id == reservation_id {
				Ok(ReserveResult::Reserved {})
			} else {
				Ok(ReserveResult::AlreadyReserved {
					reservation_id: existing_reservation_id,
				})
			}
		}
		res => {
			res.resolve()?;
			Ok(ReserveResult::Reserved {})
		}
	}
}

/// The reservation digest. `namespace` is empty for a grouped reservation; only the legacy
/// unprefixed reservation hashes its namespace's name.
fn key_digest(domain: &[u8], namespace: &[u8], name: &str, key: &str) -> [u8; 32] {
	use sha2::{Digest, Sha256};
	let mut hasher = Sha256::new();
	hasher.update(domain);
	hasher.update(namespace);
	for part in [name.as_bytes(), key.as_bytes()] {
		hasher.update((part.len() as u64).to_le_bytes());
		hasher.update(part);
	}
	hasher.finalize().into()
}

/// Counts one adoption of a key whose reservation landed while its actor's create failed. The
/// adopter creates that actor id again under the reservation it already holds.
pub fn record_adoption(adopted: bool) {
	let result = match adopted {
		true => "adopted",
		false => "error",
	};
	crate::metrics::ACTOR_KEY_ADOPTIONS
		.with_label_values(&[result])
		.inc();
}

/// The actor id of `name` and `key`, owned by the datacenter `datacenter_label`: the key's
/// placement with the datacenter's label, and nothing else. No namespace, epoch or other
/// per-lifetime value is part of it, so one `(name, key)` has one id in every namespace; the
/// group every volume key of the actor carries keeps namespaces apart.
///
/// The id is derived, not drawn, so every create of the key names the same actor: a create whose
/// reservation never landed is adopted by the next create of the key, and concurrent creates
/// converge.
///
/// **Contract:** the id bytes are the key's placement, so the id alone finds the actor's state
/// volume, its database volumes and its Epoxy key volume, all on one tablet. Routing by id needs
/// no lookup, and the hot paths (create, wake, gateway routing, and SQLite reads and writes)
/// keep every operation local. See [`crate::keys::partition::key_placement`].
pub fn keyed_actor_id(datacenter_label: u16, name: &str, key: &str) -> Id {
	let placement = crate::keys::partition::key_placement(name, key);
	Id::v1(uuid::Uuid::from_bytes(placement), datacenter_label)
}

/// Legacy: the id a keyed actor had before namespaces had groups, which hashed its namespace's
/// name ([`crate::keys::partition::legacy_key_placement`]). New behavior is [`keyed_actor_id`].
/// Taken only for a keyed actor in a namespace without a group ([`keyed_create_actor`] and
/// [`resolve_keyed_actor`]), and by [`legacy_resolve_keyed_actor`] for an actor its namespace's
/// backfill has not moved yet. Removed in 3.1 with the backfill.
pub fn legacy_keyed_actor_id(datacenter_label: u16, namespace: &str, name: &str, key: &str) -> Id {
	let placement = crate::keys::partition::legacy_key_placement(namespace, name, key);
	Id::v1(uuid::Uuid::from_bytes(placement), datacenter_label)
}

/// A keyed actor as a request in its namespace finds it: its id, and the group its volumes are
/// keyed in (`None` for the legacy unprefixed shape).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyedActor {
	pub actor_id: Id,
	pub group: Option<GroupId>,
}

/// A keyed actor found by [`resolve_keyed_actor`], with what the lookup learned about its state
/// volume, so the caller does not read the record again.
#[derive(Clone, Debug)]
pub struct ResolvedKeyedActor {
	pub actor: KeyedActor,
	pub read: ResolvedRead,
}

/// The keyed actor a create of `name` and `key` in `namespace` makes, owned by
/// `datacenter_label`. Every create in a grouped namespace, whatever its backfill state, makes
/// [`keyed_actor_id`] in the namespace's group; a namespace without a group takes
/// [`legacy_keyed_create_actor`]. A create passes the standing [`keyed_create_standing`]
/// answers, so a stale cached standing never creates at the legacy id.
pub fn keyed_create_actor(
	datacenter_label: u16,
	namespace: &crate::NamespaceStanding,
	name: &str,
	key: &str,
) -> KeyedActor {
	match namespace.group_id() {
		Some(group) => KeyedActor {
			actor_id: keyed_actor_id(datacenter_label, name, key),
			group: Some(group),
		},
		None => legacy_keyed_create_actor(datacenter_label, &namespace.name, name, key),
	}
}

/// The namespace standing a keyed create of `namespace` acts on. A grouped standing is used as
/// it is. Legacy: a standing without a group is read again uncached first, since a process whose
/// cached standing is older than the namespace's move into its group would otherwise create the
/// key at its legacy id while processes that already see the group create it at the new id,
/// leaving two live actors for one key until the cache entry expires. The legacy behavior trusted
/// the cached standing; the new behavior creates at the new id in the group whenever the re-read
/// shows one. Taken only for a keyed create in a namespace whose cached standing has no group,
/// which exists only while older binaries run, at the cost of one namespace read per such
/// create; see [`crate::catalog::legacy_fresh_ungrouped_standing`] for when the read is skipped.
///
/// Two windows still make a legacy duplicate of a grouped keyed actor. A legacy create already
/// in flight when the move lands makes the legacy actor (milliseconds). And this process's view
/// of the fleet can still list an upgraded worker as old for up to its discovery validity window
/// after another process, whose view is newer, moved the namespace: this process then skips the
/// re-read and creates at the legacy id. The namespace's backfill resolves either pair by keeping
/// the grouped actor and destroying the legacy duplicate. Removed in 3.1 with the backfill.
pub async fn keyed_create_standing(
	ctx: &EngineContext,
	namespace: &Arc<crate::NamespaceStanding>,
) -> Result<Arc<crate::NamespaceStanding>> {
	crate::catalog::legacy_fresh_ungrouped_standing(
		namespace,
		crate::catalog::namespace_volumes_open(ctx),
		|| crate::catalog::lookup_namespace(ctx, &namespace.name),
	)
	.await
}

/// Legacy: the standing a keyed create acts on after [`keyed_create_standing`]'s uncached
/// re-read of the ungrouped `cached` standing answered `reread`. The re-read is admitted again,
/// since the namespace may have moved on since it was cached. Removed in 3.1 with the backfill.
pub(crate) fn legacy_reread_standing(
	cached: &crate::NamespaceStanding,
	reread: Option<crate::namespace_wire::NamespaceState>,
) -> Result<crate::NamespaceStanding> {
	let Some(state) = reread else {
		return Err(crate::validation::NamespaceNotFound.build());
	};
	let standing = crate::NamespaceStanding::from_state(&cached.name, &state);
	standing.admit()?;
	Ok(standing)
}

/// Legacy: the keyed actor a create makes in a namespace without a group, as binaries from before
/// groups make it: the legacy id ([`legacy_keyed_actor_id`]) at the unprefixed keys, with its
/// unprefixed reservation and index entry and the namespace recorded as the actor's legacy
/// namespace. New behavior creates [`keyed_actor_id`] in the namespace's group. Taken only for a
/// create in a namespace that has no group record yet, while binaries from before groups run; the
/// backfill later moves these actors like any other legacy actor. Removed in 3.1 with the
/// backfill.
fn legacy_keyed_create_actor(
	datacenter_label: u16,
	namespace: &str,
	name: &str,
	key: &str,
) -> KeyedActor {
	KeyedActor {
		actor_id: legacy_keyed_actor_id(datacenter_label, namespace, name, key),
		group: None,
	}
}

/// Finds the actor `name` and `key` name in `namespace`, owned by `datacenter_label`. A grouped
/// namespace whose backfill is done answers [`keyed_actor_id`] in its group with no read. A
/// namespace without a group, which exists only while older binaries run, keeps every key in the
/// legacy shape, so it answers the legacy id unprefixed with no read. A done namespace whose
/// backfill skipped this key's legacy actor still reads for it. Otherwise the actor may not
/// be moved yet, and [`legacy_resolve_keyed_actor`] reads to tell, and a key it finds at its
/// unprefixed volume has its name recorded for the backfill.
pub async fn resolve_keyed_actor(
	ctx: &EngineContext,
	datacenter_label: u16,
	namespace: &crate::NamespaceStanding,
	name: &str,
	key: &str,
) -> Result<ResolvedKeyedActor> {
	match namespace.group_id() {
		Some(group) if legacy_keyed_falls_back(namespace, datacenter_label, name, key) => {
			let found = legacy_resolve_keyed_actor(
				&ctx.depot(),
				datacenter_label,
				group,
				&namespace.name,
				name,
				key,
			)
			.await?;
			// Legacy: a key found at its unprefixed volume records its name for the backfill
			// while the backfill runs. Removed in 3.1 with the backfill.
			if found.actor.group.is_none() && !namespace.backfill_done() {
				crate::legacy_backfill::legacy_record_actor_name(ctx, &namespace.name, name).await;
			}
			Ok(found)
		}
		Some(_) | None => Ok(ResolvedKeyedActor {
			actor: keyed_create_actor(datacenter_label, namespace, name, key),
			read: ResolvedRead::Unread,
		}),
	}
}

/// Legacy: whether a lookup of `name` and `key` in `namespace` reads for the actor at its
/// unprefixed keys: always while the backfill is not done, and after it only when the key's
/// legacy id is one the backfill lists as skipped, which costs a hash and a lookup in that
/// bounded set, no read. Removed in 3.1 with the backfill.
fn legacy_keyed_falls_back(
	namespace: &crate::NamespaceStanding,
	datacenter_label: u16,
	name: &str,
	key: &str,
) -> bool {
	let fallback = namespace.legacy_fallback();
	match fallback {
		crate::catalog::LegacyFallback::Off => false,
		crate::catalog::LegacyFallback::Every { .. } => true,
		crate::catalog::LegacyFallback::Skipped { .. } => {
			let legacy_id = legacy_keyed_actor_id(datacenter_label, &namespace.name, name, key);
			fallback.namespace_for(legacy_id).is_some()
		}
	}
}

/// Legacy: finds a keyed actor in the grouped namespace `namespace`, whose backfill is not done
/// or lists the key's legacy actor as skipped.
/// Actors from before namespaces had groups have the legacy id at unprefixed keys until the
/// backfill moves them to the new id in the group; new behavior has every keyed actor at
/// [`keyed_actor_id`] in its group. The new id's grouped state volume is read first; only when it
/// never existed, or holds a copy the backfill made of the legacy actor and abandoned, destroying
/// or destroyed,
/// is the legacy id's unprefixed state volume read, and it counts only when its record names this
/// namespace, since every namespace from before groups shares that key space, and the actor is
/// not destroyed (unlike
/// [`crate::legacy_volumes::legacy_resolve_ungrouped_actor`], a lookup by id, which answers a
/// destroyed record). Anything else answers the new id in the group, where a create puts the
/// actor, so no create ever reuses the legacy id. Taken only while the namespace's
/// backfill is not done; it costs one state volume read per keyed lookup, and a second for an
/// actor not yet moved, accepted until the backfill finishes. The read it answers with is
/// returned for reuse. Removed in 3.1 with the backfill.
pub async fn legacy_resolve_keyed_actor(
	depot: &depot_client::Client,
	datacenter_label: u16,
	group: GroupId,
	namespace: &str,
	name: &str,
	key: &str,
) -> Result<ResolvedKeyedActor> {
	let grouped = KeyedActor {
		actor_id: keyed_actor_id(datacenter_label, name, key),
		group: Some(group),
	};
	let legacy_id = legacy_keyed_actor_id(datacenter_label, namespace, name, key);
	let grouped_volume = ActorVolume::new(group, grouped.actor_id).volume_key();
	let grouped_read = depot.command::<ActorRead>(&grouped_volume, ()).await;
	let moved_away = match legacy_grouped_read(grouped_read, legacy_id)? {
		GroupedRead::MovedAway(copy) => Some(copy),
		GroupedRead::Found(read) => {
			record_legacy_read(LegacyReadResult::Grouped);
			return Ok(ResolvedKeyedActor {
				actor: grouped,
				read: ResolvedRead::Found(read),
			});
		}
		GroupedRead::Gone => {
			record_legacy_read(LegacyReadResult::NotFound);
			return Ok(ResolvedKeyedActor {
				actor: grouped,
				read: ResolvedRead::Gone,
			});
		}
		GroupedRead::Missing => None,
	};

	let legacy_volume = LegacyActorVolume::new(legacy_id).volume_key();
	let legacy_read = legacy_unprefixed_read(
		namespace,
		depot.command::<ActorRead>(&legacy_volume, ()).await,
	)?;
	// An abandoned copy of the legacy actor is answered only when the legacy actor is destroyed
	// too, as an existing grouped record always was.
	if let Some(copy) = moved_away {
		if let MovedAwayRead::Copy(copy) = legacy_settle_moved_away(copy, legacy_read.clone()) {
			record_legacy_read(LegacyReadResult::Grouped);
			return Ok(ResolvedKeyedActor {
				actor: grouped,
				read: ResolvedRead::Found(copy),
			});
		}
	}
	let live = legacy_read.filter(|read| !matches!(read.phase, Some(wire::ActorPhase::Destroyed)));
	match live {
		Some(read) => {
			record_legacy_read(LegacyReadResult::Legacy);
			Ok(ResolvedKeyedActor {
				actor: KeyedActor {
					actor_id: legacy_id,
					group: None,
				},
				read: ResolvedRead::Found(read),
			})
		}
		None => {
			record_legacy_read(LegacyReadResult::NotFound);
			Ok(ResolvedKeyedActor {
				actor: grouped,
				read: ResolvedRead::Gone,
			})
		}
	}
}
