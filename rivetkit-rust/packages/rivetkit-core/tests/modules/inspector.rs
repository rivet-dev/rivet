use super::*;

mod moved_tests {
	use super::{Inspector, InspectorSignal, InspectorSnapshot};
	use crate::actor::connection::{
		ConnHandle, PersistedConnection, PersistedSubscription,
		encode_persisted_connection, make_connection_key,
	};
	use crate::actor::context::tests::new_with_kv;
	use crate::actor::callbacks::StateDelta;
	use crate::inspector::InspectorAuth;
	use crate::QueueNextOpts;
	use rivet_error::RivetError;
	use std::collections::BTreeMap;
	use std::sync::Arc;
	use std::sync::Mutex;
	use std::sync::atomic::{AtomicUsize, Ordering};

	static INSPECTOR_ENV_LOCK: Mutex<()> = Mutex::new(());

	#[tokio::test]
	async fn state_updates_increment_inspector_revisions() {
		let ctx = new_with_kv(
			"actor-1",
			"inspector-state",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);
		let inspector = Inspector::new();

		ctx.configure_inspector(Some(inspector.clone()));
		ctx.set_state(vec![1, 2, 3])
			.expect("test state should update");
		ctx.save_state(vec![StateDelta::ActorState(vec![1, 2, 3])])
			.await
			.expect("state save should succeed");

		assert_eq!(
			inspector.snapshot(),
			InspectorSnapshot {
				state_revision: 2,
				..InspectorSnapshot::default()
			},
		);
	}

	#[tokio::test]
	async fn connection_lifecycle_updates_inspector_snapshot() {
		let kv = crate::kv::tests::new_in_memory();
		let ctx = new_with_kv(
			"actor-1",
			"inspector-connections",
			Vec::new(),
			"local",
			kv.clone(),
		);
		let inspector = Inspector::new();

		ctx.configure_inspector(Some(inspector.clone()));

		let conn = ConnHandle::new("conn-1", vec![1], vec![2], false);
		ctx.add_conn(conn.clone());
		assert_eq!(
			inspector.snapshot(),
			InspectorSnapshot {
				connections_revision: 1,
				active_connections: 1,
				..InspectorSnapshot::default()
			},
		);

		ctx.remove_conn(conn.id());
		assert_eq!(
			inspector.snapshot(),
			InspectorSnapshot {
				connections_revision: 2,
				active_connections: 0,
				..InspectorSnapshot::default()
			},
		);

		let restored = PersistedConnection {
			id: "restored-1".into(),
			parameters: vec![9],
			state: vec![8],
			subscriptions: vec![PersistedSubscription {
				event_name: "counter.updated".into(),
			}],
			gateway_id: vec![1],
			request_id: vec![2],
			server_message_index: 3,
			client_message_index: 4,
			request_path: "/socket".into(),
			request_headers: BTreeMap::new(),
		};
		kv.put(
			&make_connection_key(&restored.id),
			&encode_persisted_connection(&restored).expect("encode restored connection"),
		)
		.await
		.expect("persist restored connection");

		let restored_connections = ctx
			.restore_hibernatable_connections()
			.await
			.expect("restore should succeed");
		assert_eq!(restored_connections.len(), 1);
		assert_eq!(
			inspector.snapshot(),
			InspectorSnapshot {
				connections_revision: 3,
				active_connections: 1,
				..InspectorSnapshot::default()
			},
		);

		ctx.remove_conn("restored-1");
		assert_eq!(
			inspector.snapshot(),
			InspectorSnapshot {
				connections_revision: 4,
				active_connections: 0,
				..InspectorSnapshot::default()
			},
		);
	}

	#[tokio::test]
	async fn queue_lifecycle_updates_inspector_snapshot() {
		let ctx = new_with_kv(
			"actor-1",
			"inspector-queue",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);
		let inspector = Inspector::new();

		ctx.configure_inspector(Some(inspector.clone()));

		ctx.queue()
			.send("jobs", b"first")
			.await
			.expect("enqueue should succeed");
		assert_eq!(
			inspector.snapshot(),
			InspectorSnapshot {
				queue_revision: 1,
				queue_size: 1,
				..InspectorSnapshot::default()
			},
		);

		let received = ctx
			.queue()
			.next(QueueNextOpts::default())
			.await
			.expect("queue next should succeed")
			.expect("message should exist");
		assert_eq!(received.body, b"first".to_vec());
		assert_eq!(
			inspector.snapshot(),
			InspectorSnapshot {
				queue_revision: 2,
				queue_size: 0,
				..InspectorSnapshot::default()
			},
		);

		ctx.queue()
			.send("jobs", b"second")
			.await
			.expect("second enqueue should succeed");
		assert_eq!(
			inspector.snapshot(),
			InspectorSnapshot {
				queue_revision: 3,
				queue_size: 1,
				..InspectorSnapshot::default()
			},
		);

		let completable = ctx
			.queue()
			.next(QueueNextOpts {
				names: None,
				timeout: None,
				signal: None,
				completable: true,
			})
			.await
			.expect("completable receive should succeed")
			.expect("completable message should exist");
		assert_eq!(
			inspector.snapshot(),
			InspectorSnapshot {
				queue_revision: 4,
				queue_size: 1,
				..InspectorSnapshot::default()
			},
		);

		completable
			.complete(Some(vec![7]))
			.await
			.expect("queue ack should succeed");
		assert_eq!(
			inspector.snapshot(),
			InspectorSnapshot {
				queue_revision: 5,
				queue_size: 0,
				..InspectorSnapshot::default()
			},
		);
	}

