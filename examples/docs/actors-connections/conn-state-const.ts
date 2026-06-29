import { actor } from "rivetkit";

const chatRoom = actor({
  state: { messages: [] },

  // Define default connection state as a constant
  connState: {
    role: "guest",
    joinedAt: 0
  },

  onConnect: (c) => {
    // Update join timestamp when a client connects
    c.conn.state.joinedAt = Date.now();
  },

  actions: {
    // ...
  }
});
