import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const userSession = actor({
  actions: { getUserId: (c) => c.key[0] }
});

const registry = setup({ use: { userSession } });
const client = createClient<typeof registry>("http://localhost:6420");

// Pass user ID in the key for user-specific actors
const userId = "user-123";
const userSessionHandle = client.userSession.getOrCreate([userId]);
