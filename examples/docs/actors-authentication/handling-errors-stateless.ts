import { actor, setup } from "rivetkit";
import { ActorError, createClient } from "rivetkit/client";

// Define actor with protected action
const myActor = actor({
  state: {},
  actions: {
    protectedAction: (c) => ({ success: true })
  }
});

const registry = setup({ use: { myActor } });
const client = createClient<typeof registry>("http://localhost:6420");
const actorHandle = await client.myActor.getOrCreate();

// Helper to show errors
function showError(message: string) {
  console.error(message);
}

try {
  const result = await actorHandle.protectedAction();
} catch (error) {
  if (error instanceof ActorError && error.code === "forbidden") {
    window.location.href = "/login";
  } else if (error instanceof ActorError && error.code === "insufficient_permissions") {
    showError("You don't have permission for this action");
  }
}
