use std::fmt;

use anyhow::Result;
use futures::future::BoxFuture;

use crate::actor::callbacks::ActorInstanceCallbacks;
use crate::actor::context::ActorContext;
use crate::ActorConfig;

pub type ActorFactoryCreateFn =
	dyn Fn(FactoryRequest) -> BoxFuture<'static, Result<ActorInstanceCallbacks>> + Send + Sync;

pub struct ActorFactory {
	config: ActorConfig,
	create: Box<ActorFactoryCreateFn>,
}

#[derive(Clone, Debug)]
pub struct FactoryRequest {
	pub ctx: ActorContext,
	pub input: Option<Vec<u8>>,
	pub is_new: bool,
}

impl ActorFactory {
	pub fn new<F>(config: ActorConfig, create: F) -> Self
	where
		F: Fn(FactoryRequest) -> BoxFuture<'static, Result<ActorInstanceCallbacks>>
			+ Send
			+ Sync
			+ 'static,
	{
		Self {
			config,
			create: Box::new(create),
		}
	}

	pub fn config(&self) -> &ActorConfig {
		&self.config
	}

	pub async fn create(&self, request: FactoryRequest) -> Result<ActorInstanceCallbacks> {
		(self.create)(request).await
	}
}

impl fmt::Debug for ActorFactory {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ActorFactory")
			.field("config", &self.config)
			.field("create", &"<boxed callback>")
			.finish()
	}
}
