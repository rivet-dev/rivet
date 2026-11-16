//! Version envelopes for the V2 migrator's durable state and commands.
//!
//! Version 2 adds the volume group an in-flight entry is placed in. Version 1, which 3.0.2 and
//! 3.0.3 write, has only the legacy unprefixed placement, so every version 1 entry upgrades as
//! legacy and a version 2 entry placed in a group does not downgrade. The converters for the types
//! that carry an entry live in `pegboard_v2_migrator_v1_to_pegboard_v2_migrator_v2` and its
//! reverse; every other type is the same at both versions and converts through its generated
//! `From`.

mod pegboard_v2_migrator_v1_to_pegboard_v2_migrator_v2;
mod pegboard_v2_migrator_v2_to_pegboard_v2_migrator_v1;

use super::unchanged;
use crate::generated::{pegboard_v2_migrator_v1 as v1, pegboard_v2_migrator_v2 as v2};
use pegboard_v2_migrator_v1_to_pegboard_v2_migrator_v2 as up;
use pegboard_v2_migrator_v2_to_pegboard_v2_migrator_v1 as down;

/// Converts a value whose two versions are the same shape through its generated `From`.
fn same<A, B: From<A>>(value: A) -> anyhow::Result<B> {
	Ok(value.into())
}

/// Two adjacent versions with explicit converters.
macro_rules! migrator_v1_v2 {
	($name:ident, $v1:ty, $v2:ty, $up12:path, $down21:path) => {
		impl_versioned_family!($name, PEGBOARD_V2_MIGRATOR);

		pub enum $name {
			V1($v1),
			V2($v2),
		}

		impl ::vbare::Versioned for $name {
			type Latest = $v2;

			const LATEST: u16 = 2;

			const UPGRADES: &'static [::vbare::Converter<Self>] = &[|value| match value {
				Self::V1(v) => Ok(Self::V2($up12(v)?)),
				Self::V2(_) => ::anyhow::bail!("expected version 1"),
			}];

			const DOWNGRADES: &'static [::vbare::Converter<Self>] = &[|value| match value {
				Self::V2(v) => Ok(Self::V1($down21(v)?)),
				Self::V1(_) => ::anyhow::bail!("expected version 2"),
			}];

			fn wrap_latest(value: Self::Latest) -> Self {
				Self::V2(value)
			}

			fn unwrap_latest(self) -> ::anyhow::Result<Self::Latest> {
				match self {
					Self::V2(value) => Ok(value),
					Self::V1(_) => ::anyhow::bail!("expected latest version"),
				}
			}

			fn decode_version(
				version: u16,
				body: ::vbare::Bytes,
			) -> ::std::result::Result<Self, ::vbare::DecodeError> {
				match version {
					1 => Ok(Self::V1(<$v1 as ::vbare::Body>::from_body(body)?)),
					2 => Ok(Self::V2(<$v2 as ::vbare::Body>::from_body(body)?)),
					_ => Err($crate::versioned::unsupported_version(version, 2)),
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
					_ => return Err(::vbare::EncodeError::UnsupportedVersion(version)),
				}
				Ok(())
			}
		}
	};
}

migrator_v1_v2!(
	MigrateFromV2InFlight,
	v1::MigrateFromV2InFlight,
	v2::MigrateFromV2InFlight,
	up::in_flight,
	down::in_flight
);

migrator_v1_v2!(
	MigrateFromV2Pass,
	v1::MigrateFromV2Pass,
	v2::MigrateFromV2Pass,
	same,
	same
);

migrator_v1_v2!(
	MigrateFromV2Fence,
	v1::MigrateFromV2Fence,
	v2::MigrateFromV2Fence,
	same,
	same
);

migrator_v1_v2!(
	MigrateFromV2StateWriteRequest,
	v1::MigrateFromV2StateWriteRequest,
	v2::MigrateFromV2StateWriteRequest,
	up::state_write_request,
	down::state_write_request
);

migrator_v1_v2!(
	MigrateFromV2StateWriteResponse,
	v1::MigrateFromV2StateWriteResponse,
	v2::MigrateFromV2StateWriteResponse,
	same,
	same
);

migrator_v1_v2!(
	MigrateFromV2StateLoadRequest,
	v1::MigrateFromV2StateLoadRequest,
	v2::MigrateFromV2StateLoadRequest,
	same,
	same
);

migrator_v1_v2!(
	MigrateFromV2StateLoadResponse,
	v1::MigrateFromV2StateLoadResponse,
	v2::MigrateFromV2StateLoadResponse,
	up::state_load_response,
	down::state_load_response
);

migrator_v1_v2!(
	MigrateFromV2StateReadRequest,
	v1::MigrateFromV2StateReadRequest,
	v2::MigrateFromV2StateReadRequest,
	unchanged,
	unchanged
);

migrator_v1_v2!(
	MigrateFromV2ActorRequest,
	v1::MigrateFromV2ActorRequest,
	v2::MigrateFromV2ActorRequest,
	same,
	same
);

migrator_v1_v2!(
	MigrateFromV2ActorResponse,
	v1::MigrateFromV2ActorResponse,
	v2::MigrateFromV2ActorResponse,
	same,
	same
);
