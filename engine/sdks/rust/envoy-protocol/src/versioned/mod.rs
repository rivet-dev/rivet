use anyhow::Result;
use vbare::{Bytes, DecodeError, DecodeErrorKind, Encode, EncodeError, Sink, Versioned};

use crate::generated::{v8, v9, v10};

mod v8_v9;
mod v9_v10;

/// The oldest Envoy protocol version a v3 Engine accepts. v8 shipped with RivetKit 2.3.20.
pub const MIN_PROTOCOL_VERSION: u16 = 8;

/// Refuses a version below [`MIN_PROTOCOL_VERSION`], which a v3 Engine never decodes.
fn unsupported(version: u16) -> DecodeError {
	DecodeError::with_detail(
		DecodeErrorKind::UnsupportedVersion,
		0,
		format!(
			"unsupported envoy protocol version {version}, the oldest supported is {MIN_PROTOCOL_VERSION}"
		),
	)
}

/// Implements [`Versioned`] for an envelope `enum $name { V8(v8::$name), V9(v9::$name),
/// V10(v10::$name) }` with the converters between adjacent versions. Versions below v8 have
/// converter slots that refuse, since nothing below [`MIN_PROTOCOL_VERSION`] is ever decoded or
/// encoded.
macro_rules! impl_versioned {
	($name:ident, $up8:path, $down9:path, $up9:path, $down10:path) => {
		impl $name {
			fn refuse(self) -> Result<Self> {
				anyhow::bail!(
					"envoy protocol versions below {MIN_PROTOCOL_VERSION} are not supported"
				)
			}

			fn v8_to_v9(self) -> Result<Self> {
				match self {
					Self::V8(x) => Ok(Self::V9($up8(x)?)),
					Self::V9(_) | Self::V10(_) => anyhow::bail!("expected envoy protocol v8"),
				}
			}

			fn v9_to_v10(self) -> Result<Self> {
				match self {
					Self::V9(x) => Ok(Self::V10($up9(x)?)),
					Self::V8(_) | Self::V10(_) => anyhow::bail!("expected envoy protocol v9"),
				}
			}

			fn v10_to_v9(self) -> Result<Self> {
				match self {
					Self::V10(x) => Ok(Self::V9($down10(x)?)),
					Self::V8(_) | Self::V9(_) => anyhow::bail!("expected envoy protocol v10"),
				}
			}

			fn v9_to_v8(self) -> Result<Self> {
				match self {
					Self::V9(x) => Ok(Self::V8($down9(x)?)),
					Self::V8(_) | Self::V10(_) => anyhow::bail!("expected envoy protocol v9"),
				}
			}
		}

		impl Versioned for $name {
			type Latest = v10::$name;

			const LATEST: u16 = 10;

			const UPGRADES: &'static [vbare::Converter<Self>] = &[
				Self::refuse,
				Self::refuse,
				Self::refuse,
				Self::refuse,
				Self::refuse,
				Self::refuse,
				Self::refuse,
				Self::v8_to_v9,
				Self::v9_to_v10,
			];

			const DOWNGRADES: &'static [vbare::Converter<Self>] =
				&[Self::v10_to_v9, Self::v9_to_v8];

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V10(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				match self {
					Self::V10(x) => Ok(x),
					Self::V8(_) | Self::V9(_) => anyhow::bail!("expected envoy protocol v10"),
				}
			}

			fn decode_version(version: u16, body: Bytes) -> std::result::Result<Self, DecodeError> {
				match version {
					8 => Ok(Self::V8(<v8::$name as vbare::Body>::from_body(body)?)),
					9 => Ok(Self::V9(<v9::$name as vbare::Body>::from_body(body)?)),
					_ => Err(unsupported(version)),
				}
			}

			fn encode_version<S: Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> std::result::Result<(), EncodeError> {
				match (self, version) {
					(Self::V8(x), 8) => x.encode(out),
					(Self::V9(x), 9) => x.encode(out),
					(Self::V10(x), 10) => x.encode(out),
					(Self::V8(_) | Self::V9(_) | Self::V10(_), _) => {
						return Err(EncodeError::UnsupportedVersion(version));
					}
				}
				Ok(())
			}
		}
	};
}

// MARK: ToEnvoy

pub enum ToEnvoy {
	V8(v8::ToEnvoy),
	V9(v9::ToEnvoy),
	V10(v10::ToEnvoy),
}

impl_versioned!(
	ToEnvoy,
	v8_v9::convert_to_envoy_v8_to_v9,
	v8_v9::convert_to_envoy_v9_to_v8,
	v9_v10::convert_to_envoy_v9_to_v10,
	v9_v10::convert_to_envoy_v10_to_v9
);

// MARK: ToRivet

pub enum ToRivet {
	V8(v8::ToRivet),
	V9(v9::ToRivet),
	V10(v10::ToRivet),
}

impl_versioned!(
	ToRivet,
	v8_v9::convert_to_rivet_v8_to_v9,
	v8_v9::convert_to_rivet_v9_to_v8,
	v9_v10::convert_to_rivet_v9_to_v10,
	v9_v10::convert_to_rivet_v10_to_v9
);

// MARK: ToEnvoyConn

pub enum ToEnvoyConn {
	V8(v8::ToEnvoyConn),
	V9(v9::ToEnvoyConn),
	V10(v10::ToEnvoyConn),
}

impl_versioned!(
	ToEnvoyConn,
	v8_v9::convert_to_envoy_conn_v8_to_v9,
	v8_v9::convert_to_envoy_conn_v9_to_v8,
	v9_v10::convert_to_envoy_conn_v9_to_v10,
	v9_v10::convert_to_envoy_conn_v10_to_v9
);

// MARK: ToGateway

pub enum ToGateway {
	V8(v8::ToGateway),
	V9(v9::ToGateway),
	V10(v10::ToGateway),
}

impl_versioned!(
	ToGateway,
	v8_v9::convert_to_gateway_v8_to_v9,
	v8_v9::convert_to_gateway_v9_to_v8,
	v9_v10::convert_to_gateway_v9_to_v10,
	v9_v10::convert_to_gateway_v10_to_v9
);

// MARK: ToOutbound

pub enum ToOutbound {
	V8(v8::ToOutbound),
	V9(v9::ToOutbound),
	V10(v10::ToOutbound),
}

impl_versioned!(
	ToOutbound,
	v8_v9::convert_to_outbound_v8_to_v9,
	v8_v9::convert_to_outbound_v9_to_v8,
	v9_v10::convert_to_outbound_v9_to_v10,
	v9_v10::convert_to_outbound_v10_to_v9
);

// MARK: ActorCommandKeyData

pub enum ActorCommandKeyData {
	V8(v8::ActorCommandKeyData),
	V9(v9::ActorCommandKeyData),
	V10(v10::ActorCommandKeyData),
}

impl_versioned!(
	ActorCommandKeyData,
	v8_v9::convert_actor_command_key_data_v8_to_v9,
	v8_v9::convert_actor_command_key_data_v9_to_v8,
	v9_v10::convert_actor_command_key_data_v9_to_v10,
	v9_v10::convert_actor_command_key_data_v10_to_v9
);

// MARK: Tests
