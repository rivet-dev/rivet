use serde::{Deserialize, Serialize};

use super::*;
use crate::action;
use rivetkit_core::{ActorContext, ActorWorkKind};

struct EmptyActor;

impl Actor for EmptyActor {
	type Input = ();
	type ConnParams = TestConnParams;
	type ConnState = TestConnState;
	type Action = action::Raw;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TestConnParams {
	label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TestConnState {
	value: i64,
}

#[test]
fn typed_ctx_broadcast_accepts_cbor_payloads() {
	let ctx = Ctx::<EmptyActor>::new(ActorContext::new("actor-id", "test", Vec::new(), "local"));

	assert!(ctx.broadcast("x", &42u32).is_ok());
}

#[test]
fn conn_ctx_round_trips_typed_params_and_state() {
	let conn = ConnHandle::new(
		"conn-id",
		cbor(&TestConnParams {
			label: "hello".into(),
		}),
		cbor(&TestConnState { value: 5 }),
		true,
	);
	let conn_ctx = ConnCtx::<EmptyActor>::new(conn);

	assert_eq!(conn_ctx.id(), "conn-id");
	assert_eq!(
		conn_ctx.params().expect("decode params"),
		TestConnParams {
			label: "hello".into(),
		}
	);
	assert_eq!(
		conn_ctx.state().expect("decode state"),
		TestConnState { value: 5 }
	);
	assert!(conn_ctx.is_hibernatable());

	conn_ctx
		.set_state(&TestConnState { value: 8 })
		.expect("encode state");
	assert_eq!(
		conn_ctx.state().expect("decode updated state"),
		TestConnState { value: 8 }
	);
}

#[test]
fn sleep_lifecycle_methods_accessible_via_inner() {
	let inner_ctx = ActorContext::new("actor-id", "test", Vec::new(), "local");
	let ctx = Ctx::<EmptyActor>::new(inner_ctx);

	// All these methods should be accessible on ctx.inner()
	let inner = ctx.inner();

	// Methods we're testing:
	// 1. keep_awake - accessible as public async fn on ActorContext
	let _ = inner.keep_awake(async {}); // type check that method exists

	// 2. keep_awake_region - accessible as public fn on ActorContext, returns KeepAwakeRegion
	let _ = inner.keep_awake_region(); // type check that method exists

	// 3. internal_keep_awake - accessible as public async fn on ActorContext
	let _ = inner.internal_keep_awake(async {}); // type check that method exists

	// 4. register_task - accessible as public fn on ActorContext
	let _ = inner.register_task(async {}); // type check that method exists

	// 5. track_work - accessible as public async fn on ActorContext
	let _ = inner.track_work(ActorWorkKind::KeepAwake, async {}); // type check that method exists

	// 6. spawn_work - accessible as public fn on ActorContext
	let _ = inner.spawn_work(ActorWorkKind::KeepAwake, async {}); // type check that method exists

	// 7. keep_awake_count - accessible as public fn on ActorContext
	let _ = inner.keep_awake_count(); // type check that method exists

	// 8. internal_keep_awake_count - accessible as public fn on ActorContext
	let _ = inner.internal_keep_awake_count(); // type check that method exists

	// 9. is_destroy_requested - accessible as public fn on ActorContext
	let _ = inner.is_destroy_requested(); // type check that method exists

	// 10. wait_for_destroy_completion_public - accessible as public async fn on ActorContext
	let _ = inner.wait_for_destroy_completion_public(); // type check that method exists

	// 11. actor_abort_signal - accessible as public fn on ActorContext (doc hidden)
	let _ = inner.actor_abort_signal(); // type check that method exists

	// 12. shutdown_deadline_token - accessible as public fn on ActorContext (doc hidden)
	let _ = inner.shutdown_deadline_token(); // type check that method exists

	// 13. cancel_actor_abort_signal - accessible as public fn on ActorContext (doc hidden)
	let _ = inner.cancel_actor_abort_signal(); // type check that method exists
}

#[test]
fn sleep_types_accessible_from_rivetkit_core() {
	// ActorWorkKind type is accessible (exported from rivetkit-core)
	let _ = ActorWorkKind::KeepAwake;
	let _ = ActorWorkKind::InternalKeepAwake;
	let _ = ActorWorkKind::RegisteredTask;
	let _ = ActorWorkKind::WaitUntil;

	// These can be used as arguments to methods like spawn_work, track_work
	let inner_ctx = ActorContext::new("test", "test", vec![], "local");
	let _fut_kind = ActorWorkKind::KeepAwake;
	let _ = _fut_kind;
	let _ = inner_ctx;
}

fn cbor<T: Serialize>(value: &T) -> Vec<u8> {
	let mut encoded = Vec::new();
	ciborium::into_writer(value, &mut encoded).expect("encode test value as cbor");
	encoded
}
