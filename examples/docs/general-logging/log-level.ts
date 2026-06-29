import { actor, setup } from "rivetkit";

const counter = actor({
  state: { count: 0 },
  actions: { increment: (c) => c.state.count++ }
});

const registry = setup({
  use: { counter },
  logging: {
    level: "debug"
  }
});
