import { actor } from "rivetkit";

const loggingActor = actor({
  state: { requestCount: 0 },

  onBeforeActionResponse: (c, actionName, args, output) => {
    // Log action calls
    console.log(`Action ${actionName} called with args:`, args);
    console.log(`Action ${actionName} returned:`, output);

    c.state.requestCount++;
    c.broadcast("actionResponseLogged", {
      actionName,
      timestamp: Date.now(),
      requestCount: c.state.requestCount,
    });

    return output;
  },

  actions: {
    getUserData: (c, userId: string) => {
      c.state.requestCount++;

      // This response is returned after onBeforeActionResponse runs
      return {
        userId,
        profile: { name: "John Doe", email: "john@example.com" },
        lastActive: Date.now()
      };
    },

    getStats: (c) => {
      // This also passes through onBeforeActionResponse
      return {
        requestCount: c.state.requestCount,
        uptime: process.uptime()
      };
    }
  }
});
