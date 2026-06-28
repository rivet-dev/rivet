import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const userActor = actor({
  state: { email: "", lastActive: 0 },
  actions: {
    updateEmail: (c, email: string) => {
      c.state.email = email;
      c.state.lastActive = Date.now();
    },
    getUser: (c) => ({
      email: c.state.email,
      lastActive: c.state.lastActive,
    }),
  },
});

const registry = setup({ use: { userActor } });
const client = createClient<typeof registry>("http://localhost:6420");

const user = await client.userActor.create(["user-123"], {
  input: { email: "alice@example.com" },
});

// Updates state and triggers onStateChange
await user.updateEmail("alice2@example.com");

const userData = await user.getUser();
