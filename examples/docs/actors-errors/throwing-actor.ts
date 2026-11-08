import { actor, UserError } from "rivetkit";

const user = actor({
  vars: { username: "" },
  actions: {
    updateUsername: (c, username: string) => {
      // Validate username
      if (username.length > 32) {
        throw new UserError("Username is too long");
      }

      // Update username
      c.vars.username = username;
    }
  }
});
