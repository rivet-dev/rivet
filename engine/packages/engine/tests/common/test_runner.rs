//! Test runner wrapper for engine tests.
//!
//! This module provides a `TestRunnerBuilder` that wraps the standalone `rivet-engine-runner`
//! package, adding test-specific functionality like building from a `TestDatacenter`.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

// Re-export everything from the standalone package
pub use rivet_engine_runner::{
	ActorConfig, ActorEvent, ActorLifecycleEvent, ActorStartResult, ActorStopResult,
	CountingCrashActor, CrashNTimesThenSucceedActor, CrashOnStartActor, CustomActor,
	CustomActorBuilder, DelayedStartActor, EchoActor, KvRequest, NotifyOnStartActor,
	PROTOCOL_VERSION, Runner, RunnerBuilder, RunnerBuilderLegacy, RunnerConfig,
	SleepImmediatelyActor, StopImmediatelyActor, TestActor, TimeoutActor, VerifyInputActor,
	protocol_types,
};

// Type alias for backwards compatibility
pub type TestRunner = Runner;

type ActorFactory = Arc<dyn Fn(ActorConfig) -> Box<dyn TestActor> + Send + Sync>;

/// Test-specific runner builder that integrates with TestDatacenter
pub struct TestRunnerBuilder {
	namespace: String,
	runner_name: String,
	runner_key: String,
	version: u32,
	total_slots: u32,
	actor_factories: HashMap<String, ActorFactory>,
}

impl TestRunnerBuilder {
	pub fn new(namespace: &str) -> Self {
		Self {
			namespace: namespace.to_string(),
			runner_name: "test-runner".to_string(),
			runner_key: format!("key-{:012x}", rand::random::<u64>()),
			version: 1,
			total_slots: 100,
			actor_factories: HashMap::new(),
		}
	}

	pub fn with_runner_name(mut self, name: &str) -> Self {
		self.runner_name = name.to_string();
		self
	}

	pub fn with_runner_key(mut self, key: &str) -> Self {
		self.runner_key = key.to_string();
		self
	}

	pub fn with_version(mut self, version: u32) -> Self {
		self.version = version;
		self
	}

	pub fn with_total_slots(mut self, total_slots: u32) -> Self {
		self.total_slots = total_slots;
		self
	}

	/// Register an actor factory for a specific actor name
	pub fn with_actor_behavior<F>(mut self, actor_name: &str, factory: F) -> Self
	where
		F: Fn(ActorConfig) -> Box<dyn TestActor> + Send + Sync + 'static,
	{
		self.actor_factories
			.insert(actor_name.to_string(), Arc::new(factory));
		self
	}

	/// Build the runner using the TestDatacenter's guard port
	pub async fn build(self, dc: &super::TestDatacenter) -> Result<Runner> {
		let endpoint = format!("http://127.0.0.1:{}", dc.guard_port());
		let token = "dev".to_string();

		// Build the config using the new API
		let config = RunnerConfig::builder()
			.endpoint(&endpoint)
			.token(&token)
			.namespace(&self.namespace)
			.runner_name(&self.runner_name)
			.runner_key(&self.runner_key)
			.version(self.version)
			.total_slots(self.total_slots)
			.build()?;

		// Build the runner
		let mut builder = RunnerBuilder::new(config);

		// Register all actor factories
		for (name, factory) in self.actor_factories {
			builder = builder.with_actor_behavior(&name, move |config| factory(config));
		}

		builder.build()
	}
}
