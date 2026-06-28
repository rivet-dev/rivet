import { actor } from "rivetkit";

const DAY_MS = 24 * 60 * 60 * 1000;

export const dailyReport = actor({
  state: { lastRunAt: 0 },
  actions: {
    runReport: (c) => {
      // Do the job's work, then record the run.
      c.state.lastRunAt = Date.now();
      // Re-arm the next run before returning.
      c.schedule.after(DAY_MS, "runReport");
    },
  },
});
