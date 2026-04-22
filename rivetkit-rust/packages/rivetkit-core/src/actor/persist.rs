use anyhow::{Context, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::ProtocolError;

const EMBEDDED_VERSION_LEN: usize = 2;

pub(crate) fn encode_with_embedded_version<T>(
	value: &T,
	version: u16,
	label: &str,
) -> Result<Vec<u8>>
where
	T: Serialize,
{
	let payload =
		serde_bare::to_vec(value).with_context(|| format!("encode {label} bare payload"))?;
	let mut encoded = Vec::with_capacity(EMBEDDED_VERSION_LEN + payload.len());
	encoded.extend_from_slice(&version.to_le_bytes());
	encoded.extend_from_slice(&payload);
	Ok(encoded)
}

pub(crate) fn decode_with_embedded_version<T>(
	payload: &[u8],
	supported_versions: &[u16],
	label: &str,
) -> Result<T>
where
	T: DeserializeOwned,
{
	if payload.len() < EMBEDDED_VERSION_LEN {
		return Err(ProtocolError::InvalidPersistedData {
			label: label.to_owned(),
			reason: "payload too short for embedded version".to_owned(),
		}
		.build());
	}

	let version = u16::from_le_bytes([payload[0], payload[1]]);
	if !supported_versions.contains(&version) {
		return Err(ProtocolError::InvalidPersistedData {
			label: label.to_owned(),
			reason: format!(
				"unsupported version {version}; expected one of {:?}",
				supported_versions
			),
		}
		.build());
	}

	serde_bare::from_slice(&payload[EMBEDDED_VERSION_LEN..])
		.with_context(|| format!("decode {label} bare payload v{version}"))
}
