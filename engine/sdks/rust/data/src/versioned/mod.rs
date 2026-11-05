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

/// Three adjacent versions with explicit converters in upgrade and reverse downgrade order.
macro_rules! impl_versioned_v1_v2_v3 {
	($name:ident, $v1:ty, $v2:ty, $v3:ty, $up12:path, $down21:path, $up23:path, $down32:path) => {
		pub enum $name {
			V1($v1),
			V2($v2),
			V3($v3),
		}
		impl ::vbare::Versioned for $name {
			type Latest = $v3;
			const LATEST: u16 = 3;
			const UPGRADES: &'static [::vbare::Converter<Self>] = &[
				|value| match value {
					Self::V1(v) => Ok(Self::V2($up12(v)?)),
					_ => ::anyhow::bail!("expected version 1"),
				},
				|value| match value {
					Self::V2(v) => Ok(Self::V3($up23(v)?)),
					_ => ::anyhow::bail!("expected version 2"),
				},
			];
			const DOWNGRADES: &'static [::vbare::Converter<Self>] = &[
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
				Self::V3(value)
			}
			fn unwrap_latest(self) -> ::anyhow::Result<Self::Latest> {
				match self {
					Self::V3(value) => Ok(value),
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
					_ => Err($crate::versioned::unsupported_version(version, 3)),
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
					_ => return Err(::vbare::EncodeError::UnsupportedVersion(version)),
				}
				Ok(())
			}
		}
	};
}

/// Two adjacent versions for a type that version 2 introduced. Version 1 neither decodes nor
/// encodes it, so its upgrade slot refuses.
macro_rules! impl_versioned_v2_v3 {
	($name:ident, $v2:ty, $v3:ty, $up23:path, $down32:path) => {
		pub enum $name {
			V2($v2),
			V3($v3),
		}
		impl ::vbare::Versioned for $name {
			type Latest = $v3;
			const LATEST: u16 = 3;
			const UPGRADES: &'static [::vbare::Converter<Self>] = &[
				|_| ::anyhow::bail!("version 1 has no {}", stringify!($name)),
				|value| match value {
					Self::V2(v) => Ok(Self::V3($up23(v)?)),
					_ => ::anyhow::bail!("expected version 2"),
				},
			];
			const DOWNGRADES: &'static [::vbare::Converter<Self>] = &[
				|value| match value {
					Self::V3(v) => Ok(Self::V2($down32(v)?)),
					_ => ::anyhow::bail!("expected version 3"),
				},
				|_| ::anyhow::bail!("version 1 has no {}", stringify!($name)),
			];
			fn wrap_latest(value: Self::Latest) -> Self {
				Self::V3(value)
			}
			fn unwrap_latest(self) -> ::anyhow::Result<Self::Latest> {
				match self {
					Self::V3(value) => Ok(value),
					_ => ::anyhow::bail!("expected latest version"),
				}
			}
			fn decode_version(
				version: u16,
				body: ::vbare::Bytes,
			) -> ::std::result::Result<Self, ::vbare::DecodeError> {
				match version {
					2 => Ok(Self::V2(<$v2 as ::vbare::Body>::from_body(body)?)),
					_ => Err($crate::versioned::unsupported_version(version, 3)),
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
					_ => return Err(::vbare::EncodeError::UnsupportedVersion(version)),
				}
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
