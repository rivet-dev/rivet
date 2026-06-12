use rivetkit::{Action, Actor, TypedActorHandle, action};
use serde::{Deserialize, Serialize};

struct TestActor;

impl Actor for TestActor {
	type State = ();
	type Input = ();
	type Actions = ();
	type Events = ();
	type Queue = ();
	type ConnParams = ();
	type ConnState = ();
	type Action = action::Raw;
}

#[derive(Serialize, Deserialize)]
struct MissingAction;

impl Action for MissingAction {
	type Output = ();

	const NAME: &'static str = "missing";
}

async fn check(handle: TypedActorHandle<TestActor>) {
	let _ = handle.send(MissingAction).await;
}

fn main() {}
