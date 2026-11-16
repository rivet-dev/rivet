//! Compatibility with the two `pegboard.actor` layouts that released binaries wrote under the
//! version 3 header. Everything here exists only for releases 3.0.2 through 3.0.7 and for records
//! they stored; the removal plan is at the end.
//!
//! # What happened
//!
//! Release 3.0.2 shipped `pegboard.actor` version 3. Release 3.0.3 then edited that released
//! schema in place to add V2 delegation instead of adding a version 4:
//!
//! - `Create` and `ActorCreateRequest` gained a trailing `forceActorV3: bool`.
//! - `Destroying.epoxyAlreadyReserved: optional<Id>` became `keyOwner: optional<KeyOwner>`, a
//!   union of `KeyOwnerDatacenter { reservationId }` and `KeyOwnerV2`.
//! - `ReserveEpoxyPhase`, `ActorPhase`, `TryStartOutcome` and `ActorStartNoticeKind` gained a
//!   variant for a key that belongs to V2.
//!
//! Releases 3.0.3 through 3.0.7 write that edited layout under the version 3 header, so two
//! incompatible layouts share one header. A create request shows the problem. A 3.0.2 worker
//! writes `init` and `waiter`; a 3.0.3 worker writes the same bytes plus one byte for
//! `forceActorV3`. Each reader trusts the header and decodes its own layout, so between the two
//! releases every create fails:
//!
//! - A 3.0.2 receiver reading a 3.0.3 request finishes the struct with a byte left over:
//!   `trailing bytes after the message`.
//! - A 3.0.3 receiver reading a 3.0.2 request runs out of input before the new field:
//!   `input ended inside a value at forceActorV3`.
//!
//! Stored actor state is affected the same way, through `Destroying` and the new variants.
//!
//! # The versions now
//!
//! - **v3** is restored to exactly what 3.0.2 shipped, because 3.0.2 readers decode only that
//!   under header 3 and it is the released definition of version 3.
//! - **v4** holds the 3.0.3 edit, which is what 3.0.3 through 3.0.7 actually read and write. Those
//!   releases accept that layout only under header 3, so this binary writes v4 under header 3
//!   ([`legacy_header_version`]).
//! - **v5** is the same layout as v4 under its own header. Once every reader runs this binary or
//!   newer, writers select v5, so steady-state records and messages carry an unambiguous header and
//!   decode at the latest version without conversion.
//!
//! # How writers choose
//!
//! Writers take the `pegboard.actor` version from the topology read floor, the highest version
//! every live storage worker decodes. 3.0.2 and 3.0.3 through 3.0.7 advertise identical ranges, so
//! [`crate::families::legacy_decoded_max_read`] tells them apart by build version: a reader that
//! advertises version 3 on 3.0.0 through 3.0.2 decodes the released layout and counts as 3, and
//! any other build, including a custom or unparseable one, decodes v4's layout and counts as 4.
//!
//! A fleet upgrades from one release, so it never mixes 3.0.2 with 3.0.3 through 3.0.7, and that
//! mix is unsupported. Such a fleet already failed before this change; here a live 3.0.2 reader
//! makes writers select released v3, which the 3.0.3 through 3.0.7 readers then misread.
//!
//! While 3.0.2 readers are live, a value released v3 cannot hold, such as a create with
//! `forceActorV3`, is refused with [`LegacyV3Unrepresentable`] instead of being written, and the
//! create path turns that into pegboard's retryable `actor.migrate_from_v2_unsupported`, the
//! refusal a host on this binary would give.
//!
//! # How readers decode header 3
//!
//! [`legacy_decode_v3_header`] decodes a header 3 payload in both layouts. Decoding is strict:
//! bools must be 0 or 1, union and optional tags must be defined, varints must be canonical and no
//! byte may remain. When only one layout accepts the payload, it is used. When both accept it, the
//! released value upgraded to v4 must equal the edited value, which holds whenever the payload uses
//! none of the 3.0.3 additions; otherwise the payload is refused as ambiguous rather than guessed.
//!
//! The changes cannot silently collide in practice. Appended union variants are a superset, so a
//! 3.0.2 payload means the same thing in both layouts. `forceActorV3` makes the 3.0.3 encoding one
//! byte longer, so a payload that ends with the create fits only one layout. For `Destroying`,
//! `Id` is length-prefixed `data`: a 3.0.2 reservation of 16 bytes encodes as `1, 16, ...`, which
//! the 3.0.3 layout reads as union tag 16, an undefined tag, so it fails. Only a reservation ID of
//! zero or one byte could decode in both, and the equality check refuses that case.
//!
//! The bytes 3.0.3 wrote are pinned in `tests/fixtures/wire/pegboard_actor_3_0_3`, and tests check
//! that this binary decodes them and writes them back byte for byte at version 4. Envelopes that
//! fixture set predates are pinned from v4's schema, which is the 3.0.3 schema. CI also fails
//! any change to a schema version that an OSS release shipped (`scripts/schemas`), so this cannot
//! recur.
//!
//! # Removal
//!
//! Everything here can be deleted once no supported binary is older than the first release that
//! carries v5, and no stored record still has a version 3 or 4 header. Then delete this module,
//! [`crate::families::legacy_decoded_max_read`] and the topology read floor's legacy check, drop
//! the `seal` override and header 3 branch from the envelope macros, and let vbare's default seal
//! and decode run. Removing the v3 and v4 decoders themselves follows the normal rule for dropping
//! old schema versions.

