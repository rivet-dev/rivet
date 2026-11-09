import { actor, ActorContextOf } from "rivetkit";

const myActor = actor({
  vars: { startedAt: 0 },
  actions: {},
});

// Simple external function with typed context
function logActorStarted(c: ActorContextOf<typeof myActor>) {
  console.log(`Actor started at: ${c.vars.startedAt}`);
}
