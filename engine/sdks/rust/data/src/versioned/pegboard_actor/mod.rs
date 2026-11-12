//! Version envelopes for the actor state machine's state, timers, events and commands, and the
//! index, envoy, namespace and pool records stored beside them.
//!
//! Two layouts share the version 3 header because 3.0.3 edited released v3 in place. v3 is the
//! released layout, v4 the edited one, and v5 the same layout under its own header. Every piece
//! that exists only for those releases lives in [`legacy_v3`], which walks through the problem,
//! how writers and readers handle it, and when it can be deleted. v6 adds namespace and actor
//! forks and the backfill that moves ungrouped actors into their namespace's group.

pub mod legacy_v3;
mod pegboard_actor_v1_to_pegboard_actor_v2;
mod pegboard_actor_v2_to_pegboard_actor_v1;
mod pegboard_actor_v2_to_pegboard_actor_v3;
mod pegboard_actor_v3_to_pegboard_actor_v2;
mod pegboard_actor_v3_to_pegboard_actor_v4;
mod pegboard_actor_v4_to_pegboard_actor_v3;
mod pegboard_actor_v4_to_pegboard_actor_v5;
mod pegboard_actor_v5_to_pegboard_actor_v4;
mod pegboard_actor_v5_to_pegboard_actor_v6;
mod pegboard_actor_v6_to_pegboard_actor_v5;

use super::unchanged;
use crate::generated::{
	pegboard_actor_v1 as v1, pegboard_actor_v2 as v2, pegboard_actor_v3 as v3,
	pegboard_actor_v4 as v4, pegboard_actor_v5 as v5, pegboard_actor_v6 as v6,
};
use legacy_v3::{LegacyV3Layout, legacy_decode_v3_header, legacy_seal};
use pegboard_actor_v1_to_pegboard_actor_v2 as up;
use pegboard_actor_v2_to_pegboard_actor_v1 as down;
use pegboard_actor_v2_to_pegboard_actor_v3 as up23;
use pegboard_actor_v3_to_pegboard_actor_v2 as down32;
use pegboard_actor_v3_to_pegboard_actor_v4 as up34;
use pegboard_actor_v4_to_pegboard_actor_v3 as down43;
use pegboard_actor_v4_to_pegboard_actor_v5 as up45;
use pegboard_actor_v5_to_pegboard_actor_v4 as down54;

/// Upgrades a version 5 value through its `From` into version 6: generated for a type that is the
/// same at both versions, written in `pegboard_actor_v5_to_pegboard_actor_v6` for one that is not.
fn upgrade_v5_v6<V5, V6: From<V5>>(value: V5) -> anyhow::Result<V6> {
	Ok(value.into())
}

/// Downgrades a version 6 value through its [`super::Downgrade`], which refuses a value version 5
/// cannot carry.
fn downgrade_v6_v5<V6: super::Downgrade>(value: V6) -> anyhow::Result<V6::Older> {
	value.downgrade()
}

/// The envoy version read back from records written at a version that does not carry one.
/// Clients default to version 1 when none is configured, so such an envoy ranks with those.
pub const ENVOY_VERSION_BEFORE_VERSIONS: u32 = 1;

/// The first `pegboard.actor` version whose envoy records and active index state carry the
/// envoy's version.
pub const ENVOY_VERSION_ACTOR_VERSION: u16 = 2;

/// The first `pegboard.actor` version that can store V2 delegation: a forcing create, a key owned
/// or found in V2, and an actor delegated to V2.
pub const V2_DELEGATION_ACTOR_VERSION: u16 = 4;

