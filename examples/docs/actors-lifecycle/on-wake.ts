import { actor } from "rivetkit";

const counter = actor({
  state: { count: 0 },
  vars: { intervalId: null as NodeJS.Timeout | null },

  onWake: (c) => {
    console.log('Actor started with count:', c.state.count);

    // Set up interval for automatic counting
    const intervalId = setInterval(() => {
      c.state.count++;
      c.broadcast("countChanged", c.state.count);
      console.log('Auto-increment:', c.state.count);
    }, 10000);

    // Store interval ID in vars to clean up later if needed
    c.vars.intervalId = intervalId;
  },

  actions: {
    stop: (c) => {
      if (c.vars.intervalId) {
        clearInterval(c.vars.intervalId);
        c.vars.intervalId = null;
      }
    }
  }
});
