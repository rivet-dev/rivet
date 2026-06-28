import { actor, UserError } from "rivetkit";

interface ConnParams {
  userId?: string;
}

const userProfile = actor({
  state: {
    ownerId: "user-123",
    isPrivate: true,
  },

  onBeforeConnect: (c, params: ConnParams) => {
    // Use actor state to check access permissions
    if (c.state.isPrivate && params.userId !== c.state.ownerId) {
      throw new UserError("Access denied to private profile", { code: "forbidden" });
    }
  },

  actions: {
    getProfile: (c) => ({ ownerId: c.state.ownerId }),
  },
});
