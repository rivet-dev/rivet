//! Filesystem actions. Each helper takes `&AgentOs` plus typed args
//! and delegates to the matching upstream `AgentOs::*` method.

use agent_os_client::{AgentOs, FileContent, VirtualStat};
use anyhow::Result;

/// `readFile(path)` — port of [`AgentOs::read_file`].
pub async fn read_file(vm: &AgentOs, path: &str) -> Result<Vec<u8>> {
	vm.read_file(path).await
}

/// `writeFile(path, contents)` — port of [`AgentOs::write_file`].
pub async fn write_file(vm: &AgentOs, path: &str, contents: Vec<u8>) -> Result<()> {
	vm.write_file(path, FileContent::Bytes(contents)).await
}

/// `stat(path)` — port of [`AgentOs::stat`]. Returns the [`VirtualStat`]
/// structure directly; the rivetkit encoder handles cross-encoding
/// translation (bare / cbor / json) at the framework layer.
pub async fn stat(vm: &AgentOs, path: &str) -> Result<VirtualStat> {
	vm.stat(path).await
}