/// Six versions of an actor envelope. The header 3 decode branch, the v4 passthrough in the
/// v3 upgrade and the `seal` override exist only for [`legacy_v3`] and go with it.
macro_rules! impl_versioned_v1_to_v6 {
	(
		$name:ident,
		$family:ident,
		$v1:ty,
		$v2:ty,
		$v3:ty,
		$v4:ty,
		$v5:ty,
		$v6:ty,
		$up12:path,
		$down21:path,
		$up23:path,
		$down32:path,
		$up34:path,
		$down43:path,
		$up45:path,
		$down54:path,
		$up56:path,
		$down65:path
	) => {
		impl_versioned_family!($name, $family);

		pub enum $name {
			V1($v1),
			V2($v2),
			V3($v3),
			V4($v4),
			V5($v5),
			V6($v6),
		}

		impl ::vbare::Versioned for $name {
			type Latest = $v6;
			const LATEST: u16 = 6;
			const UPGRADES: &'static [::vbare::Converter<Self>] = &[
				|value| match value {
					Self::V1(v) => Ok(Self::V2($up12(v)?)),
					_ => ::anyhow::bail!("expected version 1"),
				},
				|value| match value {
					Self::V2(v) => Ok(Self::V3($up23(v)?)),
					_ => ::anyhow::bail!("expected version 2"),
				},
				// Legacy: a header 3 payload in the edited layout decodes straight to v4.
				|value| match value {
					Self::V3(v) => Ok(Self::V4($up34(v)?)),
					Self::V4(v) => Ok(Self::V4(v)),
					_ => ::anyhow::bail!("expected version 3"),
				},
				|value| match value {
					Self::V4(v) => Ok(Self::V5($up45(v)?)),
					_ => ::anyhow::bail!("expected version 4"),
				},
				|value| match value {
					Self::V5(v) => Ok(Self::V6($up56(v)?)),
					_ => ::anyhow::bail!("expected version 5"),
				},
			];
			const DOWNGRADES: &'static [::vbare::Converter<Self>] = &[
				|value| match value {
					Self::V6(v) => Ok(Self::V5($down65(v)?)),
					_ => ::anyhow::bail!("expected version 6"),
				},
				|value| match value {
					Self::V5(v) => Ok(Self::V4($down54(v)?)),
					_ => ::anyhow::bail!("expected version 5"),
				},
				|value| match value {
					Self::V4(v) => Ok(Self::V3($down43(v)?)),
					_ => ::anyhow::bail!("expected version 4"),
				},
				|value| match value {
					Self::V3(v) => Ok(Self::V2($down32(v)?)),
					_ => ::anyhow::bail!("expected version 3"),
				},
				|value| match value {
					Self::V2(v) => Ok(Self::V1($down21(v)?)),
					_ => ::anyhow::bail!("expected version 2"),
				},
			];
			fn wrap_latest(value: Self::Latest) -> Self {
				Self::V6(value)
			}
			fn unwrap_latest(self) -> ::anyhow::Result<Self::Latest> {
				match self {
					Self::V6(value) => Ok(value),
					_ => ::anyhow::bail!("expected latest version"),
				}
			}
			fn decode_version(
				version: u16,
				body: ::vbare::Bytes,
			) -> ::std::result::Result<Self, ::vbare::DecodeError> {
				match version {
					1 => Ok(Self::V1(<$v1 as ::vbare::Body>::from_body(body)?)),
					2 => Ok(Self::V2(<$v2 as ::vbare::Body>::from_body(body)?)),
					// Legacy: two layouts share the version 3 header.
					3 => Ok(
						match legacy_decode_v3_header::<$v3, $v4>(stringify!($name), body, $up34)? {
							LegacyV3Layout::Released(v) => Self::V3(v),
							LegacyV3Layout::Edited(v) => Self::V4(v),
						},
					),
					4 => Ok(Self::V4(<$v4 as ::vbare::Body>::from_body(body)?)),
					5 => Ok(Self::V5(<$v5 as ::vbare::Body>::from_body(body)?)),
					_ => Err($crate::versioned::unsupported_version(version, 6)),
				}
			}
			fn encode_version<S: ::vbare::Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> ::std::result::Result<(), ::vbare::EncodeError> {
				match (self, version) {
					(Self::V1(v), 1) => ::vbare::Body::put_body(v, out),
					(Self::V2(v), 2) => ::vbare::Body::put_body(v, out),
					(Self::V3(v), 3) => ::vbare::Body::put_body(v, out),
					(Self::V4(v), 4) => ::vbare::Body::put_body(v, out),
					(Self::V5(v), 5) => ::vbare::Body::put_body(v, out),
					(Self::V6(v), 6) => ::vbare::Body::put_body(v, out),
					_ => return Err(::vbare::EncodeError::UnsupportedVersion(version)),
				}
				Ok(())
			}
			// Legacy: writes version 4 under the version 3 header.
			fn seal<S: ::vbare::Sink>(
				value: &Self::Latest,
				version: u16,
				out: &mut S,
			) -> ::std::result::Result<(), ::vbare::EncodeError> {
				legacy_seal::<Self, S>(value, version, out)
			}
		}
	};
}

/// [`impl_versioned_v1_to_v6`] for an envelope version 2 introduced. Version 1 neither decodes
/// nor encodes it, so its upgrade slot refuses.
macro_rules! impl_versioned_v2_to_v6 {
	(
		$name:ident,
		$family:ident,
		$v2:ty,
		$v3:ty,
		$v4:ty,
		$v5:ty,
		$v6:ty,
		$up23:path,
		$down32:path,
		$up34:path,
		$down43:path,
		$up45:path,
		$down54:path,
		$up56:path,
		$down65:path
	) => {
		impl_versioned_family!($name, $family);

		pub enum $name {
			V2($v2),
			V3($v3),
			V4($v4),
			V5($v5),
			V6($v6),
		}

		impl ::vbare::Versioned for $name {
			type Latest = $v6;
			const LATEST: u16 = 6;
			const UPGRADES: &'static [::vbare::Converter<Self>] = &[
				|_| ::anyhow::bail!("version 1 has no {}", stringify!($name)),
				|value| match value {
					Self::V2(v) => Ok(Self::V3($up23(v)?)),
					_ => ::anyhow::bail!("expected version 2"),
				},
				// Legacy: a header 3 payload in the edited layout decodes straight to v4.
				|value| match value {
					Self::V3(v) => Ok(Self::V4($up34(v)?)),
					Self::V4(v) => Ok(Self::V4(v)),
					_ => ::anyhow::bail!("expected version 3"),
				},
				|value| match value {
					Self::V4(v) => Ok(Self::V5($up45(v)?)),
					_ => ::anyhow::bail!("expected version 4"),
				},
				|value| match value {
					Self::V5(v) => Ok(Self::V6($up56(v)?)),
					_ => ::anyhow::bail!("expected version 5"),
				},
			];
			const DOWNGRADES: &'static [::vbare::Converter<Self>] = &[
				|value| match value {
					Self::V6(v) => Ok(Self::V5($down65(v)?)),
					_ => ::anyhow::bail!("expected version 6"),
				},
				|value| match value {
					Self::V5(v) => Ok(Self::V4($down54(v)?)),
					_ => ::anyhow::bail!("expected version 5"),
				},
				|value| match value {
					Self::V4(v) => Ok(Self::V3($down43(v)?)),
					_ => ::anyhow::bail!("expected version 4"),
				},
				|value| match value {
					Self::V3(v) => Ok(Self::V2($down32(v)?)),
					_ => ::anyhow::bail!("expected version 3"),
				},
				|_| ::anyhow::bail!("version 1 has no {}", stringify!($name)),
			];
			fn wrap_latest(value: Self::Latest) -> Self {
				Self::V6(value)
			}
			fn unwrap_latest(self) -> ::anyhow::Result<Self::Latest> {
				match self {
					Self::V6(value) => Ok(value),
					_ => ::anyhow::bail!("expected latest version"),
				}
			}
			fn decode_version(
				version: u16,
				body: ::vbare::Bytes,
			) -> ::std::result::Result<Self, ::vbare::DecodeError> {
				match version {
					2 => Ok(Self::V2(<$v2 as ::vbare::Body>::from_body(body)?)),
					// Legacy: two layouts share the version 3 header.
					3 => Ok(
						match legacy_decode_v3_header::<$v3, $v4>(stringify!($name), body, $up34)? {
							LegacyV3Layout::Released(v) => Self::V3(v),
							LegacyV3Layout::Edited(v) => Self::V4(v),
						},
					),
					4 => Ok(Self::V4(<$v4 as ::vbare::Body>::from_body(body)?)),
					5 => Ok(Self::V5(<$v5 as ::vbare::Body>::from_body(body)?)),
					_ => Err($crate::versioned::unsupported_version(version, 6)),
				}
			}
			fn encode_version<S: ::vbare::Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> ::std::result::Result<(), ::vbare::EncodeError> {
				match (self, version) {
					(Self::V2(v), 2) => ::vbare::Body::put_body(v, out),
					(Self::V3(v), 3) => ::vbare::Body::put_body(v, out),
					(Self::V4(v), 4) => ::vbare::Body::put_body(v, out),
					(Self::V5(v), 5) => ::vbare::Body::put_body(v, out),
					(Self::V6(v), 6) => ::vbare::Body::put_body(v, out),
					_ => return Err(::vbare::EncodeError::UnsupportedVersion(version)),
				}
				Ok(())
			}
			// Legacy: writes version 4 under the version 3 header.
			fn seal<S: ::vbare::Sink>(
				value: &Self::Latest,
				version: u16,
				out: &mut S,
			) -> ::std::result::Result<(), ::vbare::EncodeError> {
				legacy_seal::<Self, S>(value, version, out)
			}
		}
	};
}

