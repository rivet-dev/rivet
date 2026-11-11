use vbare::{DecodeError, DecodeErrorKind};

use crate::generated::*;

/// Implements [`VersionedFamily`](crate::families::VersionedFamily) for an envelope of
/// `$family`, a [`VersionFamily`](crate::families::VersionFamily) constant. An envelope's
/// family is stated where it is defined because a message cannot name it: a `void` message has
/// no type of its own, and a shared schema's message can appear in envelopes of several
/// families.
macro_rules! impl_versioned_family {
	($name:ident, $family:ident) => {
		impl $crate::families::VersionedFamily for $name {
			const FAMILY: Option<$crate::families::VersionFamily> =
				Some($crate::families::VersionFamily::$family);
		}
	};
}

/// Implements [`vbare::Versioned`] for a single-version envelope `enum $name { V1($latest) }`.
/// The latest value is a generated message.
macro_rules! impl_versioned_v1 {
	($name:ident, $family:ident, $latest:ty) => {
		impl_versioned_family!($name, $family);

		impl ::vbare::Versioned for $name {
			type Latest = $latest;

			const LATEST: u16 = 1;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V1(latest)
			}

			fn unwrap_latest(self) -> ::anyhow::Result<Self::Latest> {
				let Self::V1(latest) = self;
				Ok(latest)
			}

			fn decode_version(
				version: u16,
				_body: ::vbare::Bytes,
			) -> ::std::result::Result<Self, ::vbare::DecodeError> {
				Err($crate::versioned::unsupported_version(version, 1))
			}

			fn encode_version<S: ::vbare::Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> ::std::result::Result<(), ::vbare::EncodeError> {
				if version != 1 {
					return Err(::vbare::EncodeError::UnsupportedVersion(version));
				}
				let Self::V1(latest) = self;
				::vbare::Body::put_body(latest, out);
				Ok(())
			}
		}
	};
}

/// Implements [`vbare::Versioned`] for a two-version envelope `enum $name { V1, V2 }` whose
/// latest is `$v2`. Version 1 upgrades through `From` and version 2 downgrades through
/// [`Downgrade`], which refuses a value version 1 cannot carry.
macro_rules! impl_versioned_v2 {
	($name:ident, $family:ident, $v1:ty, $v2:ty) => {
		impl_versioned_family!($name, $family);

		impl $name {
			fn upgrade_v1(self) -> ::anyhow::Result<Self> {
				match self {
					Self::V1(value) => Ok(Self::V2(value.into())),
					Self::V2(_) => ::anyhow::bail!("expected a version 1 value"),
				}
			}
			fn downgrade_v2(self) -> ::anyhow::Result<Self> {
				match self {
					Self::V2(value) => {
						Ok(Self::V1($crate::versioned::Downgrade::downgrade(value)?))
					}
					Self::V1(_) => ::anyhow::bail!("expected a version 2 value"),
				}
			}
		}
		impl ::vbare::Versioned for $name {
			type Latest = $v2;
			const LATEST: u16 = 2;
			const UPGRADES: &'static [::vbare::Converter<Self>] = &[Self::upgrade_v1];
			const DOWNGRADES: &'static [::vbare::Converter<Self>] = &[Self::downgrade_v2];
			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V2(latest)
			}
			fn unwrap_latest(self) -> ::anyhow::Result<Self::Latest> {
				match self {
					Self::V2(latest) => Ok(latest),
					Self::V1(_) => ::anyhow::bail!("expected the latest version"),
				}
			}
			fn decode_version(
				version: u16,
				body: ::vbare::Bytes,
			) -> ::std::result::Result<Self, ::vbare::DecodeError> {
				match version {
					1 => Ok(Self::V1(<$v1 as ::vbare::Body>::from_body(body)?)),
					_ => Err($crate::versioned::unsupported_version(version, 2)),
				}
			}
			fn encode_version<S: ::vbare::Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> ::std::result::Result<(), ::vbare::EncodeError> {
				match (self, version) {
					(Self::V1(value), 1) => ::vbare::Body::put_body(value, out),
					(Self::V2(value), 2) => ::vbare::Body::put_body(value, out),
					_ => return Err(::vbare::EncodeError::UnsupportedVersion(version)),
				}
				Ok(())
			}
		}
	};
}

