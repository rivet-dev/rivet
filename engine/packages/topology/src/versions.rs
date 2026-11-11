//! Version selection from advertised family ranges. Every heartbeat lists, per family, the
//! versions its binary reads and the highest it writes. Persisted data and payloads that a
//! receiver stores or forwards verbatim use the minimum `maxRead` over live storage workers,
//! capped by the writer's own `maxWrite`. A message one known worker decodes and drops uses
//! that peer's range alone.
//!
//! Stateless workers are not counted, because discovery never lists them. Most receive
//! replies encoded at their own advertised range, but a stateless engine that hosts a volume
//! decodes that volume's filesystem entries itself, so a newer entry version is safe only once
//! every stateless worker reads it too: roll stateless workers before storage workers.

use rivet_error::RivetError;
use serde::{Deserialize, Serialize};

pub use rivet_data::families::{VersionFamily, compiled_versions};
use rivet_data::families::{advertised, legacy_decoded_max_read};
pub use rivet_data::generated::topology_worker_heartbeat_v1::FamilyVersions;

/// No storage listing has completed in this process, or the last one named a storage worker no
/// listing has observed, so the readers of persisted data are unknown. Retry after the next
/// listing; discovery relists promptly while it lasts.
#[derive(RivetError, Debug, Serialize, Deserialize)]
#[error(
	"topology",
	"version_unknown",
	"The storage workers that read persisted data are unknown.",
	"Cannot select a version for family {family}: no storage listing has observed every listed storage worker yet."
)]
pub struct VersionUnknown {
	pub family: u16,
}

/// The selected version is outside what this binary can write: a live storage worker reads
/// only versions this binary no longer writes, or this binary does not know the family.
#[derive(RivetError, Debug, Serialize, Deserialize)]
#[error(
	"topology",
	"version_unavailable",
	"No version of this family is both readable by every storage worker and writable here.",
	"No version of family {family} is readable by every storage worker and writable by this binary (selected {selected}, writes {min_write} through {max_write})."
)]
pub struct VersionUnavailable {
	pub family: u16,
	pub selected: u16,
	pub min_write: u16,
	pub max_write: u16,
}

/// A peer cannot decode any version of a family that this binary writes. The caller's state
/// machine decides whether to retry elsewhere; transports never retry.
#[derive(RivetError, Debug, Serialize, Deserialize)]
#[error(
	"topology",
	"unsupported_version",
	"The peer does not read any version of this family that this binary writes.",
	"The peer reads family {family} versions {peer_min_read} through {peer_max_read}, but this binary writes {min_write} through {max_write}."
)]
pub struct UnsupportedVersion {
	pub family: u16,
	pub peer_min_read: u16,
	pub peer_max_read: u16,
	pub min_write: u16,
	pub max_write: u16,
}

/// The lowest version this binary can still write. Envelopes encode every version they
/// decode, so it equals `minRead`.
fn min_write(own: &FamilyVersions) -> u16 {
	own.min_read
}

/// Per family, the minimum `maxRead` over the storage workers that still read persisted data.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReadFloor {
	/// Active and Draining storage workers counted.
	pub readers: usize,
	/// Families advertised by at least one counted worker.
	pub max_read: std::collections::BTreeMap<u16, u16>,
}

/// One storage worker counted by the read floor: its advertised ranges and build version.
#[derive(Clone, Copy, Debug)]
pub struct Reader<'a> {
	pub versions: &'a [FamilyVersions],
	/// Read only by [`legacy_family_floor`], to tell `pegboard.actor` readers on 3.0.2 from those
	/// on 3.0.3 through 3.0.7. Drop it with that rule.
	pub build_version: &'a str,
}

