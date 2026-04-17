use super::*;

	mod moved_tests {
		use std::sync::Arc;

		use anyhow::Result;
		use async_trait::async_trait;
		use serde::{Deserialize, Serialize};

		use super::{ConnCtx, Ctx};
		use crate::actor::Actor;
		use rivetkit_core::{ActorConfig, ActorContext};

		#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
		struct TestState {
			value: i64,
		}

		#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
		struct TestConnState {
			value: i64,
		}

		#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
		struct TestConnParams {
			label: String,
		}

		#[derive(Debug, PartialEq, Eq)]
		struct TestVars {
			label: &'static str,
		}

		struct TestActor;

		#[async_trait]
		impl Actor for TestActor {
			type State = TestState;
			type ConnParams = TestConnParams;
			type ConnState = TestConnState;
			type Input = ();
			type Vars = TestVars;

			async fn create_state(
				_ctx: &Ctx<Self>,
				_input: &Self::Input,
			) -> Result<Self::State> {
				Ok(TestState { value: 0 })
			}

			async fn create_vars(_ctx: &Ctx<Self>) -> Result<Self::Vars> {
				Ok(TestVars { label: "vars" })
			}

			async fn create_conn_state(
				self: &Arc<Self>,
				_ctx: &Ctx<Self>,
				_params: &Self::ConnParams,
			) -> Result<Self::ConnState> {
				let _ = self;
				Ok(TestConnState { value: 0 })
			}

			async fn on_create(_ctx: &Ctx<Self>, _input: &Self::Input) -> Result<Self> {
				Ok(Self)
			}

			fn config() -> ActorConfig {
				ActorConfig::default()
			}
		}

		#[test]
		fn state_is_cached_until_set_state_invalidates_it() {
			let inner = ActorContext::new("actor-id", "test", Vec::new(), "local");
			inner.set_state(
				super::serialize_cbor(&TestState { value: 7 })
					.expect("serialize test state"),
			);

			let ctx = Ctx::<TestActor>::new(
				inner.clone(),
				Arc::new(TestVars { label: "vars" }),
			);
			let first = ctx.state();
			let second = ctx.state();

			assert!(Arc::ptr_eq(&first, &second));

			inner.set_state(
				super::serialize_cbor(&TestState { value: 99 })
					.expect("serialize replacement state"),
			);
			let still_cached = ctx.state();
			assert_eq!(still_cached.value, 7);

			ctx.set_state(&TestState { value: 11 });
			let refreshed = ctx.state();
			assert_eq!(refreshed.value, 11);
			assert!(!Arc::ptr_eq(&first, &refreshed));
		}

		#[test]
		fn vars_are_exposed_by_reference() {
			let ctx = Ctx::<TestActor>::new(
				ActorContext::new("actor-id", "test", Vec::new(), "local"),
				Arc::new(TestVars { label: "vars" }),
			);

			assert_eq!(ctx.vars().label, "vars");
		}

		#[test]
		fn connection_context_serializes_and_deserializes_cbor() {
			let conn = rivetkit_core::ConnHandle::new(
				"conn-id",
				super::serialize_cbor(&TestConnParams {
					label: "hello".into(),
				})
				.expect("serialize params"),
				super::serialize_cbor(&TestConnState { value: 5 })
					.expect("serialize state"),
				true,
			);
			let conn_ctx = ConnCtx::<TestActor>::new(conn);

			assert_eq!(conn_ctx.id(), "conn-id");
			assert_eq!(
				conn_ctx.params(),
				TestConnParams {
					label: "hello".into(),
				}
			);
			assert_eq!(conn_ctx.state(), TestConnState { value: 5 });
			assert!(conn_ctx.is_hibernatable());

			conn_ctx.set_state(&TestConnState { value: 8 });
			assert_eq!(conn_ctx.state(), TestConnState { value: 8 });
		}
	}