	#[test]
	fn inspector_subscriptions_track_connected_clients_and_cleanup() {
		let inspector = Inspector::new();
		let state_updates = Arc::new(AtomicUsize::new(0));
		let queue_updates = Arc::new(AtomicUsize::new(0));
		let state_updates_clone = state_updates.clone();
		let queue_updates_clone = queue_updates.clone();

		let subscription = inspector.subscribe(Arc::new(move |signal| match signal {
			InspectorSignal::StateUpdated => {
				state_updates_clone.fetch_add(1, Ordering::SeqCst);
			}
			InspectorSignal::QueueUpdated => {
				queue_updates_clone.fetch_add(1, Ordering::SeqCst);
			}
			InspectorSignal::ConnectionsUpdated | InspectorSignal::WorkflowHistoryUpdated => {}
		}));

		assert_eq!(inspector.snapshot().connected_clients, 1);

		inspector.record_state_updated();
		inspector.record_queue_updated(3);

		assert_eq!(state_updates.load(Ordering::SeqCst), 1);
		assert_eq!(queue_updates.load(Ordering::SeqCst), 1);

		drop(subscription);

		assert_eq!(inspector.snapshot().connected_clients, 0);

		inspector.record_state_updated();
		assert_eq!(state_updates.load(Ordering::SeqCst), 1);
	}

	#[tokio::test]
	async fn inspector_auth_uses_env_token_before_kv_fallback() {
		let _env_guard = INSPECTOR_ENV_LOCK.lock().expect("env lock poisoned");
		unsafe {
			std::env::set_var("RIVET_INSPECTOR_TOKEN", "env-token");
		}

		let kv = crate::kv::tests::new_in_memory();
		let ctx = new_with_kv(
			"actor-1",
			"inspector-auth-env",
			Vec::new(),
			"local",
			kv.clone(),
		);
		kv.put(&[3], b"kv-token")
			.await
			.expect("kv token should persist");

		InspectorAuth::new()
			.verify(&ctx, Some("env-token"))
			.await
			.expect("env token should authorize");

		let error = InspectorAuth::new()
			.verify(&ctx, Some("kv-token"))
			.await
			.expect_err("kv token should not bypass configured env token");
		let error = RivetError::extract(&error);
		assert_eq!(error.group(), "inspector");
		assert_eq!(error.code(), "unauthorized");

		unsafe {
			std::env::remove_var("RIVET_INSPECTOR_TOKEN");
		}
	}

	#[tokio::test]
	async fn inspector_auth_falls_back_to_actor_kv_token() {
		let _env_guard = INSPECTOR_ENV_LOCK.lock().expect("env lock poisoned");
		unsafe {
			std::env::remove_var("RIVET_INSPECTOR_TOKEN");
		}

		let kv = crate::kv::tests::new_in_memory();
		let ctx = new_with_kv(
			"actor-1",
			"inspector-auth-kv",
			Vec::new(),
			"local",
			kv.clone(),
		);
		kv.put(&[3], b"kv-token")
			.await
			.expect("kv token should persist");

		InspectorAuth::new()
			.verify(&ctx, Some("kv-token"))
			.await
			.expect("kv token should authorize");

		let error = InspectorAuth::new()
			.verify(&ctx, Some("nope"))
			.await
			.expect_err("wrong token should fail");
		let error = RivetError::extract(&error);
		assert_eq!(error.group(), "inspector");
		assert_eq!(error.code(), "unauthorized");
	}

	#[tokio::test]
	async fn inspector_auth_rejects_missing_token() {
		let _env_guard = INSPECTOR_ENV_LOCK.lock().expect("env lock poisoned");
		unsafe {
			std::env::remove_var("RIVET_INSPECTOR_TOKEN");
		}

		let ctx = new_with_kv(
			"actor-1",
			"inspector-auth-missing",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);

		let error = InspectorAuth::new()
			.verify(&ctx, None)
			.await
			.expect_err("missing token should fail");
		let error = RivetError::extract(&error);
		assert_eq!(error.group(), "inspector");
		assert_eq!(error.code(), "unauthorized");
	}
}
