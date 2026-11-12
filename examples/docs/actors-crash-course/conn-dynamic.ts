import { actor } from "rivetkit";

const chatRoom = actor({
  // Per-connection data, keyed by connection ID
  createVars: () => ({ users: new Map<string, { userId: string }>() }),
  // Params are passed from the client when it connects
  onConnect: (c, conn) => {
    const params = conn.params as { userId: string };
    c.vars.users.set(conn.id, { userId: params.userId });
  },
  onDisconnect: (c, conn) => {
    c.vars.users.delete(conn.id);
  },
  actions: {
    // Access the current connection's data and params
    whoAmI: (c) => ({
      user: c.vars.users.get(c.conn.id),
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
