import { actor } from "rivetkit";

const counter = actor({
  // Constant ephemeral value, reset each time the actor starts
  vars: { count: 0, lastAccessedAt: 0 },

  actions: {
    increment: (c) => {
      // Read and write the ephemeral vars
      c.vars.lastAccessedAt = Date.now();
      return ++c.vars.count;
    },

    getLastAccessed: (c) => c.vars.lastAccessedAt
  }
});
