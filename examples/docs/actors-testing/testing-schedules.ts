import { test, expect } from "vitest";
import { setupTest } from "rivetkit/test";
import { actor, setup } from "rivetkit";

// Helper to wait for a delay
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

// Define the scheduler actor
const scheduler = actor({
  state: {
    tasks: [] as string[],
    completedTasks: [] as string[]
  },
  actions: {
    scheduleTask: (c, taskName: string, delayMs: number) => {
      c.state.tasks.push(taskName);
      // Schedule "completeTask" to run after the specified delay
      c.schedule.after(delayMs, "completeTask", taskName);
      return { success: true };
    },
    completeTask: (c, taskName: string) => {
      // This action will be called by the scheduler when the time comes
      c.state.completedTasks.push(taskName);
      return { completed: taskName };
    },
    getCompletedTasks: (c) => {
      return c.state.completedTasks;
    }
  }
});

// Create the registry
const registry = setup({
  use: { scheduler }
});

// Test scheduled tasks
test("scheduled tasks should execute", async (testCtx) => {
  const { client } = await setupTest(testCtx, registry);
  const schedulerHandle = client.scheduler.getOrCreate(["test"]);

  // Set up a scheduled task
  await schedulerHandle.scheduleTask("reminder", 100); // 100ms in the future

  // Wait for the scheduled task to run
  await wait(150);

  // Verify the scheduled task executed
  expect(await schedulerHandle.getCompletedTasks()).toContain("reminder");
});
