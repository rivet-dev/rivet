use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use rivet_error::RivetError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::time::timeout;

use crate::actor::callbacks::{
	ActionRequest, ActorInstanceCallbacks, OnBeforeActionResponseRequest,
};
use crate::actor::config::ActorConfig;

#[derive(Clone)]
pub struct ActionInvoker {
	config: ActorConfig,
	callbacks: Arc<ActorInstanceCallbacks>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionDispatchError {
	pub group: String,
	pub code: String,
	pub message: String,
	pub metadata: Option<JsonValue>,
}

impl ActionInvoker {
	pub fn new(config: ActorConfig, callbacks: ActorInstanceCallbacks) -> Self {
		Self {
			config,
			callbacks: Arc::new(callbacks),
		}
	}

	pub fn with_shared_callbacks(
		config: ActorConfig,
		callbacks: Arc<ActorInstanceCallbacks>,
	) -> Self {
		Self { config, callbacks }
	}

	pub fn config(&self) -> &ActorConfig {
		&self.config
	}

	pub fn callbacks(&self) -> &ActorInstanceCallbacks {
		self.callbacks.as_ref()
	}

	pub async fn dispatch(
		&self,
		request: ActionRequest,
	) -> std::result::Result<Vec<u8>, ActionDispatchError> {
		let ctx = request.ctx.clone();
		let action_name = request.name.clone();
		let started_at = Instant::now();
		let _action_guard = ctx.lock_action_execution().await;
		ctx.record_action_call(&action_name);
		ctx.begin_keep_awake();

		let result = self.dispatch_inner(request).await;
		ctx.end_keep_awake();
		ctx.request_sleep_if_pending();
		ctx.trigger_throttled_state_save();
		ctx.record_action_duration(&action_name, started_at.elapsed());

		if result.is_err() {
			ctx.record_action_error(&action_name);
			tracing::error!(action_name, error = ?result.as_ref().err(), "action dispatch failed");
		}

		result
	}

	async fn dispatch_inner(
		&self,
		request: ActionRequest,
	) -> std::result::Result<Vec<u8>, ActionDispatchError> {
		if request.ctx.destroy_requested() {
			request.ctx.wait_for_destroy_completion().await;
		}

		let handler = self
			.callbacks
			.actions
			.get(&request.name)
			.ok_or_else(|| ActionDispatchError::action_not_found(&request.name))?;

		let action_name = request.name.clone();
		let action_args = request.args.clone();
		let ctx = request.ctx.clone();

		let output = timeout(self.config.action_timeout, async {
			let result = handler(request).await;
			ctx.wait_for_on_state_change_idle().await;
			result
		})
			.await
			.map_err(|_| {
				ActionDispatchError::action_timed_out(&action_name, self.config.action_timeout)
			})?
			.map_err(ActionDispatchError::from_anyhow)?;

		Ok(self
			.transform_output(ctx, action_name, action_args, output)
			.await)
	}

	async fn transform_output(
		&self,
		ctx: crate::actor::context::ActorContext,
		name: String,
		args: Vec<u8>,
		output: Vec<u8>,
	) -> Vec<u8> {
		let Some(callback) = &self.callbacks.on_before_action_response else {
			return output;
		};

		let original_output = output.clone();
		match callback(OnBeforeActionResponseRequest {
			ctx,
			name,
			args,
			output,
		})
		.await
		{
			Ok(transformed) => transformed,
			Err(error) => {
				tracing::error!(?error, "error in on_before_action_response callback");
				original_output
			}
		}
	}
}

impl ActionDispatchError {
	fn action_not_found(action_name: &str) -> Self {
		Self {
			group: "actor".to_owned(),
			code: "action_not_found".to_owned(),
			message: format!("action `{action_name}` was not found"),
			metadata: None,
		}
	}

	fn action_timed_out(action_name: &str, timeout: Duration) -> Self {
		Self {
			group: "actor".to_owned(),
			code: "action_timed_out".to_owned(),
			message: format!(
				"action `{action_name}` timed out after {} ms",
				timeout.as_millis()
			),
			metadata: None,
		}
	}

	pub(crate) fn from_anyhow(error: anyhow::Error) -> Self {
		let error = RivetError::extract(&error);
		Self {
			group: error.group().to_owned(),
			code: error.code().to_owned(),
			message: error.message().to_owned(),
			metadata: error.metadata(),
		}
	}
}

impl Default for ActionInvoker {
	fn default() -> Self {
		Self::new(ActorConfig::default(), ActorInstanceCallbacks::default())
	}
}

impl fmt::Debug for ActionInvoker {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ActionInvoker")
			.field("config", &self.config)
			.field("callbacks", &self.callbacks)
			.finish()
	}
}

#[cfg(test)]
#[path = "../../tests/modules/action.rs"]
mod tests;
