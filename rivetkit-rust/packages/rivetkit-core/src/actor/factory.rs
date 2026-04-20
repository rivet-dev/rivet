use std::fmt;

use anyhow::Result;
use futures::future::BoxFuture;

use crate::actor::callbacks::ActorStart;
use crate::ActorConfig;

pub type ActorEntryFn =
	dyn Fn(ActorStart) -> BoxFuture<'static, Result<()>> + Send + Sync;

/// Runtime extension point for building actor receive loops.
pub struct ActorFactory {
	config: ActorConfig,
	entry: Box<ActorEntryFn>,
}

impl ActorFactory {
	pub fn new<F>(config: ActorConfig, entry: F) -> Self
	where
		F: Fn(ActorStart) -> BoxFuture<'static, Result<()>> + Send + Sync + 'static,
	{
		Self {
			config,
			entry: Box::new(entry),
		}
	}

	pub fn config(&self) -> &ActorConfig {
		&self.config
	}

	pub async fn start(&self, start: ActorStart) -> Result<()> {
		(self.entry)(start).await
	}
}

impl fmt::Debug for ActorFactory {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ActorFactory")
			.field("config", &self.config)
			.field("entry", &"<boxed entry>")
			.finish()
	}
}