impl ReadFloor {
	/// Computes the floor over each counted worker's decoded versions. A worker that does not
	/// advertise a family predates it and reads none of its versions.
	pub fn new<'a>(readers: impl Iterator<Item = Reader<'a>> + Clone) -> Self {
		let mut families: Vec<u16> = readers
			.clone()
			.flat_map(|reader| reader.versions.iter().map(|range| range.family))
			.collect();
		families.sort_unstable();
		families.dedup();

		let max_read = families
			.into_iter()
			.map(|family| (family, legacy_family_floor(family, readers.clone())))
			.collect();
		Self {
			readers: readers.count(),
			max_read,
		}
	}

	/// The highest version every counted reader decodes, or `None` with no readers.
	pub fn max_read(&self, family: VersionFamily) -> Option<u16> {
		if self.readers == 0 {
			return None;
		}
		Some(self.max_read.get(&family.0).copied().unwrap_or(0))
	}
}

/// The minimum version of `family` the readers decode.
///
/// Without the legacy rule this is the minimum advertised `maxRead`. `pegboard.actor` readers on
/// 3.0.3 through 3.0.7 advertise 3 but decode v4's layout under that header, so
/// [`legacy_decoded_max_read`] counts them as 4 and a reader on 3.0.2 as 3 (see
/// `rivet_data::versioned::legacy_v3` for the walkthrough). A fleet is assumed to upgrade from
/// one release, so it never mixes 3.0.2 with 3.0.3 through 3.0.7; with a 3.0.2 reader live, the
/// released layout is selected.
///
/// Exists only for readers on 3.0.2 through 3.0.7. Without it, a fleet of 3.0.3 through 3.0.7
/// readers selects version 3 and they misread the released layout. Delete it once no reader
/// older than the first release with `pegboard.actor` v5 can be live, and compute the plain
/// minimum.
fn legacy_family_floor<'a>(family: u16, readers: impl Iterator<Item = Reader<'a>>) -> u16 {
	readers
		.map(|reader| {
			reader
				.versions
				.iter()
				.find(|range| range.family == family)
				.map_or(0, |range| {
					legacy_decoded_max_read(range, reader.build_version)
				})
		})
		.min()
		.unwrap_or(0)
}

/// The version to write for persisted `family` data: the read floor capped by `own`
/// `maxWrite`. With no storage readers listed, nothing reads the data yet, so this binary's
/// own `maxWrite` applies.
pub fn select(
	own: Option<&FamilyVersions>,
	floor: &ReadFloor,
	family: VersionFamily,
) -> anyhow::Result<u16> {
	let Some(own_range) = own else {
		return Err(VersionUnavailable {
			family: family.0,
			selected: 0,
			min_write: 0,
			max_write: 0,
		}
		.build());
	};

	let selected = match floor.max_read(family) {
		Some(max_read) => own_range.max_write.min(max_read),
		None => own_range.max_write,
	};
	if selected < min_write(own_range) {
		return Err(VersionUnavailable {
			family: family.0,
			selected,
			min_write: min_write(own_range),
			max_write: own_range.max_write,
		}
		.build());
	}
	Ok(selected)
}

/// The lowest version `own` writes for `family`, for a writer that cannot see the read floor.
/// Fails with `topology.version_unavailable` when this binary does not know the family.
pub fn lowest_write(own: Option<&FamilyVersions>, family: VersionFamily) -> anyhow::Result<u16> {
	match own {
		Some(own_range) => Ok(min_write(own_range)),
		None => Err(VersionUnavailable {
			family: family.0,
			selected: 0,
			min_write: 0,
			max_write: 0,
		}
		.build()),
	}
}

/// Chooses the version a writer encodes a family's payloads or persisted data at. Writers take
/// every encode version from here instead of the compiled one, so a rolling deploy never
/// writes a version that a live storage worker cannot read.
pub trait WriteVersions: Send + Sync + 'static {
	/// The version to encode `family` at now. A refusal means no version is safe to write.
	fn write_version(&self, family: VersionFamily) -> anyhow::Result<u16>;

	/// Whether `write_version` already rests on a completed storage listing rather than a
	/// fallback taken before the first one. Required, so a wrapper cannot silently skip the
	/// wait: an implementation whose versions never depend on a listing answers `true`.
	fn floor_listed(&self) -> bool;

	/// Waits until `floor_listed` holds or `deadline_boot_ms` passes, answering whether it
	/// holds. Opens that select versions call it once before their first write, so a process
	/// that just started does not write the lowest version while its first listing is in
	/// flight. An implementation whose versions never depend on a listing is ready at once.
	fn wait_for_read_floor(
		&self,
		deadline_boot_ms: u64,
	) -> futures_util::future::BoxFuture<'_, anyhow::Result<bool>>;
}

