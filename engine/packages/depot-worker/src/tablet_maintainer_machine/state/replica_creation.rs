//! Creation evidence kept in a volume's replica states.
//!
//! A create writes the new incarnation's immutable record parts, its base chain, retention
//! policy and ordering epoch, into the state of every replica it creates. The
//! create already waits for that quorum, so the evidence costs no extra write, and a later
//! maintainer that finds the volume before its first record reached cold can still open it
//! with its base chain and policy. Every load that creates a replica of an existing volume,
//! such as a repair or a learner, copies the evidence on.
//!
//! A stored row of the same incarnation always wins over replica evidence: materialize
//! clears the base chain and retention changes are written to the row.
//!
//! The ordering epoch says which incarnation of a name is newer. Epochs come from one
//! counter per tablet whose blocks the maintainer log reserves before use, so an incarnation
//! created after another always has a higher one, across restarts and splits. Only the epoch
//! stored at creation orders incarnations: an older incarnation's replicas can carry later
//! claim epochs, such as its delete's fence.

use super::{LoadedVolume, MaintainerState};
use depot_types::VolumeLogKey;
use rivet_data::generated::{
	depot_maintainer_log_v1::VolumeCreation, shared_depot_fs_v1::FsVolumeRecord,
};
use vbare::Versioned;

/// Encodes a creation's evidence for `LoadVolumeRequest.creation` at `version`, the grant's
/// `depot.maintainer_log` version. Shards keep it opaque and later maintainers decode it.
pub(crate) fn encode(row: &FsVolumeRecord, version: u16) -> Option<vbare::Bytes> {
	let creation = VolumeCreation {
		incarnation_epoch: row.incarnation_epoch,
		base_chain: row.base_chain.clone(),
		retention: row.retention.clone(),
	};
	match rivet_data::versioned::DepotVolumeCreationData::serialize_with_embedded_version(
		&creation, version,
	) {
		Ok(bytes) => Some(bytes.into()),
		Err(error) => {
			tracing::warn!(
				?error,
				"encoding a volume's creation evidence failed; replicas carry none"
			);
			None
		}
	}
}

/// Decodes replica creation evidence. Undecodable evidence counts as absent.
pub(crate) fn decode(bytes: &[u8]) -> Option<VolumeCreation> {
	rivet_data::versioned::DepotVolumeCreationData::deserialize_with_embedded_version(bytes).ok()
}

impl LoadedVolume {
	/// The creation evidence a member replica holds, if any. Every replica of one
	/// incarnation holds the same evidence.
	pub(crate) fn replica_creation(&self) -> Option<VolumeCreation> {
		self.members
			.values()
			.find_map(|replica| replica.creation.as_deref().and_then(decode))
	}

	/// The evidence a load that creates a replica of this volume carries: this lifetime's
	/// creation, else what the members already hold, else the stored record a cold open
	/// starts from, so even the replicas of a woken volume carry its ordering epoch.
	/// The creation evidence for a load of a member, encoded at `version` when this
	/// maintainer encodes it rather than passing on a member's.
	pub(crate) fn creation_evidence(&self, version: u16) -> Option<vbare::Bytes> {
		if let Some(creation) = &self.creation {
			return encode(&creation.row, version);
		}
		self.members
			.values()
			.find_map(|replica| replica.creation.clone())
			.or_else(|| {
				self.stored_row
					.as_deref()
					.or(self.unacknowledged_cold_open.as_deref())
					.and_then(|row| encode(row, version))
			})
	}
}

/// A loaded incarnation of `key`'s name that is newer than `key`'s and whose creation was
/// acknowledged, which makes `key`'s obsolete. A creation on fewer replicas than a quorum was
/// never acknowledged and replaces nothing. An incarnation whose epoch this maintainer does
/// not know yet has no newer one: a recovery reads its stored record first.
pub(crate) fn newer(state: &MaintainerState, key: &VolumeLogKey) -> Option<VolumeLogKey> {
	let epoch = known_epoch(state.loaded_volumes.get(key)?)?;
	state
		.loaded_volumes
		.range(super::incarnations(&key.0.volume_key))
		.filter(|(other, _)| *other != key)
		.find(|(_, volume)| {
			known_epoch(volume).is_some_and(|other| other > epoch)
				&& creation_acknowledged(state, volume)
		})
		.map(|(other, _)| other.clone())
}

/// The incarnation's ordering epoch, when its stored record, this lifetime's creation or its
/// replicas' creation evidence carries one.
pub(crate) fn known_epoch(volume: &LoadedVolume) -> Option<u64> {
	if let Some(row) = volume.stored_row.as_deref() {
		return Some(row.incarnation_epoch);
	}
	if let Some(creation) = &volume.creation {
		return Some(creation.row.incarnation_epoch);
	}
	if let Some(row) = volume.unacknowledged_cold_open.as_deref() {
		return Some(row.incarnation_epoch);
	}
	volume
		.replica_creation()
		.map(|creation| creation.incarnation_epoch)
}

/// The highest ordering epoch this maintainer knows for any incarnation of `volume_key`.
/// A new incarnation of the name is created above it.
pub(crate) fn highest_epoch(state: &MaintainerState, volume_key: &depot_types::VolumeKey) -> u64 {
	state
		.loaded_volumes
		.range(super::incarnations(volume_key))
		.filter_map(|(_, volume)| known_epoch(volume))
		.max()
		.unwrap_or(0)
}

/// Whether `volume`'s creation was acknowledged: this lifetime acknowledged it, its record is
/// stored, or a quorum of its replicas holds creation evidence.
fn creation_acknowledged(state: &MaintainerState, volume: &LoadedVolume) -> bool {
	if let Some(creation) = &volume.creation {
		return creation.acknowledged;
	}
	volume.stored_row.is_some() || acknowledged(state, volume)
}

/// Whether `volume`'s creation was acknowledged: a quorum of its replicas holds creation
/// evidence. A create writes it only to the replicas it creates, so a quorum of it means the
/// create reached its quorum; later loads of an existing incarnation, such as a cold open or a
/// delete's tombstone, also write it, and that incarnation was acknowledged long before.
pub(crate) fn acknowledged(state: &MaintainerState, volume: &LoadedVolume) -> bool {
	super::membership::quorum(&state.tablet.replica_set, |worker| {
		volume
			.members
			.get(worker)
			.is_some_and(|replica| replica.creation.is_some())
	})
}