/// Implements [`vbare::Versioned`] for a two-version envelope `enum $name { V1($v1), V2($v2) }`
/// whose versions convert through the scaffolded converters `$up` and `$down`. `$down` refuses
/// a value version 1 cannot carry.
macro_rules! impl_versioned_v1_v2 {
	($name:ident, $family:ident, $v1:ty, $v2:ty, $up:path, $down:path) => {
		impl_versioned_family!($name, $family);

		impl ::vbare::Versioned for $name {
			type Latest = $v2;

			const LATEST: u16 = 2;

			const UPGRADES: &'static [::vbare::Converter<Self>] = &[|value| match value {
				Self::V1(value) => Ok(Self::V2($up(value)?)),
				Self::V2(_) => ::anyhow::bail!("expected a version 1 value"),
			}];

			const DOWNGRADES: &'static [::vbare::Converter<Self>] = &[|value| match value {
				Self::V2(value) => Ok(Self::V1($down(value)?)),
				Self::V1(_) => ::anyhow::bail!("expected a version 2 value"),
			}];

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V2(latest)
			}

			fn unwrap_latest(self) -> ::anyhow::Result<Self::Latest> {
				match self {
					Self::V2(latest) => Ok(latest),
					Self::V1(_) => ::anyhow::bail!("expected the latest version"),
				}
			}

			fn decode_version(
				version: u16,
				body: ::vbare::Bytes,
			) -> ::std::result::Result<Self, ::vbare::DecodeError> {
				match version {
					1 => Ok(Self::V1(<$v1 as ::vbare::Body>::from_body(body)?)),
					_ => Err($crate::versioned::unsupported_version(version, 2)),
				}
			}

			fn encode_version<S: ::vbare::Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> ::std::result::Result<(), ::vbare::EncodeError> {
				match (self, version) {
					(Self::V1(value), 1) => ::vbare::Body::put_body(value, out),
					(Self::V2(value), 2) => ::vbare::Body::put_body(value, out),
					_ => return Err(::vbare::EncodeError::UnsupportedVersion(version)),
				}
				Ok(())
			}
		}
	};
}

/// Implements [`vbare::Versioned`] for a single-version envelope `enum $name { V2($v2) }` of a
/// family whose latest version is 2: a type the family's version 1 never had, so version 1 is
/// refused outright.
macro_rules! impl_versioned_v2_only {
	($name:ident, $family:ident, $v2:ty) => {
		impl_versioned_family!($name, $family);

		impl $name {
			/// The converter slot for version 1, which this type never had: `UPGRADES[i]` converts
			/// version `i + 1` and `DOWNGRADES[i]` converts to version `LATEST - i - 1`, so the
			/// slot must exist and must refuse.
			fn no_version_1(self) -> ::anyhow::Result<Self> {
				::anyhow::bail!("this type has no version 1")
			}
		}

		impl ::vbare::Versioned for $name {
			type Latest = $v2;

			const LATEST: u16 = 2;

			const UPGRADES: &'static [::vbare::Converter<Self>] = &[Self::no_version_1];

			const DOWNGRADES: &'static [::vbare::Converter<Self>] = &[Self::no_version_1];

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V2(latest)
			}

			fn unwrap_latest(self) -> ::anyhow::Result<Self::Latest> {
				let Self::V2(latest) = self;
				Ok(latest)
			}

			fn decode_version(
				version: u16,
				_body: ::vbare::Bytes,
			) -> ::std::result::Result<Self, ::vbare::DecodeError> {
				Err($crate::versioned::unsupported_version(version, 2))
			}

			fn encode_version<S: ::vbare::Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> ::std::result::Result<(), ::vbare::EncodeError> {
				if version != 2 {
					return Err(::vbare::EncodeError::UnsupportedVersion(version));
				}
				let Self::V2(latest) = self;
				::vbare::Body::put_body(latest, out);
				Ok(())
			}
		}
	};
}

