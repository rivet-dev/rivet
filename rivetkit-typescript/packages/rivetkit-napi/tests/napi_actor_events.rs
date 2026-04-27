use super::*;

mod moved_tests {
	use std::collections::HashMap;
	use std::sync::Arc as StdArc;
	use std::time::Duration;

	use rivet_error::RivetError as RivetTransportError;
	use rivetkit_core::Kv;
	use rivetkit_core::actor::state::{PERSIST_DATA_KEY, PersistedActor};
	use tokio::sync::oneshot;

	use super::*;

	fn test_adapter_config() -> AdapterConfig {
		let timeout = Duration::from_secs(1);
		AdapterConfig {
			create_state_timeout: timeout,
			on_create_timeout: timeout,
			create_vars_timeout: timeout,
			on_migrate_timeout: timeout,
			on_wake_timeout: timeout,
			on_before_actor_start_timeout: timeout,
			create_conn_state_timeout: timeout,
			on_before_connect_timeout: timeout,
			on_connect_timeout: timeout,
			action_timeout: timeout,
			on_request_timeout: timeout,
		}
	}

	fn empty_bindings() -> CallbackBindings {
		CallbackBindings {
			create_state: None,
			on_create: None,
			create_conn_state: None,
			create_vars: None,
			on_migrate: None,
			on_wake: None,
			on_before_actor_start: None,
			on_sleep: None,
			on_destroy: None,
			on_before_connect: None,
			on_connect: None,
			on_disconnect_final: None,
			on_before_subscribe: None,
			actions: HashMap::new(),
			on_before_action_response: None,
			on_queue_send: None,
			on_request: None,
			on_websocket: None,
			run: None,
			get_workflow_history: None,
			replay_workflow: None,
			serialize_state: None,
		}
	}

	fn assert_error_code(error: anyhow::Error, code: &str) {
		let error = RivetTransportError::extract(&error);
		assert_eq!(error.code(), code);
	}

	#[tokio::test(flavor = "current_thread")]
	async fn with_dispatch_cancel_token_cleans_up_after_success() {
		let cancel_token = with_dispatch_cancel_token(|cancel_token| async move {
			Ok::<_, anyhow::Error>(cancel_token)
		})
		.await
		.expect("successful dispatch should resolve");

		assert!(cancel_token.is_cancelled());
	}

	#[tokio::test(flavor = "current_thread")]
	async fn with_dispatch_cancel_token_cleans_up_after_panic() {
		let seen_cancel_token = StdArc::new(parking_lot::Mutex::new(None));

		let join_error = tokio::spawn({
			let seen_cancel_token = StdArc::clone(&seen_cancel_token);
			async move {
				let _ = with_dispatch_cancel_token(|cancel_token| async move {
					*seen_cancel_token.lock() = Some(cancel_token);
					panic!("dispatch panic");
					#[allow(unreachable_code)]
					Ok::<(), anyhow::Error>(())
				})
				.await;
			}
		})
		.await
		.expect_err("panic dispatch should panic");

		assert!(join_error.is_panic());
		let cancel_token = seen_cancel_token
			.lock()
			.clone()
			.expect("panic path should observe the dispatch token");
		assert!(cancel_token.is_cancelled());
	}

	#[tokio::test(flavor = "current_thread")]
	async fn with_dispatch_cancel_token_does_not_leak_under_mixed_load() {
		for i in 0..1000 {
			if i % 2 == 0 {
				let cancel_token = with_dispatch_cancel_token(|cancel_token| async move {
					Ok::<_, anyhow::Error>(cancel_token)
				})
				.await
				.expect("successful dispatch should resolve");
				assert!(cancel_token.is_cancelled());
				continue;
			}

			let join_error = tokio::spawn(async move {
				let _ = with_dispatch_cancel_token(|_| async move {
					panic!("dispatch panic");
					#[allow(unreachable_code)]
					Ok::<(), anyhow::Error>(())
				})
				.await;
			})
			.await
			.expect_err("panic dispatch should panic");

			assert!(join_error.is_panic());
		}
	}

