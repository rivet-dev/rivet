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
				Err($crate::versioned::unsupported_version(version))
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

/// The error for a version that a single-version envelope cannot decode.
pub(crate) fn unsupported_version(version: u16) -> DecodeError {
	DecodeError::with_detail(
		DecodeErrorKind::UnsupportedVersion,
		0,
		format!("version {version}, latest is 1"),
	)
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
