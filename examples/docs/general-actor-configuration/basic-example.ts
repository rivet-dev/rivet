import { actor, setup } from "rivetkit";

const myActor = actor({
  vars: { count: 0 },

  actions: {
    increment: (c) => {
      c.vars.count++;
      return c.vars.count;
    },
  },
  options: {
    actionTimeout: 15_000,
  }
});

const registry = setup({
  use: { myActor },
});