	#[tokio::test]
	async fn action_dispatch_missing_action_returns_not_found() {
		let bindings = Arc::new(empty_bindings());
		let config = test_adapter_config();
		let core_ctx =
			rivetkit_core::ActorContext::new("actor-missing-action", "actor", Vec::new(), "local");
		let ctx = ActorContext::new(core_ctx);
		let abort = CancellationToken::new();
		let dirty = Arc::new(AtomicBool::new(false));
		let mut tasks = JoinSet::new();
		let (_registered_task_tx, mut registered_task_rx) = unbounded_channel();
		let (tx, rx) = oneshot::channel();

		dispatch_event(
			ActorEvent::Action {
				name: "missing".to_owned(),
				args: vec![1, 2, 3],
				conn: None,
				reply: tx.into(),
			},
			&bindings,
			&config,
			&ctx,
			&abort,
			&mut tasks,
			&mut registered_task_rx,
			&dirty,
		)
		.await;

		drain_tasks(&mut tasks, &mut registered_task_rx).await;

		let error = rx
			.await
			.expect("action reply should resolve")
			.expect_err("missing action should error");
		let error = RivetTransportError::extract(&error);
		assert_eq!(error.code(), "action_not_found");
	}

	#[tokio::test]
	async fn subscribe_request_without_guard_is_allowed() {
		let bindings = Arc::new(empty_bindings());
		let config = test_adapter_config();
		let core_ctx =
			rivetkit_core::ActorContext::new("actor-subscribe", "actor", Vec::new(), "local");
		let ctx = ActorContext::new(core_ctx);
		let abort = CancellationToken::new();
		let dirty = Arc::new(AtomicBool::new(false));
		let mut tasks = JoinSet::new();
		let (_registered_task_tx, mut registered_task_rx) = unbounded_channel();
		let (tx, rx) = oneshot::channel();
		let conn = rivetkit_core::ConnHandle::new("conn-subscribe", Vec::new(), Vec::new(), false);

		dispatch_event(
			ActorEvent::SubscribeRequest {
				conn,
				event_name: "ping".to_owned(),
				reply: tx.into(),
			},
			&bindings,
			&config,
			&ctx,
			&abort,
			&mut tasks,
			&mut registered_task_rx,
			&dirty,
		)
		.await;

		drain_tasks(&mut tasks, &mut registered_task_rx).await;

		rx.await
			.expect("subscribe reply should resolve")
			.expect("subscribe without guard should be allowed");
	}

	#[tokio::test]
	async fn connection_open_without_callbacks_is_allowed() {
		let bindings = Arc::new(empty_bindings());
		let config = test_adapter_config();
		let core_ctx =
			rivetkit_core::ActorContext::new("actor-connection-open", "actor", Vec::new(), "local");
		let ctx = ActorContext::new(core_ctx);
		let abort = CancellationToken::new();
		let dirty = Arc::new(AtomicBool::new(false));
		let mut tasks = JoinSet::new();
		let (_registered_task_tx, mut registered_task_rx) = unbounded_channel();
		let (tx, rx) = oneshot::channel();
		let conn = rivetkit_core::ConnHandle::new("conn-open", vec![1, 2, 3], Vec::new(), false);

		dispatch_event(
			ActorEvent::ConnectionOpen {
				conn: conn.clone(),
				params: vec![4, 5, 6],
				request: None,
				reply: tx.into(),
			},
			&bindings,
			&config,
			&ctx,
			&abort,
			&mut tasks,
			&mut registered_task_rx,
			&dirty,
		)
		.await;

		drain_tasks(&mut tasks, &mut registered_task_rx).await;

		rx.await
			.expect("connection-open reply should resolve")
			.expect("connection-open without callbacks should be allowed");
		assert!(conn.state().is_empty());
	}

	#[tokio::test]
	async fn workflow_requests_without_callbacks_return_none() {
		let bindings = Arc::new(empty_bindings());
		let config = test_adapter_config();
		let core_ctx =
			rivetkit_core::ActorContext::new("actor-workflow", "actor", Vec::new(), "local");
		let ctx = ActorContext::new(core_ctx);
		let abort = CancellationToken::new();
		let dirty = Arc::new(AtomicBool::new(false));
		let mut tasks = JoinSet::new();
		let (_registered_task_tx, mut registered_task_rx) = unbounded_channel();
		let (history_tx, history_rx) = oneshot::channel();
		let (replay_tx, replay_rx) = oneshot::channel();

		dispatch_event(
			ActorEvent::WorkflowHistoryRequested {
				reply: history_tx.into(),
			},
			&bindings,
			&config,
			&ctx,
			&abort,
			&mut tasks,
			&mut registered_task_rx,
			&dirty,
		)
		.await;
		dispatch_event(
			ActorEvent::WorkflowReplayRequested {
				entry_id: Some("step-1".to_owned()),
				reply: replay_tx.into(),
			},
			&bindings,
			&config,
			&ctx,
			&abort,
			&mut tasks,
			&mut registered_task_rx,
			&dirty,
		)
		.await;

		drain_tasks(&mut tasks, &mut registered_task_rx).await;

		assert_eq!(
			history_rx
				.await
				.expect("workflow history reply should resolve")
				.expect("workflow history should succeed"),
			None
		);
		assert_eq!(
			replay_rx
				.await
				.expect("workflow replay reply should resolve")
				.expect("workflow replay should succeed"),
			None
		);
	}

