//! Test envoy wrapper for engine tests.
//!
//! This module provides a `TestEnvoyBuilder` that wraps the standalone `rivet-test-envoy`
//! package, adding test-specific functionality like building from a `TestDatacenter`.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

// Re-export everything from the standalone package
pub use rivet_envoy_protocol::PROTOCOL_VERSION;
pub use rivet_test_envoy::{
	ActorConfig, ActorEvent, ActorLifecycleEvent, ActorStartResult, ActorStopResult,
	CountingCrashActor, CrashNTimesThenSucceedActor, CrashOnStartActor, CustomActor,
	CustomActorBuilder, DelayedStartActor, EchoActor, Envoy, EnvoyBuilder, EnvoyConfig, KvRequest,
	NotifyOnStartActor, SleepImmediatelyActor, StopImmediatelyActor, TestActor, TimeoutActor,
	VerifyInputActor,
};

// Type alias for backwards compatibility
pub type TestEnvoy = Envoy;

type ActorFactory = Arc<dyn Fn(ActorConfig) -> Box<dyn TestActor> + Send + Sync>;

/// Test-specific envoy builder that integrates with TestDatacenter
pub struct TestEnvoyBuilder {
	namespace: String,
	pool_name: String,
	version: u32,
	actor_factories: HashMap<String, ActorFactory>,
}

impl TestEnvoyBuilder {
	pub fn new(namespace: &str) -> Self {
		Self {
			namespace: namespace.to_string(),
			pool_name: "test-envoy".to_string(),
			version: 1,
			actor_factories: HashMap::new(),
		}
	}

	pub fn with_pool_name(mut self, name: &str) -> Self {
		self.pool_name = name.to_string();
		self
	}

	pub fn with_version(mut self, version: u32) -> Self {
		self.version = version;
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

	/// Build the envoy using the TestDatacenter's guard port
	pub async fn build(self, dc: &super::TestDatacenter) -> Result<Envoy> {
		let endpoint = format!("http://127.0.0.1:{}", dc.guard_port());
		let token = "dev".to_string();

		// Build the config using the new API
		let config = EnvoyConfig::builder()
			.endpoint(&endpoint)
			.token(&token)
			.namespace(&self.namespace)
			.pool_name(&self.pool_name)
			.version(self.version)
			.build()?;

		// Build the envoy
		let mut builder = EnvoyBuilder::new(config);

		// Register all actor factories
		for (name, factory) in self.actor_factories {
			builder = builder.with_actor_behavior(&name, move |config| factory(config));
		}

		builder.build()
	}
}
