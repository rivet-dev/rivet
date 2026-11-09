import { actor } from "rivetkit";

const myActor = actor({
  vars: {
    count: 0,
    intervalId: null as ReturnType<typeof setInterval> | null,
  },

  onWake: (c) => {
    c.vars.intervalId = setInterval(() => { c.vars.count++; }, 10_000);
  },

  onSleep: (c) => {
    if (c.vars.intervalId) clearInterval(c.vars.intervalId);
  },

  actions: { /* ... */ }
});
