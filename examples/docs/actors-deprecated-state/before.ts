import { actor } from "rivetkit";

// An actor that still uses the deprecated `state` and `connState` options
const counter = actor({
  state: { count: 0 },
  connState: { joinedAt: 0 },

  onConnect: (c, conn) => {
    conn.state.joinedAt = Date.now();
  },

  actions: {
    increment: (c) => {
      c.state.count += 1;
      return c.state.count;
    },
  },
});