	#[tokio::test]
	async fn spawn_reply_sends_stopping_when_abort_is_cancelled() {
		let mut tasks = JoinSet::new();
		let (_registered_task_tx, mut registered_task_rx) = unbounded_channel();
		let abort = CancellationToken::new();
		let (tx, rx) = oneshot::channel();

		spawn_reply(&mut tasks, abort.clone(), tx.into(), async move {
			tokio::time::sleep(Duration::from_secs(60)).await;
			Ok::<_, anyhow::Error>(Vec::<u8>::new())
		});

		abort.cancel();
		drain_tasks(&mut tasks, &mut registered_task_rx).await;

		let error = rx
			.await
			.expect("abort reply should resolve")
			.expect_err("abort should return an error");
		let error = RivetTransportError::extract(&error);
		assert_eq!(error.code(), "stopping");
	}

	#[tokio::test]
	async fn callback_timeout_returns_structured_error_with_metadata() {
		let timeout = Duration::from_millis(10);
		let error = with_timeout("onWake", timeout, std::future::pending::<Result<()>>())
			.await
			.expect_err("callback timeout should fail");
		let error = RivetTransportError::extract(&error);
		assert_eq!(error.group(), "actor");
		assert_eq!(error.code(), "callback_timed_out");
		assert_eq!(
			error.message(),
			format!(
				"callback `onWake` timed out after {} ms",
				timeout.as_millis()
			)
		);
		assert_eq!(
			error.metadata(),
			Some(serde_json::json!({
				"callback_name": "onWake",
				"duration_ms": timeout.as_millis() as u64,
			}))
		);
	}

	#[tokio::test]
	async fn structured_timeout_returns_action_timeout_error() {
		let error = with_structured_timeout(
			"actor",
			"action_timed_out",
			"Action timed out",
			None,
			Duration::from_millis(10),
			std::future::pending::<Result<()>>(),
		)
		.await
		.expect_err("structured timeout should fail");
		let error = RivetTransportError::extract(&error);
		assert_eq!(error.group(), "actor");
		assert_eq!(error.code(), "action_timed_out");
		assert_eq!(error.message(), "Action timed out");
	}

	#[tokio::test]
	async fn run_adapter_loop_resets_stale_shared_end_reason_before_wake() {
		let bindings = Arc::new(empty_bindings());
		let config = Arc::new(test_adapter_config());
		let core_ctx =
			rivetkit_core::ActorContext::new("actor-wake-reset", "actor", Vec::new(), "local");
		let stale_ctx = ActorContext::new(core_ctx.clone());
		stale_ctx.set_end_reason(EndReason::Sleep);

		let (events_tx, events_rx) = unbounded_channel();
		let (first_tx, first_rx) = oneshot::channel();
		let (second_tx, second_rx) = oneshot::channel();

		events_tx
			.send(ActorEvent::Action {
				name: "missing-first".to_owned(),
				args: Vec::new(),
				conn: None,
				reply: first_tx.into(),
			})
			.expect("first action event should send");
		events_tx
			.send(ActorEvent::Action {
				name: "missing-second".to_owned(),
				args: Vec::new(),
				conn: None,
				reply: second_tx.into(),
			})
			.expect("second action event should send");
		drop(events_tx);

		run_adapter_loop(
			bindings,
			config,
			ActorStart {
				ctx: core_ctx,
				input: None,
				snapshot: Some(Vec::new()),
				hibernated: Vec::new(),
				events: ActorEvents::from(events_rx),
				startup_ready: None,
			},
		)
		.await
		.expect("adapter loop should finish cleanly");

		let first_error = first_rx
			.await
			.expect("first action reply should resolve")
			.expect_err("missing action should error");
		assert_error_code(first_error, "action_not_found");

		let second_error = second_rx
			.await
			.expect("second action reply should resolve")
			.expect_err("second missing action should error");
		assert_error_code(second_error, "action_not_found");
		assert_eq!(stale_ctx.take_end_reason(), None);
	}

