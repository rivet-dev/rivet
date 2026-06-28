import { actor, event, queue, UserError } from "rivetkit";

type ConnState = { role: "member" | "admin" };

const securedActor = actor({
  state: {},
  createConnState: (_c, params: { role?: ConnState["role"] }): ConnState => ({
    role: params.role ?? "member",
  }),

  events: {
    publicFeed: event<{ text: string }>(),
    adminFeed: event<{ text: string }>({
      canSubscribe: (c) => c.conn?.state.role === "admin",
    }),
  },

  queues: {
    jobs: queue<{ task: string }>({
      canPublish: (c) => c.conn?.state.role === "admin",
    }),
  },

  actions: {
    publicAction: () => "ok",
    privateAction: (c) => {
      if (c.conn?.state.role !== "admin") {
        throw new UserError("Forbidden", { code: "forbidden" });
      }
      return "secret";
    },
  },
});
