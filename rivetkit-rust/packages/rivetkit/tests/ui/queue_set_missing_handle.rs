use rivetkit::{Actor, QueueMessage, QueueSet, action};
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
struct MissingMessage;

impl QueueMessage for MissingMessage {
	type Reply = ();

	const NAME: &'static str = "missing";
}

fn main() {
	let _ = <(MissingMessage,) as QueueSet<TestActor>>::entries();
}
