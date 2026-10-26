import { actor, setup } from "rivetkit";

const myActor = actor({
  state: {},
  actions: {}
});

const registry = setup({
  use: { myActor },
  endpoint: "https://my-namespace:<rivet-token>@rivet.example.com",
});
