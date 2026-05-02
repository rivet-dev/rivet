use super::*;

mod moved_tests {
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};
	use std::time::Duration;

	use anyhow::{Result, anyhow};
	use futures::future::BoxFuture;

	use super::Schedule;
	use crate::actor::action::ActionInvoker;
	use crate::actor::callbacks::{ActionHandler, ActorInstanceCallbacks};
	use crate::actor::config::ActorConfig;
	use crate::actor::context::ActorContext;
	use crate::actor::state::ActorState;

	fn action_handler<F>(handler: F) -> ActionHandler
	where
		F: Fn(crate::actor::callbacks::ActionRequest) -> BoxFuture<'static, Result<Vec<u8>>>
			+ Send
			+ Sync
			+ 'static,
	{
		Box::new(handler)
	}

	#[test]
	fn at_inserts_events_in_timestamp_order() {
		let schedule = Schedule::default();

		schedule.at(50, "later", b"");
		schedule.at(10, "sooner", b"");
		schedule.at(30, "middle", b"");

		let actions: Vec<_> = schedule
			.all_events()
			.into_iter()
			.map(|event| event.action)
			.collect();

		assert_eq!(actions, vec!["sooner", "middle", "later"]);
	}

	#[test]
	fn after_creates_future_event() {
		let schedule = Schedule::default();

		schedule.after(Duration::from_millis(5), "ping", b"abc");

		let event = schedule.next_event().expect("scheduled event should exist");
		assert_eq!(event.action, "ping");
		assert_eq!(event.args.as_deref(), Some(b"abc".as_slice()));
		assert!(event.timestamp >= super::now_timestamp_ms());
	}

	#[tokio::test]
	async fn handle_alarm_dispatches_due_events_and_removes_them() {
		let schedule = Schedule::new(ActorState::default(), "actor-1", ActorConfig::default());
		let ctx = ActorContext::default();
		let mut callbacks = ActorInstanceCallbacks::default();
		let seen = Arc::new(AtomicUsize::new(0));
		let seen_clone = seen.clone();
		callbacks.actions.insert(
			"run".to_owned(),
			action_handler(move |request| {
				let seen_clone = seen_clone.clone();
				Box::pin(async move {
					assert_eq!(request.args, b"payload");
					seen_clone.fetch_add(1, Ordering::SeqCst);
					Ok(Vec::new())
				})
			}),
		);

		let invoker = ActionInvoker::new(ActorConfig::default(), callbacks);
		schedule.at(
			super::now_timestamp_ms().saturating_sub(1),
			"run",
			b"payload",
		);
		schedule.at(
			super::now_timestamp_ms().saturating_add(60_000),
			"later",
			b"",
		);

		let executed = schedule.handle_alarm(&ctx, &invoker).await;

		assert_eq!(executed, 1);
		assert_eq!(seen.load(Ordering::SeqCst), 1);
		assert_eq!(schedule.all_events().len(), 1);
		assert_eq!(schedule.next_event().expect("future event").action, "later");
	}

	#[tokio::test]
	async fn handle_alarm_continues_after_errors_and_uses_keep_awake_wrapper() {
		let schedule = Schedule::new(ActorState::default(), "actor-1", ActorConfig::default());
		let ctx = ActorContext::default();
		let mut callbacks = ActorInstanceCallbacks::default();
		let keep_awake_calls = Arc::new(AtomicUsize::new(0));
		let keep_awake_calls_clone = keep_awake_calls.clone();
		schedule.set_internal_keep_awake(Some(Arc::new(move |future| {
			let keep_awake_calls_clone = keep_awake_calls_clone.clone();
			Box::pin(async move {
				keep_awake_calls_clone.fetch_add(1, Ordering::SeqCst);
				future.await
			})
		})));

		let succeeded = Arc::new(AtomicUsize::new(0));
		let succeeded_clone = succeeded.clone();
		callbacks.actions.insert(
			"ok".to_owned(),
			action_handler(move |_| {
				let succeeded_clone = succeeded_clone.clone();
				Box::pin(async move {
					succeeded_clone.fetch_add(1, Ordering::SeqCst);
					Ok(Vec::new())
				})
			}),
		);
		callbacks.actions.insert(
			"fail".to_owned(),
			action_handler(|_| Box::pin(async move { Err(anyhow!("boom")) })),
		);

		let invoker = ActionInvoker::new(ActorConfig::default(), callbacks);
		schedule.at(super::now_timestamp_ms().saturating_sub(1), "fail", b"");
		schedule.at(super::now_timestamp_ms().saturating_sub(1), "ok", b"");

		let executed = schedule.handle_alarm(&ctx, &invoker).await;

		assert_eq!(executed, 2);
		assert_eq!(keep_awake_calls.load(Ordering::SeqCst), 2);
		assert_eq!(succeeded.load(Ordering::SeqCst), 1);
		assert!(schedule.all_events().is_empty());
	}

	#[test]
	fn set_alarm_requires_envoy_handle() {
		let schedule = Schedule::default();
		let error = schedule
			.set_alarm(Some(123))
			.expect_err("set_alarm should fail without envoy");

		assert!(
			error
				.to_string()
				.contains("schedule alarm handle is not configured")
		);
	}
}
