import { actor, ActorContextOf } from "rivetkit";

const myActor = actor({
  state: { count: 0 },
  actions: {},
});

// Simple external function with typed context
function logActorStarted(c: ActorContextOf<typeof myActor>) {
  console.log(`Actor started with count: ${c.state.count}`);
}
