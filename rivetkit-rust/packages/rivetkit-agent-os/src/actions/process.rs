//! Process actions. Each helper takes `&AgentOs` plus typed args and
//! delegates to the matching upstream `AgentOs::*` method. DTOs used
//! by `exec` and other arms that need camelCase serialization live
//! here so the dispatcher arms can reply directly.

use agent_os_client::{
	AgentOs, ExecOptions, ExecResult, ProcessInfo, ProcessTreeNode, SpawnHandle,
	SpawnOptions, SpawnedProcessInfo,
};
use anyhow::Result;
use futures::StreamExt;
use rivetkit::Ctx;
use serde::Serialize;

use crate::actor::AgentOsActor;

/// `exec(command)` — port of [`AgentOs::exec`] with default options.
/// Returns an [`ExecResultDto`] with camelCase `exitCode` for the JS side.
pub async fn exec(vm: &AgentOs, command: &str) -> Result<ExecResultDto> {
	vm.exec(command, ExecOptions::default())
		.await
		.map(ExecResultDto::from)
}

/// `spawn(command, args)` — port of [`AgentOs::spawn`]. Returns the
/// [`SpawnHandle`] `{ pid }`. Side effect: spawns background tasks
/// that subscribe to the new process's stdout/stderr and exit, then
/// rebroadcast them on the actor via
/// `ctx.broadcast("processOutput", ...)` and
/// `ctx.broadcast("processExit", ...)`. The forwarders self-terminate
/// when the underlying streams close (process exits).
pub fn spawn(
	vm: &AgentOs,
	ctx: &Ctx<AgentOsActor>,
	command: &str,
	args: Vec<String>,
) -> Result<SpawnHandle> {
	let handle = vm.spawn(command, args, SpawnOptions::default())?;
	let pid = handle.pid;
	spawn_output_forwarder(vm, ctx, pid, OutputStream::Stdout);
	spawn_output_forwarder(vm, ctx, pid, OutputStream::Stderr);
	spawn_exit_forwarder(vm, ctx, pid);
	Ok(handle)
}

#[derive(Clone, Copy)]
enum OutputStream {
	Stdout,
	Stderr,
}

impl OutputStream {
	fn name(self) -> &'static str {
		match self {
			Self::Stdout => "stdout",
			Self::Stderr => "stderr",
		}
	}
}

fn spawn_output_forwarder(
	vm: &AgentOs,
	ctx: &Ctx<AgentOsActor>,
	pid: u32,
	stream_kind: OutputStream,
) {
	let stream_result = match stream_kind {
		OutputStream::Stdout => vm.on_process_stdout(pid),
		OutputStream::Stderr => vm.on_process_stderr(pid),
	};
	let mut byte_stream = match stream_result {
		Ok(stream) => stream,
		Err(error) => {
			tracing::warn!(
				?error,
				pid,
				stream = stream_kind.name(),
				"failed to subscribe to process output; broadcast disabled"
			);
			return;
		}
	};
	let ctx = ctx.clone();
	tokio::spawn(async move {
		use base64::Engine as _;
		let encoder = base64::engine::general_purpose::STANDARD;
		while let Some(chunk) = byte_stream.next().await {
			// Rivetkit's `ctx.broadcast` uses raw CBOR encoding without
			// the `JsonCompatAdapter` byte-wrap. CBOR byte strings can't
			// pass through a JSON-encoding subscriber cell, so we
			// base64-encode the data ourselves and let the TS side
			// decode. Pre-wrapping as `["$Uint8Array", base64]` here
			// would be cleaner, but the broadcast pipe doesn't trust
			// payload shape uniformly across encodings — base64-string
			// works in all three.
			let payload = ProcessOutputPayload {
				pid,
				stream: stream_kind.name(),
				data_base64: encoder.encode(&chunk),
			};
			if let Err(error) = ctx.broadcast("processOutput", &(payload,)) {
				tracing::warn!(
					?error,
					pid,
					stream = stream_kind.name(),
					"processOutput broadcast failed"
				);
			}
		}
	});
}

fn spawn_exit_forwarder(vm: &AgentOs, ctx: &Ctx<AgentOsActor>, pid: u32) {
	let vm = vm.clone();
	let ctx = ctx.clone();
	tokio::spawn(async move {
		match vm.wait_process(pid).await {
			Ok(exit_code) => {
				let payload = ProcessExitPayload { pid, exit_code };
				if let Err(error) = ctx.broadcast("processExit", &(payload,)) {
					tracing::warn!(
						?error,
						pid,
						"processExit broadcast failed"
					);
				}
			}
			Err(error) => {
				tracing::warn!(
					?error,
					pid,
					"wait_process failed in exit forwarder"
				);
			}
		}
	});
}

