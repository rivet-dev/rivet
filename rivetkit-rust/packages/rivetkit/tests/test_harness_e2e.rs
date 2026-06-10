use std::{future::Future, pin::Pin, sync::Arc};

use async_trait::async_trait;
use rivetkit::{Action, Actor, Ctx, Handles, Registry, action, test};
use serde::{Deserialize, Serialize};

type BoxFuture<T> = Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send>>;

struct HarnessActor;

#[derive(Debug, Serialize, Deserialize)]
struct Echo {
	value: String,
}

impl Action for Echo {
	type Output = String;

	const NAME: &'static str = "echo";
}

#[async_trait]
impl Actor for HarnessActor {
	type State = ();
	type Input = ();
	type Actions = (Echo,);
	type Events = ();
	type Queue = ();
	type ConnParams = ();
	type ConnState = ();
	type Action = action::Raw;

	async fn create_state(_ctx: &Ctx<Self>, _input: Self::Input) -> anyhow::Result<Self::State> {
		Ok(())
	}

	async fn create(_ctx: &Ctx<Self>) -> anyhow::Result<Self> {
		Ok(Self)
	}
}

impl Handles<Echo> for HarnessActor {
	type Future = BoxFuture<String>;

	fn handle(self: Arc<Self>, _ctx: Ctx<Self>, action: Echo) -> Self::Future {
		Box::pin(async move { Ok(action.value) })
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn typed_test_harness_sends_round_trip() -> anyhow::Result<()> {
	let mut registry = Registry::new();
	registry.register_actor::<HarnessActor>("harnessActor");

	let h = test::setup(registry).await?;

	let actor = h.actor::<HarnessActor>("harnessActor");
	assert_eq!(
		actor
			.send(Echo {
				value: "hello".to_owned(),
			})
			.await?,
		"hello"
	);

	h.shutdown().await;
	Ok(())
}
