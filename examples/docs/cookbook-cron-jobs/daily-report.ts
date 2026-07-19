import { actor } from "rivetkit";

export const dailyReport = actor({
  state: { lastRunAt: 0 },
  onCreate: async (c) => {
    await c.cron.set({
      name: "daily-report",
      expression: "0 9 * * *",
      action: "runReport",
    });
  },
  actions: {
    runReport: (c) => {
      // Do the job's work, then record the run.
      c.state.lastRunAt = Date.now();
    },
  },
});
