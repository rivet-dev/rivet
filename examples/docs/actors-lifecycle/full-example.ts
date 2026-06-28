import { actor } from "rivetkit";

interface CounterInput {
  initialCount?: number;
  stepSize?: number;
  name?: string;
}

interface CounterState {
  count: number;
  stepSize: number;
  name: string;
  requestCount: number;
}

interface ConnParams {
  userId: string;
  role: string;
}

interface ConnState {
  userId: string;
  role: string;
  connectedAt: number;
}

const counter = actor({
  // Initialize state with input
  createState: (_c, input: CounterInput): CounterState => ({
    count: input.initialCount ?? 0,
    stepSize: input.stepSize ?? 1,
    name: input.name ?? "Unnamed Counter",
    requestCount: 0,
  }),

  // Initialize actor (run setup that doesn't affect initial state)
  onCreate: (c, input: CounterInput) => {
    console.log(`Counter "${input.name}" initialized`);
    // Set up external resources, logging, etc.
  },

  // Dynamically create connection state from params
  createConnState: (c, params: ConnParams): ConnState => {
    return {
      userId: params.userId,
      role: params.role,
      connectedAt: Date.now()
    };
  },

  // Lifecycle hooks
  onWake: (c) => {
    console.log(`Counter "${c.state.name}" started with count:`, c.state.count);
  },

  // Background task (does not block startup)
  run: async (c) => {
    while (!c.aborted) {
      // Example: periodic logging
      console.log(`Counter "${c.state.name}" is at ${c.state.count}`);
      await new Promise<void>((resolve) => {
        const timeout = setTimeout(resolve, 60000);
        c.abortSignal.addEventListener("abort", () => {
          clearTimeout(timeout);
          resolve();
        }, { once: true });
      });
    }
  },

  onStateChange: (c, newState) => {
    c.broadcast('countUpdated', {
      count: newState.count,
      name: newState.name
    });
  },

  onBeforeConnect: (c, params: ConnParams) => {
    // Validate connection params
    if (!params.userId) {
      throw new Error("userId is required");
    }
    console.log(`User ${params.userId} attempting to connect`);
  },

  onConnect: (c, conn) => {
    console.log(`User ${conn.state.userId} connected to "${c.state.name}"`);
  },

  onDisconnect: (c, conn) => {
    console.log(`User ${conn.state.userId} disconnected from "${c.state.name}"`);
  },

  // Observe action responses before they are sent
  onBeforeActionResponse: (c, actionName, args, output) => {
    c.state.requestCount++;
    console.log(`Action ${actionName} called`, args);
    return output;
  },

  // Define actions
  actions: {
    increment: (c, amount?: number) => {
      const step = amount ?? c.state.stepSize;
      c.state.count += step;
      return c.state.count;
    },

    getInfo: (c) => ({
      name: c.state.name,
      count: c.state.count,
      stepSize: c.state.stepSize,
      totalRequests: c.state.requestCount,
    }),
  }
});

export default counter;
