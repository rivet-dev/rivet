import { actor, setup } from "rivetkit";

// Create simple counter
const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c, count: number) => {
      c.state.count += count;
      return c.state.count;
    }
  }
});

// Create and export the registry
export const registry = setup({
  use: { counter }
});
