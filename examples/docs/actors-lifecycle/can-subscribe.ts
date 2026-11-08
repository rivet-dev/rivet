import { actor, event, UserError } from "rivetkit";

type Role = "member" | "admin";

const securedActor = actor({
  // Each connection's role is tracked in memory, keyed by connection ID
  createVars: () => ({ roles: new Map<string, Role>() }),

  onConnect: (c, conn) => {
    const params = conn.params as { role?: Role } | undefined;
    c.vars.roles.set(conn.id, params?.role ?? "member");
  },

  onDisconnect: (c, conn) => {
    c.vars.roles.delete(conn.id);
  },

  events: {
    publicFeed: event<{ text: string }>(),
    adminFeed: event<{ text: string }>({
      canSubscribe: (c) => c.conn !== undefined && c.vars.roles.get(c.conn.id) === "admin",
    }),
  },

  actions: {
    publicAction: () => "ok",
    privateAction: (c) => {
      if (c.vars.roles.get(c.conn.id) !== "admin") {
        throw new UserError("Forbidden", { code: "forbidden" });
      }
      return "secret";
    },
  },
});
