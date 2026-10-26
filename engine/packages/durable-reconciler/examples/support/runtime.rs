//! Shared example runtime; each example or test uses a different subset of these helpers.
#![allow(dead_code)]

use rivet_env::{Env, ProductionEnv};
use std::{future::Future, sync::Arc, time::Duration};

pub struct Runtime {
	pub env: Arc<dyn Env>,
}

impl Runtime {
	pub fn new() -> anyhow::Result<Self> {
		Ok(Self {
			env: Arc::new(ProductionEnv::new()?),
		})
	}

	pub async fn sleep(&self, duration: Duration) {
		rivet_env::sleep(self.env.clock().as_ref(), duration)
			.await
			.expect("example clock");
	}

	pub async fn timeout<F: Future>(
		&self,
		duration: Duration,
		future: F,
	) -> anyhow::Result<F::Output> {
		rivet_env::timeout(self.env.clock().as_ref(), duration, future)
			.await
			.map_err(Into::into)
	}

	#[track_caller]
	pub fn spawn<F>(&self, future: F) -> rivet_env::capabilities::tokio::task::JoinHandle<F::Output>
	where
		F: Future + Send + 'static,
		F::Output: Send + 'static,
	{
		rivet_env::capabilities::tokio::task::spawn(self.env.as_ref(), future)
	}
}
