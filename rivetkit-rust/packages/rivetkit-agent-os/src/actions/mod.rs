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
		_ => action.err(not_implemented(&name)),
	}
}

fn not_implemented(name: &str) -> anyhow::Error {
	anyhow!("agent-os action not implemented yet: {name}")
}
