import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const userActor = actor({
  vars: { email: "", lastActive: 0 },
  actions: {
    updateEmail: (c, email: string) => {
      c.vars.email = email;
      c.vars.lastActive = Date.now();
    },
    getUser: (c) => ({
      email: c.vars.email,
      lastActive: c.vars.lastActive,
    }),
  },
});

const registry = setup({ use: { userActor } });
const client = createClient<typeof registry>("http://localhost:6420");

const user = await client.userActor.create(["user-123"], {
  input: { email: "alice@example.com" },
});

// Updates the actor's data and syncs it to the external database
await user.updateEmail("alice2@example.com");

const userData = await user.getUser();
