//! Filesystem actions. Each helper takes `&AgentOs` plus typed args
//! and delegates to the matching upstream `AgentOs::*` method.

use agent_os_client::AgentOs;
use anyhow::Result;

/// `readFile(path)` — port of [`AgentOs::read_file`].
pub async fn read_file(vm: &AgentOs, path: &str) -> Result<Vec<u8>> {
	vm.read_file(path).await
}
