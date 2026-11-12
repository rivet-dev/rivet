import { test, expect } from "vitest";
import { setupTest } from "rivetkit/test";
import { actor, setup } from "rivetkit";

// Define the actor
const myActor = actor({
  vars: { value: "initial" },
  actions: {
    someAction: (c) => {
      c.vars.value = "updated";
      return c.vars.value;
    },
    getValue: (c) => {
      return c.vars.value;
    }
  }
});

// Create the registry
const registry = setup({
  use: { myActor }
});

// Test the actor
test("my actor test", async (testCtx) => {
  const { client } = await setupTest(testCtx, registry);

  // Now you can interact with your actor through the client
  const myActorHandle = client.myActor.getOrCreate(["test"]);

  // Test your actor's functionality
  await myActorHandle.someAction();

  // Make assertions
  const result = await myActorHandle.getValue();
  expect(result).toEqual("updated");
});
