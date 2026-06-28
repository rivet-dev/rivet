import { actor } from "rivetkit";

const counter = actor({
  state: { count: 0 },

  // Constant ephemeral value, reset each time the actor starts
  vars: { lastAccessedAt: 0 },

  actions: {
    increment: (c) => {
      // Read and write the ephemeral var
      c.vars.lastAccessedAt = Date.now();
      return ++c.state.count;
    },

    getLastAccessed: (c) => c.vars.lastAccessedAt
  }
});
