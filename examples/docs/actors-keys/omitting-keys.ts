import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const globalActor = actor({
  state: { config: {} },
  actions: {}
});

const registry = setup({ use: { globalActor } });
const client = createClient<typeof registry>("http://localhost:6420");

// Get the singleton session
const globalActorHandle = client.globalActor.getOrCreate();
