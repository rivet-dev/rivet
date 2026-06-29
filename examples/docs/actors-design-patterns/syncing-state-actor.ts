import { actor, setup } from "rivetkit";

// Mock database interface for demonstration
const db = {
  users: {
    insert: async (data: { id: string; email: string; createdAt: number }) => {},
    update: async (id: string, data: { email: string; lastActive: number }) => {},
  },
};

const userActor = actor({
  state: {
    email: "",
    lastActive: 0,
  },

  onCreate: async (c, input: { email: string }) => {
    // Insert into database on actor creation
    await db.users.insert({
      id: c.key.join("-"),
      email: input.email,
      createdAt: Date.now(),
    });
  },

  onStateChange: async (c, newState) => {
    // Sync any state changes to database
    await db.users.update(c.key.join("-"), {
      email: newState.email,
      lastActive: newState.lastActive,
    });
  },

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

const registry = setup({
  use: { userActor },
});
