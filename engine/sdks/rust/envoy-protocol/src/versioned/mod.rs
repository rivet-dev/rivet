use anyhow::Result;
use vbare::{Bytes, DecodeError, DecodeErrorKind, Encode, EncodeError, Sink, Versioned};

use crate::generated::v8;

/// The oldest Envoy protocol version a v3 Engine accepts. v8 shipped with RivetKit 2.3.20.
pub const MIN_PROTOCOL_VERSION: u16 = 8;

/// Implements [`Versioned`] for an envelope `enum $name { V8(v8::$name) }`.
///
/// v8 is both the latest and the oldest supported version, so there are no converters. vbare
/// never calls `decode_version` for the latest version, so every version it sees is refused.
macro_rules! impl_versioned {
	($name:ident) => {
		impl Versioned for $name {
			type Latest = v8::$name;

			const LATEST: u16 = 8;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V8(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				match self {
					Self::V8(x) => Ok(x),
				}
			}

			fn decode_version(version: u16, _body: Bytes) -> std::result::Result<Self, DecodeError> {
				Err(DecodeError::with_detail(
					DecodeErrorKind::UnsupportedVersion,
					0,
					format!(
						"unsupported envoy protocol version {version}, the oldest supported is {MIN_PROTOCOL_VERSION}"
					),
				))
			}

			fn encode_version<S: Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> std::result::Result<(), EncodeError> {
				if version < MIN_PROTOCOL_VERSION {
					return Err(EncodeError::UnsupportedVersion(version));
				}
				match self {
					Self::V8(x) => x.encode(out),
				}
				Ok(())
			}
		}
	};
}

// MARK: ToEnvoy

pub enum ToEnvoy {
	V8(v8::ToEnvoy),
}

impl_versioned!(ToEnvoy);

// MARK: ToRivet

pub enum ToRivet {
	V8(v8::ToRivet),
}

impl_versioned!(ToRivet);

// MARK: ToEnvoyConn

pub enum ToEnvoyConn {
	V8(v8::ToEnvoyConn),
}

impl_versioned!(ToEnvoyConn);

// MARK: ToGateway

pub enum ToGateway {
	V8(v8::ToGateway),
}

impl_versioned!(ToGateway);

// MARK: ToOutbound

pub enum ToOutbound {
	V8(v8::ToOutbound),
}

impl_versioned!(ToOutbound);

// MARK: ActorCommandKeyData

pub enum ActorCommandKeyData {
	V8(v8::ActorCommandKeyData),
}

impl_versioned!(ActorCommandKeyData);

// MARK: Tests