/// Chooses the version of a message to one known worker that decodes, acts on and drops it,
/// such as a volume request to a tablet maintainer. Reads memory only, so a sender may call it
/// per message.
pub trait PeerVersions: Send + Sync + 'static {
	/// The version to encode a message of `family` to `worker` at: `min(own maxWrite, worker's
	/// maxRead)` when the in-memory view of the fleet knows `worker`, and otherwise the
	/// family's write version, the fleet's read floor or this binary's lowest write version.
	/// Fails when `worker` reads nothing of `family` this binary writes.
	fn peer_version(
		&self,
		family: VersionFamily,
		worker: &crate::worker::WorkerId,
	) -> anyhow::Result<u16>;

	/// The retry hint of a message refused because the version `worker` reads cannot carry it:
	/// the prompt relist interval, after which the view may show the worker upgraded.
	fn refresh_hint_ms(&self) -> u64;
}

/// The version for a message to one peer that decodes, acts on and drops it:
/// `min(own maxWrite, peer maxRead)`, refused when the peer reads nothing this binary writes.
pub fn for_peer(
	own: &[FamilyVersions],
	peer: &[FamilyVersions],
	family: VersionFamily,
) -> anyhow::Result<u16> {
	let own_range = advertised(own, family);
	let peer_range = advertised(peer, family);
	let (min_write, max_write) =
		own_range.map_or((0, 0), |range| (min_write(range), range.max_write));
	let (peer_min_read, peer_max_read) =
		peer_range.map_or((0, 0), |range| (range.min_read, range.max_read));

	let version = max_write.min(peer_max_read);
	if own_range.is_none() || peer_range.is_none() || version < min_write.max(peer_min_read) {
		return Err(UnsupportedVersion {
			family: family.0,
			peer_min_read,
			peer_max_read,
			min_write,
			max_write,
		}
		.build());
	}
	Ok(version)
}

/// A family another storage worker would write at a version this binary cannot read.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Incompatibility {
	pub family: u16,
	/// The highest version a writer counting only the others could select.
	pub selected: u16,
	/// `None` when this binary does not know the family.
	pub own_range: Option<(u16, u16)>,
}

/// Finds a family that a writer counting only the other live storage workers could select at
/// a version this binary does not read: their minimum `maxRead`. Writers that listed after this worker registered
/// count it; writers that listed before counted only workers it now also sees, so this view
/// bounds what any of them can select.
///
/// External message families, which this binary knows as `FamilyInfo::external`, are skipped:
/// nothing persists them, so reading an older version of one puts no stored data at risk. A
/// family this binary does not know cannot be told apart and still counts.
pub fn incompatibility(
	own: &[FamilyVersions],
	others: &[&[FamilyVersions]],
) -> Option<Incompatibility> {
	let mut families: Vec<u16> = others
		.iter()
		.flat_map(|versions| versions.iter().map(|range| range.family))
		.collect();
	families.sort_unstable();
	families.dedup();

	for family in families {
		let external = VersionFamily(family)
			.info()
			.is_some_and(|info| info.external);
		if external {
			continue;
		}

		// The floor alone bounds what any writer selects, including stateless writers whose
		// own maxWrite is not listed here.
		let mut selected = u16::MAX;
		for versions in others {
			let range = versions.iter().find(|range| range.family == family);
			selected = selected.min(range.map_or(0, |range| range.max_read));
		}
		if selected == 0 {
			continue;
		}

		let own_range = own
			.iter()
			.find(|range| range.family == family)
			.map(|range| (range.min_read, range.max_read));
		let readable =
			own_range.is_some_and(|(min_read, max_read)| (min_read..=max_read).contains(&selected));
		if !readable {
			return Some(Incompatibility {
				family,
				selected,
				own_range,
			});
		}
	}
	None
}
