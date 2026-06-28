import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

interface User {
  id: string;
  email: string;
  name: string;
}

const userSession = actor({
  state: { requestCount: 0 },
  createVars: () => ({ user: null as User | null }),
  actions: {
    getProfile: (c) => c.vars.user,
    updateEmail: async (c, email: string) => {},
  },
});

const registry = setup({ use: { userSession } });
const client = createClient<typeof registry>("http://localhost:6420");

const session = client.userSession.getOrCreate(["user-123"]);

// Get profile (loaded from database on actor wake)
const profile = await session.getProfile();

// Update email (writes to database and refreshes cache)
await session.updateEmail("alice@example.com");
