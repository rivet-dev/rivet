import { actor } from "rivetkit";

const myActor = actor({
  state: { count: 0 },
  vars: { intervalId: null as ReturnType<typeof setInterval> | null },

  onWake: (c) => {
    c.vars.intervalId = setInterval(() => { c.state.count++; }, 10_000);
  },

  onSleep: (c) => {
    if (c.vars.intervalId) clearInterval(c.vars.intervalId);
  },

  actions: { /* ... */ }
});
