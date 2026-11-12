import { actor, setup } from "rivetkit";

const counter = actor({
  vars: { count: 0 },
  actions: { increment: (c) => c.vars.count++ }
});

const registry = setup({
  use: { counter },
  logging: {
    level: "debug"
  }
});
