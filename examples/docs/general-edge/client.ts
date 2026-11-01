import { createClient } from "rivetkit/client";
import { actor, setup } from "rivetkit";

const example = actor({ state: {}, actions: {} });
const registry = setup({ use: { example } });

const client = createClient<typeof registry>("http://localhost:6420");

// Create actor in a specific region using getOrCreate
const actorHandle = client.example.getOrCreate(["my-actor"], {
  createInRegion: "atl"
});
