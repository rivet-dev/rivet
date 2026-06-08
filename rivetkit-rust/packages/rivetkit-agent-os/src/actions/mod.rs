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
use filesystem::{WriteFileContent, WriteFilesEntryArg};

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
		"mkdir" => {
			let args: Result<(String,)> = action.decode_as();
			match args {
				Ok((path,)) => match filesystem::mkdir(vm, &path).await {
					Ok(()) => action.ok(&()),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"readdir" => {
			let args: Result<(String,)> = action.decode_as();
			match args {
				Ok((path,)) => match filesystem::readdir(vm, &path).await {
					Ok(entries) => action.ok(&entries),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"exists" => {
			let args: Result<(String,)> = action.decode_as();
			match args {
				Ok((path,)) => match filesystem::exists(vm, &path).await {
					Ok(present) => action.ok(&present),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"move" => {
			let args: Result<(String, String)> = action.decode_as();
			match args {
				Ok((from, to)) => match filesystem::move_path(vm, &from, &to).await {
					Ok(()) => action.ok(&()),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"deleteFile" => {
			let args: Result<(String,)> = action.decode_as();
			match args {
				Ok((path,)) => match filesystem::delete_file(vm, &path).await {
					Ok(()) => action.ok(&()),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"writeFiles" => {
			let args: Result<(Vec<WriteFilesEntryArg>,)> = action.decode_as();
			match args {
				Ok((entries,)) => {
					let results = filesystem::write_files(vm, entries).await;
					action.ok(&results);
				}
				Err(error) => action.err(error),
			}
		}
		"readFiles" => {
			let args: Result<(Vec<String>,)> = action.decode_as();
			match args {
				Ok((paths,)) => {
					let results = filesystem::read_files(vm, paths).await;
					action.ok(&results);
				}
				Err(error) => action.err(error),
			}
		}
		"readdirRecursive" => {
			let args: Result<(String,)> = action.decode_as();
			match args {
				Ok((path,)) => match filesystem::readdir_recursive(vm, &path).await {
					Ok(entries) => action.ok(&entries),
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
