import { actor } from "rivetkit";

const counter = actor({
  // Constant initial state
  state: { count: 0 },

  actions: {
    get: (c) => c.state.count,

    // Update state, changes are persisted automatically
    increment: (c) => {
      c.state.count += 1;
      return c.state.count;
    }
  }
});
