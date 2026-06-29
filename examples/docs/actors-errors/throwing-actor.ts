import { actor, UserError } from "rivetkit";

const user = actor({
  state: { username: "" },
  actions: {
    updateUsername: (c, username: string) => {
      // Validate username
      if (username.length > 32) {
        throw new UserError("Username is too long");
      }

      // Update username
      c.state.username = username;
    }
  }
});
