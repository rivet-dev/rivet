import { actor, setup } from "rivetkit";

const userSession = actor({
  createVars: (c): { userId: string; loginTime: number } => ({
    userId: c.key[0], // Extract user ID from key
    loginTime: Date.now(),
  }),

  actions: {
    getUserId: (c) => c.vars.userId
  }
});

export const registry = setup({
  use: { userSession }
});
