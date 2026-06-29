import { actor, setup } from "rivetkit";

interface User {
  id: string;
  email: string;
  name: string;
}

// Mock database interface for demonstration
const db = {
  users: {
    findById: async (id: string): Promise<User> => ({ id, email: "user@example.com", name: "User" }),
    update: async (id: string, data: Partial<User>) => {},
  },
};

const userSession = actor({
  state: { requestCount: 0 },

  // createVars runs on every wake (after restarts, crashes, or sleep), so
  // external data stays fresh.
  createVars: async (c): Promise<{ user: User }> => {
    // Load from database on every wake
    const user = await db.users.findById(c.key.join("-"));
    return { user };
  },

  actions: {
    getProfile: (c) => {
      c.state.requestCount++;
      return c.vars.user;
    },
    updateEmail: async (c, email: string) => {
      c.state.requestCount++;
      await db.users.update(c.key.join("-"), { email });
      // Refresh cached data
      c.vars.user = await db.users.findById(c.key.join("-"));
    },
  },
});

const registry = setup({
  use: { userSession },
});
