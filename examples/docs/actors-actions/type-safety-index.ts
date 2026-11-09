import { actor, setup } from "rivetkit";

// Create simple counter
const counter = actor({
  vars: { count: 0 },
  actions: {
    increment: (c, count: number) => {
      c.vars.count += count;
      return c.vars.count;
    }
  }
});

// Create and export the registry
export const registry = setup({
  use: { counter }
});
