import { actor } from "rivetkit";

const chatRoom = actor({
  state: {},
  // params passed from client
  createConnState: (c, params: { userId: string }) => ({
    userId: params.userId,
  }),
  actions: {
    // Access current connection's state and params
    whoAmI: (c) => ({
      state: c.conn.state,
      params: c.conn.params,
    }),
    // Iterate all connections with c.conns
    notifyOthers: (c, text: string) => {
      for (const conn of c.conns.values()) {
        if (conn !== c.conn) conn.send("notification", { text });
      }
    },
  },
});
