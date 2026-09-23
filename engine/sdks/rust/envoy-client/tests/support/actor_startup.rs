#[tokio::test]
async fn cancellation_buffered_during_startup_prevents_fetch() {
	for fail in [false, true] {
		let (started_tx, mut started_rx) = oneshot::channel();
		let gate = Arc::new(Notify::new());
		let mut callbacks = TestCallbacks::completing(started_tx, Arc::new(Notify::new()));
		callbacks.startup_gate = Some(gate.clone());
		callbacks.fail_startup = fail;
		let (shared, mut events) = build_shared_context(Arc::new(callbacks));
		let (actor, _) = create_actor(
			shared,
			"startup-cancel".into(),
			1,
			actor_config(),
			Vec::new(),
			None,
		);
		let message_id = protocol::MessageId {
			gateway_id: [1; 4],
			request_id: [2; 4],
			message_index: 0,
		};
		actor
			.send(ToActor::ReqStart {
				message_id: message_id.clone(),
				connection_session: 0,
				req: protocol::ToEnvoyRequestStart {
					actor_id: "startup-cancel".into(),
					actor_generation: Some(1),
					method: "GET".into(),
					path: "/".into(),
					headers: Default::default(),
					body: None,
					stream: false,
					response_stream: false,
				},
			})
			.unwrap();
		actor
			.send(ToActor::ReqAbort {
				message_id,
				reason: protocol::HttpStreamAbortReason {
					kind: protocol::HttpStreamAbortReasonKind::Cancelled,
					detail: None,
				},
			})
			.unwrap();
		gate.notify_one();
		if fail {
			wait_for_stopped_event(&mut events).await;
		}
		assert!(
			!matches!(
				tokio::time::timeout(Duration::from_millis(50), &mut started_rx).await,
				Ok(Ok(()))
			),
			"cancelled startup request invoked fetch"
		);
		let _ = actor.send(ToActor::Lost);
	}
}
