use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use rivet_error::RivetError;
use serde::{Deserialize, Serialize};
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDispatchError {
	pub group: String,
	pub code: String,
	pub message: String,
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

		let result = self.dispatch_inner(request).await;
		ctx.trigger_throttled_state_save();

		if result.is_err() {
			tracing::error!(action_name, error = ?result.as_ref().err(), "action dispatch failed");
		}

		result
	}

	async fn dispatch_inner(
		&self,
		request: ActionRequest,
	) -> std::result::Result<Vec<u8>, ActionDispatchError> {
		let handler = self
			.callbacks
			.actions
			.get(&request.name)
			.ok_or_else(|| ActionDispatchError::action_not_found(&request.name))?;

		let action_name = request.name.clone();
		let action_args = request.args.clone();
		let ctx = request.ctx.clone();

		let output = timeout(self.config.action_timeout, handler(request))
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
		}
	}

	fn from_anyhow(error: anyhow::Error) -> Self {
		let error = RivetError::extract(&error);
		Self {
			group: error.group().to_owned(),
			code: error.code().to_owned(),
			message: error.message().to_owned(),
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
mod tests {
	use std::time::Duration;

	use anyhow::Result;
	use futures::future::BoxFuture;
	use rivet_error::INTERNAL_ERROR;
	use tokio::time::sleep;

	use super::{ActionDispatchError, ActionInvoker};
	use crate::actor::callbacks::{
		ActionHandler, ActionRequest, ActorInstanceCallbacks,
		BeforeActionResponseCallback,
	};
	use crate::actor::config::ActorConfig;
	use crate::actor::connection::ConnHandle;
	use crate::actor::context::ActorContext;

	fn action_request(name: &str, args: &[u8]) -> ActionRequest {
		ActionRequest {
			ctx: ActorContext::default(),
			conn: ConnHandle::default(),
			name: name.to_owned(),
			args: args.to_vec(),
		}
	}

	fn action_handler<F>(handler: F) -> ActionHandler
	where
		F: Fn(ActionRequest) -> BoxFuture<'static, Result<Vec<u8>>> + Send + Sync + 'static,
	{
		Box::new(handler)
	}

	fn before_action_response<F>(handler: F) -> BeforeActionResponseCallback
	where
		F: Fn(
				crate::actor::callbacks::OnBeforeActionResponseRequest,
			) -> BoxFuture<'static, Result<Vec<u8>>>
			+ Send
			+ Sync
			+ 'static,
	{
		Box::new(handler)
	}

	#[tokio::test]
	async fn dispatch_returns_handler_output() {
		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.actions.insert(
			"echo".to_owned(),
			action_handler(|request| Box::pin(async move { Ok(request.args) })),
		);

		let invoker = ActionInvoker::new(ActorConfig::default(), callbacks);
		let output = invoker
			.dispatch(action_request("echo", b"ping"))
			.await
			.expect("action should succeed");

		assert_eq!(output, b"ping");
	}

	#[tokio::test]
	async fn dispatch_transforms_output_before_returning() {
		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.actions.insert(
			"echo".to_owned(),
			action_handler(|request| Box::pin(async move { Ok(request.args) })),
		);
		callbacks.on_before_action_response = Some(before_action_response(|request| {
			Box::pin(async move {
				let mut output = request.output;
				output.extend_from_slice(b"-done");
				Ok(output)
			})
		}));

		let invoker = ActionInvoker::new(ActorConfig::default(), callbacks);
		let output = invoker
			.dispatch(action_request("echo", b"ping"))
			.await
			.expect("action should succeed");

		assert_eq!(output, b"ping-done");
	}

	#[tokio::test]
	async fn dispatch_uses_original_output_when_response_hook_fails() {
		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.actions.insert(
			"echo".to_owned(),
			action_handler(|request| Box::pin(async move { Ok(request.args) })),
		);
		callbacks.on_before_action_response = Some(before_action_response(|_| {
			Box::pin(async move { Err(INTERNAL_ERROR.build()) })
		}));

		let invoker = ActionInvoker::new(ActorConfig::default(), callbacks);
		let output = invoker
			.dispatch(action_request("echo", b"ping"))
			.await
			.expect("action should succeed");

		assert_eq!(output, b"ping");
	}

	#[tokio::test]
	async fn dispatch_returns_action_not_found_error() {
		let invoker = ActionInvoker::default();
		let error = invoker
			.dispatch(action_request("missing", b""))
			.await
			.expect_err("missing action should fail");

		assert_eq!(
			error,
			ActionDispatchError {
				group: "actor".to_owned(),
				code: "action_not_found".to_owned(),
				message: "action `missing` was not found".to_owned(),
			}
		);
	}

	#[tokio::test]
	async fn dispatch_returns_timeout_error() {
		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.actions.insert(
			"slow".to_owned(),
			action_handler(|_| {
				Box::pin(async move {
					sleep(Duration::from_millis(25)).await;
					Ok(Vec::new())
				})
			}),
		);

		let invoker = ActionInvoker::new(
			ActorConfig {
				action_timeout: Duration::from_millis(5),
				..ActorConfig::default()
			},
			callbacks,
		);

		let error = invoker
			.dispatch(action_request("slow", b""))
			.await
			.expect_err("slow action should time out");

		assert_eq!(error.group, "actor");
		assert_eq!(error.code, "action_timed_out");
		assert!(error.message.contains("slow"));
	}

	#[tokio::test]
	async fn dispatch_extracts_group_code_and_message_from_anyhow_errors() {
		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.actions.insert(
			"explode".to_owned(),
			action_handler(|_| Box::pin(async move { Err(INTERNAL_ERROR.build()) })),
		);

		let invoker = ActionInvoker::new(ActorConfig::default(), callbacks);
		let error = invoker
			.dispatch(action_request("explode", b""))
			.await
			.expect_err("action should fail");

		assert_eq!(error.group, "core");
		assert_eq!(error.code, "internal_error");
		assert_eq!(error.message, "An internal error occurred");
	}
}
