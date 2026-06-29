import { createClient, ActorError } from "rivetkit/client";
import { actor, setup } from "rivetkit";

const myActor = actor({
  state: { count: 0 },
  actions: { increment: (c) => c.state.count++ }
});
const registry = setup({ use: { myActor } });

const client = createClient<typeof registry>("http://localhost:6420");

try {
  const actorHandle = client.myActor.getOrCreate(["my-actor"]);
  const conn = actorHandle.connect();

  // Connection will be established or error will be thrown
  await conn.increment();
} catch (error) {
  if (error instanceof ActorError && error.code === "origin_not_allowed") {
    console.error("Connection rejected: Origin not allowed");
  }
}
