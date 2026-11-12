import { actor } from "rivetkit";

const counter = actor({
  vars: {
    count: 0,
    intervalId: null as NodeJS.Timeout | null,
  },

  onWake: (c) => {
    // Set up interval when actor wakes
    c.vars.intervalId = setInterval(() => {
      c.vars.count++;
      console.log('Auto-increment:', c.vars.count);
    }, 10000);
  },

  onSleep: (c) => {
    console.log('Actor going to sleep, cleaning up...');

    // Clean up interval before sleeping
    if (c.vars.intervalId) {
      clearInterval(c.vars.intervalId);
      c.vars.intervalId = null;
    }

    // Perform any other cleanup
    console.log('Final count:', c.vars.count);
  },

  actions: { /* ... */ }
});
