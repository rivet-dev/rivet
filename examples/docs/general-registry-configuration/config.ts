import { setup, actor } from "rivetkit";

const myActor = actor({ state: {}, actions: {} });

const registry = setup({
  use: { myActor },
  endpoint: "https://api.rivet.dev",
  token: process.env.RIVET_TOKEN,
  namespace: "production",
});
