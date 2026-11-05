import { actor } from "rivetkit";

interface Presence {
  name: string;
  joinedAt: number;
}

const room = actor({
  // Live connections are tracked in memory, keyed by connection ID
  createVars: () => ({ presence: new Map<string, Presence>() }),

  onConnect: (c, conn) => {
    const { name } = conn.params as { name: string };
    c.vars.presence.set(conn.id, { name, joinedAt: Date.now() });
  },

  onDisconnect: (c, conn) => {
    c.vars.presence.delete(conn.id);
  },

  actions: {
    whoAmI: (c) => c.vars.presence.get(c.conn.id),
    online: (c) => [...c.vars.presence.values()],
  },
});
