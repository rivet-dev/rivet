use serde::{Deserialize, Serialize};

use super::*;
use crate::action;

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

fn cbor<T: Serialize>(value: &T) -> Vec<u8> {
	let mut encoded = Vec::new();
	ciborium::into_writer(value, &mut encoded).expect("encode test value as cbor");
	encoded
}
