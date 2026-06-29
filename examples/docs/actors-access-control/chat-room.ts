import { actor, event, queue, UserError } from "rivetkit";

type ConnParams = {
  authToken: string;
};

type ConnState = {
  userId: string;
  role: "member" | "admin";
};

async function authenticate(
  authToken: string,
): Promise<ConnState | null> {
  if (authToken === "admin-token") {
    return { userId: "admin-1", role: "admin" };
  }
  if (authToken === "member-token") {
    return { userId: "member-1", role: "member" };
  }
  return null;
}

export const chatRoom = actor({
  state: { messages: [] as Array<{ userId: string; text: string }> },

  onBeforeConnect: async (_c, params: ConnParams) => {
    if (!params.authToken) {
      throw new UserError("Forbidden", { code: "forbidden" });
    }

    const session = await authenticate(params.authToken);
    if (!session) {
      throw new UserError("Forbidden", { code: "forbidden" });
    }
  },

  createConnState: async (_c, params: ConnParams): Promise<ConnState> => {
    const session = await authenticate(params.authToken);
    if (!session) {
      throw new UserError("Forbidden", { code: "forbidden" });
    }
    return session;
  },

  events: {
    messages: event<{ userId: string; text: string }>(),
    moderationLog: event<{ entry: string }>({
      canSubscribe: (c) => {
        if (c.conn?.state.role === "admin") {
          return true;
        }
        return false;
      },
    }),
  },

  queues: {
    moderationJobs: queue<{ action: "ban"; userId: string }>({
      canPublish: (c) => {
        if (c.conn?.state.role === "admin") {
          return true;
        }
        return false;
      },
    }),
  },

  actions: {
    sendMessage: (c, text: string) => {
      const role = c.conn?.state.role;
      const userId = c.conn?.state.userId;

      if (!userId || (role !== "member" && role !== "admin")) {
        throw new UserError("Forbidden", { code: "forbidden" });
      }

      const message = { userId, text };
      c.state.messages.push(message);
      c.broadcast("messages", message);
    },
  },
});
