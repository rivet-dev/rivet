import { test, expect } from "vitest";
import { setupTest } from "rivetkit/test";
import { actor, setup } from "rivetkit";

// Define the counter actor
const counter = actor({
  state: { count: 0 },
  actions: {
    increment: (c) => {
      c.state.count += 1;
      c.broadcast("newCount", c.state.count);
      return c.state.count;
    },
    getCount: (c) => {
      return c.state.count;
    }
  }
});

// Create the registry
const registry = setup({
  use: { counter }
});

// Test state persistence
test("actor should persist state", async (testCtx) => {
  const { client } = await setupTest(testCtx, registry);
  const counterHandle = client.counter.getOrCreate(["test"]);

  // Initial state
  expect(await counterHandle.getCount()).toBe(0);

  // Modify state
  await counterHandle.increment();

  // Verify state was updated
  expect(await counterHandle.getCount()).toBe(1);
});
