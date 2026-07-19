import { expect, test } from "vitest";
import { actor, setup } from "rivetkit";
import { setupTest } from "rivetkit/test";

const scheduler = actor({
  state: { completedTasks: [] as string[] },
  actions: {
    scheduleTask: async (c, taskName: string) => {
      await c.schedule.after(50, "completeTask", taskName);
    },
    completeTask: (c, taskName: string) => {
      c.state.completedTasks.push(taskName);
    },
    getCompletedTasks: (c) => c.state.completedTasks,
  },
});

const registry = setup({ use: { scheduler } });

test("scheduled work updates observable state", async (testCtx) => {
  const { client } = await setupTest(testCtx, registry);
  const handle = client.scheduler.getOrCreate(["test"]);

  await handle.scheduleTask("reminder");

  await expect
    .poll(() => handle.getCompletedTasks(), { timeout: 2_000, interval: 25 })
    .toContain("reminder");
});
