//! vbare version wrappers for Actor Runtime Socket frames.
use anyhow::Result;
use vbare::{Bytes, DecodeError, DecodeErrorKind, Encode, EncodeError, Message, Sink, Versioned};

use crate::generated::v1;

macro_rules! versioned_v1 {
	($name:ident, $type:ty) => {
		pub enum $name {
			V1($type),
		}

		impl Versioned for $name {
			const LATEST: u16 = 1;

			type Latest = $type;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V1(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				match self {
					Self::V1(value) => Ok(value),
				}
			}

			fn decode_version(
				version: u16,
				payload: Bytes,
			) -> std::result::Result<Self, DecodeError> {
				match version {
					1 => Ok(Self::V1(Message::decode(payload)?)),
					_ => Err(unsupported_version(format!(
						"unsupported Actor Runtime Socket protocol version: {version}"
					))),
				}
			}

			fn encode_version<S: Sink>(
				&self,
				version: u16,
				out: &mut S,
			) -> std::result::Result<(), EncodeError> {
				match (self, version) {
					(Self::V1(value), 1) => {
						value.encode(out);
						Ok(())
					}
					(_, version) => return Err(EncodeError::UnsupportedVersion(version)),
				}
			}
		}
	};
}

versioned_v1!(ClientHello, v1::ClientHello);
versioned_v1!(ServerHello, v1::ServerHello);
versioned_v1!(ClientFrame, v1::ClientFrame);
versioned_v1!(ServerFrame, v1::ServerFrame);

/// The error for a version this envelope cannot decode.
fn unsupported_version(detail: String) -> DecodeError {
	DecodeError::with_detail(DecodeErrorKind::UnsupportedVersion, 0, detail)
}