/// A type that version 6 introduced. Versions 1 to 5 neither decode nor encode it, so every
/// converter slot refuses.
macro_rules! impl_versioned_v6_only {
	($(#[$meta:meta])* $name:ident, $v6:ty) => {
		$(#[$meta])*
		pub enum $name {
			V6($v6),
		}

		impl_versioned_family!($name, PEGBOARD_ACTOR);

		impl ::vbare::Versioned for $name {
			type Latest = $v6;

			const LATEST: u16 = 6;

			const UPGRADES: &'static [::vbare::Converter<Self>] = &[
				|_| Err($crate::versioned::Inexpressible::lacks(1, stringify!($name))),
				|_| Err($crate::versioned::Inexpressible::lacks(2, stringify!($name))),
				|_| Err($crate::versioned::Inexpressible::lacks(3, stringify!($name))),
				|_| Err($crate::versioned::Inexpressible::lacks(4, stringify!($name))),
				|_| Err($crate::versioned::Inexpressible::lacks(5, stringify!($name))),
			];

			const DOWNGRADES: &'static [::vbare::Converter<Self>] = &[
				|_| Err($crate::versioned::Inexpressible::lacks(5, stringify!($name))),
				|_| Err($crate::versioned::Inexpressible::lacks(4, stringify!($name))),
				|_| Err($crate::versioned::Inexpressible::lacks(3, stringify!($name))),
				|_| Err($crate::versioned::Inexpressible::lacks(2, stringify!($name))),
				|_| Err($crate::versioned::Inexpressible::lacks(1, stringify!($name))),
			];

			fn wrap_latest(value: Self::Latest) -> Self {
				Self::V6(value)
			}

			fn unwrap_latest(self) -> ::anyhow::Result<Self::Latest> {
				match self {
					Self::V6(value) => Ok(value),
				}
			}

			fn decode_version(
				version: u16,
				_body: ::vbare::Bytes,
			) -> ::std::result::Result<Self, ::vbare::DecodeError> {
				Err($crate::versioned::unsupported_version(version, 6))
			}

			fn encode_version<S: ::vbare::Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> ::std::result::Result<(), ::vbare::EncodeError> {
				match (self, version) {
					(Self::V6(v), 6) => ::vbare::Body::put_body(v, out),
					_ => return Err(::vbare::EncodeError::UnsupportedVersion(version)),
				}
				Ok(())
			}
		}
	};
}

