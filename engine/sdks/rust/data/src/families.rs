//! Version families: one per top-level vbare schema, each with a stable ID from
//! `families.list`. A binary advertises, per family, the versions it reads and the highest it
//! writes. Persisted data records a high-water mark per family, and a binary refuses data whose
//! mark exceeds what it reads.
//!
//! A family listed with a version range is an external message protocol (`FamilyInfo::external`):
//! peers pick its version per message, nothing persists it, and it never fences a storage
//! worker.

use serde::{Deserialize, Serialize};

use crate::generated::{
	shared_depot_types_v1::VersionMark, topology_worker_heartbeat_v1::FamilyVersions,
};

/// The stable ID of one version family. IDs are permanent and carried in heartbeats and
/// persisted marks, so a binary can name families that another binary does not know.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VersionFamily(pub u16);

/// This binary's support for one family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FamilyInfo {
	pub family: VersionFamily,
	/// The schema identifier, such as `depot.log_shard`.
	pub schema: &'static str,
	/// The oldest version this binary decodes.
	pub min_read: u16,
	/// The newest version this binary decodes.
	pub max_read: u16,
	/// The newest version this binary encodes. A new binary must still read and write the
	/// previous version, so one rolling deploy can move the fleet up.
	pub max_write: u16,
	/// A message protocol whose schema lives outside the data directory, listed with its
	/// version range in `families.list`. Nothing persists it, so a storage worker that reads
	/// an older version of it holds no data it cannot decode, and the storage incompatibility
	/// fence ignores it.
	pub external: bool,
}

/// The family a versioned envelope's payloads belong to. A payload that crosses between
/// binaries during a rolling deploy is encoded at its family's negotiated version, the highest
/// every live storage worker reads, never at the compiled latest.
pub trait VersionedFamily: vbare::Versioned {
	/// `None` only for an envelope whose protocol pins its single version out of band.
	const FAMILY: Option<VersionFamily>;
}

include!(concat!(env!("OUT_DIR"), "/families.rs"));

impl VersionFamily {
	/// This binary's support for the family, or `None` when the family is newer than it.
	pub fn info(self) -> Option<&'static FamilyInfo> {
		FAMILIES
			.binary_search_by_key(&self, |info| info.family)
			.ok()
			.map(|index| &FAMILIES[index])
	}

	/// The schema identifier for logs and errors, or `"unknown"`.
	pub fn schema(self) -> &'static str {
		self.info().map_or("unknown", |info| info.schema)
	}
}

impl std::fmt::Display for VersionFamily {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} ({})", self.schema(), self.0)
	}
}

impl FamilyInfo {
	/// The advertised form of this binary's support for the family.
	pub fn versions(&self) -> FamilyVersions {
		FamilyVersions {
			family: self.family.0,
			min_read: self.min_read,
			max_read: self.max_read,
			max_write: self.max_write,
		}
	}
}

/// What this binary advertises in its topology heartbeat, ordered by family ID.
pub fn compiled_versions() -> Vec<FamilyVersions> {
	FAMILIES.iter().map(FamilyInfo::versions).collect()
}

/// Finds a family in an advertised list. Lists are short and sorted by family ID.
pub fn advertised(versions: &[FamilyVersions], family: VersionFamily) -> Option<&FamilyVersions> {
	versions.iter().find(|entry| entry.family == family.0)
}

/// The highest version of `range`'s family a reader decodes, as the version a writer must select
/// for it. See `crate::versioned::legacy_v3` for the full walkthrough.
///
/// An advertised range is exact except for `pegboard.actor` at version 3. Releases 3.0.0 through
/// 3.0.2 decode the released v3 layout under that header, while every later build that advertises
/// version 3 decodes only v4's layout under it, so it reads what a writer selects as version 4. The
/// two advertise identical ranges, so the reader's build version, which is otherwise only
/// diagnostic, tells them apart. Only a build version that parses as 3.0.0, 3.0.1 or 3.0.2, with or
/// without a prerelease suffix, counts as released v3; anything else, including a custom or
/// unparseable build, counts as the edited layout every build after 3.0.2 reads.
///
/// Exists only for readers on 3.0.2 through 3.0.7. Without it, a writer in a fleet of 3.0.3
/// through 3.0.7 readers selects version 3 and writes the released layout they misread. Delete it
/// once no reader older than the first release with `pegboard.actor` v5 can be live; then the
/// advertised `maxRead` is exact for every reader.
pub fn legacy_decoded_max_read(range: &FamilyVersions, build_version: &str) -> u16 {
	if range.family != VersionFamily::PEGBOARD_ACTOR.0 || range.max_read != 3 {
		return range.max_read;
	}
	if legacy_reads_released_v3(build_version) {
		3
	} else {
		crate::versioned::V2_DELEGATION_ACTOR_VERSION
	}
}

