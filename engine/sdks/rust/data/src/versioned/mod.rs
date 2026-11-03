use vbare::{DecodeError, DecodeErrorKind};

use crate::generated::*;

/// Implements [`vbare::Versioned`] for a single-version envelope `enum $name { V1($latest) }`.
/// The latest value is a generated message.
macro_rules! impl_versioned_v1 {
	($name:ident, $latest:ty) => {
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

/// Declares the envelope `enum $name { V1($v1), V2($v2) }` and implements [`vbare::Versioned`]
/// for it. `$upgrade` and `$downgrade` are the field-by-field converters between the two
/// versions; a downgrade fails for a value version 1 cannot represent.
macro_rules! impl_versioned_v1_v2 {
	($name:ident, $v1:ty, $v2:ty, $upgrade:path, $downgrade:path) => {
		pub enum $name {
			V1($v1),
			V2($v2),
		}

		impl $name {
			fn upgrade_v1(self) -> ::anyhow::Result<Self> {
				match self {
					Self::V1(value) => Ok(Self::V2($upgrade(value)?)),
					Self::V2(_) => {
						::anyhow::bail!(concat!("expected a version 1 ", stringify!($name)))
					}
				}
			}

			fn downgrade_v2(self) -> ::anyhow::Result<Self> {
				match self {
					Self::V2(value) => Ok(Self::V1($downgrade(value)?)),
					Self::V1(_) => {
						::anyhow::bail!(concat!("expected a version 2 ", stringify!($name)))
					}
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
					Self::V1(_) => {
						::anyhow::bail!(concat!("expected the latest ", stringify!($name)))
					}
				}
			}

			fn decode_version(
				version: u16,
				body: ::vbare::Bytes,
			) -> ::std::result::Result<Self, ::vbare::DecodeError> {
				if version != 1 {
					return Err($crate::versioned::unsupported_version(version, 2));
				}
				Ok(Self::V1(<$v1 as ::vbare::Body>::from_body(body)?))
			}

			fn encode_version<S: ::vbare::Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> ::std::result::Result<(), ::vbare::EncodeError> {
				match (self, version) {
					(Self::V1(value), 1) => {
						::vbare::Body::put_body(value, out);
						Ok(())
					}
					(Self::V2(value), 2) => {
						::vbare::Body::put_body(value, out);
						Ok(())
					}
					_ => Err(::vbare::EncodeError::UnsupportedVersion(version)),
				}
			}
		}
	};
}

/// Declares the envelope `enum $name { V2($latest) }` for a type that version 2 introduced, and
/// implements [`vbare::Versioned`] for it. Version 1 neither decodes nor encodes it.
macro_rules! impl_versioned_v2_only {
	($name:ident, $latest:ty) => {
		pub enum $name {
			V2($latest),
		}

		impl ::vbare::Versioned for $name {
			type Latest = $latest;

			const LATEST: u16 = 2;

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

mod pegboard_v2_migrator;
pub use pegboard_v2_migrator::*;

mod depot_fs;
pub use depot_fs::*;

pub enum AuthJwtEmergencyRotateRequestData {
	V1(auth_jwt_rotation_v1::OperatorRequest),
}
impl_versioned_v1!(
	AuthJwtEmergencyRotateRequestData,
	auth_jwt_rotation_v1::OperatorRequest
);
pub enum AuthJwtEmergencyRotateResponseData {
	V1(auth_jwt_rotation_v1::EmergencyRotateResponse),
}
impl_versioned_v1!(
	AuthJwtEmergencyRotateResponseData,
	auth_jwt_rotation_v1::EmergencyRotateResponse
);
