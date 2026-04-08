//! Test runner wrapper for engine tests.
//!
//! This module now adapts the Rust `rivet-test-envoy` harness to the legacy
//! runner-oriented test surface that the engine tests still import.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub use rivet_envoy_protocol::PROTOCOL_VERSION;
pub use rivet_runner_protocol as protocol_types;
pub use rivet_test_envoy::{
	ActorConfig, ActorEvent, ActorLifecycleEvent, ActorStartResult, ActorStopResult,
	CountingCrashActor, CrashNTimesThenSucceedActor, CrashOnStartActor, CustomActor,
	CustomActorBuilder, DelayedStartActor, EchoActor, Envoy, EnvoyBuilder as TestEnvoyBuilderImpl,
	EnvoyConfig, KvRequest, NotifyOnStartActor, SleepImmediatelyActor, StopImmediatelyActor,
	TestActor, TimeoutActor, VerifyInputActor,
};

type ActorFactory = Arc<dyn Fn(ActorConfig) -> Box<dyn TestActor> + Send + Sync>;

pub type TestRunner = Runner;
pub type RunnerBuilderLegacy = RunnerBuilder;

#[derive(Clone)]
pub struct RunnerConfig {
	endpoint: String,
	token: String,
	namespace: String,
	runner_name: String,
	runner_key: String,
	version: u32,
	total_slots: u32,
}

impl RunnerConfig {
	pub fn builder() -> RunnerConfigBuilder {
		RunnerConfigBuilder::default()
	}
}

#[derive(Default)]
pub struct RunnerConfigBuilder {
	endpoint: Option<String>,
	token: Option<String>,
	namespace: Option<String>,
	runner_name: Option<String>,
	runner_key: Option<String>,
	version: Option<u32>,
	total_slots: Option<u32>,
}

impl RunnerConfigBuilder {
	pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
		self.endpoint = Some(endpoint.into());
		self
	}

	pub fn token(mut self, token: impl Into<String>) -> Self {
		self.token = Some(token.into());
		self
	}

	pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
		self.namespace = Some(namespace.into());
		self
	}

	pub fn runner_name(mut self, runner_name: impl Into<String>) -> Self {
		self.runner_name = Some(runner_name.into());
		self
	}

	pub fn runner_key(mut self, runner_key: impl Into<String>) -> Self {
		self.runner_key = Some(runner_key.into());
		self
	}

	pub fn version(mut self, version: u32) -> Self {
		self.version = Some(version);
		self
	}

	pub fn total_slots(mut self, total_slots: u32) -> Self {
		self.total_slots = Some(total_slots);
		self
	}

	pub fn build(self) -> Result<RunnerConfig> {
		Ok(RunnerConfig {
			endpoint: self
				.endpoint
				.ok_or_else(|| anyhow::anyhow!("endpoint is required"))?,
			token: self.token.unwrap_or_else(|| "dev".to_string()),
			namespace: self
				.namespace
				.ok_or_else(|| anyhow::anyhow!("namespace is required"))?,
			runner_name: self
				.runner_name
				.unwrap_or_else(|| "test-runner".to_string()),
			runner_key: self
				.runner_key
				.unwrap_or_else(|| format!("key-{:012x}", rand::random::<u64>())),
			version: self.version.unwrap_or(1),
			total_slots: self.total_slots.unwrap_or(100),
		})
	}
}

pub struct RunnerBuilder {
	config: RunnerConfig,
	actor_factories: HashMap<String, ActorFactory>,
}

impl RunnerBuilder {
	pub fn new(config: RunnerConfig) -> Self {
		Self {
			config,
			actor_factories: HashMap::new(),
		}
	}

	pub fn with_actor_behavior<F>(mut self, actor_name: &str, factory: F) -> Self
	where
		F: Fn(ActorConfig) -> Box<dyn TestActor> + Send + Sync + 'static,
	{
		self.actor_factories
			.insert(actor_name.to_string(), Arc::new(factory));
		self
	}

	pub fn build(self) -> Result<Runner> {
		let envoy_config = EnvoyConfig::builder()
			.endpoint(&self.config.endpoint)
			.token(&self.config.token)
			.namespace(&self.config.namespace)
			.pool_name(&self.config.runner_name)
			.version(self.config.version)
			.metadata(serde_json::json!({
				"runner_key": self.config.runner_key,
				"total_slots": self.config.total_slots,
			}))
			.build()?;

		let mut builder = TestEnvoyBuilderImpl::new(envoy_config);
		for (name, factory) in self.actor_factories {
			builder = builder.with_actor_behavior(&name, move |config| factory(config));
		}

		Ok(Runner {
			runner_id: format!("runner-{}", uuid::Uuid::new_v4()),
			runner_name: self.config.runner_name,
			envoy: builder.build()?,
		})
	}
}

pub struct Runner {
	pub runner_id: String,
	runner_name: String,
	envoy: Envoy,
}

impl Runner {
	pub async fn start(&self) -> Result<()> {
		self.envoy.start().await
	}

	pub async fn wait_ready(&self) -> String {
		self.envoy.wait_ready().await;
		self.runner_id.clone()
	}

	pub async fn has_actor(&self, actor_id: &str) -> bool {
		self.envoy.has_actor(actor_id).await
	}

	pub async fn get_actor_ids(&self) -> Vec<String> {
		self.envoy.get_actor_ids().await
	}

	pub fn name(&self) -> &str {
		&self.runner_name
	}

	pub fn subscribe_lifecycle_events(
		&self,
	) -> tokio::sync::broadcast::Receiver<ActorLifecycleEvent> {
		self.envoy.subscribe_lifecycle_events()
	}

	pub async fn shutdown(&self) {
		self.envoy.shutdown().await;
	}

	pub async fn crash(&self) {
		self.envoy.crash().await;
	}
}

/// Test-specific runner builder that integrates with TestDatacenter.
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

	pub fn with_actor_behavior<F>(mut self, actor_name: &str, factory: F) -> Self
	where
		F: Fn(ActorConfig) -> Box<dyn TestActor> + Send + Sync + 'static,
	{
		self.actor_factories
			.insert(actor_name.to_string(), Arc::new(factory));
		self
	}

	pub async fn build(self, dc: &super::TestDatacenter) -> Result<Runner> {
		let endpoint = format!("http://127.0.0.1:{}", dc.guard_port());
		let token = "dev".to_string();

		let config = RunnerConfig::builder()
			.endpoint(&endpoint)
			.token(&token)
			.namespace(&self.namespace)
			.runner_name(&self.runner_name)
			.runner_key(&self.runner_key)
			.version(self.version)
			.total_slots(self.total_slots)
			.build()?;

		let mut builder = RunnerBuilder::new(config);
		for (name, factory) in self.actor_factories {
			builder = builder.with_actor_behavior(&name, move |config| factory(config));
		}

		builder.build()
	}
}
