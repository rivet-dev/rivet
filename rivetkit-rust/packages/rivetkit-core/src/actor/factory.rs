use std::fmt;

use anyhow::Result;
use futures::future::BoxFuture;

use crate::ActorConfig;
use crate::actor::lifecycle_hooks::ActorStart;

pub type ActorEntryFn = dyn Fn(ActorStart) -> BoxFuture<'static, Result<()>> + Send + Sync;

/// Runtime extension point for building actor receive loops.
pub struct ActorFactory {
	config: ActorConfig,
	entry: Box<ActorEntryFn>,
	manual_startup_ready: bool,
}

impl ActorFactory {
	pub fn new<F>(config: ActorConfig, entry: F) -> Self
	where
		F: Fn(ActorStart) -> BoxFuture<'static, Result<()>> + Send + Sync + 'static,
	{
		Self {
			config,
			entry: Box::new(entry),
			manual_startup_ready: false,
		}
	}

	/// Builds a factory whose runtime will explicitly signal `startup_ready`
	/// after its own startup preamble finishes.
	pub fn new_with_manual_startup_ready<F>(config: ActorConfig, entry: F) -> Self
	where
		F: Fn(ActorStart) -> BoxFuture<'static, Result<()>> + Send + Sync + 'static,
	{
		Self {
			config,
			entry: Box::new(entry),
			manual_startup_ready: true,
		}
	}

	pub fn config(&self) -> &ActorConfig {
		&self.config
	}

	pub(crate) fn requires_manual_startup_ready(&self) -> bool {
		self.manual_startup_ready
	}

	pub async fn start(&self, start: ActorStart) -> Result<()> {
		(self.entry)(start).await
	}
}

impl fmt::Debug for ActorFactory {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ActorFactory")
			.field("config", &self.config)
			.field("manual_startup_ready", &self.manual_startup_ready)
			.field("entry", &"<boxed entry>")
			.finish()
	}
}
