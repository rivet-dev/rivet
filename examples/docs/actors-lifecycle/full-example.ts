import { type ActorContextOf, actor } from "rivetkit";
import { db } from "rivetkit/db";

interface CounterInput {
  initialCount?: number;
  stepSize?: number;
  name?: string;
}

interface ConnParams {
  userId: string;
  role: string;
}

interface ConnInfo {
  userId: string;
  role: string;
  connectedAt: number;
}

const counter = actor({
  // Durable data lives in SQLite
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS counter (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          count INTEGER NOT NULL,
          step_size INTEGER NOT NULL,
          name TEXT NOT NULL,
          request_count INTEGER NOT NULL DEFAULT 0
        );
      `);
    },
  }),

  // Seed the database from input. This runs only once per actor.
  onCreate: async (c, input: CounterInput) => {
    await c.db.execute(
      "INSERT INTO counter (id, count, step_size, name) VALUES (1, ?, ?, ?)",
      input.initialCount ?? 0,
      input.stepSize ?? 1,
      input.name ?? "Unnamed Counter",
    );
    console.log(`Counter "${input.name}" initialized`);
  },

  // Ephemeral data, rebuilt empty on every start
  createVars: () => ({
    connections: new Map<string, ConnInfo>(),
    // `onWake` stores its task here so `onSleep` can wait for it.
    reporter: null as Promise<void> | null,
  }),

  // Lifecycle hooks
  onWake: async (c) => {
    const [{ name, count }] = await c.db.execute<{ name: string; count: number }>(
      "SELECT name, count FROM counter WHERE id = 1",
    );
    console.log(`Counter "${name}" started with count:`, count);

    // Start a background task without blocking startup
    c.vars.reporter = reportCount(c).catch((error) => {
      c.log.error({ msg: "count reporter failed", error });
    });
  },

  // `c.abortSignal` fires before `onSleep`, so the task is already exiting
  onSleep: async (c) => {
    await c.vars.reporter;
  },

  onBeforeConnect: (c, rawParams) => {
    // Validate connection params
    const params = rawParams as ConnParams;
    if (!params.userId) {
      throw new Error("userId is required");
    }
    console.log(`User ${params.userId} attempting to connect`);
  },

  // Track each connection in memory, keyed by connection ID
  onConnect: (c, conn) => {
    const { userId, role } = conn.params as ConnParams;
    c.vars.connections.set(conn.id, { userId, role, connectedAt: Date.now() });
    console.log(`User ${userId} connected`);
  },

  onDisconnect: (c, conn) => {
    const info = c.vars.connections.get(conn.id);
    c.vars.connections.delete(conn.id);
    console.log(`User ${info?.userId} disconnected`);
  },

  // Observe action responses before they are sent, counting them in SQLite so the total survives sleep
  onBeforeActionResponse: async (c, actionName, args, output) => {
    await c.db.execute("UPDATE counter SET request_count = request_count + 1 WHERE id = 1");
    console.log(`Action ${actionName} called`, args);
    return output;
  },

  // Define actions
  actions: {
    increment: async (c, amount?: number) => {
      // Add the amount, or the stored step size when none is passed, in one atomic statement
      const [{ count, name }] = await c.db.execute<{ count: number; name: string }>(
        "UPDATE counter SET count = count + COALESCE(?, step_size) WHERE id = 1 RETURNING count, name",
        amount ?? null,
      );

      // React to the change right where it happens
      c.broadcast("countUpdated", { count, name });
      return count;
    },

    getInfo: async (c) => {
      const [info] = await c.db.execute<{
        name: string;
        count: number;
        stepSize: number;
        totalRequests: number;
      }>(
        "SELECT name, count, step_size AS stepSize, request_count AS totalRequests FROM counter WHERE id = 1",
      );
      return info;
    },
  }
});

export default counter;

// Log the count every minute until the actor starts shutting down
async function reportCount(c: ActorContextOf<typeof counter>): Promise<void> {
  while (!c.aborted) {
    const [{ name, count }] = await c.db.execute<{ name: string; count: number }>(
      "SELECT name, count FROM counter WHERE id = 1",
    );
    console.log(`Counter "${name}" is at ${count}`);
    await delay(60_000, c.abortSignal);
  }
}

// Resolve after `ms`, or as soon as the actor starts shutting down
function delay(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal.aborted) {
      resolve();
      return;
    }

    const finish = () => {
      clearTimeout(timer);
      signal.removeEventListener("abort", finish);
      resolve();
    };
    const timer = setTimeout(finish, ms);
    signal.addEventListener("abort", finish, { once: true });
  });
}
