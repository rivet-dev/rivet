//! Action dispatcher entry point.
//!
//! Each arm decodes its positional args with `action.decode_as::<(...)>()`
//! (TS sends args as a CBOR array) and replies via [`Action::ok`] or
//! [`Action::err`]. Byte payloads auto-wrap via the rivetkit
//! `JSON_COMPAT_UINT8_ARRAY` convention thanks to `Action::ok` running
//! through `encode_json_compat`.

pub mod filesystem;
pub mod process;

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
		"exec" => {
			let args: Result<(String,)> = action.decode_as();
			match args {
				Ok((command,)) => match process::exec(vm, &command).await {
					Ok(result) => action.ok(&result),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"spawn" => {
			let args: Result<(String, Vec<String>)> = action.decode_as();
			match args {
				Ok((command, spawn_args)) => match process::spawn(vm, &command, spawn_args) {
					Ok(handle) => action.ok(&handle),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"waitProcess" => {
			let args: Result<(u32,)> = action.decode_as();
			match args {
				Ok((pid,)) => match process::wait_process(vm, pid).await {
					Ok(code) => action.ok(&code),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"killProcess" => {
			let args: Result<(u32,)> = action.decode_as();
			match args {
				Ok((pid,)) => match process::kill_process(vm, pid) {
					Ok(()) => action.ok(&()),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"stopProcess" => {
			let args: Result<(u32,)> = action.decode_as();
			match args {
				Ok((pid,)) => match process::stop_process(vm, pid) {
					Ok(()) => action.ok(&()),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"listProcesses" => {
			// No args.
			let processes = process::list_processes(vm);
			action.ok(&processes);
		}
		"allProcesses" => {
			match process::all_processes(vm).await {
				Ok(processes) => action.ok(&processes),
				Err(error) => action.err(error),
			}
		}
		"processTree" => match process::process_tree(vm).await {
			Ok(tree) => action.ok(&tree),
			Err(error) => action.err(error),
		},
		"getProcess" => {
			let args: Result<(u32,)> = action.decode_as();
			match args {
				Ok((pid,)) => match process::get_process(vm, pid) {
					Ok(info) => action.ok(&info),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"writeProcessStdin" => {
			let args: Result<(u32, WriteFileContent)> = action.decode_as();
			match args {
				Ok((pid, data)) => match process::write_process_stdin(vm, pid, data) {
					Ok(()) => action.ok(&()),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			}
		}
		"closeProcessStdin" => {
			let args: Result<(u32,)> = action.decode_as();
			match args {
				Ok((pid,)) => match process::close_process_stdin(vm, pid) {
					Ok(()) => action.ok(&()),
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
