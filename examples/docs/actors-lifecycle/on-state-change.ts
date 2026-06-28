import { actor } from "rivetkit";

const counter = actor({
  state: { count: 0 },

  onStateChange: (c, newState) => {
    // Broadcast the new count to all connected clients
    c.broadcast('countUpdated', {
      count: newState.count
    });
  },

  actions: {
    increment: (c) => {
      c.state.count++;
      return c.state.count;
    }
  }
});
