//! Process actions. Each helper takes `&AgentOs` plus typed args and
//! delegates to the matching upstream `AgentOs::*` method. DTOs used
//! by `exec` and other arms that need camelCase serialization live
//! here so the dispatcher arms can reply directly.

use agent_os_client::{
	AgentOs, ExecOptions, ExecResult, ProcessInfo, ProcessTreeNode, SpawnHandle,
	SpawnOptions, SpawnedProcessInfo,
};
use anyhow::Result;
use serde::Serialize;

/// `exec(command)` — port of [`AgentOs::exec`] with default options.
/// Returns an [`ExecResultDto`] with camelCase `exitCode` for the JS side.
pub async fn exec(vm: &AgentOs, command: &str) -> Result<ExecResultDto> {
	vm.exec(command, ExecOptions::default())
		.await
		.map(ExecResultDto::from)
}

/// `spawn(command, args)` — port of [`AgentOs::spawn`]. Returns the
/// [`SpawnHandle`] `{ pid }` directly; the underlying type already
/// derives `Serialize`.
pub fn spawn(vm: &AgentOs, command: &str, args: Vec<String>) -> Result<SpawnHandle> {
	vm.spawn(command, args, SpawnOptions::default())
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