/// The error for a version that an envelope cannot decode.
pub(crate) fn unsupported_version(version: u16, latest: u16) -> DecodeError {
	DecodeError::with_detail(
		DecodeErrorKind::UnsupportedVersion,
		0,
		format!("version {version}, latest is {latest}"),
	)
}

/// The converter for a type whose two versions are the same Rust type, such as a `void` message.
pub(crate) fn unchanged<T>(value: T) -> anyhow::Result<T> {
	Ok(value)
}

mod auth_grants;
mod auth_jwt_key_ring;
mod depot;
mod depot_alarms;
mod depot_durable_state_machine;
mod depot_group;
mod depot_split_snapshot;
mod epoxy_control_state;
mod epoxy_key_state;
mod rivet_cluster_config;
mod rivet_singleton_coordinator;
mod sqlite;
mod topology_worker_heartbeat;

pub use auth_grants::*;
pub use auth_jwt_key_ring::*;
pub use depot::*;
pub use depot_alarms::*;
pub use depot_durable_state_machine::*;
pub use depot_group::*;
pub use depot_split_snapshot::*;
pub use epoxy_control_state::*;
pub use epoxy_key_state::*;
pub use rivet_cluster_config::*;
pub use rivet_singleton_coordinator::*;
pub use sqlite::*;
pub use topology_worker_heartbeat::*;

mod ups_request;
pub use ups_request::*;

mod ups_packet;
pub use ups_packet::*;

mod ups_channel;
pub use ups_channel::*;

mod pegboard_actor;
pub use pegboard_actor::*;
mod pegboard_namespace;
pub use pegboard_namespace::*;

mod pegboard_v2_migrator;
pub use pegboard_v2_migrator::*;

mod depot_fs;
pub use depot_fs::*;

mod depot_fs_v2;
pub use depot_fs_v2::Downgrade;

mod depot_v2;

pub enum AuthJwtEmergencyRotateRequestData {
	V1(auth_jwt_rotation_v1::OperatorRequest),
}
impl_versioned_v1!(
	AuthJwtEmergencyRotateRequestData,
	AUTH_JWT_ROTATION,
	auth_jwt_rotation_v1::OperatorRequest
);
pub enum AuthJwtEmergencyRotateResponseData {
	V1(auth_jwt_rotation_v1::EmergencyRotateResponse),
}
impl_versioned_v1!(
	AuthJwtEmergencyRotateResponseData,
	AUTH_JWT_ROTATION,
	auth_jwt_rotation_v1::EmergencyRotateResponse
);

/// A downgrade refused because the older version has no way to express the value, as opposed
/// to a converter or encoding failure. A caller that tries versions in order to find the lowest
/// one that expresses a value, such as the durable runner's timer and state encodings,
/// skips only this refusal and fails on any other error. Families encoded by the durable runner
/// (state and timers) raise it; other families' refusals are plain errors.
#[derive(Debug)]
pub struct Inexpressible {
	/// The older version that cannot express the value.
	pub version: u16,
	/// What the older version lacks, for the error message.
	pub what: &'static str,
}

impl std::fmt::Display for Inexpressible {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "version {} has no {}", self.version, self.what)
	}
}

impl std::error::Error for Inexpressible {}

impl Inexpressible {
	/// The refusal of `version`, which has no `what`, as a converter's error.
	pub(crate) fn lacks(version: u16, what: &'static str) -> anyhow::Error {
		Inexpressible { version, what }.into()
	}
}
