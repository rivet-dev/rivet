import { actor, setup } from "rivetkit";

const myActor = actor({
  state: {},
  actions: {}
});

const registry = setup({
  use: { myActor },
  serverless: {
    publicEndpoint: "https://my-namespace:pk_xxxxx@api.rivet.dev",
  },
});
