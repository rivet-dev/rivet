use super::*;

mod moved_tests {
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
