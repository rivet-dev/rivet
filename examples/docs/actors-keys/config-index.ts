import { actor, setup } from "rivetkit";

interface UserSessionState {
  userId: string;
  loginTime: number;
  preferences: Record<string, unknown>;
}

const userSession = actor({
  createState: (c): UserSessionState => ({
    userId: c.key[0], // Extract user ID from key
    loginTime: Date.now(),
    preferences: {}
  }),

  actions: {
    getUserId: (c) => c.state.userId
  }
});

export const registry = setup({
  use: { userSession }
});
