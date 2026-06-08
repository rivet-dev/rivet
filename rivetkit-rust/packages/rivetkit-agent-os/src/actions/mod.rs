//! Action dispatcher entry point.
//!
//! Each arm decodes its positional args with `action.decode_as::<(...)>()`
//! (TS sends args as a CBOR array) and replies via [`Action::ok`] or
//! [`Action::err`]. Byte payloads auto-wrap via the rivetkit
//! `JSON_COMPAT_UINT8_ARRAY` convention thanks to `Action::ok` running
//! through `encode_json_compat`.

pub mod filesystem;

use agent_os_client::AgentOs;
use anyhow::{Result, anyhow};
use rivetkit::Action;

use crate::actor::AgentOsActor;

/// Dispatch one action against a live VM. Each arm decodes its args,
/// calls the helper, and replies through `action.ok` / `action.err`.
pub async fn dispatch(vm: &AgentOs, action: Action<AgentOsActor>) {
	let name = action.name().to_owned();
	match name.as_str() {
		"readFile" => {
			let args: Result<(String,)> = action.decode_as();
			match args {
				Ok((path,)) => match filesystem::read_file(vm, &path).await {
					Ok(bytes) => {
						// Wrap as serde_bytes so it serializes as a byte
						// string, which the rivetkit JsonCompatAdapter then
						// re-wraps as `["$Uint8Array", base64]`.
						action.ok(&serde_bytes::ByteBuf::from(bytes));
					}
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"stat" => {
			let args: Result<(String,)> = action.decode_as();
			match args {
				Ok((path,)) => match filesystem::stat(vm, &path).await {
					Ok(vstat) => action.ok(&vstat),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"writeFile" => {
			// TS sends `contents` as either a `string` (CBOR text string),
			// a `Uint8Array` / `Buffer` (CBOR byte string -> `ByteBuf`), or
			// a `["$Uint8Array", base64]` wrapper. Accept any of those and
			// coerce to raw bytes.
			let args: Result<(String, WriteFileContent)> = action.decode_as();
			match args {
				Ok((path, contents)) => {
					match filesystem::write_file(vm, &path, contents.into_bytes()).await {
						Ok(()) => action.ok(&()),
						Err(error) => action.err(error),
					}
				}
				Err(error) => action.err(error),
			}
		}
		_ => action.err(not_implemented(&name)),
	}
}

fn not_implemented(name: &str) -> anyhow::Error {
	anyhow!("agent-os action not implemented yet: {name}")
}

/// Accept either a CBOR text string, a CBOR byte string (via `ByteBuf`), or
/// the `["$Uint8Array", base64]` wrapper that TS encoders emit when the
/// outer codec is JSON-compatible. Used by `writeFile` and similar
/// byte-payload action arms.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum WriteFileContent {
	String(String),
	Bytes(serde_bytes::ByteBuf),
	Wrapped(JsonCompatUint8Array),
}

impl WriteFileContent {
	fn into_bytes(self) -> Vec<u8> {
		match self {
			Self::String(s) => s.into_bytes(),
			Self::Bytes(b) => b.into_vec(),
			Self::Wrapped(w) => w.bytes,
		}
	}
}

/// Deserializer for the `["$Uint8Array", base64]` envelope. Used as part
/// of [`WriteFileContent`]'s untagged enum so the same arm accepts wrapped
/// bytes from the JSON encoder path.
struct JsonCompatUint8Array {
	bytes: Vec<u8>,
}

impl<'de> serde::Deserialize<'de> for JsonCompatUint8Array {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
		let (tag, base64): (String, String) =
			serde::Deserialize::deserialize(deserializer)?;
		if tag != "$Uint8Array" {
			return Err(serde::de::Error::custom(format!(
				"expected $Uint8Array wrapper, got {tag}"
			)));
		}
		let bytes = BASE64
			.decode(&base64)
			.map_err(|error| serde::de::Error::custom(format!("base64 decode: {error}")))?;
		Ok(Self { bytes })
	}
}