impl_versioned_v1_to_v6!(
	ActorData,
	PEGBOARD_ACTOR,
	v1::ActorData,
	v2::ActorData,
	v3::ActorData,
	v4::ActorData,
	v5::ActorData,
	v6::ActorData,
	up::convert_actor_data_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_data_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_data_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_data_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_actor_data_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_actor_data_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_actor_data_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_actor_data_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	ActorState,
	PEGBOARD_ACTOR,
	v1::ActorState,
	v2::ActorState,
	v3::ActorState,
	v4::ActorState,
	v5::ActorState,
	v6::ActorState,
	up::convert_actor_state_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_state_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_state_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_state_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_actor_state_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_actor_state_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_actor_state_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_actor_state_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	ActorTimer,
	PEGBOARD_ACTOR,
	v1::ActorTimer,
	v2::ActorTimer,
	v3::ActorTimer,
	v4::ActorTimer,
	v5::ActorTimer,
	v6::ActorTimer,
	up::convert_actor_timer_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_timer_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_timer_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_timer_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_actor_timer_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_actor_timer_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_actor_timer_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_actor_timer_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	ActorCreateRequest,
	PEGBOARD_ACTOR,
	v1::ActorCreateRequest,
	v2::ActorCreateRequest,
	v3::ActorCreateRequest,
	v4::ActorCreateRequest,
	v5::ActorCreateRequest,
	v6::ActorCreateRequest,
	up::convert_actor_create_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_create_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_create_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_create_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_actor_create_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_actor_create_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_actor_create_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_actor_create_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	ActorCreateResponse,
	PEGBOARD_ACTOR,
	v1::ActorCreateResponse,
	v2::ActorCreateResponse,
	v3::ActorCreateResponse,
	v4::ActorCreateResponse,
	v5::ActorCreateResponse,
	v6::ActorCreateResponse,
	up::convert_actor_create_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_create_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_create_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_create_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_actor_create_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_actor_create_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_actor_create_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_actor_create_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	ActorReadRequest,
	PEGBOARD_ACTOR,
	v1::ActorReadRequest,
	v2::ActorReadRequest,
	v3::ActorReadRequest,
	v4::ActorReadRequest,
	v5::ActorReadRequest,
	v6::ActorReadRequest,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_to_v6!(
	ActorReadResponse,
	PEGBOARD_ACTOR,
	v1::ActorReadResponse,
	v2::ActorReadResponse,
	v3::ActorReadResponse,
	v4::ActorReadResponse,
	v5::ActorReadResponse,
	v6::ActorReadResponse,
	up::convert_actor_read_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_read_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_read_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_read_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_actor_read_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_actor_read_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_actor_read_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_actor_read_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	ActorStartNotice,
	PEGBOARD_ACTOR,
	v1::ActorStartNotice,
	v2::ActorStartNotice,
	v3::ActorStartNotice,
	v4::ActorStartNotice,
	v5::ActorStartNotice,
	v6::ActorStartNotice,
	up::convert_actor_start_notice_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_start_notice_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_start_notice_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_start_notice_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_actor_start_notice_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_actor_start_notice_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_actor_start_notice_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_actor_start_notice_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	ActorEvent,
	PEGBOARD_ACTOR,
	v1::ActorEvent,
	v2::ActorEvent,
	v3::ActorEvent,
	v4::ActorEvent,
	v5::ActorEvent,
	v6::ActorEvent,
	up::convert_actor_event_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_event_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_event_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_event_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_actor_event_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_actor_event_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_actor_event_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_actor_event_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	ActorEventResponse,
	PEGBOARD_ACTOR,
	v1::ActorEventResponse,
	v2::ActorEventResponse,
	v3::ActorEventResponse,
	v4::ActorEventResponse,
	v5::ActorEventResponse,
	v6::ActorEventResponse,
	up::convert_actor_event_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_event_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_event_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_event_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_actor_event_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_actor_event_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_actor_event_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_actor_event_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexInsertActorRequest,
	PEGBOARD_ACTOR,
	v1::IndexInsertActorRequest,
	v2::IndexInsertActorRequest,
	v3::IndexInsertActorRequest,
	v4::IndexInsertActorRequest,
	v5::IndexInsertActorRequest,
	v6::IndexInsertActorRequest,
	up::convert_index_insert_actor_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_insert_actor_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_insert_actor_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_insert_actor_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_insert_actor_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_insert_actor_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_insert_actor_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_insert_actor_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexInsertActorResponse,
	PEGBOARD_ACTOR,
	v1::IndexInsertActorResponse,
	v2::IndexInsertActorResponse,
	v3::IndexInsertActorResponse,
	v4::IndexInsertActorResponse,
	v5::IndexInsertActorResponse,
	v6::IndexInsertActorResponse,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_to_v6!(
	IndexDeleteActorRequest,
	PEGBOARD_ACTOR,
	v1::IndexDeleteActorRequest,
	v2::IndexDeleteActorRequest,
	v3::IndexDeleteActorRequest,
	v4::IndexDeleteActorRequest,
	v5::IndexDeleteActorRequest,
	v6::IndexDeleteActorRequest,
	up::convert_index_delete_actor_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_delete_actor_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_delete_actor_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_delete_actor_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_delete_actor_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_delete_actor_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_delete_actor_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_delete_actor_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexDeleteActorResponse,
	PEGBOARD_ACTOR,
	v1::IndexDeleteActorResponse,
	v2::IndexDeleteActorResponse,
	v3::IndexDeleteActorResponse,
	v4::IndexDeleteActorResponse,
	v5::IndexDeleteActorResponse,
	v6::IndexDeleteActorResponse,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_to_v6!(
	IndexListActorsRequest,
	PEGBOARD_ACTOR,
	v1::IndexListActorsRequest,
	v2::IndexListActorsRequest,
	v3::IndexListActorsRequest,
	v4::IndexListActorsRequest,
	v5::IndexListActorsRequest,
	v6::IndexListActorsRequest,
	up::convert_index_list_actors_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_actors_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_actors_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_actors_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_list_actors_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_list_actors_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_list_actors_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_list_actors_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexListActorsResponse,
	PEGBOARD_ACTOR,
	v1::IndexListActorsResponse,
	v2::IndexListActorsResponse,
	v3::IndexListActorsResponse,
	v4::IndexListActorsResponse,
	v5::IndexListActorsResponse,
	v6::IndexListActorsResponse,
	up::convert_index_list_actors_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_actors_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_actors_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_actors_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_list_actors_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_list_actors_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_list_actors_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_list_actors_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	DestroyedActorIndexEntry,
	PEGBOARD_ACTOR,
	v1::DestroyedActorIndexEntry,
	v2::DestroyedActorIndexEntry,
	v3::DestroyedActorIndexEntry,
	v4::DestroyedActorIndexEntry,
	v5::DestroyedActorIndexEntry,
	v6::DestroyedActorIndexEntry,
	up::convert_destroyed_actor_index_entry_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_destroyed_actor_index_entry_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_destroyed_actor_index_entry_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_destroyed_actor_index_entry_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_destroyed_actor_index_entry_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_destroyed_actor_index_entry_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_destroyed_actor_index_entry_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_destroyed_actor_index_entry_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexReadDestroyedActorRequest,
	PEGBOARD_ACTOR,
	v1::IndexReadDestroyedActorRequest,
	v2::IndexReadDestroyedActorRequest,
	v3::IndexReadDestroyedActorRequest,
	v4::IndexReadDestroyedActorRequest,
	v5::IndexReadDestroyedActorRequest,
	v6::IndexReadDestroyedActorRequest,
	up::convert_index_read_destroyed_actor_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_read_destroyed_actor_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_read_destroyed_actor_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_read_destroyed_actor_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_read_destroyed_actor_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_read_destroyed_actor_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_read_destroyed_actor_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_read_destroyed_actor_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexReadDestroyedActorResponse,
	PEGBOARD_ACTOR,
	v1::IndexReadDestroyedActorResponse,
	v2::IndexReadDestroyedActorResponse,
	v3::IndexReadDestroyedActorResponse,
	v4::IndexReadDestroyedActorResponse,
	v5::IndexReadDestroyedActorResponse,
	v6::IndexReadDestroyedActorResponse,
	up::convert_index_read_destroyed_actor_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_read_destroyed_actor_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_read_destroyed_actor_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_read_destroyed_actor_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_read_destroyed_actor_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_read_destroyed_actor_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_read_destroyed_actor_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_read_destroyed_actor_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexListEnvoysRequest,
	PEGBOARD_ACTOR,
	v1::IndexListEnvoysRequest,
	v2::IndexListEnvoysRequest,
	v3::IndexListEnvoysRequest,
	v4::IndexListEnvoysRequest,
	v5::IndexListEnvoysRequest,
	v6::IndexListEnvoysRequest,
	up::convert_index_list_envoys_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_envoys_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_envoys_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_envoys_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_list_envoys_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_list_envoys_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_list_envoys_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_list_envoys_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexListEnvoysResponse,
	PEGBOARD_ACTOR,
	v1::IndexListEnvoysResponse,
	v2::IndexListEnvoysResponse,
	v3::IndexListEnvoysResponse,
	v4::IndexListEnvoysResponse,
	v5::IndexListEnvoysResponse,
	v6::IndexListEnvoysResponse,
	up::convert_index_list_envoys_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_envoys_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_envoys_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_envoys_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_list_envoys_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_list_envoys_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_list_envoys_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_list_envoys_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	EpoxyActorByKeyEntry,
	PEGBOARD_ACTOR,
	v1::EpoxyActorByKeyEntry,
	v2::EpoxyActorByKeyEntry,
	v3::EpoxyActorByKeyEntry,
	v4::EpoxyActorByKeyEntry,
	v5::EpoxyActorByKeyEntry,
	v6::EpoxyActorByKeyEntry,
	up::convert_epoxy_actor_by_key_entry_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_epoxy_actor_by_key_entry_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_epoxy_actor_by_key_entry_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_epoxy_actor_by_key_entry_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_epoxy_actor_by_key_entry_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_epoxy_actor_by_key_entry_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_epoxy_actor_by_key_entry_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_epoxy_actor_by_key_entry_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	KvValue,
	PEGBOARD_ACTOR,
	v1::KvValue,
	v2::KvValue,
	v3::KvValue,
	v4::KvValue,
	v5::KvValue,
	v6::KvValue,
	up::convert_kv_value_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_kv_value_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_kv_value_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_kv_value_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_kv_value_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_kv_value_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_kv_value_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_kv_value_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	EnvoyData,
	PEGBOARD_ACTOR,
	v1::EnvoyData,
	v2::EnvoyData,
	v3::EnvoyData,
	v4::EnvoyData,
	v5::EnvoyData,
	v6::EnvoyData,
	up::convert_envoy_data_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_envoy_data_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_envoy_data_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_envoy_data_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_envoy_data_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_envoy_data_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_envoy_data_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_envoy_data_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	EnvoyUpsertRequest,
	PEGBOARD_ACTOR,
	v1::EnvoyUpsertRequest,
	v2::EnvoyUpsertRequest,
	v3::EnvoyUpsertRequest,
	v4::EnvoyUpsertRequest,
	v5::EnvoyUpsertRequest,
	v6::EnvoyUpsertRequest,
	up::convert_envoy_upsert_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_envoy_upsert_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_envoy_upsert_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_envoy_upsert_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_envoy_upsert_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_envoy_upsert_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_envoy_upsert_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_envoy_upsert_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	EnvoyUpsertResponse,
	PEGBOARD_ACTOR,
	v1::EnvoyUpsertResponse,
	v2::EnvoyUpsertResponse,
	v3::EnvoyUpsertResponse,
	v4::EnvoyUpsertResponse,
	v5::EnvoyUpsertResponse,
	v6::EnvoyUpsertResponse,
	up::convert_envoy_upsert_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_envoy_upsert_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_envoy_upsert_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_envoy_upsert_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_envoy_upsert_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_envoy_upsert_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_envoy_upsert_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_envoy_upsert_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	EnvoyReadRequest,
	PEGBOARD_ACTOR,
	v1::EnvoyReadRequest,
	v2::EnvoyReadRequest,
	v3::EnvoyReadRequest,
	v4::EnvoyReadRequest,
	v5::EnvoyReadRequest,
	v6::EnvoyReadRequest,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_to_v6!(
	EnvoyReadResponse,
	PEGBOARD_ACTOR,
	v1::EnvoyReadResponse,
	v2::EnvoyReadResponse,
	v3::EnvoyReadResponse,
	v4::EnvoyReadResponse,
	v5::EnvoyReadResponse,
	v6::EnvoyReadResponse,
	up::convert_envoy_read_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_envoy_read_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_envoy_read_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_envoy_read_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_envoy_read_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_envoy_read_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_envoy_read_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_envoy_read_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexInsertEnvoyRequest,
	PEGBOARD_ACTOR,
	v1::IndexInsertEnvoyRequest,
	v2::IndexInsertEnvoyRequest,
	v3::IndexInsertEnvoyRequest,
	v4::IndexInsertEnvoyRequest,
	v5::IndexInsertEnvoyRequest,
	v6::IndexInsertEnvoyRequest,
	up::convert_index_insert_envoy_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_insert_envoy_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_insert_envoy_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_insert_envoy_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_insert_envoy_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_insert_envoy_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_insert_envoy_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_insert_envoy_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexInsertEnvoyResponse,
	PEGBOARD_ACTOR,
	v1::IndexInsertEnvoyResponse,
	v2::IndexInsertEnvoyResponse,
	v3::IndexInsertEnvoyResponse,
	v4::IndexInsertEnvoyResponse,
	v5::IndexInsertEnvoyResponse,
	v6::IndexInsertEnvoyResponse,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_to_v6!(
	IndexUpsertActiveEnvoyRequest,
	PEGBOARD_ACTOR,
	v1::IndexUpsertActiveEnvoyRequest,
	v2::IndexUpsertActiveEnvoyRequest,
	v3::IndexUpsertActiveEnvoyRequest,
	v4::IndexUpsertActiveEnvoyRequest,
	v5::IndexUpsertActiveEnvoyRequest,
	v6::IndexUpsertActiveEnvoyRequest,
	up::convert_index_upsert_active_envoy_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_upsert_active_envoy_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_upsert_active_envoy_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_upsert_active_envoy_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_upsert_active_envoy_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_upsert_active_envoy_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_upsert_active_envoy_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_upsert_active_envoy_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexUpsertActiveEnvoyResponse,
	PEGBOARD_ACTOR,
	v1::IndexUpsertActiveEnvoyResponse,
	v2::IndexUpsertActiveEnvoyResponse,
	v3::IndexUpsertActiveEnvoyResponse,
	v4::IndexUpsertActiveEnvoyResponse,
	v5::IndexUpsertActiveEnvoyResponse,
	v6::IndexUpsertActiveEnvoyResponse,
	up::convert_index_upsert_active_envoy_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_upsert_active_envoy_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_upsert_active_envoy_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_upsert_active_envoy_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_upsert_active_envoy_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_upsert_active_envoy_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_upsert_active_envoy_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_upsert_active_envoy_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexDeleteActiveEnvoyRequest,
	PEGBOARD_ACTOR,
	v1::IndexDeleteActiveEnvoyRequest,
	v2::IndexDeleteActiveEnvoyRequest,
	v3::IndexDeleteActiveEnvoyRequest,
	v4::IndexDeleteActiveEnvoyRequest,
	v5::IndexDeleteActiveEnvoyRequest,
	v6::IndexDeleteActiveEnvoyRequest,
	up::convert_index_delete_active_envoy_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_delete_active_envoy_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_delete_active_envoy_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_delete_active_envoy_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_delete_active_envoy_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_delete_active_envoy_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_delete_active_envoy_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_delete_active_envoy_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexDeleteActiveEnvoyResponse,
	PEGBOARD_ACTOR,
	v1::IndexDeleteActiveEnvoyResponse,
	v2::IndexDeleteActiveEnvoyResponse,
	v3::IndexDeleteActiveEnvoyResponse,
	v4::IndexDeleteActiveEnvoyResponse,
	v5::IndexDeleteActiveEnvoyResponse,
	v6::IndexDeleteActiveEnvoyResponse,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_to_v6!(
	IndexPutEnvoyPoolRequest,
	PEGBOARD_ACTOR,
	v1::IndexPutEnvoyPoolRequest,
	v2::IndexPutEnvoyPoolRequest,
	v3::IndexPutEnvoyPoolRequest,
	v4::IndexPutEnvoyPoolRequest,
	v5::IndexPutEnvoyPoolRequest,
	v6::IndexPutEnvoyPoolRequest,
	up::convert_index_put_envoy_pool_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_put_envoy_pool_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_put_envoy_pool_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_put_envoy_pool_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_put_envoy_pool_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_put_envoy_pool_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_put_envoy_pool_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_put_envoy_pool_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexPutEnvoyPoolResponse,
	PEGBOARD_ACTOR,
	v1::IndexPutEnvoyPoolResponse,
	v2::IndexPutEnvoyPoolResponse,
	v3::IndexPutEnvoyPoolResponse,
	v4::IndexPutEnvoyPoolResponse,
	v5::IndexPutEnvoyPoolResponse,
	v6::IndexPutEnvoyPoolResponse,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_to_v6!(
	IndexListEnvoyPoolsRequest,
	PEGBOARD_ACTOR,
	v1::IndexListEnvoyPoolsRequest,
	v2::IndexListEnvoyPoolsRequest,
	v3::IndexListEnvoyPoolsRequest,
	v4::IndexListEnvoyPoolsRequest,
	v5::IndexListEnvoyPoolsRequest,
	v6::IndexListEnvoyPoolsRequest,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_to_v6!(
	IndexListEnvoyPoolsResponse,
	PEGBOARD_ACTOR,
	v1::IndexListEnvoyPoolsResponse,
	v2::IndexListEnvoyPoolsResponse,
	v3::IndexListEnvoyPoolsResponse,
	v4::IndexListEnvoyPoolsResponse,
	v5::IndexListEnvoyPoolsResponse,
	v6::IndexListEnvoyPoolsResponse,
	up::convert_index_list_envoy_pools_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_envoy_pools_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_envoy_pools_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_envoy_pools_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_list_envoy_pools_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_list_envoy_pools_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_list_envoy_pools_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_list_envoy_pools_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexListActiveEnvoysRequest,
	PEGBOARD_ACTOR,
	v1::IndexListActiveEnvoysRequest,
	v2::IndexListActiveEnvoysRequest,
	v3::IndexListActiveEnvoysRequest,
	v4::IndexListActiveEnvoysRequest,
	v5::IndexListActiveEnvoysRequest,
	v6::IndexListActiveEnvoysRequest,
	up::convert_index_list_active_envoys_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_active_envoys_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_active_envoys_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_active_envoys_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_list_active_envoys_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_list_active_envoys_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_list_active_envoys_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_list_active_envoys_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexListActiveEnvoysResponse,
	PEGBOARD_ACTOR,
	v1::IndexListActiveEnvoysResponse,
	v2::IndexListActiveEnvoysResponse,
	v3::IndexListActiveEnvoysResponse,
	v4::IndexListActiveEnvoysResponse,
	v5::IndexListActiveEnvoysResponse,
	v6::IndexListActiveEnvoysResponse,
	up::convert_index_list_active_envoys_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_active_envoys_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_active_envoys_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_active_envoys_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_list_active_envoys_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_list_active_envoys_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_list_active_envoys_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_list_active_envoys_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexListActiveEnvoyStatesRequest,
	PEGBOARD_ACTOR,
	v1::IndexListActiveEnvoyStatesRequest,
	v2::IndexListActiveEnvoyStatesRequest,
	v3::IndexListActiveEnvoyStatesRequest,
	v4::IndexListActiveEnvoyStatesRequest,
	v5::IndexListActiveEnvoyStatesRequest,
	v6::IndexListActiveEnvoyStatesRequest,
	up::convert_index_list_active_envoy_states_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_active_envoy_states_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_active_envoy_states_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_active_envoy_states_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_list_active_envoy_states_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_list_active_envoy_states_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_list_active_envoy_states_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_list_active_envoy_states_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	IndexListActiveEnvoyStatesResponse,
	PEGBOARD_ACTOR,
	v1::IndexListActiveEnvoyStatesResponse,
	v2::IndexListActiveEnvoyStatesResponse,
	v3::IndexListActiveEnvoyStatesResponse,
	v4::IndexListActiveEnvoyStatesResponse,
	v5::IndexListActiveEnvoyStatesResponse,
	v6::IndexListActiveEnvoyStatesResponse,
	up::convert_index_list_active_envoy_states_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_active_envoy_states_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_active_envoy_states_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_active_envoy_states_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_index_list_active_envoy_states_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_index_list_active_envoy_states_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_index_list_active_envoy_states_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_index_list_active_envoy_states_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	ActiveEnvoyIndexEntry,
	PEGBOARD_ACTOR,
	v1::ActiveEnvoyIndexEntry,
	v2::ActiveEnvoyIndexEntry,
	v3::ActiveEnvoyIndexEntry,
	v4::ActiveEnvoyIndexEntry,
	v5::ActiveEnvoyIndexEntry,
	v6::ActiveEnvoyIndexEntry,
	up::convert_active_envoy_index_entry_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_active_envoy_index_entry_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_active_envoy_index_entry_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_active_envoy_index_entry_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_active_envoy_index_entry_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_active_envoy_index_entry_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_active_envoy_index_entry_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_active_envoy_index_entry_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	ActiveEnvoyState,
	PEGBOARD_ACTOR,
	v1::ActiveEnvoyState,
	v2::ActiveEnvoyState,
	v3::ActiveEnvoyState,
	v4::ActiveEnvoyState,
	v5::ActiveEnvoyState,
	v6::ActiveEnvoyState,
	up::convert_active_envoy_state_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_active_envoy_state_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_active_envoy_state_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_active_envoy_state_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_active_envoy_state_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_active_envoy_state_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_active_envoy_state_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_active_envoy_state_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

// Legacy: a namespace record row in the shared catalog volume. Namespaces now live in their own
// namespace volumes (`pegboard.namespace`). Read only for a namespace written before namespace
// volumes, or created while the fleet cannot read them yet, until it is moved.
impl_versioned_v1_to_v6!(
	LegacyNamespaceRecord,
	PEGBOARD_ACTOR,
	v1::NamespaceRecord,
	v2::NamespaceRecord,
	v3::NamespaceRecord,
	v4::NamespaceRecord,
	v5::NamespaceRecord,
	v6::NamespaceRecord,
	up::convert_namespace_record_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_record_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_record_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_record_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_namespace_record_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_namespace_record_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_namespace_record_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_namespace_record_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

// Legacy: creates a record row in the shared catalog. New binaries send it only while the
// fleet cannot read namespace volumes; old binaries send it at any time.
impl_versioned_v1_to_v6!(
	LegacyNamespaceCreateRequest,
	PEGBOARD_ACTOR,
	v1::NamespaceCreateRequest,
	v2::NamespaceCreateRequest,
	v3::NamespaceCreateRequest,
	v4::NamespaceCreateRequest,
	v5::NamespaceCreateRequest,
	v6::NamespaceCreateRequest,
	up::convert_namespace_create_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_create_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_create_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_create_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_namespace_create_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_namespace_create_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_namespace_create_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_namespace_create_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

// Legacy: the answer to [`LegacyNamespaceCreateRequest`].
impl_versioned_v1_to_v6!(
	LegacyNamespaceCreateResponse,
	PEGBOARD_ACTOR,
	v1::NamespaceCreateResponse,
	v2::NamespaceCreateResponse,
	v3::NamespaceCreateResponse,
	v4::NamespaceCreateResponse,
	v5::NamespaceCreateResponse,
	v6::NamespaceCreateResponse,
	up::convert_namespace_create_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_create_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_create_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_create_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_namespace_create_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_namespace_create_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_namespace_create_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_namespace_create_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

// Legacy: reads one record row of the shared catalog. Sent for a name whose namespace volume
// holds no state yet, and by old binaries.
impl_versioned_v1_to_v6!(
	LegacyNamespaceReadRequest,
	PEGBOARD_ACTOR,
	v1::NamespaceReadRequest,
	v2::NamespaceReadRequest,
	v3::NamespaceReadRequest,
	v4::NamespaceReadRequest,
	v5::NamespaceReadRequest,
	v6::NamespaceReadRequest,
	up::convert_namespace_read_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_read_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_read_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_read_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_namespace_read_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_namespace_read_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_namespace_read_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_namespace_read_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

// Legacy: the answer to [`LegacyNamespaceReadRequest`].
impl_versioned_v1_to_v6!(
	LegacyNamespaceReadResponse,
	PEGBOARD_ACTOR,
	v1::NamespaceReadResponse,
	v2::NamespaceReadResponse,
	v3::NamespaceReadResponse,
	v4::NamespaceReadResponse,
	v5::NamespaceReadResponse,
	v6::NamespaceReadResponse,
	up::convert_namespace_read_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_read_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_read_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_read_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_namespace_read_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_namespace_read_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_namespace_read_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_namespace_read_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

// Legacy: pages the shared catalog's record rows. Old binaries list namespaces with it; new
// binaries page the listing index, which merges these rows until none are left.
impl_versioned_v1_to_v6!(
	LegacyNamespaceListRequest,
	PEGBOARD_ACTOR,
	v1::NamespaceListRequest,
	v2::NamespaceListRequest,
	v3::NamespaceListRequest,
	v4::NamespaceListRequest,
	v5::NamespaceListRequest,
	v6::NamespaceListRequest,
	up::convert_namespace_list_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_list_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_list_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_list_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_namespace_list_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_namespace_list_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_namespace_list_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_namespace_list_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

// Legacy: the answer to [`LegacyNamespaceListRequest`].
impl_versioned_v1_to_v6!(
	LegacyNamespaceListResponse,
	PEGBOARD_ACTOR,
	v1::NamespaceListResponse,
	v2::NamespaceListResponse,
	v3::NamespaceListResponse,
	v4::NamespaceListResponse,
	v5::NamespaceListResponse,
	v6::NamespaceListResponse,
	up::convert_namespace_list_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_list_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_list_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_list_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_namespace_list_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_namespace_list_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_namespace_list_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_namespace_list_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	NodePoolConfig,
	PEGBOARD_ACTOR,
	v1::NodePoolConfig,
	v2::NodePoolConfig,
	v3::NodePoolConfig,
	v4::NodePoolConfig,
	v5::NodePoolConfig,
	v6::NodePoolConfig,
	up::convert_node_pool_config_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_node_pool_config_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_node_pool_config_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_node_pool_config_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_node_pool_config_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_node_pool_config_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_node_pool_config_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_node_pool_config_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	PoolRequest,
	PEGBOARD_ACTOR,
	v1::PoolRequest,
	v2::PoolRequest,
	v3::PoolRequest,
	v4::PoolRequest,
	v5::PoolRequest,
	v6::PoolRequest,
	up::convert_pool_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_pool_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_pool_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_pool_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_pool_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_pool_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_pool_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	PoolResponse,
	PEGBOARD_ACTOR,
	v1::PoolResponse,
	v2::PoolResponse,
	v3::PoolResponse,
	v4::PoolResponse,
	v5::PoolResponse,
	v6::PoolResponse,
	up::convert_pool_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_pool_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_pool_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_pool_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_pool_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_pool_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_pool_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	PoolLease,
	PEGBOARD_ACTOR,
	v1::PoolLease,
	v2::PoolLease,
	v3::PoolLease,
	v4::PoolLease,
	v5::PoolLease,
	v6::PoolLease,
	up::convert_pool_lease_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_lease_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_pool_lease_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_pool_lease_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_pool_lease_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_pool_lease_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_pool_lease_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_pool_lease_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	PoolListRequest,
	PEGBOARD_ACTOR,
	v1::PoolListRequest,
	v2::PoolListRequest,
	v3::PoolListRequest,
	v4::PoolListRequest,
	v5::PoolListRequest,
	v6::PoolListRequest,
	up::convert_pool_list_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_list_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_pool_list_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_pool_list_request_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_pool_list_request_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_pool_list_request_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_pool_list_request_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_pool_list_request_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v1_to_v6!(
	PoolListResponse,
	PEGBOARD_ACTOR,
	v1::PoolListResponse,
	v2::PoolListResponse,
	v3::PoolListResponse,
	v4::PoolListResponse,
	v5::PoolListResponse,
	v6::PoolListResponse,
	up::convert_pool_list_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_list_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_pool_list_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_pool_list_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_pool_list_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_pool_list_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_pool_list_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_pool_list_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v2_to_v6!(
	MigrateFromV2Begin,
	PEGBOARD_ACTOR,
	v2::MigrateFromV2Begin,
	v3::MigrateFromV2Begin,
	v4::MigrateFromV2Begin,
	v5::MigrateFromV2Begin,
	v6::MigrateFromV2Begin,
	up23::convert_migrate_from_v2_begin_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_migrate_from_v2_begin_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_migrate_from_v2_begin_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_migrate_from_v2_begin_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_migrate_from_v2_begin_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_migrate_from_v2_begin_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v2_to_v6!(
	ActorMigrateFromV2BeginResponse,
	PEGBOARD_ACTOR,
	v2::ActorMigrateFromV2BeginResponse,
	v3::ActorMigrateFromV2BeginResponse,
	v4::ActorMigrateFromV2BeginResponse,
	v5::ActorMigrateFromV2BeginResponse,
	v6::ActorMigrateFromV2BeginResponse,
	up23::convert_actor_migrate_from_v2_begin_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_migrate_from_v2_begin_response_pegboard_actor_v3_to_pegboard_actor_v2,
	up34::convert_actor_migrate_from_v2_begin_response_pegboard_actor_v3_to_pegboard_actor_v4,
	down43::convert_actor_migrate_from_v2_begin_response_pegboard_actor_v4_to_pegboard_actor_v3,
	up45::convert_actor_migrate_from_v2_begin_response_pegboard_actor_v4_to_pegboard_actor_v5,
	down54::convert_actor_migrate_from_v2_begin_response_pegboard_actor_v5_to_pegboard_actor_v4,
	upgrade_v5_v6,
	downgrade_v6_v5
);

impl_versioned_v6_only!(
	/// Legacy: the backfill's read of an actor's scheduled alarm, which versions 1 to 5 never
	/// had. Removed in 3.1 with the backfill.
	LegacyActorAlarmReadRequest,
	v6::LegacyActorAlarmReadRequest
);

impl_versioned_v6_only!(
	/// Legacy: see [`LegacyActorAlarmReadRequest`]. Removed in 3.1 with the backfill.
	LegacyActorAlarmReadResponse,
	v6::LegacyActorAlarmReadResponse
);
