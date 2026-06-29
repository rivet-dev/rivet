import { actor, setup } from "rivetkit";

const myActor = actor({
  state: { count: 0 },

  actions: {
    increment: (c) => {
      c.state.count++;
      return c.state.count;
    },
  },
  options: {
    actionTimeout: 15_000,
  }
});

const registry = setup({
  use: { myActor },
});
