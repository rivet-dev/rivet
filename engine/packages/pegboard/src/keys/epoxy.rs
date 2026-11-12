//! Logical Epoxy keys owned by pegboard. Epoxy stores each one in its own key volume.

use rivet_keys::keys::codec::{GroupId, Placement};

// The reservation of a keyed actor's name and key, under the namespace's volume group, so the
// reservation lands with the group's copy of the actor and a fork reserves its own. It carries
// no namespace string. `placement` is the key's placement, the bytes of the keyed actor's id, so
// Epoxy places its key volume with the actor; it packs exactly as a derived placement does.
// Its value is encoded at the version its writer selects for `pegboard.actor`, the highest every
// live storage worker reads, as is the legacy reservation's below, which a process of the
// previous release reads.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(
	key = [PLACED, placement, ACTOR_KEY, name, key],
	value = vbare_selected(rivet_data::versioned::EpoxyActorByKeyEntry)
)]
pub struct ActorKeyKey {
	#[group]
	pub group: GroupId,
	pub placement: Placement,
	pub name: String,
	pub key: String,
}

/// Legacy: a keyed actor's reservation at its unprefixed key, which names its namespace.
/// Reservations made before namespaces had groups live at unprefixed keys; new behavior reserves
/// every key under its namespace's group with no namespace string ([`ActorKeyKey`]). Taken only
/// to read a reservation of a namespace whose backfill is not done. Removed in 3.1 with the
/// backfill.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(
	key = [ACTOR_KEY, namespace, name, key],
	placement = legacy_reservation_placement(namespace, name, key),
	value = vbare_selected(rivet_data::versioned::EpoxyActorByKeyEntry)
)]
pub struct LegacyActorKeyKey {
	pub namespace: String,
	pub name: String,
	pub key: String,
}

impl ActorKeyKey {}

/// Legacy: the placement of an unprefixed reservation, the legacy keyed id's placement, so it
/// stays with the volumes of the actor it reserves. Removed in 3.1 with the backfill.
fn legacy_reservation_placement(namespace: &str, name: &str, key: &str) -> Placement {
	crate::keys::partition::legacy_key_placement(namespace, name, key)
}