	#[tokio::test]
	async fn preamble_marks_initialized_and_reloads_as_wake() {
		let kv = Kv::new_in_memory();
		let config = test_adapter_config();
		let bindings = empty_bindings();

		let first_core_ctx = rivetkit_core::ActorContext::new_with_kv(
			"actor-preamble-first",
			"actor",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let first_ctx = ActorContext::new(first_core_ctx.clone());
		first_ctx
			.set_state_initial(vec![9, 9, 9])
			.expect("initial state should set");

		run_preamble(&bindings, &config, &first_ctx, None, None, Vec::new())
			.await
			.expect("first-create preamble should succeed");

		let persisted_bytes = kv
			.get(PERSIST_DATA_KEY)
			.await
			.expect("persisted actor read should succeed")
			.expect("persisted actor bytes should exist");
		let embedded_version = u16::from_le_bytes([persisted_bytes[0], persisted_bytes[1]]);
		assert!(matches!(embedded_version, 3 | 4));
		let persisted: PersistedActor =
			serde_bare::from_slice(&persisted_bytes[2..]).expect("persisted actor should decode");
		assert!(persisted.has_initialized);

		let second_core_ctx = rivetkit_core::ActorContext::new_with_kv(
			"actor-preamble-second",
			"actor",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let second_ctx = ActorContext::new(second_core_ctx);
		let snapshot = persisted.has_initialized.then_some(persisted.state.clone());
		assert!(snapshot.is_some());

		run_preamble(&bindings, &config, &second_ctx, None, snapshot, Vec::new())
			.await
			.expect("wake preamble should succeed");

		assert_eq!(second_ctx.inner().state(), vec![9, 9, 9]);
	}

	#[tokio::test]
	async fn maybe_serialize_skips_save_when_adapter_is_clean() {
		let bindings = empty_bindings();
		let core_ctx =
			rivetkit_core::ActorContext::new("actor-serialize-clean", "actor", Vec::new(), "local");
		let ctx = ActorContext::new(core_ctx);
		let dirty = AtomicBool::new(false);

		let deltas = maybe_serialize(&bindings, &ctx, &dirty, SerializeStateReason::Save)
			.await
			.expect("clean save serialize should not fail");

		assert!(deltas.is_empty());
		assert!(!dirty.load(Ordering::Acquire));
	}

	#[tokio::test]
	async fn maybe_serialize_inspector_does_not_consume_pending_save() {
		let bindings = empty_bindings();
		let core_ctx = rivetkit_core::ActorContext::new(
			"actor-serialize-inspector",
			"actor",
			Vec::new(),
			"local",
		);
		let ctx = ActorContext::new(core_ctx);
		let dirty = AtomicBool::new(true);
		let calls = Arc::new(Mutex::new(Vec::new()));

		let inspector_deltas = maybe_serialize_with(
			&bindings,
			&ctx,
			&dirty,
			SerializeStateReason::Inspector,
			|_, _, reason| {
				let calls = Arc::clone(&calls);
				async move {
					calls.lock().push(reason);
					Ok(vec![StateDelta::ActorState(vec![1, 2, 3])])
				}
			},
		)
		.await
		.expect("inspector serialize should succeed");

		assert_eq!(inspector_deltas.len(), 1);
		assert!(dirty.load(Ordering::Acquire));

		let save_deltas = maybe_serialize_with(
			&bindings,
			&ctx,
			&dirty,
			SerializeStateReason::Save,
			|_, _, reason| {
				let calls = Arc::clone(&calls);
				async move {
					calls.lock().push(reason);
					Ok(vec![StateDelta::ActorState(vec![4, 5, 6])])
				}
			},
		)
		.await
		.expect("save serialize should still run after inspector");

		assert_eq!(save_deltas.len(), 1);
		assert!(!dirty.load(Ordering::Acquire));
		assert_eq!(*calls.lock(), vec!["inspector", "save"]);
	}
}