/// Payload for `processOutput` broadcasts. `dataBase64` is the chunk
/// encoded as standard base64 (TS side decodes via `atob`/Buffer).
#[derive(Serialize)]
struct ProcessOutputPayload {
	pid: u32,
	/// `"stdout"` or `"stderr"`.
	stream: &'static str,
	#[serde(rename = "dataBase64")]
	data_base64: String,
}

/// Payload for `processExit` broadcasts.
#[derive(Serialize)]
struct ProcessExitPayload {
	pid: u32,
	#[serde(rename = "exitCode")]
	exit_code: i32,
}

/// `waitProcess(pid)` — port of [`AgentOs::wait_process`]. Returns the
/// exit code (`i32`).
pub async fn wait_process(vm: &AgentOs, pid: u32) -> Result<i32> {
	vm.wait_process(pid).await.map_err(anyhow::Error::from)
}

/// `killProcess(pid)` — port of [`AgentOs::kill_process`] (sync).
pub fn kill_process(vm: &AgentOs, pid: u32) -> Result<()> {
	vm.kill_process(pid).map_err(anyhow::Error::from)
}

/// `stopProcess(pid)` — port of [`AgentOs::stop_process`] (sync).
pub fn stop_process(vm: &AgentOs, pid: u32) -> Result<()> {
	vm.stop_process(pid).map_err(anyhow::Error::from)
}

/// `listProcesses()` — port of [`AgentOs::list_processes`]. Returns the
/// SDK-spawned processes (not kernel processes); already camelCase via
/// `#[serde(rename = "exitCode")]` on `SpawnedProcessInfo`.
pub fn list_processes(vm: &AgentOs) -> Vec<SpawnedProcessInfo> {
	vm.list_processes()
}

/// `allProcesses()` — port of [`AgentOs::all_processes`]. Returns the
/// full kernel process snapshot.
pub async fn all_processes(vm: &AgentOs) -> Result<Vec<ProcessInfo>> {
	vm.all_processes().await
}

/// `processTree()` — port of [`AgentOs::process_tree`]. Returns the
/// kernel process forest.
pub async fn process_tree(vm: &AgentOs) -> Result<Vec<ProcessTreeNode>> {
	vm.process_tree().await
}

/// `getProcess(pid)` — port of [`AgentOs::get_process`] (sync).
pub fn get_process(vm: &AgentOs, pid: u32) -> Result<SpawnedProcessInfo> {
	vm.get_process(pid).map_err(anyhow::Error::from)
}

/// `writeProcessStdin(pid, data)` — port of
/// [`AgentOs::write_process_stdin`]. Accepts string or bytes content
/// via the same coercion rules as `writeFile`.
pub fn write_process_stdin(
	vm: &AgentOs,
	pid: u32,
	data: super::filesystem::WriteFileContent,
) -> Result<()> {
	use agent_os_client::StdinInput;
	let stdin = StdinInput::Bytes(data.into_bytes());
	vm.write_process_stdin(pid, stdin)
		.map_err(anyhow::Error::from)
}

/// `closeProcessStdin(pid)` — port of [`AgentOs::close_process_stdin`].
pub fn close_process_stdin(vm: &AgentOs, pid: u32) -> Result<()> {
	vm.close_process_stdin(pid).map_err(anyhow::Error::from)
}

// ---------------------------------------------------------------------------
// Action reply DTOs
// ---------------------------------------------------------------------------

/// Serializable mirror of [`ExecResult`] with camelCase `exitCode`. The
/// upstream type doesn't derive `Serialize`, and the field name is
/// `exit_code` (snake_case) which the JS test expects as `exitCode`.
#[derive(Serialize)]
pub struct ExecResultDto {
	#[serde(rename = "exitCode")]
	pub exit_code: i32,
	pub stdout: String,
	pub stderr: String,
}

impl From<ExecResult> for ExecResultDto {
	fn from(value: ExecResult) -> Self {
		Self {
			exit_code: value.exit_code,
			stdout: value.stdout,
			stderr: value.stderr,
		}
	}
}