/// Released 3.0.3 through 3.0.7 decode v4's layout only under the version 3 header. Without this,
/// this binary would write header 4, which they reject as an unsupported version. Delete it with
/// this module, once no reader older than the first release with v5 can be live.
pub const fn legacy_header_version(version: u16) -> u16 {
	if version == 4 { 3 } else { version }
}

/// A value that released v3 cannot hold, such as a create with `forceActorV3` or anything
/// delegated to V2, refused when writing version 3 for 3.0.2 readers. Exists only for that write
/// path; without it the refusal is an unstructured conversion error that callers cannot tell from
/// a bug. Callers that send such values map it to their own retryable refusal, such as pegboard's
/// `actor.migrate_from_v2_unsupported`. Delete it with this module, once no reader of released v3
/// can be live.
#[derive(Debug)]
pub struct LegacyV3Unrepresentable {
	/// What could not be written, such as "a create that forces actor v3".
	pub value: &'static str,
}

impl std::fmt::Display for LegacyV3Unrepresentable {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{} has no released pegboard.actor v3 form, which readers on 3.0.2 decode",
			self.value
		)
	}
}

impl std::error::Error for LegacyV3Unrepresentable {}

/// A header 3 payload that both layouts decode, to different values. Exists because 3.0.2 and
/// 3.0.3 through 3.0.7 wrote different layouts under that header; without it such a payload would
/// be read in whichever layout happened to be tried first. Delete it with this module, once no
/// stored record has a version 3 header.
#[derive(Debug)]
pub struct LegacyAmbiguousV3Layout {
	pub envelope: &'static str,
}

impl std::fmt::Display for LegacyAmbiguousV3Layout {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"a pegboard.actor {} payload under the version 3 header decodes as both the released v3 layout and the 3.0.3 layout with different values; refusing to guess",
			self.envelope
		)
	}
}

impl std::error::Error for LegacyAmbiguousV3Layout {}

/// A header 3 payload that neither layout decodes. Exists so the error names both attempts that
/// decoding a header 3 payload from 3.0.2 through 3.0.7 makes. Delete it with this module, once no
/// stored record has a version 3 header.
#[derive(Debug)]
pub struct LegacyInvalidV3Payload {
	pub envelope: &'static str,
	pub released: vbare::DecodeError,
	pub edited: vbare::DecodeError,
}

impl std::fmt::Display for LegacyInvalidV3Payload {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"a pegboard.actor {} payload under the version 3 header decodes in neither the released v3 layout ({}) nor the 3.0.3 layout ({})",
			self.envelope, self.released, self.edited
		)
	}
}

impl std::error::Error for LegacyInvalidV3Payload {}

/// Which of the two layouts a header 3 payload was decoded in. Delete it with this module, once
/// no stored record has a version 3 header.
pub enum LegacyV3Layout<R, E> {
	/// Released v3, written by 3.0.2 and by this binary for 3.0.2 readers.
	Released(R),
	/// v4's layout, written by 3.0.3 through 3.0.7 and by this binary for their readers.
	Edited(E),
}

/// Decodes a payload written under the version 3 header in both layouts, by the rule in this
/// module's documentation. Exists because 3.0.2 and 3.0.3 through 3.0.7 wrote different layouts
/// under that header and stored records of both; without it, a record from one of them fails to
/// decode or, worse, decodes wrongly. Delete it with this module, once no stored record or live
/// reader uses the version 3 header.
pub fn legacy_decode_v3_header<R, E>(
	envelope: &'static str,
	body: vbare::Bytes,
	upgrade: fn(R) -> anyhow::Result<E>,
) -> Result<LegacyV3Layout<R, E>, vbare::DecodeError>
where
	R: vbare::Body + Clone,
	E: vbare::Body + PartialEq,
{
	let released = R::from_body(body.clone());
	let edited = E::from_body(body);
	match (released, edited) {
		(Ok(released), Err(_)) => Ok(LegacyV3Layout::Released(released)),
		(Err(_), Ok(edited)) => Ok(LegacyV3Layout::Edited(edited)),
		(Ok(released), Ok(edited)) => {
			let upgraded = upgrade(released.clone()).map_err(vbare::DecodeError::conversion)?;
			if upgraded == edited {
				Ok(LegacyV3Layout::Released(released))
			} else {
				Err(vbare::DecodeError::conversion(
					LegacyAmbiguousV3Layout { envelope }.into(),
				))
			}
		}
		(Err(released), Err(edited)) => Err(vbare::DecodeError::conversion(
			LegacyInvalidV3Payload {
				envelope,
				released,
				edited,
			}
			.into(),
		)),
	}
}

/// Writes `version`'s header and body, putting version 4 under the version 3 header per
/// [`legacy_header_version`]. A converted body is encoded before the header, so a refused
/// downgrade writes nothing. Exists only for that header mapping; without it 3.0.3 through 3.0.7
/// could not read what this binary writes for them. Delete it with this module and let the
/// envelopes use vbare's default seal.
pub fn legacy_seal<T: vbare::Versioned, S: vbare::Sink>(
	value: &T::Latest,
	version: u16,
	out: &mut S,
) -> Result<(), vbare::EncodeError> {
	if version == T::LATEST {
		out.put_u16_le(version);
		vbare::Body::put_body(value, out);
		return Ok(());
	}
	let mut body = Vec::new();
	T::encode_body(value, version, &mut body)?;
	out.put_u16_le(legacy_header_version(version));
	out.put_slice(&body);
	Ok(())
}
