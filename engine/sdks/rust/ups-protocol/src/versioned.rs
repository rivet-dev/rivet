use anyhow::Result;
use vbare::{Bytes, DecodeError, DecodeErrorKind, Encode, EncodeError, Message, Sink, Versioned};

use crate::generated::v3;

/// Only v3 is spoken. Its layout is byte for byte the v3 released with Rivet 2.3; v1 and v2 are
/// older released layouts that this build neither reads nor writes.
pub enum UpsMessage {
	V3(v3::UpsMessage),
}

impl Versioned for UpsMessage {
	const LATEST: u16 = 3;

	type Latest = v3::UpsMessage;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		let Self::V3(data) = self;
		Ok(data)
	}

	fn decode_version(version: u16, payload: Bytes) -> std::result::Result<Self, DecodeError> {
		match version {
			3 => Ok(Self::V3(Message::decode(payload)?)),
			_ => Err(unsupported_version(format!("invalid version: {version}"))),
		}
	}

	fn encode_version<S: Sink>(
		&self,
		version: u16,
		out: &mut S,
	) -> std::result::Result<(), EncodeError> {
		if version != Self::LATEST {
			return Err(EncodeError::UnsupportedVersion(version));
		}
		let Self::V3(data) = self;
		data.encode(out);
		Ok(())
	}
}

/// The error for a version this envelope cannot decode.
fn unsupported_version(detail: String) -> DecodeError {
	DecodeError::with_detail(DecodeErrorKind::UnsupportedVersion, 0, detail)
}
