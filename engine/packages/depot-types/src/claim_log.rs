//! Claim log names: a volume's name tuple extended by one element per claim log.
//!
//! A claim log is an ordinary volume log whose name is the base volume's name tuple with
//! `[CLAIM_LOG, claim_log]` appended, under the same partition key and incarnation. Claim log
//! 0 keeps the base key, so every single-writer volume is unchanged byte for byte. The tuple
//! codec's subspace range (`packed + 0x00` through `packed + 0xff`) holds every key that
//! extends the base name, so one prefix range over names finds a volume's claim logs, and the
//! trailing element is parsed to tell a claim log from an unrelated longer name. An unnamed
//! volume has no tuple to extend and cannot have claim logs.
//!
//! The range is a byte range over packed names. `VolumeLogKey` orders names by their BARE
//! length prefix first, so in a map keyed by `VolumeLogKey` a volume's claim logs are
//! contiguous per name length, not in one run after the base.

use anyhow::{Result, ensure};
use foundationdb_tuple::{self as tuple, Element, TupleDepth, TupleUnpack};
use rivet_keys::keys::{codec::BinaryComponent, tags::partition::CLAIM_LOG};
use std::ops::Range;

use crate::VolumeKey;

pub use rivet_data::ROOT_CLAIM_LOG;

/// The name of claim log `claim_log` of the volume whose packed name tuple is `base`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClaimLogName {
	/// The base volume's packed name tuple.
	pub base: Vec<u8>,
	/// Never `ROOT_CLAIM_LOG`: claim log 0 is the base name itself.
	pub claim_log: u16,
}

/// Every element of one packed tuple with the offset it starts at.
struct Elements<'a>(Vec<(usize, Element<'a>)>);

impl<'de> TupleUnpack<'de> for Elements<'de> {
	fn unpack(input: &'de [u8], depth: TupleDepth) -> tuple::PackResult<(&'de [u8], Self)> {
		let mut remaining = input;
		let mut elements = Vec::new();
		// At depth 0 `Element::unpack` gathers the whole input into one tuple, so each
		// element is read as a nested one, which reads exactly one element.
		let depth = depth.increment();
		while !remaining.is_empty() {
			let offset = input.len() - remaining.len();
			let (rest, element) = Element::unpack(remaining, depth)?;
			elements.push((offset, element));
			remaining = rest;
		}
		Ok((remaining, Self(elements)))
	}
}

fn elements(name: &[u8]) -> Result<Vec<(usize, Element<'_>)>> {
	let parsed: Elements<'_> = tuple::unpack(name)?;
	Ok(parsed.0)
}

impl ClaimLogName {
	/// Names claim log `claim_log` of the volume whose packed name tuple is `base`. Refuses
	/// claim log 0, which keeps the base name, and a base that is not a packed tuple, which
	/// no extension could be parsed back out of.
	pub fn new(base: impl Into<Vec<u8>>, claim_log: u16) -> Result<Self> {
		ensure!(
			claim_log != ROOT_CLAIM_LOG,
			"claim log 0 keeps the base name"
		);
		let base = base.into();
		elements(&base)?;
		Ok(Self { base, claim_log })
	}

	/// The packed name: the base tuple followed by the claim log element.
	pub fn pack(&self) -> Vec<u8> {
		let mut out = self.base.clone();
		BinaryComponent::pack_into(&(CLAIM_LOG as u64), &mut out);
		self.claim_log.pack_into(&mut out);
		out
	}

	/// Parses a claim log name. A base name, a name whose trailing element is another tag and
	/// a noncanonical encoding all fail.
	pub fn unpack(name: &[u8]) -> Result<Self> {
		let elements = elements(name)?;
		let [.., (offset, tag), (_, claim_log)] = elements.as_slice() else {
			anyhow::bail!("a claim log name has at least two elements");
		};
		ensure!(
			matches!(tag, Element::Int(tag) if *tag == CLAIM_LOG as i64),
			"the name does not end in a claim log element"
		);
		let Element::Int(claim_log) = claim_log else {
			anyhow::bail!("the claim log element is not an integer");
		};
		let claim_log = u16::try_from(*claim_log)?;
		ensure!(
			claim_log != ROOT_CLAIM_LOG,
			"claim log 0 keeps the base name"
		);
		let parsed = Self {
			base: name[..*offset].to_vec(),
			claim_log,
		};
		ensure!(parsed.pack() == name, "noncanonical claim log name");
		Ok(parsed)
	}
}

/// The key of claim log `claim_log` of `base`. Claim log 0 is `base` itself. Fails for an
/// unnamed volume, which cannot have claim logs, and for a base that already names a claim
/// log, since claims never nest by name.
pub fn claim_log_key(base: &VolumeKey, claim_log: u16) -> Result<VolumeKey> {
	if claim_log == ROOT_CLAIM_LOG {
		return Ok(base.clone());
	}
	let Some(name) = &base.name else {
		anyhow::bail!("an unnamed volume has no claim logs");
	};
	ensure!(
		is_claim_log(base).is_none(),
		"a claim log key cannot be extended again"
	);
	Ok(VolumeKey::named(
		base.partition_key.to_vec(),
		ClaimLogName::new(name.to_vec(), claim_log)?.pack(),
	))
}

/// The base key and claim log of `key` when its name is a claim log name, or `None` for a
/// base key, an unnamed key or any other name.
pub fn is_claim_log(key: &VolumeKey) -> Option<(VolumeKey, u16)> {
	let name = key.name.as_ref()?;
	// Opens, loads and applies ask this of every volume, nearly none of them a claim log, so a
	// name whose tail cannot be a claim log element is refused before its tuple is parsed.
	if !ends_like_claim_log(name) {
		return None;
	}
	let parsed = ClaimLogName::unpack(name).ok()?;
	Some((
		VolumeKey::named(key.partition_key.to_vec(), parsed.base),
		parsed.claim_log,
	))
}

// The tag packs as a one-byte tuple integer, which the tail check below relies on.
const _: () = assert!(CLAIM_LOG > 0 && CLAIM_LOG <= u8::MAX as usize);

/// Whether `name` ends as a claim log name does: the packed claim log tag, then a positive
/// integer of one or two bytes. A name that passes still has to parse.
fn ends_like_claim_log(name: &[u8]) -> bool {
	const TAG: [u8; 2] = [0x15, CLAIM_LOG as u8];
	match name {
		[.., tag, value, 0x15, _] | [.., tag, value, 0x16, _, _] => [*tag, *value] == TAG,
		_ => false,
	}
}

/// The range of packed names that extend `base`'s name tuple. Every claim log of `base` has
/// its name in it, and so does any unrelated name that extends the tuple, which
/// `is_claim_log` tells apart. `None` for an unnamed volume, which has no claim logs.
pub fn claim_logs(base: &VolumeKey) -> Option<Range<Vec<u8>>> {
	let name = base.name.as_ref()?;
	let mut start = name.to_vec();
	let mut end = start.clone();
	start.push(0x00);
	end.push(0xff);
	Some(start..end)
}