/// Whether `build_version` is 3.0.0, 3.0.1 or 3.0.2, the releases that read released
/// `pegboard.actor` v3, with or without a prerelease or build suffix. Used only by
/// [`legacy_decoded_max_read`] and deleted with it.
fn legacy_reads_released_v3(build_version: &str) -> bool {
	let core = build_version
		.split(|c| c == '-' || c == '+')
		.next()
		.unwrap_or_default();
	let parts: Vec<&str> = core.split('.').collect();
	let [major, minor, patch] = parts.as_slice() else {
		return false;
	};
	let (Ok(major), Ok(minor), Ok(patch)) = (
		major.parse::<u64>(),
		minor.parse::<u64>(),
		patch.parse::<u64>(),
	) else {
		return false;
	};
	major == 3 && minor == 0 && patch <= 2
}

/// Why persisted data cannot be opened by this binary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionAboveRead {
	pub family: VersionFamily,
	pub mark: u16,
	/// `None` when this binary does not know the family at all.
	pub max_read: Option<u16>,
}

impl std::fmt::Display for VersionAboveRead {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self.max_read {
			Some(max_read) => write!(
				f,
				"data holds family {} at version {}, but this binary reads at most version \
				 {max_read}; run a binary that reads it, since versions only move forward",
				self.family, self.mark
			),
			None => write!(
				f,
				"data holds family {} at version {}, which this binary does not know; run a \
				 binary that reads it, since versions only move forward",
				self.family, self.mark
			),
		}
	}
}

impl std::error::Error for VersionAboveRead {}

/// The recorded version of `family`, or `None` when no mark names it. Missing marks are
/// either data written before marks existed, which was version 1, or no data of the family.
pub fn mark_of(marks: &[VersionMark], family: VersionFamily) -> Option<u16> {
	marks
		.iter()
		.find(|mark| mark.family == family.0)
		.map(|mark| mark.version)
}

/// Checks that this binary reads every recorded mark. A mark for an unknown family refuses,
/// because a newer binary wrote data this one cannot interpret.
pub fn check_marks(marks: &[VersionMark]) -> Result<(), VersionAboveRead> {
	for mark in marks {
		let family = VersionFamily(mark.family);
		let max_read = family.info().map(|info| info.max_read);
		if max_read.is_none_or(|max_read| mark.version > max_read) {
			return Err(VersionAboveRead {
				family,
				mark: mark.version,
				max_read,
			});
		}
	}
	Ok(())
}

/// Checks the embedded vbare version header of a single-object record, which is that
/// record's mark, before decoding it, so a record from a newer binary fails with an explicit
/// error instead of a decode error.
pub fn check_embedded(family: VersionFamily, payload: &[u8]) -> Result<(), VersionAboveRead> {
	let Some(header) = payload.get(..2) else {
		return Ok(());
	};
	let version = u16::from_le_bytes([header[0], header[1]]);
	let max_read = family.info().map(|info| info.max_read);
	if max_read.is_none_or(|max_read| version > max_read) {
		return Err(VersionAboveRead {
			family,
			mark: version,
			max_read,
		});
	}
	Ok(())
}

/// Checks that a reader with the advertised `versions` reads every mark. Used before handing
/// data to another worker, such as granting a tablet.
pub fn readable_by(
	marks: &[VersionMark],
	versions: &[FamilyVersions],
) -> Result<(), VersionAboveRead> {
	for mark in marks {
		let family = VersionFamily(mark.family);
		let max_read = advertised(versions, family).map(|entry| entry.max_read);
		if max_read.is_none_or(|max_read| mark.version > max_read) {
			return Err(VersionAboveRead {
				family,
				mark: mark.version,
				max_read,
			});
		}
	}
	Ok(())
}

/// Raises the mark for `family` to `version`. Returns whether the marks changed, so the
/// caller knows it must persist them before the record they cover. An absent mark is
/// recorded even at version 1, so a binary that does not know the family refuses the data.
/// Marks stay sorted by family ID and never decrease.
pub fn raise_mark(marks: &mut Vec<VersionMark>, family: VersionFamily, version: u16) -> bool {
	match marks.binary_search_by_key(&family.0, |mark| mark.family) {
		Ok(index) => {
			if marks[index].version >= version {
				return false;
			}
			marks[index].version = version;
			true
		}
		Err(index) => {
			marks.insert(
				index,
				VersionMark {
					family: family.0,
					version,
				},
			);
			true
		}
	}
}

/// Sorts marks decoded from untrusted bytes by family ID and keeps the highest version of a
/// family listed twice, so `raise_mark` and `merge_marks` can rely on the order. Returns
/// whether anything changed.
pub fn normalize_marks(marks: &mut Vec<VersionMark>) -> bool {
	let sorted = marks.windows(2).all(|pair| pair[0].family < pair[1].family);
	if sorted {
		return false;
	}
	marks.sort_by(|a, b| a.family.cmp(&b.family).then(b.version.cmp(&a.version)));
	marks.dedup_by_key(|mark| mark.family);
	true
}

/// Merges `other` into `marks`, keeping the higher version of each family. Used when one
/// piece of data inherits another's contents, such as a split child.
pub fn merge_marks(marks: &mut Vec<VersionMark>, other: &[VersionMark]) -> bool {
	let mut changed = false;
	for mark in other {
		changed |= raise_mark(marks, VersionFamily(mark.family), mark.version);
	}
	changed
}
