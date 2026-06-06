use std::sync::Arc;

use agent_os_client::AgentOsConfig;

/// Configuration for the agent-os actor.
///
/// `build_options` is a closure that yields a fresh [`AgentOsConfig`]
/// on every call. `AgentOsConfig` is non-`Clone` (it holds
/// `Arc<dyn ScheduleDriver>` and other trait-object handles), and the
/// actor needs to rebuild it across sleep/wake cycles, so the config
/// is expressed as a factory rather than a value.
#[derive(Clone)]
pub struct AgentOsActorConfig {
	build_options: Arc<dyn Fn() -> AgentOsConfig + Send + Sync>,
}

impl AgentOsActorConfig {
	/// Construct from a closure that builds a fresh [`AgentOsConfig`]
	/// each time the actor needs to bring up a VM.
	pub fn from_builder<F>(builder: F) -> Self
	where
		F: Fn() -> AgentOsConfig + Send + Sync + 'static,
	{
		Self {
			build_options: Arc::new(builder),
		}
	}

	/// Yield a fresh [`AgentOsConfig`] for VM bring-up.
	pub fn build_options(&self) -> AgentOsConfig {
		(self.build_options)()
	}
}

impl Default for AgentOsActorConfig {
	/// Default config: every bring-up uses [`AgentOsConfig::default`].
	fn default() -> Self {
		Self::from_builder(AgentOsConfig::default)
	}
}
