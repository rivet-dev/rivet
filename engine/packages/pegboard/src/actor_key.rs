use crate::keys::epoxy::ActorKeyKey;
use crate::wire;
use anyhow::Result;
use epoxy::ops::{
	kv::get::{self, ReadMode},
	propose::{
		CheckAndSetCommand, Command, CommandKind, ConsensusFailedReason, Input, Proposal,
		ProposalResult,
	},
};
use rivet_engine_context::EngineContext;
use rivet_keys::keys::{KeyAddress, codec::FormalKey};
use rivet_util::Id;

// Every `key` in this module is the canonical actor key that `rivetkit_shared_types::actor_key`
// encodes. Edges translate what clients send before calling in, because the reservation, the
// keyed id and the stored key compare the string byte for byte.

pub async fn get(
	ctx: &EngineContext,
	namespace: &str,
	name: &str,
	key: &str,
) -> Result<Option<Id>> {
	let epoxy_key = ActorKeyKey::new(namespace.to_owned(), name.to_owned(), key.to_owned());

	let output = ctx
		.epoxy()?
		.op(get::Input {
			key: epoxy_key.pack().to_vec(),
			mode: ReadMode::LatestReachable,
		})
		.await?;
	let Some(value) = output.value.and_then(|committed| committed.value) else {
		return Ok(None);
	};

	let entry = epoxy_key.deserialize(value)?;
	let reservation_id = Id::from_slice(&entry.reservation_id)?;

	Ok(Some(reservation_id))
}

pub(crate) enum ReserveResult {
	Reserved {},
	AlreadyReserved { reservation_id: Id },
}

pub(crate) async fn reserve(
	ctx: &EngineContext,
	namespace: String,
	name: String,
	key: String,
) -> Result<ReserveResult> {
	let digest = key_digest(b"rivet.actor.key.reservation.v1", &namespace, &name, &key);
	let mut bytes = [0; 16];
	bytes.copy_from_slice(&digest[..16]);
	let reservation_id = Id::v1(uuid::Uuid::from_bytes(bytes), ctx.config().dc_label());

	// Reservations are immutable. An existing key can be recreated under its original
	// reservation without proposing another conditional write. A missing read still takes
	// the compare-and-set path below, which resolves concurrent first reservations.
	if let Some(existing) = get(ctx, &namespace, &name, &key).await? {
		return Ok(if existing == reservation_id {
			ReserveResult::Reserved {}
		} else {
			ReserveResult::AlreadyReserved {
				reservation_id: existing,
			}
		});
	}

	let epoxy_key = ActorKeyKey::new(namespace, name, key);
	let value = epoxy_key
		.serialize(wire::EpoxyActorByKeyEntry {
			reservation_id: reservation_id.as_bytes().into(),
		})?
		.to_vec();

	let proposal_res = ctx
		.epoxy()?
		.propose(Input {
			proposal: Proposal {
				commands: vec![Command {
					kind: CommandKind::CheckAndSetCommand(CheckAndSetCommand {
						key: epoxy_key.pack().to_vec(),
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
				Id::from_slice(&epoxy_key.deserialize(current.into())?.reservation_id)?;

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

fn key_digest(domain: &[u8], namespace: &str, name: &str, key: &str) -> [u8; 32] {
	use sha2::{Digest, Sha256};
	let mut hasher = Sha256::new();
	hasher.update(domain);
	hasher.update(namespace.as_bytes());
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
/// placement with the datacenter's label, and nothing else. No epoch or other per-lifetime
/// value is part of it.
///
/// The id is derived, not drawn, so every create of the key names the same actor: a create whose
/// reservation never landed is adopted by the next create of the key, and concurrent creates
/// converge.
///
/// **Contract:** the id bytes are the key's placement, so the id alone finds the actor's state
/// volume, its database volumes and its Epoxy key volume, all on one tablet. Routing by id needs
/// no lookup, and the hot paths (create, wake, gateway routing, and SQLite reads and writes)
/// keep every operation local. See [`crate::keys::partition::key_placement`].
pub fn keyed_actor_id(datacenter_label: u16, namespace: &str, name: &str, key: &str) -> Id {
	let placement = crate::keys::partition::key_placement(namespace, name, key);
	Id::v1(uuid::Uuid::from_bytes(placement), datacenter_label)
}
